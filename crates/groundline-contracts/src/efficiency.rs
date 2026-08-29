use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Map, Value, json};

use crate::ContractError;

const METRIC_NAMES: &[&str] = &[
    "input_tokens",
    "cached_input_tokens",
    "non_cached_input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
    "total_tokens",
    "tool_calls",
    "compactions",
    "failure_signals",
    "long_turns",
];

const CHRONICLE_SIGNAL_FIELDS: &[&str] = &[
    "goal_switches",
    "implementation_restarts",
    "user_corrections",
    "app_context_switches",
    "completed_outcome_observations",
];

const COMPARISON_COHORT_FIELDS: &[&str] = &[
    "schema_version",
    "groundline_version",
    "os_family",
    "runtime_family",
    "execution_mode",
    "model_family",
    "effort",
];

const COMPARISON_METRIC_FIELDS: &[&str] = &[
    "tokens_per_completed_root",
    "compactions_per_root",
    "compactions_per_completed_turn",
    "long_turn_ratio",
    "repeated_call_ratio",
    "failed_call_ratio",
    "verification_success_ratio",
    "verification_outcome_coverage",
    "broad_scope_ratio",
    "wall_turn_p90_ms",
];

const RATIO_METRIC_FIELDS: &[&str] = &[
    "long_turn_ratio",
    "repeated_call_ratio",
    "failed_call_ratio",
    "verification_success_ratio",
    "verification_outcome_coverage",
    "broad_scope_ratio",
];

const OPTIMIZATION_ACTIONS: &[(&str, &[&str])] = &[
    (
        "collect_synthesize_freeze",
        &[
            "collect_observations",
            "synthesize_once",
            "freeze_one_batch",
            "finish_frozen_batch",
            "defer_nonblocking_additions",
            "open_new_task_on_outcome_change",
        ],
    ),
    (
        "diagnose_before_retry",
        &[
            "stop_same_condition_after_two_failures",
            "require_new_evidence_before_retry",
            "keep_verification_bounded",
        ],
    ),
    (
        "measure_outcomes_before_effort_change",
        &[
            "keep_active_task_model_unchanged",
            "record_task_shape_and_outcome",
            "compare_effort_at_next_task_boundary",
        ],
    ),
    (
        "preserve_current_workflow",
        &[
            "keep_current_batch_boundary",
            "review_again_after_five_completed_roots",
        ],
    ),
];

struct Reductions {
    cached_input: f64,
    non_cached_input: f64,
    output: f64,
    reasoning: f64,
    tool_calls: f64,
    compactions: f64,
    failures: f64,
    long_turns: f64,
}

const SCENARIOS: &[(&str, Reductions)] = &[
    (
        "conservative",
        Reductions {
            cached_input: 0.40,
            non_cached_input: 0.10,
            output: 0.22,
            reasoning: 0.25,
            tool_calls: 0.24,
            compactions: 0.45,
            failures: 0.20,
            long_turns: 0.20,
        },
    ),
    (
        "expected",
        Reductions {
            cached_input: 0.56,
            non_cached_input: 0.22,
            output: 0.36,
            reasoning: 0.45,
            tool_calls: 0.385,
            compactions: 0.65,
            failures: 0.35,
            long_turns: 0.40,
        },
    ),
    (
        "optimistic",
        Reductions {
            cached_input: 0.69,
            non_cached_input: 0.32,
            output: 0.50,
            reasoning: 0.60,
            tool_calls: 0.5125,
            compactions: 0.80,
            failures: 0.50,
            long_turns: 0.55,
        },
    ),
];

fn object<'a>(value: &'a Value, error: &str) -> Result<&'a Map<String, Value>, ContractError> {
    value
        .as_object()
        .ok_or_else(|| ContractError(error.to_owned()))
}

fn non_negative_int(map: &Map<String, Value>, name: &str) -> u64 {
    map.get(name).and_then(Value::as_u64).unwrap_or(0)
}

fn checked_sum(values: impl IntoIterator<Item = u64>) -> Result<u64, ContractError> {
    values.into_iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| ContractError("numeric_overflow".to_owned()))
    })
}

fn reduced(value: u64, fraction: f64) -> u64 {
    ((value as f64) * (1.0 - fraction)).round_ties_even() as u64
}

fn round_to(value: f64, digits: i32) -> f64 {
    let scale = 10_f64.powi(digits);
    (value * scale).round_ties_even() / scale
}

fn bounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    round_to((numerator as f64 / denominator as f64).clamp(0.0, 1.0), 4)
}

fn normalized_utc_timestamp(value: Option<&Value>) -> Option<String> {
    let parsed = DateTime::parse_from_rfc3339(value?.as_str()?).ok()?;
    Some(
        parsed
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::AutoSi, true),
    )
}

