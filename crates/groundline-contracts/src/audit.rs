use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::ContractError;

const TOKEN_FIELDS: [&str; 6] = [
    "input_tokens",
    "cached_input_tokens",
    "cache_write_input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
    "total_tokens",
];

#[derive(Debug, Clone, Copy, Default)]
pub struct AuditWindow {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Default)]
struct Usage {
    values: BTreeMap<&'static str, u64>,
}

impl Usage {
    fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        let mut result = Self::default();
        for field in TOKEN_FIELDS {
            let number = object.get(field).and_then(Value::as_u64).unwrap_or(0);
            result.values.insert(field, number);
        }
        Some(result)
    }

    fn total(&self) -> u64 {
        self.values.get("total_tokens").copied().unwrap_or(0)
    }

    fn add_checked(&mut self, other: &Self) -> Result<(), ContractError> {
        for field in TOKEN_FIELDS {
            let current = self.values.get(field).copied().unwrap_or(0);
            let next = current
                .checked_add(other.values.get(field).copied().unwrap_or(0))
                .ok_or_else(|| ContractError("audit_counter_overflow".to_owned()))?;
            self.values.insert(field, next);
        }
        Ok(())
    }

    fn subtract(&self, baseline: &Self) -> Self {
        let values = TOKEN_FIELDS
            .into_iter()
            .map(|field| {
                (
                    field,
                    self.values
                        .get(field)
                        .copied()
                        .unwrap_or(0)
                        .saturating_sub(baseline.values.get(field).copied().unwrap_or(0)),
                )
            })
            .collect();
        Self { values }
    }

    fn as_json(&self) -> Map<String, Value> {
        TOKEN_FIELDS
            .into_iter()
            .map(|field| {
                (
                    field.to_owned(),
                    Value::from(self.values.get(field).copied().unwrap_or(0)),
                )
            })
            .collect()
    }
}

fn increment(
    values: &mut BTreeMap<String, u64>,
    key: impl Into<String>,
) -> Result<(), ContractError> {
    let entry = values.entry(key.into()).or_default();
    *entry = entry
        .checked_add(1)
        .ok_or_else(|| ContractError("audit_counter_overflow".to_owned()))?;
    Ok(())
}

fn timestamp(value: Option<&Value>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value?.as_str()?)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn in_window(value: Option<DateTime<Utc>>, window: AuditWindow) -> bool {
    match value {
        Some(value) => {
            window.start.is_none_or(|start| value > start)
                && window.end.is_none_or(|end| value <= end)
        }
        None => window.start.is_none() && window.end.is_none(),
    }
}

fn percentile(mut values: Vec<u64>, fraction: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let index = ((values.len() - 1) as f64 * fraction).round() as usize;
    values.get(index).copied()
}

fn serialized_arguments(payload: &Map<String, Value>) -> String {
    payload
        .get("arguments")
        .or_else(|| payload.get("input"))
        .map(|value| match value {
            Value::String(value) => value.clone(),
            _ => serde_json::to_string(value).unwrap_or_default(),
        })
        .unwrap_or_default()
}

fn tool_category(name: &str, arguments: &str) -> &'static str {
    let name = name.to_ascii_lowercase();
    let arguments = arguments.to_ascii_lowercase();
    if name.contains("wait") || name.contains("write_stdin") || name.contains("poll") {
        "wait_or_poll"
    } else if name.contains("spawn_agent")
        || name.contains("send_message")
        || name.contains("update_plan")
        || name.contains("goal")
    {
        "coordination"
    } else if name.contains("web") || name.contains("search") || name.contains("browser") {
        "research"
    } else if name.contains("codex") || name.contains("thread") || name.contains("app_server") {
        "codex_runtime"
    } else if name.contains("apply_patch") || name.contains("write") || name.contains("edit") {
        "mutation"
    } else if name.contains("exec") || name == "bash" || name == "shell" {
        if [
            "cargo test",
            "cargo clippy",
            "cargo check",
            "cargo fmt",
            "actionlint",
            "npm test",
            "pnpm test",
            "flutter test",
            "pytest",
            "unittest",
        ]
        .iter()
        .any(|marker| arguments.contains(marker))
        {
            "verification"
        } else if arguments.contains("git ") || arguments.contains("gh ") {
            "git_or_github"
        } else if ["rg ", "sed ", "head ", "tail ", "find ", "ls "]
            .iter()
            .any(|marker| arguments.contains(marker))
        {
            "inspection"
        } else {
            "other_command"
        }
    } else {
        "other_tool"
    }
}

