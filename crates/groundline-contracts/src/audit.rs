use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::ContractError;
use crate::rollout::Record;

const TOKEN_FIELDS: [&str; 6] = [
    "input_tokens",
    "cached_input_tokens",
    "cache_write_input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
    "total_tokens",
];

const MAX_ERROR_EXAMPLES: usize = 32;
const MAX_AUDIT_RECORDS: usize = 1_000_000;
const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;

#[derive(Default)]
struct Diagnostics {
    examples: Vec<String>,
    count: u64,
}

impl Diagnostics {
    fn push(&mut self, message: String) {
        self.count = self.count.saturating_add(1);
        if self.examples.len() < MAX_ERROR_EXAMPLES {
            self.examples.push(message);
        }
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AuditWindow {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Clone)]
struct Usage {
    values: BTreeMap<&'static str, u64>,
}

impl Usage {
    fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        if !TOKEN_FIELDS
            .iter()
            .any(|field| object.get(*field).is_some_and(Value::is_u64))
        {
            return None;
        }
        let mut result = Self::default();
        for field in TOKEN_FIELDS {
            let number = object.get(field).and_then(Value::as_u64).unwrap_or(0);
            result.values.insert(field, number);
        }
        Some(result)
    }

    fn covers(&self, earlier: &Self) -> bool {
        TOKEN_FIELDS.iter().all(|field| {
            self.values.get(field).copied().unwrap_or(0)
                >= earlier.values.get(field).copied().unwrap_or(0)
        })
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

fn increment<K: Ord>(
    values: &mut BTreeMap<K, u64>,
    key: impl Into<K>,
) -> Result<(), ContractError> {
    let entry = values.entry(key.into()).or_default();
    *entry = entry
        .checked_add(1)
        .ok_or_else(|| ContractError("audit_counter_overflow".to_owned()))?;
    Ok(())
}

fn timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value?)
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

fn percentile(values: &[u64], fraction: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
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

fn projected_signals(signals: &BTreeSet<&'static str>) -> Value {
    json!({
        "exit_code": if signals.contains("nonzero_exit") {1} else {0},
        "markers": signals.iter().filter_map(|signal| match *signal {
            "timeout" => Some("timed out"),
            "yielded_for_wait" => Some("script running with cell id"),
            "invalid_arguments" => Some("invalid argument"),
            "rejected" => Some("permission denied"),
            _ => None,
        }).collect::<Vec<_>>()
    })
}

/// Scan large native tool results without allocating their image/content trees.
/// Only fixed outcome markers survive; raw and retained byte limits are separate.
pub(crate) fn project_raw_tool_output(
    raw: &serde_json::value::RawValue,
    payload: &Map<String, Value>,
) -> serde_json::Result<Value> {
    use serde::Deserializer;
    use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};

    if raw.get().len() > 64 * 1024 * 1024 {
        return Err(serde::de::Error::custom("tool output byte limit exceeded"));
    }
    if raw.get().trim() == "null" {
        return Ok(Value::Null);
    }
    struct Scan<'a> {
        signals: &'a mut BTreeSet<&'static str>,
        root: bool,
    }
    impl Scan<'_> {
        fn text(&mut self, value: &str) {
            self.signals.extend(failure_signals(
                &Value::String(value.to_owned()),
                &Map::new(),
            ));
        }
    }
    impl<'de> DeserializeSeed<'de> for Scan<'_> {
        type Value = ();
        fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<(), D::Error> {
            de.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Scan<'_> {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a native tool result")
        }
        fn visit_str<E: serde::de::Error>(mut self, value: &str) -> Result<(), E> {
            self.text(value);
            Ok(())
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
            Ok(())
        }
        fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
            Ok(())
        }
        fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
            Ok(())
        }
        fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
            Ok(())
        }
        fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
            Ok(())
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
            while seq
                .next_element_seed(Scan {
                    signals: self.signals,
                    root: false,
                })?
                .is_some()
            {}
            Ok(())
        }
        fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<(), A::Error> {
            let mut nonzero_exit = false;
            while let Some(key) = map.next_key::<String>()? {
                self.text(&key);
                if self.root && key == "exit_code" {
                    let value = map.next_value::<&serde_json::value::RawValue>()?;
                    nonzero_exit = serde_json::from_str::<i64>(value.get()).is_ok_and(|v| v != 0);
                    Scan {
                        signals: self.signals,
                        root: false,
                    }
                    .deserialize(value)
                    .map_err(serde::de::Error::custom)?;
                } else {
                    map.next_value_seed(Scan {
                        signals: self.signals,
                        root: false,
                    })?;
                }
            }
            if nonzero_exit {
                self.signals.insert("nonzero_exit");
            }
            Ok(())
        }
    }
    let mut signals = failure_signals(&Value::Null, payload);
    Scan {
        signals: &mut signals,
        root: true,
    }
    .deserialize(raw)?;
    Ok(projected_signals(&signals))
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

    let mut errors = Diagnostics::default();
    let mut processed_records = 0_usize;
    let mut known_scope_exclusions = 0_u64;
    let mut record_count = 0_u64;
    let mut first_timestamp: Option<DateTime<Utc>> = None;
    let mut last_timestamp: Option<DateTime<Utc>> = None;
    let mut event_counts = BTreeMap::<String, u64>::new();
    let mut model_effort = BTreeMap::<String, u64>::new();
    let mut transitions = 0_u64;
    let mut user_lengths = Vec::<u64>::new();
    let mut broad_scope_count = 0_u64;
    let mut tool_names = BTreeMap::<String, u64>::new();
    let mut tool_categories = BTreeMap::<String, u64>::new();
    let mut call_signatures = BTreeMap::<[u8; 32], u64>::new();
    let mut failure_counts = BTreeMap::<String, u64>::new();
    let mut completed_durations = Vec::<u64>::new();
    let mut provider_usage = Usage::default();
    let mut usage_rollouts = 0_u64;
    let mut usage_events = 0_u64;
    let mut cumulative_rollouts = 0_u64;
    let mut fallback_rollouts = 0_u64;
    let mut response_rollouts = 0_u64;
    let mut compacted_records = 0_u64;
    let mut compactions = 0_u64;
    let mut long_lived_rollouts = 0_u64;
    let mut boundary_review_rollouts = 0_u64;
    let mut verification_success = 0_u64;
    let mut verification_failure = 0_u64;

    for (rollout_index, contents) in rollouts.iter().enumerate() {
        let metadata = contents
            .lines()
            .take(1024)
            .filter(|line| line.len() <= MAX_RECORD_BYTES)
            .find_map(|line| {
                let record = Record::parse(line).ok()??;
                if record.string("type").as_deref() != Some("session_meta") {
                    return None;
                }
                record.value("payload").ok().flatten()
            })
            .unwrap_or(Value::Null);
        let inherited = [
            "history_base",
            "forked_from_id",
            "subagent_history_start_ordinal",
        ]
        .iter()
        .any(|field| metadata.get(*field).is_some_and(|value| !value.is_null()));
        let paginated = metadata.get("history_mode").and_then(Value::as_str) == Some("paginated");
        let history_base = metadata
            .pointer("/history_base/end_ordinal_exclusive")
            .and_then(Value::as_u64);
        let fork_boundary = metadata
            .get("forked_from_ordinal_exclusive")
            .and_then(Value::as_u64);
        let child_boundary = metadata
            .get("subagent_history_start_ordinal")
            .and_then(Value::as_u64);
        let boundary = [history_base, fork_boundary, child_boundary]
            .into_iter()
            .flatten()
            .max();
        // Never traverse metadata paths or sum inherited cumulative snapshots.
        // Native ordinals identify the readable, locally owned suffix. Legacy
        // copied history without an ownership boundary stays explicitly partial.
        let known_boundary = paginated
            && boundary.is_some()
            && (metadata.get("history_base").is_none_or(Value::is_null) || history_base.is_some())
            && (metadata.get("forked_from_id").is_none_or(Value::is_null)
                || fork_boundary.is_some()
                || child_boundary.is_some());
        if inherited && !known_boundary {
            errors.push(format!(
                "rollout[{rollout_index}] inherited history attribution unavailable"
            ));
            continue;
        }
        if inherited {
            known_scope_exclusions += 1;
            errors.push(format!(
                "rollout[{rollout_index}] shared history prefix not read; local suffix only"
            ));
        }
        let owner = metadata
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty());
        let mut response_usage = Usage::default();
        let mut response_ids = BTreeSet::<String>::new();
        let mut response_events = 0_u64;
        let mut last_ordinal = None;
        let mut previous_model = None;
        let mut active_tasks = BTreeMap::<String, DateTime<Utc>>::new();
        let mut verification_calls = BTreeSet::<String>::new();
        let mut verification_resolved = BTreeSet::<String>::new();
        let mut latest_usage: Option<Usage> = None;
        let mut baseline_usage: Option<Usage> = None;
        let mut checkpoint_usage: Option<Usage> = None;
        let mut native_checkpoint: Option<Usage> = None;
        let mut native_latest: Option<Usage> = None;
        let mut native_baseline: Option<Usage> = None;
        let mut native_anchor_matches = false;
        let mut native_zero_anchor = false;
        let mut ui_usage_errors = 0_u64;
        let mut uncovered_response = false;
        let mut native_uncovered_response = false;
        let mut fallback_usage = Usage::default();
        let mut has_fallback = false;
        let mut rollout_compactions = 0_u64;
        let mut rollout_compaction_events = 0_u64;
        let mut rollout_broad_scope = 0_u64;

        for (line_index, line) in contents.lines().enumerate() {
            if processed_records >= MAX_AUDIT_RECORDS {
                errors.push("audit record budget exhausted".to_owned());
                break;
            }
            processed_records += 1;
            if line.len() > MAX_RECORD_BYTES {
                errors.push(format!(
                    "rollout[{rollout_index}] record byte limit exceeded"
                ));
                continue;
            }
            let record = match Record::parse(line) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(_) => {
                    errors.push(format!(
                        "rollout[{rollout_index}] has invalid JSON at line {}",
                        line_index + 1
                    ));
                    continue;
                }
            };
            if paginated {
                let Some(ordinal) = record.unsigned("ordinal") else {
                    errors.push(format!("rollout[{rollout_index}] missing history ordinal"));
                    continue;
                };
                // Native settings notifications may share the preceding history
                // ordinal. They contain no usage and must not relax ordering for
                // token records or any other event.
                let settings_notification = record.string("type").as_deref() == Some("event_msg")
                    && record.payload_type().as_deref() == Some("thread_settings_applied");
                if last_ordinal.is_some_and(|previous| {
                    ordinal < previous || (ordinal == previous && !settings_notification)
                }) {
                    errors.push(format!(
                        "rollout[{rollout_index}] non-increasing history ordinal"
                    ));
                    continue;
                }
                last_ordinal = Some(ordinal);
                if boundary.is_some_and(|boundary| ordinal < boundary) {
                    continue;
                }
            }
            let observed_at = timestamp(record.string("timestamp").as_deref());
            let record_in_window = in_window(observed_at, window);
            if record_in_window {
                record_count = record_count
                    .checked_add(1)
                    .ok_or_else(|| ContractError("audit_counter_overflow".to_owned()))?;
                if let Some(value) = observed_at {
                    first_timestamp = Some(first_timestamp.map_or(value, |at| at.min(value)));
                    last_timestamp = Some(last_timestamp.map_or(value, |at| at.max(value)));
                }
            }
            let kind = record.string("type");
            let record_type = kind.as_deref().unwrap_or_default();
            if observed_at.is_none()
                && (window.start.is_some() || window.end.is_some())
                && matches!(
                    record_type,
                    "compacted"
                        | "turn_context"
                        | "event_msg"
                        | "response_item"
                        | "token_usage_record"
                )
            {
                errors.push(format!(
                    "rollout[{rollout_index}] metric timestamp unavailable"
                ));
            }
            // Unknown/context-storage records still count toward coverage and
            // syntax validation, but their potentially large bodies are unused.
            if !matches!(
                record_type,
                "compacted" | "turn_context" | "event_msg" | "response_item" | "token_usage_record"
            ) {
                continue;
            }
            let payload = match record.value("payload") {
                Ok(Some(Value::Object(payload))) => payload,
                Ok(_) => continue,
                Err(_) => {
                    errors.push(format!(
                        "rollout[{rollout_index}] has invalid JSON at line {}",
                        line_index + 1
                    ));
                    continue;
                }
            };
            if record_type == "token_usage_record"
                && owner.is_some()
                && payload.get("thread_id").and_then(Value::as_str) == owner
                && let Some(response_id) = payload
                    .get("response_id")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                && let Some(usage) = payload.get("usage").and_then(Usage::from_value)
                && response_ids.insert(response_id.to_owned())
            {
                if record_in_window {
                    response_usage.add_checked(&usage)?;
                    response_events = response_events.saturating_add(1);
                    uncovered_response = true;
                    native_uncovered_response = true;
                }
                // Native thread totals are safe only for an unshared owner.
                // Shared/forked logs continue to use owned response deltas.
                if !inherited
                    && let Some(total) = payload
                        .get("thread_token_usage")
                        .and_then(Usage::from_value)
                    && (record_in_window
                        || window
                            .start
                            .is_some_and(|start| observed_at.is_some_and(|at| at <= start)))
                {
                    if record_in_window
                        && (!total.covers(&usage)
                            || native_checkpoint
                                .as_ref()
                                .is_some_and(|old| !total.covers(old)))
                    {
                        errors.push(format!(
                            "rollout[{rollout_index}] inconsistent cumulative usage"
                        ));
                    } else {
                        if native_checkpoint.is_none() {
                            native_anchor_matches = checkpoint_usage
                                .as_ref()
                                .is_some_and(|old| total.subtract(&usage).values == old.values);
                            // A resumed native stream can start its own counter
                            // at zero. Its first owned response proves that
                            // anchor only when nothing earlier in this window
                            // has usage that would otherwise be discarded.
                            native_zero_anchor = record_in_window
                                && total.values == usage.values
                                && response_events == 1
                                && latest_usage.is_none()
                                && !has_fallback;
                        }
                        native_checkpoint = Some(total.clone());
                        if record_in_window {
                            native_latest = Some(total);
                        } else {
                            native_baseline = Some(total);
                        }
                        uncovered_response = false;
                        native_uncovered_response = false;
                    }
                }
            }

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
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .or_else(|| payload.get("reasoning_effort").and_then(Value::as_str))
                    .unwrap_or("unset");
                let label = format!(
                    "{}|{}",
                    crate::model::family(model),
                    crate::model::effort(effort)
                );
                increment(&mut model_effort, label.clone())?;
                if previous_model
                    .as_ref()
                    .is_some_and(|previous| previous != &label)
                {
                    transitions = transitions.saturating_add(1);
                }
                previous_model = Some(label);
            }
            if record_type == "event_msg" {
                let event_type = payload
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if record_in_window && !event_type.is_empty() {
                    increment(&mut event_counts, event_type)?;
                    if event_type == "context_compacted" {
                        rollout_compaction_events = rollout_compaction_events.saturating_add(1);
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
                } else if event_type == "token_count" && !inherited {
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
                        if record_in_window
                            || window
                                .start
                                .is_some_and(|start| observed_at.is_some_and(|at| at <= start))
                        {
                            if record_in_window
                                && checkpoint_usage
                                    .as_ref()
                                    .is_some_and(|old| !cumulative.covers(old))
                            {
                                ui_usage_errors = ui_usage_errors.saturating_add(1);
                            } else {
                                checkpoint_usage = Some(cumulative.clone());
                                if record_in_window {
                                    latest_usage = Some(cumulative);
                                } else {
                                    baseline_usage = Some(cumulative);
                                }
                                uncovered_response = false;
                            }
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
                let arguments = serialized_arguments(&payload);
                let category = tool_category(name, &arguments);
                increment(&mut tool_names, name)?;
                increment(&mut tool_categories, category)?;
                let mut digest = Sha256::new();
                digest.update(name.as_bytes());
                digest.update([0]);
                digest.update(arguments.as_bytes());
                increment(&mut call_signatures, <[u8; 32]>::from(digest.finalize()))?;
                if category == "verification"
                    && let Some(call_id) = payload.get("call_id").and_then(Value::as_str)
                {
                    verification_calls.insert(call_id.to_owned());
                }
            } else if is_tool_output(item_type) {
                let output = payload.get("output").unwrap_or(&Value::Null);
                let signals = failure_signals(output, &payload);
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

        let rollout_compactions = rollout_compactions.max(rollout_compaction_events);
        compactions = compactions.saturating_add(rollout_compactions);
        if rollout_compactions >= 2 {
            long_lived_rollouts = long_lived_rollouts.saturating_add(1);
        }
        if rollout_compactions >= 2 || rollout_broad_scope >= 3 {
            boundary_review_rollouts = boundary_review_rollouts.saturating_add(1);
        }
        // UI token-count notifications and persisted native response totals can
        // have different baselines. Validate each stream independently; never
        // interpret their alternating records as a counter reset.
        if native_latest.is_some() {
            latest_usage = native_latest;
            uncovered_response = native_uncovered_response;
            baseline_usage = if native_baseline.is_some() {
                native_baseline
            } else if native_zero_anchor {
                Some(Usage::default())
            } else if native_anchor_matches {
                baseline_usage
            } else {
                if baseline_usage.is_some() {
                    errors.push(format!(
                        "rollout[{rollout_index}] native cumulative baseline unavailable"
                    ));
                }
                None
            };
        } else {
            for _ in 0..ui_usage_errors {
                errors.push(format!(
                    "rollout[{rollout_index}] inconsistent cumulative usage"
                ));
            }
        }
        if let Some(latest) = latest_usage {
            if uncovered_response {
                errors.push(format!("rollout[{rollout_index}] response usage after last cumulative checkpoint is unverified"));
            }
            let window_usage = match baseline_usage.as_ref() {
                Some(baseline) => latest.subtract(baseline),
                None => latest,
            };
            provider_usage.add_checked(&window_usage)?;
            usage_rollouts = usage_rollouts.saturating_add(1);
            cumulative_rollouts = cumulative_rollouts.saturating_add(1);
        } else if response_events > 0 {
            provider_usage.add_checked(&response_usage)?;
            usage_rollouts = usage_rollouts.saturating_add(1);
            fallback_rollouts = fallback_rollouts.saturating_add(1);
            response_rollouts = response_rollouts.saturating_add(1);
            usage_events = usage_events.saturating_add(response_events);
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
    let (repeated_groups, repeated_calls, max_repeat) =
        call_signatures
            .values()
            .fold((0_u64, 0_u64, 0_u64), |(groups, calls, maximum), &count| {
                (
                    groups + u64::from(count >= 3),
                    calls + if count >= 3 { count } else { 0 },
                    maximum.max(count),
                )
            });
    let user_messages = user_lengths.len() as u64;
    let short_messages = user_lengths.iter().filter(|value| **value <= 30).count() as u64;
    let verification_tool_calls = tool_categories.get("verification").copied().unwrap_or(0);
    let verification_unresolved = verification_tool_calls
        .saturating_sub(verification_success)
        .saturating_sub(verification_failure);
    let duration_threshold = long_turn_minutes.saturating_mul(60_000);
    let start = first_timestamp.as_ref().map(DateTime::<Utc>::to_rfc3339);
    let end = last_timestamp.as_ref().map(DateTime::<Utc>::to_rfc3339);
    completed_durations.sort_unstable();
    user_lengths.sort_unstable();
    let usage_source = if response_rollouts > 0 {
        if response_rollouts == usage_rollouts {
            "codex-response-usage-records"
        } else {
            "codex-mixed-usage-sources"
        }
    } else if window.start.is_some() {
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
        "collection_complete": errors.count == known_scope_exclusions,
        "error_count": errors.count,
        "error_examples_omitted": errors.count.saturating_sub(errors.examples.len() as u64),
        "errors": errors.examples,
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
            "compactions": compactions,
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
            "median_ms": percentile(&completed_durations, 0.5),
            "p90_ms": percentile(&completed_durations, 0.9),
            "max_ms": completed_durations.last(),
            "long_turn_threshold_minutes": long_turn_minutes,
            "long_turn_count": completed_durations.iter().filter(|duration| **duration >= duration_threshold).count(),
        },
        "prompt_shape": {
            "short_message_threshold_chars": 30,
            "short_message_count": short_messages,
            "broad_scope_message_count": broad_scope_count,
            "average_user_message_chars": if user_messages == 0 { None } else { Some(((user_lengths.iter().sum::<u64>() as f64 / user_messages as f64) * 10.0).round() / 10.0) },
            "median_user_message_chars": percentile(&user_lengths, 0.5),
            "max_user_message_chars": user_lengths.last(),
        },
        "tools": {
            "call_count": tool_names.values().copied().sum::<u64>(),
            "by_name": tool_names,
            "by_category": tool_categories,
            "verification_success_count": verification_success,
            "verification_failure_count": verification_failure,
            "verification_unresolved_count": verification_unresolved,
            "failure_signals": failure_counts,
            "exact_repeated_call_groups": repeated_groups,
            "calls_in_exact_repeated_groups": repeated_calls,
            "max_exact_repeat_count": max_repeat,
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
            "response_usage_requires_matching_thread_and_unique_response": true,
            "inherited_cumulative_usage_excluded": true,
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

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap()
    }
    fn cumulative(seconds: i64, tokens: u64) -> Value {
        json!({"timestamp":at(seconds).to_rfc3339(),"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":tokens}}}})
    }
    fn response(seconds: i64, id: &str, tokens: u64, total: Option<u64>) -> Value {
        json!({"timestamp":at(seconds).to_rfc3339(),"type":"token_usage_record","payload":{"thread_id":"owner","response_id":id,"usage":{"total_tokens":tokens},"thread_token_usage":total.map(|n| json!({"total_tokens":n}))}})
    }
    fn total_in(data: &str, start: i64, end: i64) -> Value {
        audit_rollouts(
            &[data],
            data.len() as u64,
            20,
            AuditWindow {
                start: Some(at(start)),
                end: Some(at(end)),
            },
        )
        .unwrap()
    }

    #[test]
    fn adjacent_windows_and_appends_preserve_totals_for_both_native_sources() {
        for native in [false, true] {
            let mut records = vec![json!({"type":"session_meta","payload":{"id":"owner"}})];
            let mut sum = 0;
            for n in 1..=20 {
                sum += n;
                records.push(if native {
                    response(n as i64, &format!("r-{n}"), n, Some(sum))
                } else {
                    cumulative(n as i64, sum)
                });
            }
            let data = lines(&records);
            for boundary in 1..20 {
                let a = total_in(&data, 0, boundary);
                let b = total_in(&data, boundary, 20);
                assert_eq!(a["status"], "PASS");
                assert_eq!(b["status"], "PASS");
                assert_eq!(
                    a["provider_reported_usage"]["total_tokens"]
                        .as_u64()
                        .unwrap()
                        + b["provider_reported_usage"]["total_tokens"]
                            .as_u64()
                            .unwrap(),
                    210
                );
            }
            let old = total_in(&data, 0, 10);
            records.push(cumulative(21, 999));
            assert_eq!(
                old["provider_reported_usage"],
                total_in(&lines(&records), 0, 10)["provider_reported_usage"]
            );
        }
    }

    #[test]
    fn mixed_checkpoints_include_new_responses_without_double_counting() {
        let mut records = vec![
            json!({"type":"session_meta","payload":{"id":"owner"}}),
            cumulative(5, 10),
            response(6, "r", 7, Some(17)),
        ];
        for extra in [
            None,
            Some(response(6, "r", 7, Some(17))),
            Some(cumulative(7, 17)),
        ] {
            let mut current = records.clone();
            current.extend(extra);
            let audit = total_in(&lines(&current), 0, 10);
            assert_eq!(audit["status"], "PASS");
            assert_eq!(audit["provider_reported_usage"]["total_tokens"], 17);
            assert_eq!(
                total_in(&lines(&current), 5, 10)["provider_reported_usage"]["total_tokens"],
                7
            );
        }
        // The selected native stream must still reject its own reset.
        records.push(response(8, "reset", 3, Some(3)));
        assert_eq!(
            total_in(&lines(&records), 0, 10)["status"],
            "PARTIAL",
            "counter reset is not a verified zero delta"
        );
    }

    #[test]
    fn unanchored_mixed_response_suffix_is_not_a_false_pass() {
        let data = lines(&[
            json!({"type":"session_meta","payload":{"id":"owner"}}),
            cumulative(5, 10),
            response(6, "r", 7, None),
        ]);
        assert_eq!(total_in(&data, 0, 10)["status"], "PARTIAL");
    }

    #[test]
    fn native_and_ui_totals_have_independent_monotonic_baselines() {
        let data = lines(&[
            json!({"type":"session_meta","payload":{"id":"owner"}}),
            response(1, "before", 20, Some(120)),
            cumulative(1, 90),
            response(2, "one", 7, Some(127)),
            cumulative(2, 97),
            response(3, "two", 5, Some(132)),
            cumulative(3, 102),
        ]);
        let audit = total_in(&data, 1, 3);
        assert_eq!(audit["collection_complete"], true);
        assert_eq!(audit["provider_reported_usage"]["total_tokens"], 12);
        assert_eq!(
            total_in(&data, 0, 3)["provider_reported_usage"]["total_tokens"],
            132
        );
        let reset = format!("{data}\n{}", cumulative(4, 2));
        assert_eq!(
            total_in(&reset, 1, 4)["collection_complete"],
            true,
            "a UI counter reset cannot invalidate independent native usage"
        );
        let native_reset = format!("{data}\n{}", response(4, "reset", 2, Some(2)));
        assert_eq!(total_in(&native_reset, 1, 4)["collection_complete"], false);
        let uncovered = format!(
            "{data}\n{}\n{}",
            response(4, "unanchored", 2, None),
            cumulative(4, 104)
        );
        assert_eq!(
            total_in(&uncovered, 1, 4)["collection_complete"],
            false,
            "a UI checkpoint cannot cover a trailing response in the selected native stream"
        );
    }

    #[test]
    fn first_owned_native_response_can_prove_a_new_zero_baseline() {
        let meta = json!({"type":"session_meta","payload":{"id":"owner"}});
        let records = vec![
            meta.clone(),
            cumulative(1, 100),
            response(3, "new", 7, Some(7)),
            cumulative(3, 107),
            response(4, "next", 5, Some(12)),
        ];
        let audit = total_in(&lines(&records), 2, 5);
        assert_eq!(audit["collection_complete"], true);
        assert_eq!(audit["provider_reported_usage"]["total_tokens"], 12);
        assert_eq!(
            total_in(&lines(&records), 2, 3)["provider_reported_usage"]["total_tokens"],
            7
        );
        assert_eq!(
            total_in(&lines(&records), 3, 5)["provider_reported_usage"]["total_tokens"],
            5
        );
        for preceding in [cumulative(2, 101), response(2, "unanchored", 2, None)] {
            let data = lines(&[
                meta.clone(),
                cumulative(1, 100),
                preceding,
                response(3, "new", 7, Some(7)),
            ]);
            assert_eq!(
                total_in(&data, 1, 4)["collection_complete"],
                false,
                "a zero anchor cannot discard earlier in-window usage"
            );
        }
        let unknown = lines(&[meta, cumulative(1, 100), response(3, "new", 7, Some(9))]);
        assert_eq!(
            total_in(&unknown, 2, 4)["collection_complete"],
            false,
            "an unexplained native prefix is still incomplete"
        );
    }

    #[test]
    fn counter_resets_before_the_window_do_not_poison_a_later_baseline() {
        for native in [false, true] {
            let records = lines(&[
                json!({"type":"session_meta","payload":{"id":"owner"}}),
                if native {
                    response(1, "a", 100, Some(100))
                } else {
                    cumulative(1, 100)
                },
                if native {
                    response(2, "b", 5, Some(5))
                } else {
                    cumulative(2, 5)
                },
                if native {
                    response(3, "c", 7, Some(12))
                } else {
                    cumulative(3, 12)
                },
            ]);
            assert_eq!(total_in(&records, 0, 3)["collection_complete"], false);
            let later = total_in(&records, 2, 3);
            assert_eq!(later["collection_complete"], true);
            assert_eq!(later["provider_reported_usage"]["total_tokens"], 7);
        }
    }

    #[test]
    fn audit_projection_preserves_metrics_and_failure_signal_combinations() {
        let mut records = vec![
            json!({"type":"session_meta","payload":{"id":"owner","base_instructions":"not emitted"}}),
            json!({"type":"world_state","payload":{"large":"not emitted".repeat(1000)}}),
            json!({"type":"compacted","payload":{"message":"not emitted"}}),
            json!({"type":"turn_context","payload":{"model":"gpt-6-astra","effort":"high","cwd":"not emitted"}}),
            json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"text":"not emitted"}]}}),
        ];
        for (index, output) in [
            Value::Null,
            json!(""),
            json!({"exit_code":0}),
            json!({"exit_code":4}),
            json!("timed out: permission denied; invalid argument"),
            json!("script running with cell id"),
        ]
        .into_iter()
        .enumerate()
        {
            records.push(json!({"type":"response_item","payload":{"type":"function_call","name":"exec","arguments":"cargo test","call_id":index.to_string()}}));
            records.push(json!({"type":"response_item","payload":{"type":"function_call_output","output":output,"call_id":index.to_string()}}));
        }
        let original = lines(&records);
        let projected = original
            .lines()
            .map(|line| {
                Record::parse(line)
                    .unwrap()
                    .unwrap()
                    .audit_projection()
                    .unwrap()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let expected = audit_rollouts(
            &[&original],
            original.len() as u64,
            20,
            AuditWindow::default(),
        )
        .unwrap();
        let actual = audit_rollouts(
            &[&projected],
            original.len() as u64,
            20,
            AuditWindow::default(),
        )
        .unwrap();
        assert_eq!(actual, expected);
        assert!(!projected.contains("not emitted"));
    }

    #[test]
    fn child_owned_suffix_boundary_also_anchors_its_fork_metadata() {
        let data = lines(&[
            json!({"ordinal":0,"type":"session_meta","payload":{"id":"owner","history_mode":"paginated","forked_from_id":"parent","subagent_history_start_ordinal":2}}),
            json!({"ordinal":1,"timestamp":at(1).to_rfc3339(),"type":"token_usage_record","payload":{"thread_id":"owner","response_id":"old","usage":{"total_tokens":100}}}),
            json!({"ordinal":2,"timestamp":at(2).to_rfc3339(),"type":"token_usage_record","payload":{"thread_id":"owner","response_id":"new","usage":{"total_tokens":7}}}),
        ]);
        let result = total_in(&data, 0, 3);
        assert_eq!(result["collection_complete"], true);
        assert_eq!(result["provider_reported_usage"]["total_tokens"], 7);
    }

    #[test]
    fn diagnostics_have_bounded_examples_and_exact_omitted_count() {
        let malformed = "!\n".repeat(100_000);
        let result = audit_rollouts(
            &[&malformed],
            malformed.len() as u64,
            20,
            AuditWindow::default(),
        )
        .unwrap();
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["error_count"], 100_000);
        assert_eq!(
            result["errors"].as_array().unwrap().len(),
            MAX_ERROR_EXAMPLES
        );
        assert_eq!(
            result["error_examples_omitted"],
            100_000 - MAX_ERROR_EXAMPLES
        );
        assert!(serde_json::to_vec(&result).unwrap().len() < 16 * 1024);
        let huge = "x".repeat(MAX_RECORD_BYTES + 1);
        assert_eq!(
            audit_rollouts(&[&huge], 0, 20, AuditWindow::default()).unwrap()["status"],
            "PARTIAL"
        );
        let many = "{}\n".repeat(MAX_AUDIT_RECORDS + 1);
        let result = audit_rollouts(&[&many], 0, 20, AuditWindow::default()).unwrap();
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["coverage"]["record_count"], MAX_AUDIT_RECORDS);
    }

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

    #[test]
    fn unused_payloads_preserve_coverage_and_malformed_records_stay_partial() {
        let data = lines(&[
            json!({"timestamp":"2026-09-03T00:00:00Z","type":"world_state","payload":{"private":"not emitted".repeat(1000)}}),
            json!({"timestamp":"2026-09-01T00:00:00Z","type":"future_record","payload":[1,2,3]}),
            json!({"timestamp":"2026-09-02T00:00:00Z","type":"event_msg","payload":{"type":"task_complete","duration_ms":30}}),
            json!({"timestamp":"2026-09-02T00:00:00Z","type":"event_msg","payload":{"type":"task_complete","duration_ms":10}}),
            json!({"timestamp":"2026-09-02T00:00:00Z","type":"event_msg","payload":{"type":"task_complete","duration_ms":20}}),
        ]);
        let result = audit_rollouts(&[&data], 0, 20, AuditWindow::default()).unwrap();
        assert_eq!(result["status"], "PASS");
        assert_eq!(result["coverage"]["record_count"], 5);
        assert_eq!(
            result["coverage"]["time_window"]["start"],
            "2026-09-01T00:00:00+00:00"
        );
        assert_eq!(
            result["coverage"]["time_window"]["end"],
            "2026-09-03T00:00:00+00:00"
        );
        assert_eq!(result["task_latency"]["median_ms"], 20);
        assert_eq!(result["task_latency"]["p90_ms"], 30);
        assert_eq!(result["task_latency"]["max_ms"], 30);
        assert!(!result.to_string().contains("not emitted"));
        let broken = format!("{data}\n{}", r#"{"type":"world_state","payload":[1,]}"#);
        let result = audit_rollouts(&[&broken], 0, 20, AuditWindow::default()).unwrap();
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["coverage"]["record_count"], 5);
        assert_eq!(result["errors"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn percentile_reads_sorted_data_without_changing_rounding() {
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(percentile(&[7], 0.9), Some(7));
        assert_eq!(percentile(&[10, 20], 0.5), Some(20));
        assert_eq!(percentile(&[10, 20, 30, 40], 0.5), Some(30));
        assert_eq!(percentile(&[10, 20, 30, 40], 0.9), Some(40));
    }

    fn lines(records: &[Value]) -> String {
        records
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn model_context_is_bounded_and_null_effort_uses_available_metadata() {
        let data = lines(&[
            json!({"type":"turn_context","payload":{"model":"gpt-6-astra","effort":null,"reasoning_effort":"high"}}),
            json!({"type":"turn_context","payload":{"model":"secret-custom-model","effort":"secret-custom-effort"}}),
        ]);
        let result = audit_rollouts(&[&data], 0, 20, AuditWindow::default()).unwrap();
        assert_eq!(
            result["model_effort"]["counts"],
            json!({"astra|high":1,"other|unknown":1})
        );
        assert_eq!(result["model_effort"]["transition_count"], 1);
        assert!(!result.to_string().contains("secret-custom"));
    }

    #[test]
    fn turn_call_and_model_state_do_not_cross_rollout_boundaries() {
        let call = json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"same","arguments":"cargo test"}});
        let output = json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"same","output":{"exit_code":0}}});
        let first = lines(&[
            json!({"type":"turn_context","payload":{"model":"gpt-6-astra","effort":"high"}}),
            call.clone(),
            output.clone(),
            json!({"timestamp":"2026-09-01T00:00:00Z","type":"event_msg","payload":{"type":"task_started","turn_id":"same"}}),
        ]);
        let second = lines(&[
            json!({"type":"turn_context","payload":{"model":"gpt-5.6-sol","effort":"low"}}),
            call,
            output,
            json!({"timestamp":"2026-09-01T01:00:00Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"same"}}),
        ]);
        let result = audit_rollouts(&[&first, &second], 0, 20, AuditWindow::default()).unwrap();
        assert_eq!(result["model_effort"]["transition_count"], 0);
        assert_eq!(result["tools"]["verification_success_count"], 2);
        assert_eq!(result["task_latency"]["completed_count"], 0);
    }

    #[test]
    fn paired_compaction_records_count_once_per_rollout() {
        let compacted = json!({"type":"compacted","payload":{"message":"do not emit"}});
        let event = json!({"type":"event_msg","payload":{"type":"context_compacted"}});
        let paired = lines(&[compacted.clone(), event.clone()]);
        let event_only = lines(&[event]);
        let record_only = lines(&[compacted]);
        let result = audit_rollouts(
            &[&paired, &event_only, &record_only],
            0,
            20,
            AuditWindow::default(),
        )
        .unwrap();
        assert_eq!(result["activity"]["compactions"], 3);
        assert_eq!(result["boundary_signals"]["long_lived_root_count"], 0);
    }

    #[test]
    fn cumulative_window_usage_does_not_double_count_new_response_records() {
        let data = lines(&[
            json!({"timestamp":"2026-09-01T00:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":100}}}}),
            json!({"timestamp":"2026-09-01T00:01:00Z","type":"token_usage_record","payload":{"usage":{"total_tokens":50},"thread_token_usage":{"total_tokens":150},"response_id":"private-response"}}),
            json!({"timestamp":"2026-09-01T00:01:00Z","type":"event_msg","payload":{"type":"raw_response_completed","token_usage":{"total_tokens":50}}}),
            json!({"timestamp":"2026-09-01T00:01:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":150},"last_token_usage":{"total_tokens":50}}}}),
        ]);
        let result = audit_rollouts(
            &[&data],
            0,
            20,
            AuditWindow {
                start: Some("2026-09-01T00:00:00Z".parse().unwrap()),
                end: Some("2026-09-01T00:02:00Z".parse().unwrap()),
            },
        )
        .unwrap();
        assert_eq!(result["provider_reported_usage"]["total_tokens"], 50);
        assert!(!result.to_string().contains("private-response"));
        let fallback = lines(&[
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{},"last_token_usage":{"total_tokens":7}}}}),
        ]);
        let result = audit_rollouts(&[&fallback], 0, 20, AuditWindow::default()).unwrap();
        assert_eq!(result["provider_reported_usage"]["total_tokens"], 7);
    }

    #[test]
    fn inherited_history_cannot_masquerade_as_new_child_usage() {
        let data = lines(&[
            json!({"type":"session_meta","payload":{"history_base":{"path":"private-parent"}}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":500}}}}),
        ]);
        let result = audit_rollouts(&[&data], 0, 20, AuditWindow::default()).unwrap();
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["provider_reported_usage"]["total_tokens"], 0);
        assert!(!result.to_string().contains("private-parent"));
    }

    #[test]
    fn shared_suffix_counts_only_owned_unique_responses_in_window() {
        let response = |ordinal, at, owner, id, total| {
            json!({
                "ordinal":ordinal,"timestamp":at,"type":"token_usage_record",
                "payload":{"thread_id":owner,"response_id":id,"usage":{"total_tokens":total},
                    "thread_token_usage":{"total_tokens":999999}}
            })
        };
        let data = lines(&[
            json!({"ordinal":10,"type":"session_meta","payload":{"id":"private-child","history_mode":"paginated","history_base":{"end_ordinal_exclusive":10},"forked_from_id":"private-parent","forked_from_ordinal_exclusive":12}}),
            response(
                11,
                "2026-09-01T00:01:00Z",
                "private-child",
                "inherited",
                400,
            ),
            response(
                12,
                "2026-09-01T00:00:00Z",
                "private-child",
                "before-window",
                100,
            ),
            response(13, "2026-09-01T00:01:00Z", "private-parent", "foreign", 500),
            response(14, "2026-09-01T00:01:00Z", "private-child", "own", 7),
            response(15, "2026-09-01T00:01:01Z", "private-child", "own", 7),
            response(
                16,
                "2026-09-01T00:03:00Z",
                "private-child",
                "after-window",
                800,
            ),
            json!({"ordinal":17,"timestamp":"2026-09-01T00:01:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":999999}}}}),
            json!({"ordinal":18,"timestamp":"2026-09-01T00:01:00Z","type":"turn_context","payload":{"model":"gpt-6-astra","effort":"high"}}),
        ]);
        let result = audit_rollouts(
            &[&data],
            0,
            20,
            AuditWindow {
                start: Some("2026-09-01T00:00:00Z".parse().unwrap()),
                end: Some("2026-09-01T00:02:00Z".parse().unwrap()),
            },
        )
        .unwrap();
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["provider_reported_usage"]["total_tokens"], 7);
        assert_eq!(
            result["collection_complete"], true,
            "known inherited prefix is outside the owned suffix, not a retryable gap"
        );
        assert_eq!(
            result["provider_reported_usage"]["source"],
            "codex-response-usage-records"
        );
        assert_eq!(result["provider_reported_usage"]["usage_event_count"], 1);
        assert_eq!(
            result["provider_reported_usage"]["fallback_rollout_count"],
            1
        );
        assert_eq!(result["model_effort"]["counts"]["astra|high"], 1);
        assert!(!result.to_string().contains("private-child"));
    }

    #[test]
    fn response_fallback_does_not_add_to_cumulative_or_last_snapshots() {
        let response_only = lines(&[
            json!({"type":"session_meta","payload":{"id":"owner"}}),
            json!({"type":"token_usage_record","payload":{"thread_id":"owner","response_id":"r","usage":{"total_tokens":7}}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"total_tokens":7}}}}),
        ]);
        let cumulative = format!(
            "{response_only}\n{}",
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":100}}}})
        );
        let result = audit_rollouts(
            &[&response_only, &cumulative],
            0,
            20,
            AuditWindow::default(),
        )
        .unwrap();
        assert_eq!(result["provider_reported_usage"]["total_tokens"], 107);
        assert_eq!(
            result["provider_reported_usage"]["source"],
            "codex-mixed-usage-sources"
        );
        assert_eq!(
            result["provider_reported_usage"]["cumulative_rollout_count"],
            1
        );
        assert_eq!(
            result["provider_reported_usage"]["fallback_rollout_count"],
            1
        );
    }

    #[test]
    fn native_settings_notification_can_share_ordinal_without_relaxing_usage_order() {
        let records = [
            json!({"ordinal":0,"type":"session_meta","payload":{"id":"owner","history_mode":"paginated"}}),
            json!({"ordinal":1,"type":"token_usage_record","payload":{"thread_id":"owner","response_id":"a","usage":{"total_tokens":7}}}),
            json!({"ordinal":1,"type":"event_msg","payload":{"type":"thread_settings_applied"}}),
            json!({"ordinal":2,"type":"token_usage_record","payload":{"thread_id":"owner","response_id":"b","usage":{"total_tokens":3}}}),
        ];
        let audit = |records: &[Value]| {
            audit_rollouts(&[&lines(records)], 0, 20, AuditWindow::default()).unwrap()
        };
        let result = audit(&records);
        assert_eq!(result["collection_complete"], true);
        assert_eq!(result["provider_reported_usage"]["total_tokens"], 10);
        let mut duplicate_usage = records.clone();
        duplicate_usage[2] = json!({"ordinal":1,"type":"token_usage_record","payload":{"thread_id":"owner","response_id":"different","usage":{"total_tokens":999}}});
        assert_eq!(audit(&duplicate_usage)["collection_complete"], false);
        let mut reordered = records;
        reordered[2]["ordinal"] = json!(0);
        assert_eq!(audit(&reordered)["collection_complete"], false);
    }

    #[test]
    fn raw_tool_output_keeps_outcomes_and_discards_large_bodies() {
        for output in [
            json!("TIMED OUT: invalid argument; permission denied; script running with cell id"),
            json!({"exit_code":2,"nested":[true,null,42,1.5,"rejected"]}),
            json!({"content":[{"text":"Process running with session id"},{"image":"x".repeat(5 * 1024 * 1024)}]}),
        ] {
            let raw = serde_json::value::to_raw_value(&output).unwrap();
            let projected = project_raw_tool_output(&raw, &Map::new()).unwrap();
            assert_eq!(
                projected,
                projected_signals(&failure_signals(&output, &Map::new()))
            );
            assert!(projected.to_string().len() < 300);
        }
        let raw = serde_json::value::RawValue::from_string(
            r#"{"exit_code":2,"exit_code":0,"text":"t\u0069meout"}"#.to_owned(),
        )
        .unwrap();
        let projected = project_raw_tool_output(&raw, &Map::new()).unwrap();
        assert_eq!(projected["exit_code"], 0);
        assert_eq!(projected["markers"], json!(["timed out"]));
    }

    #[test]
    fn invalid_or_missing_ordinals_cannot_attribute_inherited_usage() {
        let data = lines(&[
            json!({"ordinal":10,"type":"session_meta","payload":{"id":"owner","history_mode":"paginated","history_base":{"end_ordinal_exclusive":10}}}),
            json!({"type":"token_usage_record","payload":{"thread_id":"owner","response_id":"missing","usage":{"total_tokens":50}}}),
            json!({"ordinal":9,"type":"token_usage_record","payload":{"thread_id":"owner","response_id":"old","usage":{"total_tokens":50}}}),
        ]);
        let result = audit_rollouts(&[&data], 0, 20, AuditWindow::default()).unwrap();
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["provider_reported_usage"]["total_tokens"], 0);
        assert_eq!(result["errors"].as_array().unwrap().len(), 3);
    }
}
