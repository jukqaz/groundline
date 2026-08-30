use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{ContractError, insights::validate_basic_event_bytes, version::strict_version};

const COLLECTION_TRIGGERS: &[&str] = &[
    "manual",
    "history_sync",
    "session_start_hook",
    "stop_hook",
    "post_compact_hook",
    "session_end_hook",
];

#[derive(Debug, Clone)]
pub struct CollectorIdentity<'a> {
    pub instance_id: Uuid,
    pub os_family: &'a str,
    pub runtime_family: &'a str,
    pub execution_mode: &'a str,
}

#[derive(Debug, Clone)]
pub struct ConsentReceipt<'a> {
    pub receipt_id: Uuid,
    pub accepted_at_utc: &'a str,
}

fn object(value: Option<&Value>) -> &Map<String, Value> {
    value.and_then(Value::as_object).unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn count(value: Option<&Value>) -> u64 {
    value.and_then(Value::as_u64).unwrap_or(0)
}

fn optional_number(value: Option<&Value>) -> Value {
    match value.and_then(Value::as_f64) {
        Some(value) if value.is_finite() && value >= 0.0 => Value::from(value),
        _ => Value::Null,
    }
}

fn timestamp(value: Option<&Value>) -> Option<String> {
    let parsed = DateTime::parse_from_rfc3339(value?.as_str()?).ok()?;
    let utc = parsed.with_timezone(&Utc);
    let format = if utc.nanosecond() == 0 {
        SecondsFormat::Secs
    } else {
        SecondsFormat::Micros
    };
    Some(utc.to_rfc3339_opts(format, true))
}

fn component_status(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str) {
        Some("PASS") => "PASS",
        Some("PARTIAL") => "PARTIAL",
        Some("INSUFFICIENT_EVIDENCE") => "INSUFFICIENT_EVIDENCE",
        Some("FAIL") => "FAIL",
        _ => "UNKNOWN",
    }
}

fn filtered_counts(value: Option<&Value>, allowed: &[&str]) -> Value {
    let source = object(value);
    Value::Object(
        allowed
            .iter()
            .filter_map(|key| {
                let value = source.get(*key).and_then(Value::as_u64)?;
                (value > 0).then(|| ((*key).to_owned(), Value::from(value)))
            })
            .collect(),
    )
}

fn model_family(value: &str) -> &'static str {
    let value = value.to_ascii_lowercase();
    if value.contains("luna") {
        "luna"
    } else if value.contains("terra") {
        "terra"
    } else if value.contains("sol") {
        "sol"
    } else if value.starts_with("gpt-5") {
        "gpt-5"
    } else if matches!(value.as_str(), "unknown" | "unset" | "") {
        "unknown"
    } else {
        "other"
    }
}

fn normalized_effort(value: &str) -> &'static str {
    match value {
        "none" => "none",
        "unset" => "unset",
        "minimal" => "minimal",
        "low" => "low",
        "medium" => "medium",
        "high" => "high",
        "xhigh" => "xhigh",
        "max" => "max",
        "ultra" => "ultra",
        _ => "unknown",
    }
}

fn model_effort(component: &Map<String, Value>) -> Value {
    let counts = object(
        component
            .get("model_effort")
            .and_then(|value| value.get("counts")),
    );
    let mut normalized = BTreeMap::<(String, String), u64>::new();
    for (key, value) in counts {
        let amount = value.as_u64().unwrap_or(0);
        let (model, effort) = key.split_once('|').unwrap_or((key, "unknown"));
        let key = (
            model_family(model).to_owned(),
            normalized_effort(effort).to_owned(),
        );
        let current = normalized.get(&key).copied().unwrap_or(0);
        if let Some(next) = current.checked_add(amount) {
            normalized.insert(key, next);
        }
    }
    Value::Array(
        normalized
            .into_iter()
            .take(16)
            .map(|((model_family, effort), count)| {
                json!({"model_family":model_family,"effort":effort,"count":count})
            })
            .collect(),
    )
}