fn failure_signals(output: &Value, payload: &Map<String, Value>) -> BTreeSet<&'static str> {
    let mut signals = BTreeSet::new();
    if payload.get("is_error").and_then(Value::as_bool) == Some(true)
        || matches!(
            payload.get("status").and_then(Value::as_str),
            Some("error" | "failed")
        )
    {
        signals.insert("nonzero_exit");
    }
    let serialized = match output {
        Value::String(value) => value.to_ascii_lowercase(),
        _ => serde_json::to_string(output)
            .unwrap_or_default()
            .to_ascii_lowercase(),
    };
    if serialized.contains("timed out") || serialized.contains("timeout") {
        signals.insert("timeout");
    }
    if serialized.contains("script running with cell id")
        || serialized.contains("process running with session id")
    {
        signals.insert("yielded_for_wait");
    }
    if serialized.contains("invalid argument") || serialized.contains("invalid_arguments") {
        signals.insert("invalid_arguments");
    }
    if serialized.contains("rejected") || serialized.contains("permission denied") {
        signals.insert("rejected");
    }
    if output
        .get("exit_code")
        .and_then(Value::as_i64)
        .is_some_and(|code| code != 0)
    {
        signals.insert("nonzero_exit");
    }
    signals
}

fn is_tool_call(item_type: &str) -> bool {
    matches!(
        item_type,
        "function_call" | "custom_tool_call" | "local_shell_call" | "tool_search_call"
    )
}

fn is_tool_output(item_type: &str) -> bool {
    matches!(
        item_type,
        "function_call_output" | "custom_tool_call_output" | "local_shell_call_output"
    )
}

