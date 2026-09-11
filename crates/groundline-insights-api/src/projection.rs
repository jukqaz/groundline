//! One payload-to-column contract shared by serialization and ClickHouse guards.
use super::{ApiError, checked_u32_sum};
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
enum Kind {
    UInt,
    Text,
    Uuid,
    Bool,
    NullableFloat,
    Time,
    NullableTime,
    Sum(&'static [&'static str]),
    ArrayText(&'static str),
    ArrayUInt(&'static str),
}
use Kind::*;

// Keep the declarative mapping compact enough to review as one contract.
#[rustfmt::skip]
const FIELDS: &[(&str, &[&str], Kind)] = &[
    ("schema_version", &["schema_version"], UInt),
    ("event_id", &["event_id"], Uuid),
    ("idempotency_key", &["idempotency_key"], Text),
    ("collector_id", &["collector", "instance_id"], Uuid),
    ("collection_generation", &["source", "collection_generation"], UInt),
    ("collection_trigger", &["source", "collection_trigger"], Text),
    ("groundline_version", &["source", "groundline_version"], Text),
    ("os_family", &["collector", "os_family"], Text),
    ("runtime_family", &["collector", "runtime_family"], Text),
    ("execution_mode", &["collector", "execution_mode"], Text),
    ("period_start", &["period", "start_utc"], NullableTime),
    ("period_end", &["period", "end_utc"], NullableTime),
    ("generated_at", &["period", "generated_at_utc"], Time),
    ("selection_mode", &["sample", "selection_mode"], Text),
    ("requested_days", &["sample", "requested_days"], UInt),
    ("root_count", &["sample", "root_count"], UInt),
    ("observed_root_count", &["sample", "observed_root_count"], UInt),
    ("eligible_root_count", &["sample", "eligible_root_count"], UInt),
    ("selected_root_count", &["sample", "selected_root_count"], UInt),
    ("root_truncated_count", &["sample", "root_truncated_count"], UInt),
    ("selection_coverage", &["sample", "selection_coverage"], NullableFloat),
    ("selected_recency_start", &["sample", "selected_recency_start_utc"], NullableTime),
    ("selected_recency_end", &["sample", "selected_recency_end_utc"], NullableTime),
    ("minimum_root_count", &["sample", "minimum_root_count"], UInt),
    ("sample_sufficient", &["sample", "sample_sufficient"], Bool),
    ("delegated_count", &["sample", "delegated_count"], UInt),
    ("guardian_count", &["sample", "guardian_count"], UInt),
    ("guardian_incomplete_excluded_count", &["sample", "guardian_incomplete_excluded_count"], UInt),
    ("unreadable_root_count", &["sample", "unreadable_completed_root_count"], UInt),
    ("originator_unclassified_excluded_root_count", &["sample", "originator_unclassified_excluded_root_count"], UInt),
    ("originator_source_fallback_root_count", &["sample", "originator_source_fallback_root_count"], UInt),
    ("truncated_count", &["sample", "delegated_truncated_count"], Sum(&["sample", "guardian_truncated_count"])),
    ("capability_completed_root_coverage", &["capabilities", "completed_root_coverage"], Bool),
    ("capability_latency_completed_count", &["capabilities", "latency_completed_count"], Bool),
    ("capability_root_boundary_counts", &["capabilities", "root_boundary_counts"], Bool),
    ("capability_guardian_workspace_attribution", &["capabilities", "guardian_workspace_attribution"], Bool),
    ("root_status", &["metrics", "root", "status"], Text),
    ("delegated_status", &["metrics", "delegated", "status"], Text),
    ("guardian_status", &["metrics", "guardian", "status"], Text),
    ("model_families", &["metrics", "root", "model_effort"], ArrayText("model_family")),
    ("efforts", &["metrics", "root", "model_effort"], ArrayText("effort")),
    ("model_effort_counts", &["metrics", "root", "model_effort"], ArrayUInt("count")),
    ("usage_source", &["metrics", "root", "usage", "source"], Text),
    ("delegated_usage_source", &["metrics", "delegated", "usage", "source"], Text),
    ("guardian_usage_source", &["metrics", "guardian", "usage", "source"], Text),
    ("input_tokens", &["metrics", "root", "usage", "input_tokens"], UInt),
    ("cached_input_tokens", &["metrics", "root", "usage", "cached_input_tokens"], UInt),
    ("output_tokens", &["metrics", "root", "usage", "output_tokens"], UInt),
    ("reasoning_output_tokens", &["metrics", "root", "usage", "reasoning_output_tokens"], UInt),
    ("total_tokens", &["metrics", "root", "usage", "total_tokens"], UInt),
    ("non_cached_input_tokens", &["metrics", "root", "usage", "non_cached_input_tokens"], UInt),
    ("cached_input_ratio", &["metrics", "root", "usage", "cached_input_ratio"], NullableFloat),
    ("cumulative_rollout_count", &["metrics", "root", "usage", "cumulative_rollout_count"], UInt),
    ("fallback_rollout_count", &["metrics", "root", "usage", "fallback_rollout_count"], UInt),
    ("delegated_fallback_rollout_count", &["metrics", "delegated", "usage", "fallback_rollout_count"], UInt),
    ("guardian_fallback_rollout_count", &["metrics", "guardian", "usage", "fallback_rollout_count"], UInt),
    ("task_started", &["metrics", "root", "activity", "task_started"], UInt),
    ("task_completed", &["metrics", "root", "activity", "task_completed"], UInt),
    ("turn_contexts", &["metrics", "root", "activity", "turn_contexts"], UInt),
    ("compactions", &["metrics", "root", "activity", "compactions"], UInt),
    ("user_messages_with_text", &["metrics", "root", "activity", "user_messages_with_text"], UInt),
    ("latency_median_ms", &["metrics", "root", "latency", "median_ms"], NullableFloat),
    ("latency_p90_ms", &["metrics", "root", "latency", "p90_ms"], NullableFloat),
    ("latency_max_ms", &["metrics", "root", "latency", "max_ms"], NullableFloat),
    ("latency_completed_count", &["metrics", "root", "latency", "completed_count"], UInt),
    ("long_turn_count", &["metrics", "root", "latency", "long_turn_count"], UInt),
    ("verification_tool_calls", &["metrics", "root", "quality_proxies", "verification_tool_calls"], UInt),
    ("verification_success_count", &["metrics", "root", "quality_proxies", "verification_success_count"], UInt),
    ("verification_failure_count", &["metrics", "root", "quality_proxies", "verification_failure_count"], UInt),
    ("verification_unresolved_count", &["metrics", "root", "quality_proxies", "verification_unresolved_count"], UInt),
    ("tool_call_count", &["metrics", "root", "quality_proxies", "tool_call_count"], UInt),
    ("short_message_count", &["metrics", "root", "quality_proxies", "short_message_count"], UInt),
    ("broad_scope_message_count", &["metrics", "root", "quality_proxies", "broad_scope_message_count"], UInt),
    ("nonzero_exit_count", &["metrics", "root", "quality_proxies", "failure_signals", "nonzero_exit"], UInt),
    ("timeout_count", &["metrics", "root", "quality_proxies", "failure_signals", "timeout"], UInt),
    ("rejected_count", &["metrics", "root", "quality_proxies", "failure_signals", "rejected"], UInt),
    ("exact_repeated_call_groups", &["metrics", "root", "quality_proxies", "exact_repeated_call_groups"], UInt),
    ("calls_in_exact_repeated_groups", &["metrics", "root", "quality_proxies", "calls_in_exact_repeated_groups"], UInt),
    ("boundary_review_recommended", &["metrics", "root", "quality_proxies", "task_boundary_review_recommended"], Bool),
    ("long_lived_root_session", &["metrics", "root", "quality_proxies", "long_lived_root_session"], Bool),
    ("boundary_review_root_count", &["metrics", "root", "quality_proxies", "boundary_review_root_count"], UInt),
    ("long_lived_root_count", &["metrics", "root", "quality_proxies", "long_lived_root_count"], UInt),
    ("delegated_total_tokens", &["metrics", "delegated", "usage", "total_tokens"], UInt),
    ("guardian_total_tokens", &["metrics", "guardian", "usage", "total_tokens"], UInt),
    ("guardian_review_count", &["metrics", "guardian", "review_count"], UInt),
    ("guardian_workspace_attributed_review_count", &["metrics", "guardian", "signals", "workspace_attributed_review_count"], UInt),
    ("guardian_workspace_attribution_coverage", &["metrics", "guardian", "signals", "workspace_attribution_coverage"], NullableFloat),
    ("consent_receipt_id", &["consent", "receipt_id"], Uuid),
];

fn at<'a>(event: &'a Value, path: &[&str]) -> &'a Value {
    path.iter()
        .fold(event, |value, key| value.get(*key).unwrap_or(&Value::Null))
}