pub fn audit_metrics(audit: &Value) -> Result<BTreeMap<&'static str, u64>, ContractError> {
    let audit = object(audit, "unsupported_codex_audit")?;
    if audit.get("kind").and_then(Value::as_str) != Some("groundline-codex-session-audit")
        || audit.get("schema").and_then(Value::as_u64) != Some(1)
    {
        return Err(ContractError("unsupported_codex_audit".to_owned()));
    }
    let usage = audit
        .get("provider_reported_usage")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("incomplete_codex_audit".to_owned()))?;
    let activity = audit
        .get("activity")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("incomplete_codex_audit".to_owned()))?;
    let tools = audit
        .get("tools")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("incomplete_codex_audit".to_owned()))?;
    let latency = audit
        .get("task_latency")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("incomplete_codex_audit".to_owned()))?;
    let failure_signals = match tools.get("failure_signals") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(value)) => value.clone(),
        Some(_) => return Err(ContractError("invalid_failure_signals".to_owned())),
    };
    let input_tokens = non_negative_int(usage, "input_tokens");
    let cached_input_tokens = non_negative_int(usage, "cached_input_tokens").min(input_tokens);

    let failure_signal_count = checked_sum(failure_signals.values().filter_map(Value::as_u64))?;
    Ok(BTreeMap::from([
        ("input_tokens", input_tokens),
        ("cached_input_tokens", cached_input_tokens),
        (
            "non_cached_input_tokens",
            input_tokens - cached_input_tokens,
        ),
        ("output_tokens", non_negative_int(usage, "output_tokens")),
        (
            "reasoning_output_tokens",
            non_negative_int(usage, "reasoning_output_tokens"),
        ),
        ("total_tokens", non_negative_int(usage, "total_tokens")),
        ("tool_calls", non_negative_int(tools, "call_count")),
        ("compactions", non_negative_int(activity, "compactions")),
        ("failure_signals", failure_signal_count),
        ("long_turns", non_negative_int(latency, "long_turn_count")),
    ]))
}

pub fn simulate(audits: &[Value]) -> Result<Value, ContractError> {
    let mut baseline = METRIC_NAMES
        .iter()
        .map(|name| (*name, 0_u64))
        .collect::<BTreeMap<_, _>>();
    for audit in audits {
        for (name, value) in audit_metrics(audit)? {
            let total = baseline.entry(name).or_default();
            *total = total
                .checked_add(value)
                .ok_or_else(|| ContractError("numeric_overflow".to_owned()))?;
        }
    }
    let computed_total = checked_sum([baseline["input_tokens"], baseline["output_tokens"]])?;
    let baseline_total = baseline["total_tokens"].max(computed_total);
    let mut scenarios = Map::new();
    for (name, reduction) in SCENARIOS {
        let cached = reduced(baseline["cached_input_tokens"], reduction.cached_input);
        let non_cached = reduced(
            baseline["non_cached_input_tokens"],
            reduction.non_cached_input,
        );
        let output = reduced(baseline["output_tokens"], reduction.output);
        let projected_total = checked_sum([cached, non_cached, output])?;
        let reduction_ratio = if baseline_total == 0 {
            0.0
        } else {
            round_to(1.0 - (projected_total as f64 / baseline_total as f64), 4)
        };
        scenarios.insert(
            (*name).to_owned(),
            json!({
                "total_tokens": projected_total,
                "total_reduction_ratio": reduction_ratio,
                "cached_input_tokens": cached,
                "non_cached_input_tokens": non_cached,
                "output_tokens": output,
                "reasoning_output_tokens": reduced(baseline["reasoning_output_tokens"], reduction.reasoning),
                "tool_calls": reduced(baseline["tool_calls"], reduction.tool_calls),
                "compactions": reduced(baseline["compactions"], reduction.compactions),
                "failure_signals": reduced(baseline["failure_signals"], reduction.failures),
                "long_turns": reduced(baseline["long_turns"], reduction.long_turns),
            }),
        );
    }
    Ok(json!({
        "kind": "groundline-efficiency-simulation",
        "schema": 1,
        "status": "PASS",
        "audit_count": audits.len(),
        "baseline": baseline,
        "scenarios": scenarios,
        "evidence_class": "counterfactual_not_measured",
        "billing_inference_performed": false,
        "quality_regression_allowed": false,
        "mutation_performed": false,
        "raw_content_emitted": false,
    }))
}