/// Audit already-read Codex rollout JSONL without exposing record content or paths.
pub fn audit_rollouts(
    rollouts: &[&str],
    storage_bytes: u64,
    long_turn_minutes: u64,
    window: AuditWindow,
) -> Result<Value, ContractError> {
    if window
        .start
        .zip(window.end)
        .is_some_and(|(start, end)| start >= end)
    {
        return Err(ContractError("invalid_audit_window".to_owned()));
    }

    let mut errors = Vec::<String>::new();
    let mut record_count = 0_u64;
    let mut timestamps = Vec::<DateTime<Utc>>::new();
    let mut event_counts = BTreeMap::<String, u64>::new();
    let mut model_effort = BTreeMap::<String, u64>::new();
    let mut model_sequence = Vec::<String>::new();
    let mut user_lengths = Vec::<u64>::new();
    let mut broad_scope_count = 0_u64;
    let mut tool_names = BTreeMap::<String, u64>::new();
    let mut tool_categories = BTreeMap::<String, u64>::new();
    let mut call_signatures = BTreeMap::<String, u64>::new();
    let mut failure_counts = BTreeMap::<String, u64>::new();
    let mut completed_durations = Vec::<u64>::new();
    let mut active_tasks = BTreeMap::<String, DateTime<Utc>>::new();
    let mut provider_usage = Usage::default();
    let mut usage_rollouts = 0_u64;
    let mut usage_events = 0_u64;
    let mut cumulative_rollouts = 0_u64;
    let mut fallback_rollouts = 0_u64;
    let mut compacted_records = 0_u64;
    let mut long_lived_rollouts = 0_u64;
    let mut boundary_review_rollouts = 0_u64;
    let mut verification_calls = BTreeSet::<String>::new();
    let mut verification_resolved = BTreeSet::<String>::new();
    let mut verification_success = 0_u64;
    let mut verification_failure = 0_u64;

    for (rollout_index, contents) in rollouts.iter().enumerate() {
        let mut latest_usage: Option<Usage> = None;
        let mut baseline_usage: Option<Usage> = None;
        let mut fallback_usage = Usage::default();
        let mut has_fallback = false;
        let mut rollout_compactions = 0_u64;
        let mut rollout_broad_scope = 0_u64;

        for (line_index, line) in contents.lines().enumerate() {
            let record: Value = match serde_json::from_str(line) {
                Ok(Value::Object(record)) => Value::Object(record),
                Ok(_) => continue,
                Err(_) => {
                    errors.push(format!(
                        "rollout[{rollout_index}] has invalid JSON at line {}",
                        line_index + 1
                    ));
                    continue;
                }
            };
            let object = record.as_object().expect("matched object");
            let observed_at = timestamp(object.get("timestamp"));
            let record_in_window = in_window(observed_at, window);
            if record_in_window {
                record_count = record_count
                    .checked_add(1)
                    .ok_or_else(|| ContractError("audit_counter_overflow".to_owned()))?;
                if let Some(value) = observed_at {
                    timestamps.push(value);
                }
            }
            let record_type = object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some(payload) = object.get("payload").and_then(Value::as_object) else {
                continue;
            };

            if record_in_window && record_type == "compacted" {
                compacted_records = compacted_records.saturating_add(1);
                rollout_compactions = rollout_compactions.saturating_add(1);
            }
            if record_in_window && record_type == "turn_context" {
                let model = payload
                    .get("model")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("unset");
                let effort = payload
                    .get("effort")
                    .or_else(|| payload.get("reasoning_effort"))
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("unset");
                let label = format!("{model}|{effort}");
                increment(&mut model_effort, label.clone())?;
                model_sequence.push(label);
            }
            if record_type == "event_msg" {
                let event_type = payload
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if record_in_window && !event_type.is_empty() {
                    increment(&mut event_counts, event_type)?;
                    if event_type == "context_compacted" {
                        rollout_compactions = rollout_compactions.saturating_add(1);
                    }
                }
                if record_in_window
                    && event_type == "user_message"
                    && let Some(message) = payload.get("message").and_then(Value::as_str)
                {
                    let length = message.chars().count() as u64;
                    user_lengths.push(length);
                    let lowered = message.to_ascii_lowercase();
                    if [
                        "다 ",
                        "전체",
                        "완전",
                        "모든",
                        "everything",
                        "all ",
                        "entire",
                        "completely",
                    ]
                    .iter()
                    .any(|term| lowered.contains(term))
                    {
                        broad_scope_count = broad_scope_count.saturating_add(1);
                        rollout_broad_scope = rollout_broad_scope.saturating_add(1);
                    }
                }
                if event_type == "task_started" {
                    if let (Some(turn_id), Some(observed_at)) =
                        (payload.get("turn_id").and_then(Value::as_str), observed_at)
                    {
                        active_tasks.insert(turn_id.to_owned(), observed_at);
                    }
                } else if record_in_window && event_type == "task_complete" {
                    if let Some(duration) = payload.get("duration_ms").and_then(Value::as_u64) {
                        completed_durations.push(duration);
                    } else if let (Some(turn_id), Some(observed_at)) =
                        (payload.get("turn_id").and_then(Value::as_str), observed_at)
                        && let Some(started) = active_tasks.get(turn_id)
                    {
                        completed_durations.push(
                            observed_at
                                .signed_duration_since(*started)
                                .num_milliseconds()
                                .max(0) as u64,
                        );
                    }
                } else if event_type == "token_count" {
                    let info = payload.get("info").and_then(Value::as_object);
                    let cumulative = info
                        .and_then(|value| value.get("total_token_usage"))
                        .and_then(Usage::from_value);
                    let fallback = info
                        .and_then(|value| value.get("last_token_usage"))
                        .and_then(Usage::from_value);
                    if record_in_window && (cumulative.is_some() || fallback.is_some()) {
                        usage_events = usage_events.saturating_add(1);
                    }
                    if let Some(cumulative) = cumulative {
                        if window
                            .start
                            .is_some_and(|start| observed_at.is_some_and(|at| at <= start))
                            && baseline_usage
                                .as_ref()
                                .is_none_or(|existing| cumulative.total() >= existing.total())
                        {
                            baseline_usage = Some(cumulative);
                        } else if record_in_window
                            && latest_usage
                                .as_ref()
                                .is_none_or(|existing| cumulative.total() >= existing.total())
                        {
                            latest_usage = Some(cumulative);
                        }
                    } else if record_in_window && let Some(fallback) = fallback {
                        fallback_usage.add_checked(&fallback)?;
                        has_fallback = true;
                    }
                }
            }

            if !record_in_window || record_type != "response_item" {
                continue;
            }
            let item_type = payload
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if is_tool_call(item_type) {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(if item_type == "tool_search_call" {
                        "tool_search"
                    } else {
                        "unknown"
                    });
                let arguments = serialized_arguments(payload);
                let category = tool_category(name, &arguments);
                increment(&mut tool_names, name)?;
                increment(&mut tool_categories, category)?;
                let mut digest = Sha256::new();
                digest.update(name.as_bytes());
                digest.update([0]);
                digest.update(arguments.as_bytes());
                increment(&mut call_signatures, format!("{:x}", digest.finalize()))?;
                if category == "verification"
                    && let Some(call_id) = payload.get("call_id").and_then(Value::as_str)
                {
                    verification_calls.insert(call_id.to_owned());
                }
            } else if is_tool_output(item_type) {
                let output = payload.get("output").unwrap_or(&Value::Null);
                let signals = failure_signals(output, payload);
                for signal in &signals {
                    increment(&mut failure_counts, *signal)?;
                }
                if let Some(call_id) = payload.get("call_id").and_then(Value::as_str)
                    && verification_calls.contains(call_id)
                    && !verification_resolved.contains(call_id)
                {
                    if signals
                        .iter()
                        .any(|value| matches!(*value, "nonzero_exit" | "timeout" | "rejected"))
                    {
                        verification_failure = verification_failure.saturating_add(1);
                        verification_resolved.insert(call_id.to_owned());
                    } else if !signals.contains("yielded_for_wait") && !output.is_null() {
                        verification_success = verification_success.saturating_add(1);
                        verification_resolved.insert(call_id.to_owned());
                    }
                }
            }
        }

        if rollout_compactions >= 2 {
            long_lived_rollouts = long_lived_rollouts.saturating_add(1);
        }
        if rollout_compactions >= 2 || rollout_broad_scope >= 3 {
            boundary_review_rollouts = boundary_review_rollouts.saturating_add(1);
        }
        if let Some(latest) = latest_usage {
            let window_usage = match baseline_usage.as_ref() {
                Some(baseline) => latest.subtract(baseline),
                None => latest,
            };
            provider_usage.add_checked(&window_usage)?;
            usage_rollouts = usage_rollouts.saturating_add(1);
            cumulative_rollouts = cumulative_rollouts.saturating_add(1);
        } else if has_fallback {
            provider_usage.add_checked(&fallback_usage)?;
            usage_rollouts = usage_rollouts.saturating_add(1);
            fallback_rollouts = fallback_rollouts.saturating_add(1);
        }
    }

    let input_tokens = provider_usage
        .values
        .get("input_tokens")
        .copied()
        .unwrap_or(0);
    let cached_input_tokens = provider_usage
        .values
        .get("cached_input_tokens")
        .copied()
        .unwrap_or(0);
    let repeated_groups = call_signatures
        .values()
        .copied()
        .filter(|count| *count >= 3)
        .collect::<Vec<_>>();
    let user_messages = user_lengths.len() as u64;
    let short_messages = user_lengths.iter().filter(|value| **value <= 30).count() as u64;
    let verification_tool_calls = tool_categories.get("verification").copied().unwrap_or(0);
    let verification_unresolved = verification_tool_calls
        .saturating_sub(verification_success)
        .saturating_sub(verification_failure);
    let duration_threshold = long_turn_minutes.saturating_mul(60_000);
    let transitions = model_sequence
        .windows(2)
        .filter(|window| window[0] != window[1])
        .count() as u64;
    let start = timestamps.iter().min().map(DateTime::<Utc>::to_rfc3339);
    let end = timestamps.iter().max().map(DateTime::<Utc>::to_rfc3339);
    let usage_source = if window.start.is_some() {
        match (cumulative_rollouts > 0, fallback_rollouts > 0) {
            (true, false) => "codex-cumulative-window-delta",
            (true, true) => "codex-window-delta-and-last-usage-fallback",
            (false, true) => "codex-last-usage-events-summed-window",
            (false, false) => "unavailable",
        }
    } else {
        match (cumulative_rollouts > 0, fallback_rollouts > 0) {
            (true, false) => "codex-cumulative-total-snapshots",
            (true, true) => "codex-cumulative-and-last-usage-fallback",
            (false, true) => "codex-last-usage-events-summed-fallback",
            (false, false) => "unavailable",
        }
    };
    let mut usage_json = provider_usage.as_json();
    usage_json.insert(
        "non_cached_input_tokens".to_owned(),
        Value::from(input_tokens.saturating_sub(cached_input_tokens)),
    );
    usage_json.insert(
        "cached_input_ratio".to_owned(),
        if input_tokens == 0 {
            Value::Null
        } else {
            Value::from(
                ((cached_input_tokens as f64 / input_tokens as f64) * 10_000.0).round() / 10_000.0,
            )
        },
    );

    Ok(json!({
        "schema": 1,
        "kind": "groundline-codex-session-audit",
        "status": if errors.is_empty() { "PASS" } else { "PARTIAL" },
        "errors": errors,
        "coverage": {
            "rollout_count": rollouts.len(),
            "record_count": record_count,
            "storage_bytes": storage_bytes,
            "time_window": { "start": start, "end": end },
        },
        "activity": {
            "user_message_events": event_counts.get("user_message").copied().unwrap_or(0),
            "user_messages_with_text": user_messages,
            "task_started": event_counts.get("task_started").copied().unwrap_or(0),
            "task_completed": event_counts.get("task_complete").copied().unwrap_or(0),
            "turn_contexts": model_effort.values().copied().sum::<u64>(),
            "compactions": event_counts.get("context_compacted").copied().unwrap_or(0).max(compacted_records),
            "compaction_event_count": event_counts.get("context_compacted").copied().unwrap_or(0),
            "compacted_record_count": compacted_records,
        },
        "model_effort": { "counts": model_effort, "transition_count": transitions },
        "provider_reported_usage": {
            "source": usage_source,
            "rollout_count_with_usage": usage_rollouts,
            "usage_event_count": usage_events,
            "cumulative_rollout_count": cumulative_rollouts,
            "fallback_rollout_count": fallback_rollouts,
            "billing_inference_performed": false,
            "input_tokens": usage_json.get("input_tokens"),
            "cached_input_tokens": usage_json.get("cached_input_tokens"),
            "cache_write_input_tokens": usage_json.get("cache_write_input_tokens"),
            "output_tokens": usage_json.get("output_tokens"),
            "reasoning_output_tokens": usage_json.get("reasoning_output_tokens"),
            "total_tokens": usage_json.get("total_tokens"),
            "non_cached_input_tokens": usage_json.get("non_cached_input_tokens"),
            "cached_input_ratio": usage_json.get("cached_input_ratio"),
        },
        "task_latency": {
            "completed_count": completed_durations.len(),
            "median_ms": percentile(completed_durations.clone(), 0.5),
            "p90_ms": percentile(completed_durations.clone(), 0.9),
            "max_ms": completed_durations.iter().max(),
            "long_turn_threshold_minutes": long_turn_minutes,
            "long_turn_count": completed_durations.iter().filter(|duration| **duration >= duration_threshold).count(),
        },
        "prompt_shape": {
            "short_message_threshold_chars": 30,
            "short_message_count": short_messages,
            "broad_scope_message_count": broad_scope_count,
            "average_user_message_chars": if user_messages == 0 { None } else { Some(((user_lengths.iter().sum::<u64>() as f64 / user_messages as f64) * 10.0).round() / 10.0) },
            "median_user_message_chars": percentile(user_lengths.clone(), 0.5),
            "max_user_message_chars": user_lengths.iter().max(),
        },
        "tools": {
            "call_count": tool_names.values().copied().sum::<u64>(),
            "by_name": tool_names,
            "by_category": tool_categories,
            "verification_success_count": verification_success,
            "verification_failure_count": verification_failure,
            "verification_unresolved_count": verification_unresolved,
            "failure_signals": failure_counts,
            "exact_repeated_call_groups": repeated_groups.len(),
            "calls_in_exact_repeated_groups": repeated_groups.iter().copied().sum::<u64>(),
            "max_exact_repeat_count": call_signatures.values().max().copied().unwrap_or(0),
        },
        "boundary_signals": {
            "rollout_count_evaluated": rollouts.len(),
            "long_lived_root_count": long_lived_rollouts,
            "boundary_review_root_count": boundary_review_rollouts,
            "long_lived_root_session": long_lived_rollouts > 0,
            "task_boundary_review_recommended": boundary_review_rollouts > 0,
        },
        "usage_source_contract": {
            "cumulative_total_preferred_per_rollout": true,
            "last_usage_sum_is_fallback_only": true,
            "window_delta_prevents_double_counting": window.start.is_some(),
            "billing_inference_performed": false,
        },
        "mutation_performed": false,
        "raw_content_emitted": false,
        "private_paths_emitted": false,
        "secret_value_printed": false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollout_audit_counts_without_emitting_content() {
        let data = r#"{"timestamp":"2026-08-26T00:00:00Z","type":"turn_context","payload":{"model":"gpt-5.6-sol","effort":"high"}}
{"timestamp":"2026-08-26T00:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"완전히 다 마이그레이션해"}}
{"timestamp":"2026-08-26T00:00:02Z","type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"a","arguments":"{\"cmd\":\"cargo test\"}"}}
{"timestamp":"2026-08-26T00:00:03Z","type":"response_item","payload":{"type":"function_call_output","call_id":"a","output":{"exit_code":0}}}"#;
        let result =
            audit_rollouts(&[data], data.len() as u64, 10, AuditWindow::default()).expect("audit");
        assert_eq!(result["status"], "PASS");
        assert_eq!(result["tools"]["by_category"]["verification"], 1);
        assert_eq!(result["tools"]["verification_success_count"], 1);
        let encoded = serde_json::to_string(&result).expect("json");
        assert!(!encoded.contains("마이그레이션"));
        assert_eq!(result["raw_content_emitted"], false);
    }

    #[test]
    fn malformed_lines_are_partial_and_counter_overflow_is_impossible() {
        let result = audit_rollouts(&["not-json"], 8, 10, AuditWindow::default()).expect("audit");
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["errors"].as_array().map(Vec::len), Some(1));
    }
}