pub(super) fn row(event: &Value) -> Result<Map<String, Value>, ApiError> {
    FIELDS
        .iter()
        .map(|(column, path, kind)| {
            let value = at(event, path);
            let projected = match kind {
                UInt => Value::from(value.as_u64().unwrap_or(0)),
                Bool => Value::from(u8::from(value.as_bool().unwrap_or(false))),
                Sum(other) => Value::from(checked_u32_sum(
                    value.as_u64().unwrap_or(0),
                    at(event, other).as_u64().unwrap_or(0),
                )?),
                ArrayText(key) | ArrayUInt(key) => Value::Array(
                    value
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|item| item[*key].clone())
                        .collect(),
                ),
                _ => value.clone(),
            };
            Ok(((*column).to_owned(), projected))
        })
        .collect()
}

// All paths and columns are compile-time identifiers, never request input.
fn arguments(path: &[&str]) -> String {
    path.iter()
        .map(|key| format!("'{key}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn extract(function: &str, path: &[&str]) -> String {
    format!("{function}(payload_json, {})", arguments(path))
}

pub(super) fn consistency_sql() -> String {
    let mut checks = vec![
        "isValidJSON(payload_json)".to_owned(),
        "schema_version = 5".to_owned(),
    ];
    for (column, path, kind) in FIELDS {
        let expected = match kind {
            UInt => extract("JSONExtractUInt", path),
            Text | Uuid => extract("JSONExtractString", path),
            Bool => extract("JSONExtractBool", path),
            NullableFloat => format!(
                "JSONExtract(payload_json, {}, 'Nullable(Float64)')",
                arguments(path)
            ),
            Time | NullableTime => format!(
                "parseDateTime64BestEffortOrNull({}, 3, 'UTC')",
                extract("JSONExtractString", path)
            ),
            Sum(other) => format!(
                "({} + {})",
                extract("JSONExtractUInt", path),
                extract("JSONExtractUInt", other)
            ),
            ArrayText(key) => format!(
                "arrayMap(item -> JSONExtractString(item, '{key}'), {})",
                extract("JSONExtractArrayRaw", path)
            ),
            ArrayUInt(key) => format!(
                "arrayMap(item -> JSONExtractUInt(item, '{key}'), {})",
                extract("JSONExtractArrayRaw", path)
            ),
        };
        let actual = if matches!(kind, Uuid) {
            format!("toString({column})")
        } else {
            (*column).to_owned()
        };
        checks.push(if matches!(kind, NullableFloat | NullableTime) {
            format!("if(isNull({actual}), isNull({expected}), ifNull({actual} = {expected}, 0))")
        } else {
            format!("ifNull({actual} = {expected}, 0)")
        });
    }
    checks.join(" AND ")
}