fn chronicle_signals(packet: &Value) -> Result<BTreeMap<&'static str, u64>, ContractError> {
    let packet = object(packet, "unsupported_chronicle_aggregate")?;
    if packet.get("kind").and_then(Value::as_str) != Some("groundline-chronicle-aggregate")
        || packet.get("schema").and_then(Value::as_u64) != Some(1)
    {
        return Err(ContractError("unsupported_chronicle_aggregate".to_owned()));
    }
    if packet.get("raw_content_excluded").and_then(Value::as_bool) != Some(true) {
        return Err(ContractError(
            "chronicle_raw_content_not_excluded".to_owned(),
        ));
    }
    if packet
        .get("chronicle_state_changed")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err(ContractError(
            "chronicle_state_change_not_allowed".to_owned(),
        ));
    }
    if packet
        .get("experiment_ledger_changed")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err(ContractError(
            "chronicle_ledger_change_not_allowed".to_owned(),
        ));
    }
    let signals = packet
        .get("signals")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("invalid_chronicle_signals".to_owned()))?;
    if signals.len() != CHRONICLE_SIGNAL_FIELDS.len()
        || !CHRONICLE_SIGNAL_FIELDS
            .iter()
            .all(|name| signals.contains_key(*name))
    {
        return Err(ContractError("invalid_chronicle_signals".to_owned()));
    }
    CHRONICLE_SIGNAL_FIELDS
        .iter()
        .map(|name| {
            signals
                .get(*name)
                .and_then(Value::as_u64)
                .map(|value| (*name, value))
                .ok_or_else(|| ContractError(format!("invalid_non_negative_count:{name}")))
        })
        .collect()
}

pub fn fuse(audit: &Value, chronicle: &Value) -> Result<Value, ContractError> {
    let metrics = audit_metrics(audit)?;
    let signals = chronicle_signals(chronicle)?;
    let recommendation = if signals["goal_switches"] > 0 {
        "open_new_task_at_goal_boundary"
    } else if signals["implementation_restarts"] >= 2 || signals["user_corrections"] >= 3 {
        "collect_synthesize_freeze_before_implementation"
    } else if signals["app_context_switches"] >= 5 {
        "review_task_boundary"
    } else {
        "preserve_current_boundary"
    };
    Ok(json!({
        "kind": "groundline-codex-chronicle-evidence",
        "schema": 1,
        "status": "PASS",
        "codex_usage": metrics,
        "chronicle_signals": signals,
        "recommendation": recommendation,
        "exact_usage_source": "codex_reported",
        "chronicle_role": "behavior_boundary_only",
        "token_conversion_performed": false,
        "chronicle_state_changed": false,
        "experiment_ledger_changed": false,
        "mutation_performed": false,
        "raw_content_emitted": false,
    }))
}

fn exact_keys(map: &Map<String, Value>, keys: &[&str]) -> bool {
    map.len() == keys.len() && keys.iter().all(|key| map.contains_key(*key))
}

fn required_count(
    map: &Map<String, Value>,
    field: &str,
    diagnostic: &str,
) -> Result<u64, ContractError> {
    map.get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| ContractError(format!("invalid_non_negative_count:{diagnostic}")))
}

fn required_number(
    map: &Map<String, Value>,
    field: &str,
    diagnostic: &str,
) -> Result<f64, ContractError> {
    map.get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| ContractError(format!("invalid_non_negative_number:{diagnostic}")))
}

#[derive(Clone)]
struct ComparisonSnapshot {
    cohort: Map<String, Value>,
    sample: Map<String, Value>,
    metrics: BTreeMap<String, f64>,
}

fn allowed(value: Option<&Value>, options: &[&str]) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|value| options.contains(&value))
}