fn usage(component: &Map<String, Value>, include_non_cached: bool) -> Value {
    let source = object(component.get("provider_reported_usage"));
    let input = count(source.get("input_tokens"));
    let cached = count(source.get("cached_input_tokens"));
    let mut result = json!({
        "source": source.get("source").and_then(Value::as_str).filter(|value| matches!(
            *value,
            "codex-cumulative-total-snapshots"
                | "codex-cumulative-and-last-usage-fallback"
                | "codex-last-usage-events-summed-fallback"
                | "codex-cumulative-window-delta"
                | "codex-window-delta-and-last-usage-fallback"
                | "codex-last-usage-events-summed-window"
                | "unavailable"
        )).unwrap_or("unknown"),
        "rollout_count_with_usage":count(source.get("rollout_count_with_usage")),
        "cumulative_rollout_count":count(source.get("cumulative_rollout_count")),
        "fallback_rollout_count":count(source.get("fallback_rollout_count")),
        "input_tokens":input,
        "cached_input_tokens":cached,
        "cache_write_input_tokens":count(source.get("cache_write_input_tokens")),
        "output_tokens":count(source.get("output_tokens")),
        "reasoning_output_tokens":count(source.get("reasoning_output_tokens")),
        "total_tokens":count(source.get("total_tokens")),
        "cached_input_ratio":optional_number(source.get("cached_input_ratio")),
    });
    if include_non_cached {
        result.as_object_mut().expect("object").insert(
            "non_cached_input_tokens".to_owned(),
            Value::from(input.saturating_sub(cached)),
        );
    }
    result
}

fn session_metrics(component: Option<&Value>) -> Value {
    let component = object(component);
    let activity = object(component.get("activity"));
    let latency = object(component.get("task_latency"));
    let prompts = object(component.get("prompt_shape"));
    let tools = object(component.get("tools"));
    let categories = object(tools.get("by_category"));
    let boundary = object(component.get("boundary_signals"));
    let verification = count(categories.get("verification"));
    let success = count(tools.get("verification_success_count")).min(verification);
    let failure =
        count(tools.get("verification_failure_count")).min(verification.saturating_sub(success));
    let unresolved = verification.saturating_sub(success).saturating_sub(failure);
    json!({
        "status":component_status(component.get("status")),
        "activity":{
            "task_started":count(activity.get("task_started")),
            "task_completed":count(activity.get("task_completed")),
            "turn_contexts":count(activity.get("turn_contexts")),
            "compactions":count(activity.get("compactions")),
            "user_messages_with_text":count(activity.get("user_messages_with_text")),
        },
        "model_effort":model_effort(component),
        "usage":usage(component, true),
        "latency":{
            "completed_count":count(latency.get("completed_count")),
            "median_ms":optional_number(latency.get("median_ms")),
            "p90_ms":optional_number(latency.get("p90_ms")),
            "max_ms":optional_number(latency.get("max_ms")),
            "long_turn_count":count(latency.get("long_turn_count")),
        },
        "quality_proxies":{
            "verification_tool_calls":verification,
            "verification_success_count":success,
            "verification_failure_count":failure,
            "verification_unresolved_count":unresolved,
            "tool_call_count":count(tools.get("call_count")),
            "short_message_count":count(prompts.get("short_message_count")),
            "broad_scope_message_count":count(prompts.get("broad_scope_message_count")),
            "failure_signals":filtered_counts(tools.get("failure_signals"), &["nonzero_exit","yielded_for_wait","timeout","invalid_arguments","rejected"]),
            "exact_repeated_call_groups":count(tools.get("exact_repeated_call_groups")),
            "calls_in_exact_repeated_groups":count(tools.get("calls_in_exact_repeated_groups")),
            "task_boundary_review_recommended":boundary.get("task_boundary_review_recommended").and_then(Value::as_bool).unwrap_or(false),
            "long_lived_root_session":boundary.get("long_lived_root_session").and_then(Value::as_bool).unwrap_or(false),
            "boundary_review_root_count":count(boundary.get("boundary_review_root_count")),
            "long_lived_root_count":count(boundary.get("long_lived_root_count")),
        },
        "tool_categories":filtered_counts(tools.get("by_category"), &["wait_or_poll","coordination","mutation","research","other_tool","verification","codex_runtime","git_or_github","inspection","other_command"]),
    })
}

