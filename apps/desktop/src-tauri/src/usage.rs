use chrono::{DateTime, Local, TimeZone, Utc};
use groundline_runtime::audit_store;
use serde_json::{Value, json};
use std::path::Path;

pub fn summarize(audit: &Value, start: DateTime<Utc>, end: DateTime<Utc>) -> Value {
    let sum = |name: &str| -> Option<u64> {
        ["root", "delegated", "guardian"]
            .iter()
            .try_fold(0_u64, |sum, component| {
                let usage = &audit[component]["provider_reported_usage"];
                let count = usage["rollout_count_with_usage"].as_u64().unwrap_or(0);
                let value = if count == 0 { 0 } else { usage[name].as_u64()? };
                sum.checked_add(value)
            })
    };
    let input = sum("input_tokens");
    let cached = sum("cached_input_tokens");
    let cache_ratio = input.zip(cached).and_then(|(input, cached)| {
        (input > 0 && cached <= input).then(|| cached as f64 / input as f64)
    });
    json!({
        "start_utc":start.to_rfc3339(),"end_utc":end.to_rfc3339(),
        "complete":audit["collection_complete"] == true,
        "root_count":audit.pointer("/scope/observed_root_sample_count").and_then(Value::as_u64),
        "total_tokens":sum("total_tokens"),"input_tokens":input,"cached_input_tokens":cached,
        "cache_ratio":cache_ratio,"source":"local_codex_activity",
        "billing_inference_performed":false,"network_performed":false
    })
}

pub fn load(home: &Path, runtime: &str, period: &str) -> Result<Value, String> {
    let days = match period {
        "today" => 0,
        "week" => 6,
        _ => return Err("invalid_usage_period".into()),
    };
    if !matches!(runtime, "codex_app" | "codex_cli") {
        return Err("invalid_runtime".into());
    }
    let local = Local::now();
    let date = local
        .date_naive()
        .checked_sub_days(chrono::Days::new(days))
        .ok_or("invalid_usage_period")?;
    let start = Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).ok_or("invalid_usage_period")?)
        .earliest()
        .ok_or("invalid_usage_period")?
        .with_timezone(&Utc);
    let end = local.with_timezone(&Utc);
    let audit = audit_store::collect_audit(home, start, end, Some(runtime), false)
        .map_err(|_| "usage_unavailable")?;
    Ok(summarize(&audit, start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sums_disjoint_roles_and_uses_total_input_as_cache_denominator() {
        let audit = json!({"collection_complete":false,"scope":{"observed_root_sample_count":3},
            "root":{"provider_reported_usage":{"rollout_count_with_usage":2,"total_tokens":100,"input_tokens":80,"cached_input_tokens":40}},
            "delegated":{"provider_reported_usage":{"rollout_count_with_usage":1,"total_tokens":50,"input_tokens":20,"cached_input_tokens":10}},
            "guardian":{"provider_reported_usage":{"rollout_count_with_usage":0}},"private_field":"must not escape"});
        let value = summarize(&audit, Utc::now(), Utc::now());
        assert_eq!(value["total_tokens"], 150);
        assert_eq!(value["cache_ratio"], 0.5);
        assert_eq!(value["complete"], false);
        assert!(!value.to_string().contains("must not escape"));
        let mut missing = audit;
        missing["root"]["provider_reported_usage"]["total_tokens"] = Value::Null;
        assert!(summarize(&missing, Utc::now(), Utc::now())["total_tokens"].is_null());
    }
    #[test]
    fn empty_input_has_no_cache_percentage() {
        assert!(summarize(&json!({}), Utc::now(), Utc::now())["cache_ratio"].is_null());
    }
}