fn comparison_snapshot(value: &Value, label: &str) -> Result<ComparisonSnapshot, ContractError> {
    let value = object(value, &format!("invalid_comparison_snapshot:{label}"))?;
    if !exact_keys(value, &["cohort", "sample", "metrics"]) {
        return Err(ContractError(format!(
            "invalid_comparison_snapshot:{label}"
        )));
    }
    let cohort = value
        .get("cohort")
        .and_then(Value::as_object)
        .filter(|cohort| exact_keys(cohort, COMPARISON_COHORT_FIELDS))
        .ok_or_else(|| ContractError(format!("invalid_comparison_cohort:{label}")))?;
    if !matches!(
        cohort.get("schema_version").and_then(Value::as_u64),
        Some(1..=4)
    ) {
        return Err(ContractError(format!("invalid_comparison_schema:{label}")));
    }
    let version = cohort
        .get("groundline_version")
        .and_then(Value::as_str)
        .ok_or_else(|| ContractError(format!("invalid_comparison_version:{label}")))?;
    crate::version::strict_version(version)
        .map_err(|_| ContractError(format!("invalid_comparison_version:{label}")))?;
    for (field, options) in [
        ("os_family", &["macos", "windows", "linux", "unknown"][..]),
        ("runtime_family", &["codex_app", "codex_cli", "unknown"][..]),
        (
            "execution_mode",
            &["desktop", "local_headless", "remote_headless", "unknown"][..],
        ),
        (
            "model_family",
            &["luna", "terra", "sol", "gpt-5", "other", "unknown"][..],
        ),
        (
            "effort",
            &[
                "none", "unset", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
                "unknown",
            ][..],
        ),
    ] {
        if !allowed(cohort.get(field), options) {
            return Err(ContractError(format!(
                "invalid_comparison_cohort:{label}:{field}"
            )));
        }
    }

    let sample_fields = [
        "root_count",
        "installation_count",
        "sample_sufficient",
        "unreadable_root_count",
        "fallback_rollout_count",
    ];
    let sample = value
        .get("sample")
        .and_then(Value::as_object)
        .filter(|sample| exact_keys(sample, &sample_fields))
        .ok_or_else(|| ContractError(format!("invalid_comparison_sample:{label}")))?;
    let mut normalized_sample = Map::new();
    for field in sample_fields {
        if field == "sample_sufficient" {
            let sample_sufficient = sample
                .get(field)
                .and_then(Value::as_bool)
                .ok_or_else(|| ContractError(format!("invalid_comparison_sample:{label}")))?;
            normalized_sample.insert(field.to_owned(), Value::Bool(sample_sufficient));
        } else {
            normalized_sample.insert(
                field.to_owned(),
                Value::from(required_count(sample, field, &format!("{label}.{field}"))?),
            );
        }
    }

    let metrics = value
        .get("metrics")
        .and_then(Value::as_object)
        .filter(|metrics| exact_keys(metrics, COMPARISON_METRIC_FIELDS))
        .ok_or_else(|| ContractError(format!("invalid_comparison_metrics:{label}")))?;
    let normalized_metrics = COMPARISON_METRIC_FIELDS
        .iter()
        .map(|field| {
            required_number(metrics, field, &format!("{label}.{field}"))
                .map(|value| ((*field).to_owned(), value))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    if RATIO_METRIC_FIELDS
        .iter()
        .any(|field| normalized_metrics[*field] > 1.0)
    {
        return Err(ContractError(format!("invalid_comparison_ratio:{label}")));
    }
    Ok(ComparisonSnapshot {
        cohort: cohort.clone(),
        sample: normalized_sample,
        metrics: normalized_metrics,
    })
}

pub fn compare_aggregate_periods(packet: &Value) -> Result<Value, ContractError> {
    let packet = object(packet, "invalid_comparison_input")?;
    let expected_keys = [
        "kind",
        "schema",
        "mode",
        "changed_dimension",
        "same_installation_confirmed",
        "baseline",
        "candidate",
        "privacy",
    ];
    if !exact_keys(packet, &expected_keys) {
        return Err(ContractError("invalid_comparison_input".to_owned()));
    }
    if packet.get("kind").and_then(Value::as_str) != Some("groundline-comparison-input")
        || packet.get("schema").and_then(Value::as_u64) != Some(1)
    {
        return Err(ContractError("unsupported_comparison_input".to_owned()));
    }
    let mode = packet
        .get("mode")
        .and_then(Value::as_str)
        .filter(|value| matches!(*value, "personal_longitudinal" | "cross_install"))
        .ok_or_else(|| ContractError("invalid_comparison_mode".to_owned()))?;
    let changed_dimension = packet
        .get("changed_dimension")
        .and_then(Value::as_str)
        .filter(|value| matches!(*value, "none" | "groundline_version" | "effort"))
        .ok_or_else(|| ContractError("invalid_changed_dimension".to_owned()))?;
    let same_installation = packet
        .get("same_installation_confirmed")
        .and_then(Value::as_bool)
        .ok_or_else(|| ContractError("invalid_same_installation_confirmation".to_owned()))?;
    if packet.get("privacy")
        != Some(&json!({
            "aggregate_only": true,
            "installation_ids_included": false,
            "raw_content_included": false,
            "private_paths_included": false,
        }))
    {
        return Err(ContractError("unsafe_comparison_input".to_owned()));
    }
    let baseline = comparison_snapshot(packet.get("baseline").expect("validated key"), "baseline")?;
    let candidate =
        comparison_snapshot(packet.get("candidate").expect("validated key"), "candidate")?;
    let mismatch_fields = COMPARISON_COHORT_FIELDS
        .iter()
        .filter(|field| {
            **field != changed_dimension
                && baseline.cohort.get(**field) != candidate.cohort.get(**field)
        })
        .copied()
        .collect::<Vec<_>>();
    let changed_value_differs = changed_dimension == "none"
        || baseline.cohort.get(changed_dimension) != candidate.cohort.get(changed_dimension);
    let mut reasons = Vec::new();
    let status = if !mismatch_fields.is_empty() || !changed_value_differs {
        reasons.extend(
            mismatch_fields
                .iter()
                .map(|field| format!("cohort_mismatch:{field}")),
        );
        if !changed_value_differs {
            reasons.push("declared_dimension_did_not_change".to_owned());
        }
        "COHORT_MISMATCH"
    } else {
        let (minimum_roots, minimum_installations) = if mode == "personal_longitudinal" {
            (10, 1)
        } else {
            (5, 5)
        };
        for (label, snapshot) in [("baseline", &baseline), ("candidate", &candidate)] {
            if snapshot.sample["root_count"].as_u64().unwrap_or(0) < minimum_roots {
                reasons.push(format!("insufficient_roots:{label}"));
            }
            if snapshot.sample["installation_count"].as_u64().unwrap_or(0) < minimum_installations {
                reasons.push(format!("insufficient_installations:{label}"));
            }
            if snapshot.sample["sample_sufficient"].as_bool() != Some(true) {
                reasons.push(format!("sample_not_sufficient:{label}"));
            }
            if snapshot.sample["unreadable_root_count"]
                .as_u64()
                .unwrap_or(0)
                > 0
            {
                reasons.push(format!("unreadable_roots_present:{label}"));
            }
            if snapshot.sample["fallback_rollout_count"]
                .as_u64()
                .unwrap_or(0)
                > 0
            {
                reasons.push(format!("fallback_usage_present:{label}"));
            }
        }
        if mode == "personal_longitudinal" && !same_installation {
            reasons.push("same_installation_not_confirmed".to_owned());
        }
        if mode == "personal_longitudinal"
            && [&baseline, &candidate]
                .iter()
                .any(|snapshot| snapshot.sample["installation_count"].as_u64() != Some(1))
        {
            reasons.push("personal_installation_count_not_one".to_owned());
        }
        if reasons.is_empty() {
            "READY"
        } else {
            "INSUFFICIENT"
        }
    };
    let metric_deltas = COMPARISON_METRIC_FIELDS
        .iter()
        .map(|field| {
            let before = baseline.metrics[*field];
            let after = candidate.metrics[*field];
            (
                (*field).to_owned(),
                json!({
                    "baseline": before,
                    "candidate": after,
                    "absolute_delta": round_to(after - before, 6),
                    "relative_delta": if before == 0.0 { Value::Null } else { json!(round_to((after - before) / before, 6)) },
                }),
            )
        })
        .collect::<Map<_, _>>();
    let minimum_roots = baseline.sample["root_count"]
        .as_u64()
        .unwrap_or(0)
        .min(candidate.sample["root_count"].as_u64().unwrap_or(0));
    let confidence = if status == "READY" && minimum_roots >= 30 {
        "high"
    } else if status == "READY" {
        "medium"
    } else {
        "low"
    };
    Ok(json!({
        "kind": "groundline-comparison-readiness",
        "schema": 1,
        "status": status,
        "mode": mode,
        "changed_dimension": changed_dimension,
        "confidence": confidence,
        "reason_codes": reasons,
        "metric_deltas": metric_deltas,
        "quality_contract": {
            "correlation_is_not_causation": true,
            "quality_regression_blocks_adoption": true,
            "wall_turn_latency_is_not_model_latency": true,
            "automatic_application_allowed": false,
        },
        "mutation_performed": false,
        "raw_content_emitted": false,
        "private_paths_emitted": false,
        "installation_ids_emitted": false,
    }))
}

pub fn recommend_weekly_optimization(audit: &Value) -> Result<Value, ContractError> {
    let audit = object(audit, "unsupported_weekly_audit")?;
    if audit.get("kind").and_then(Value::as_str) != Some("groundline-codex-weekly-audit")
        || audit.get("schema").and_then(Value::as_u64) != Some(1)
    {
        return Err(ContractError("unsupported_weekly_audit".to_owned()));
    }
    let audit_status = audit.get("status").and_then(Value::as_str).unwrap_or("");
    if !matches!(audit_status, "PASS" | "PARTIAL" | "INSUFFICIENT_EVIDENCE") {
        return Err(ContractError("invalid_weekly_audit_status".to_owned()));
    }
    for field in [
        "raw_content_emitted",
        "private_paths_emitted",
        "thread_ids_emitted",
        "rollout_paths_emitted",
        "secret_value_printed",
    ] {
        if audit.get(field).and_then(Value::as_bool) != Some(false) {
            return Err(ContractError(format!("unsafe_weekly_audit:{field}")));
        }
    }
    let scope = audit
        .get("scope")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("incomplete_weekly_audit".to_owned()))?;
    let root = audit
        .get("root")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("incomplete_weekly_audit".to_owned()))?;
    let generated_at = normalized_utc_timestamp(scope.get("generated_at"))
        .ok_or_else(|| ContractError("invalid_weekly_audit_timestamp".to_owned()))?;
    let component = |name: &str| {
        root.get(name)
            .and_then(Value::as_object)
            .ok_or_else(|| ContractError("incomplete_weekly_root_audit".to_owned()))
    };
    let activity = component("activity")?;
    let model_effort = component("model_effort")?;
    let latency = component("task_latency")?;
    let prompt_shape = component("prompt_shape")?;
    let tools = component("tools")?;
    let boundary = component("boundary_signals")?;

    let root_count = non_negative_int(scope, "completed_root_sample_count");
    let compactions = non_negative_int(activity, "compactions");
    let completed_turns = non_negative_int(latency, "completed_count");
    let long_turns = non_negative_int(latency, "long_turn_count").min(completed_turns);
    let call_count = non_negative_int(tools, "call_count");
    let repeated_calls = non_negative_int(tools, "calls_in_exact_repeated_groups").min(call_count);
    let failure_signals = tools
        .get("failure_signals")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let nonzero_exits = non_negative_int(&failure_signals, "nonzero_exit").min(call_count);
    let message_count = non_negative_int(activity, "user_messages_with_text");
    let short_messages = non_negative_int(prompt_shape, "short_message_count").min(message_count);
    let broad_messages =
        non_negative_int(prompt_shape, "broad_scope_message_count").min(message_count);
    let model_counts = model_effort
        .get("counts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut turn_contexts = 0_u64;
    let mut high_depth_contexts = 0_u64;
    for (label, raw_count) in model_counts {
        let count = raw_count.as_u64().unwrap_or(0);
        turn_contexts = turn_contexts
            .checked_add(count)
            .ok_or_else(|| ContractError("numeric_overflow".to_owned()))?;
        let effort = label
            .rsplit('|')
            .next()
            .unwrap_or(&label)
            .to_ascii_lowercase();
        if matches!(effort.as_str(), "xhigh" | "max" | "ultra") {
            high_depth_contexts = high_depth_contexts
                .checked_add(count)
                .ok_or_else(|| ContractError("numeric_overflow".to_owned()))?;
        }
    }
    let by_category = tools
        .get("by_category")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let verification_calls = non_negative_int(&by_category, "verification");
    let verification_success =
        non_negative_int(tools, "verification_success_count").min(verification_calls);
    let verification_failure = non_negative_int(tools, "verification_failure_count")
        .min(verification_calls.saturating_sub(verification_success));
    let verification_unresolved = verification_calls
        .saturating_sub(verification_success)
        .saturating_sub(verification_failure);
    let resolved_verification = verification_success + verification_failure;
    let boundary_review = boundary
        .get("task_boundary_review_recommended")
        .and_then(Value::as_bool)
        == Some(true);
    let signals = json!({
        "completed_root_count": root_count,
        "compactions_per_root": if root_count == 0 { 0.0 } else { round_to(compactions as f64 / root_count as f64, 4) },
        "long_turn_ratio": bounded_ratio(long_turns, completed_turns),
        "repeated_call_ratio": bounded_ratio(repeated_calls, call_count),
        "nonzero_exit_ratio": bounded_ratio(nonzero_exits, call_count),
        "short_message_ratio": bounded_ratio(short_messages, message_count),
        "broad_scope_message_count": broad_messages,
        "high_depth_effort_ratio": bounded_ratio(high_depth_contexts, turn_contexts),
        "verification_tool_calls": verification_calls,
        "verification_success_count": verification_success,
        "verification_failure_count": verification_failure,
        "verification_unresolved_count": verification_unresolved,
        "verification_outcome_coverage": bounded_ratio(resolved_verification, verification_calls),
        "verification_success_ratio": bounded_ratio(verification_success, resolved_verification),
        "task_boundary_review_recommended": boundary_review,
    });
    let evidence_quality = if audit_status == "PASS"
        && scope.get("sample_sufficient").and_then(Value::as_bool) == Some(true)
        && root_count >= 5
    {
        "sufficient"
    } else if root_count >= 5 {
        "partial"
    } else {
        "insufficient"
    };
    let bounded_partial = evidence_quality == "partial"
        && audit_status == "PARTIAL"
        && scope.get("sample_sufficient").and_then(Value::as_bool) == Some(true)
        && root_count >= 5
        && non_negative_int(scope, "root_truncated_count") > 0
        && non_negative_int(scope, "unreadable_completed_root_count") == 0
        && non_negative_int(scope, "delegated_truncated_count") == 0
        && non_negative_int(scope, "guardian_truncated_count") == 0
        && ["root", "delegated", "guardian"].iter().all(|name| {
            audit
                .get(*name)
                .and_then(Value::as_object)
                .and_then(|value| value.get("status"))
                .and_then(Value::as_str)
                == Some("PASS")
        });
    let candidate_evidence = evidence_quality == "sufficient" || bounded_partial;
    let recommendation_scope = if evidence_quality == "sufficient" {
        "weekly_completed_root_cohort"
    } else if bounded_partial {
        "selected_completed_root_sample"
    } else {
        "unavailable"
    };
    let signal_number = |name: &str| signals.get(name).and_then(Value::as_f64).unwrap_or(0.0);
    let mut candidates = Vec::new();
    if candidate_evidence {
        let boundary_pressure = boundary_review
            || signal_number("compactions_per_root") >= 2.0
            || signal_number("long_turn_ratio") >= 0.20
            || (signal_number("short_message_ratio") >= 0.50
                && broad_messages >= (root_count * 2).max(1));
        if boundary_pressure {
            candidates.push("collect_synthesize_freeze");
        }
        if signal_number("repeated_call_ratio") >= 0.10
            || signal_number("nonzero_exit_ratio") >= 0.04
        {
            candidates.push("diagnose_before_retry");
        }
        if signal_number("high_depth_effort_ratio") >= 0.80 {
            candidates.push("measure_outcomes_before_effort_change");
        }
    }
    let recommended = candidates
        .first()
        .copied()
        .unwrap_or("preserve_current_workflow");
    let confidence = if evidence_quality == "sufficient"
        && recommended == "collect_synthesize_freeze"
        && boundary_review
        && signal_number("compactions_per_root") >= 2.0
    {
        "high"
    } else if evidence_quality == "sufficient" {
        "medium"
    } else {
        "low"
    };
    let actions = OPTIMIZATION_ACTIONS
        .iter()
        .find(|(code, _)| *code == recommended)
        .map(|(_, actions)| *actions)
        .expect("known recommendation");
    Ok(json!({
        "kind": "groundline-weekly-optimization-review",
        "schema": 1,
        "status": "PASS",
        "generated_at_utc": generated_at,
        "evidence_quality": evidence_quality,
        "recommendation_scope": recommendation_scope,
        "signals": signals,
        "recommended_change": {
            "code": recommended,
            "confidence": confidence,
            "proposed_agent_actions": actions,
            "user_behavior_change_required": false,
            "user_decision_required": true,
        },
        "deferred_candidate_codes": candidates.into_iter().skip(1).collect::<Vec<_>>(),
        "quality_contract": {
            "verification_is_a_tool_call_proxy": true,
            "verification_outcome_is_a_tool_result_proxy": true,
            "direct_completion_outcome_observed": false,
            "direct_rework_observed": false,
            "model_or_effort_change_allowed": false,
            "one_candidate_only": true,
            "bounded_partial_sample_used": bounded_partial,
            "generalization_to_unselected_roots_allowed": evidence_quality == "sufficient",
        },
        "automatic_application_allowed": false,
        "weekly_review_required": true,
        "settings_changed": false,
        "mutation_performed": false,
        "raw_content_emitted": false,
        "private_paths_emitted": false,
        "thread_ids_emitted": false,
        "rollout_paths_emitted": false,
        "secret_value_printed": false,
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{compare_aggregate_periods, fuse, recommend_weekly_optimization, simulate};

    fn audit() -> serde_json::Value {
        json!({
            "kind": "groundline-codex-session-audit",
            "schema": 1,
            "provider_reported_usage": {
                "input_tokens": 1000,
                "cached_input_tokens": 700,
                "non_cached_input_tokens": 300,
                "output_tokens": 200,
                "reasoning_output_tokens": 40,
                "total_tokens": 1200
            },
            "activity": {"compactions": 2},
            "tools": {"call_count": 20, "failure_signals": {"nonzero_exit": 2}},
            "task_latency": {"long_turn_count": 3}
        })
    }

    #[test]
    fn simulation_matches_the_existing_counterfactual_contract() {
        let result = simulate(&[audit()]).expect("valid audit");
        assert_eq!(result["baseline"]["non_cached_input_tokens"], 300);
        assert_eq!(result["scenarios"]["expected"]["total_tokens"], 670);
        assert_eq!(
            result["scenarios"]["expected"]["total_reduction_ratio"],
            0.4417
        );
        assert_eq!(result["evidence_class"], "counterfactual_not_measured");
    }

    #[test]
    fn oversized_aggregate_counters_fail_without_panicking() {
        let mut overflowing = audit();
        overflowing["tools"]["failure_signals"] = json!({
            "nonzero_exit": u64::MAX,
            "timeout": 1,
        });

        assert_eq!(simulate(&[overflowing]).unwrap_err().0, "numeric_overflow");
    }

    #[test]
    fn chronicle_fusion_keeps_usage_and_behavior_roles_separate() {
        let result = fuse(
            &audit(),
            &json!({
                "kind": "groundline-chronicle-aggregate",
                "schema": 1,
                "raw_content_excluded": true,
                "chronicle_state_changed": false,
                "experiment_ledger_changed": false,
                "signals": {
                    "goal_switches": 0,
                    "implementation_restarts": 2,
                    "user_corrections": 1,
                    "app_context_switches": 3,
                    "completed_outcome_observations": 1
                }
            }),
        )
        .expect("valid evidence");
        assert_eq!(
            result["recommendation"],
            "collect_synthesize_freeze_before_implementation"
        );
        assert_eq!(result["chronicle_role"], "behavior_boundary_only");
        assert_eq!(result["token_conversion_performed"], false);
    }

    #[test]
    fn negative_chronicle_counts_are_rejected() {
        let result = fuse(
            &audit(),
            &json!({
                "kind": "groundline-chronicle-aggregate",
                "schema": 1,
                "raw_content_excluded": true,
                "chronicle_state_changed": false,
                "experiment_ledger_changed": false,
                "signals": {
                    "goal_switches": -1,
                    "implementation_restarts": 0,
                    "user_corrections": 0,
                    "app_context_switches": 0,
                    "completed_outcome_observations": 0
                }
            }),
        );
        assert_eq!(
            result.unwrap_err().0,
            "invalid_non_negative_count:goal_switches"
        );
    }

    #[test]
    fn comparison_requires_matching_cohorts_and_sufficient_samples() {
        let metrics = json!({
            "tokens_per_completed_root": 1000,
            "compactions_per_root": 2.0,
            "compactions_per_completed_turn": 0.5,
            "long_turn_ratio": 0.2,
            "repeated_call_ratio": 0.1,
            "failed_call_ratio": 0.04,
            "verification_success_ratio": 0.8,
            "verification_outcome_coverage": 1.0,
            "broad_scope_ratio": 0.3,
            "wall_turn_p90_ms": 120000
        });
        let snapshot = json!({
            "cohort": {
                "schema_version": 3,
                "groundline_version": "0.13.0",
                "os_family": "macos",
                "runtime_family": "codex_app",
                "execution_mode": "desktop",
                "model_family": "sol",
                "effort": "xhigh"
            },
            "sample": {
                "root_count": 30,
                "installation_count": 1,
                "sample_sufficient": true,
                "unreadable_root_count": 0,
                "fallback_rollout_count": 0
            },
            "metrics": metrics
        });
        let mut candidate = snapshot.clone();
        candidate["cohort"]["groundline_version"] = json!("0.13.1");
        candidate["metrics"]["compactions_per_root"] = json!(1.0);
        candidate["metrics"]["verification_success_ratio"] = json!(0.9);
        let packet = json!({
            "kind": "groundline-comparison-input",
            "schema": 1,
            "mode": "personal_longitudinal",
            "changed_dimension": "groundline_version",
            "same_installation_confirmed": true,
            "baseline": snapshot,
            "candidate": candidate,
            "privacy": {
                "aggregate_only": true,
                "installation_ids_included": false,
                "raw_content_included": false,
                "private_paths_included": false
            }
        });
        let ready = compare_aggregate_periods(&packet).expect("ready comparison");
        assert_eq!(ready["status"], "READY");
        assert_eq!(ready["confidence"], "high");
        assert_eq!(
            ready["metric_deltas"]["compactions_per_root"]["relative_delta"],
            -0.5
        );

        let mut mismatch_packet = packet.clone();
        mismatch_packet["candidate"]["cohort"]["effort"] = json!("high");
        let mismatch = compare_aggregate_periods(&mismatch_packet).unwrap();
        assert_eq!(mismatch["status"], "COHORT_MISMATCH");
        assert!(
            mismatch["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!("cohort_mismatch:effort"))
        );
    }

    #[test]
    fn weekly_recommendation_matches_the_existing_quality_contract() {
        let result = recommend_weekly_optimization(&json!({
            "kind": "groundline-codex-weekly-audit",
            "schema": 1,
            "status": "PASS",
            "scope": {
                "generated_at": "2026-08-03T00:00:00Z",
                "completed_root_sample_count": 10,
                "minimum_root_sample_count": 5,
                "sample_sufficient": true
            },
            "root": {
                "activity": {"compactions": 30, "user_messages_with_text": 20},
                "model_effort": {"counts": {"gpt-5.6-sol|xhigh": 18, "gpt-5.6-sol|high": 2}},
                "task_latency": {"completed_count": 10, "long_turn_count": 3},
                "prompt_shape": {"short_message_count": 12, "broad_scope_message_count": 10},
                "tools": {
                    "call_count": 100,
                    "by_category": {"verification": 10},
                    "verification_success_count": 7,
                    "verification_failure_count": 2,
                    "verification_unresolved_count": 1,
                    "failure_signals": {"nonzero_exit": 5},
                    "calls_in_exact_repeated_groups": 20
                },
                "boundary_signals": {"task_boundary_review_recommended": true}
            },
            "raw_content_emitted": false,
            "private_paths_emitted": false,
            "thread_ids_emitted": false,
            "rollout_paths_emitted": false,
            "secret_value_printed": false
        }))
        .expect("valid weekly audit");
        assert_eq!(
            result["recommended_change"]["code"],
            "collect_synthesize_freeze"
        );
        assert_eq!(result["recommended_change"]["confidence"], "high");
        assert_eq!(result["signals"]["verification_outcome_coverage"], 0.9);
        assert_eq!(result["signals"]["verification_success_ratio"], 0.7778);
        assert_eq!(
            result["deferred_candidate_codes"],
            json!([
                "diagnose_before_retry",
                "measure_outcomes_before_effort_change"
            ])
        );
    }
}