fn guardian_metrics(component: Option<&Value>) -> Value {
    let component = object(component);
    let signals = object(component.get("signals"));
    json!({
        "status":component_status(component.get("status")),
        "rollout_count":count(component.get("rollout_count")),
        "review_count":count(component.get("review_count")),
        "usage":usage(component, false),
        "outcomes":filtered_counts(component.get("outcomes"), &["approved","rejected","cancelled","error","unknown"]),
        "risk_levels":filtered_counts(component.get("risk_levels"), &["low","medium","high","critical","unknown"]),
        "signals":{
            "outside_workspace_action_rate":optional_number(signals.get("outside_workspace_action_rate")),
            "temporary_workspace_action_rate":optional_number(signals.get("temporary_workspace_action_rate")),
            "reviewer_already_low_effort":signals.get("reviewer_already_low_effort").and_then(Value::as_bool).unwrap_or(false),
            "workspace_attributed_review_count":count(signals.get("workspace_attributed_review_count")),
            "workspace_attribution_coverage":optional_number(signals.get("workspace_attribution_coverage")),
        }
    })
}

pub fn build_basic_event(
    audit: &Value,
    identity: CollectorIdentity<'_>,
    consent: ConsentReceipt<'_>,
    groundline_version: &str,
    collection_generation: u32,
    collection_trigger: &str,
) -> Result<Value, ContractError> {
    strict_version(groundline_version)?;
    if !COLLECTION_TRIGGERS.contains(&collection_trigger) {
        return Err(ContractError("invalid_collection_trigger".to_owned()));
    }
    let audit_object = audit
        .as_object()
        .ok_or_else(|| ContractError("invalid_audit".to_owned()))?;
    let audit_kind = audit_object
        .get("kind")
        .and_then(Value::as_str)
        .filter(|value| {
            matches!(
                *value,
                "groundline-codex-weekly-audit" | "groundline-codex-activity-audit"
            )
        })
        .ok_or_else(|| ContractError("invalid_audit".to_owned()))?;
    if audit_object.get("schema").and_then(Value::as_u64) != Some(1)
        || !matches!(
            audit_object.get("status").and_then(Value::as_str),
            Some("PASS" | "PARTIAL" | "INSUFFICIENT_EVIDENCE")
        )
        || [
            "mutation_performed",
            "raw_content_emitted",
            "private_paths_emitted",
            "thread_ids_emitted",
            "rollout_paths_emitted",
            "secret_value_printed",
        ]
        .iter()
        .any(|key| audit_object.get(*key) != Some(&Value::Bool(false)))
    {
        return Err(ContractError("invalid_audit".to_owned()));
    }
    let scope = object(audit_object.get("scope"));
    let generated = timestamp(scope.get("generated_at"))
        .ok_or_else(|| ContractError("invalid_audit".to_owned()))?;
    let selection = scope
        .get("selection_mode")
        .and_then(Value::as_str)
        .filter(|value| {
            matches!(
                *value,
                "last_7_days"
                    | "latest_completed_fallback"
                    | "requested_window"
                    | "activity_window"
            )
        })
        .ok_or_else(|| ContractError("invalid_audit".to_owned()))?;
    let root = audit_object.get("root");
    let delegated = audit_object.get("delegated");
    let guardian = audit_object.get("guardian");
    let root_window = object(
        root.and_then(|value| value.get("coverage"))
            .and_then(|value| value.get("time_window")),
    );
    let start = if matches!(selection, "requested_window" | "activity_window") {
        timestamp(scope.get("requested_window_start"))
    } else {
        timestamp(root_window.get("start"))
    };
    let end = if matches!(selection, "requested_window" | "activity_window") {
        timestamp(scope.get("requested_window_end"))
    } else {
        timestamp(root_window.get("end"))
    };
    if start
        .as_deref()
        .zip(end.as_deref())
        .is_some_and(|(start, end)| start >= end)
    {
        return Err(ContractError("invalid_audit".to_owned()));
    }
    let mut event = json!({
        "schema_version":5,
        "kind":"groundline-insights-basic-weekly",
        "collector":{"instance_id":identity.instance_id,"os_family":identity.os_family,"runtime_family":identity.runtime_family,"execution_mode":identity.execution_mode},
        "source":{"groundline_version":groundline_version,"audit_schema":1,"audit_kind":audit_kind,"collection_generation":collection_generation,"collection_trigger":collection_trigger},
        "period":{"start_utc":start,"end_utc":end,"generated_at_utc":generated},
        "capabilities":{"completed_root_coverage":audit_kind == "groundline-codex-weekly-audit","latency_completed_count":true,"root_boundary_counts":true,"guardian_workspace_attribution":false},
        "sample":{
            "selection_mode":selection,"requested_days":count(scope.get("requested_days")),"root_count":count(scope.get("completed_root_sample_count")),
            "observed_root_count":count(scope.get("observed_root_sample_count")).max(count(scope.get("completed_root_sample_count"))),
            "minimum_root_count":count(scope.get("minimum_root_sample_count")),"sample_sufficient":scope.get("sample_sufficient").and_then(Value::as_bool).unwrap_or(false),
            "delegated_count":count(scope.get("delegated_rollout_count")),"guardian_count":count(scope.get("guardian_rollout_count")),
            "guardian_incomplete_excluded_count":count(scope.get("guardian_incomplete_excluded_count")),"unreadable_completed_root_count":count(scope.get("unreadable_completed_root_count")),
            "originator_unclassified_excluded_root_count":count(scope.get("originator_unclassified_excluded_root_count")),"originator_source_fallback_root_count":count(scope.get("originator_source_fallback_root_count")),
            "delegated_truncated_count":count(scope.get("delegated_truncated_count")),"guardian_truncated_count":count(scope.get("guardian_truncated_count")),
            "eligible_root_count":count(scope.get("eligible_root_count")),"selected_root_count":count(scope.get("selected_root_count")),"root_truncated_count":count(scope.get("root_truncated_count")),
            "selection_coverage":optional_number(scope.get("selection_coverage")),"selected_recency_start_utc":timestamp(scope.get("selected_recency_start_utc")),"selected_recency_end_utc":timestamp(scope.get("selected_recency_end_utc")),
        },
        "metrics":{"root":session_metrics(root),"delegated":session_metrics(delegated),"guardian":guardian_metrics(guardian)},
        "quality_contract":{"provider_usage_only":true,"billing_inference_performed":false,"verification_is_a_tool_call_proxy":true,"verification_outcome_is_a_tool_result_proxy":true,"rework_not_observed":true,"correlation_is_not_causation":true},
        "privacy":{"basic_aggregate_only":true},
        "consent":{"scope":"basic_weekly","receipt_id":consent.receipt_id,"accepted_at_utc":consent.accepted_at_utc},
    });
    let encoded =
        serde_json::to_vec(&event).map_err(|_| ContractError("invalid_basic_event".to_owned()))?;
    let digest = format!("{:x}", Sha256::digest(encoded));
    let object = event.as_object_mut().expect("event object");
    object.insert(
        "event_id".to_owned(),
        Value::from(Uuid::new_v5(&Uuid::NAMESPACE_URL, digest.as_bytes()).to_string()),
    );
    object.insert(
        "idempotency_key".to_owned(),
        Value::from(format!("sha256:{digest}")),
    );
    validate_basic_event_bytes(
        &serde_json::to_vec(&event).map_err(|_| ContractError("invalid_basic_event".to_owned()))?,
    )?;
    Ok(event)
}
