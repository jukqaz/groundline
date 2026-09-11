use serde_json::{Map, Value, json};

use crate::ContractError;

const PHASES: &[&str] = &[
    "collect",
    "synthesize",
    "freeze",
    "implement",
    "verify",
    "release",
];

const GOAL_STATUSES: &[&str] = &[
    "none",
    "active",
    "paused",
    "blocked",
    "usageLimited",
    "budgetLimited",
    "complete",
];

fn boolean(map: &Map<String, Value>, name: &str) -> bool {
    map.get(name).and_then(Value::as_bool) == Some(true)
}

fn non_negative_int(map: &Map<String, Value>, name: &str) -> u64 {
    map.get(name).and_then(Value::as_u64).unwrap_or(0)
}

fn next_phase<'a>(phase: &'a str, signals: &Map<String, Value>) -> &'a str {
    match phase {
        "collect" if boolean(signals, "freeze_requested") => "synthesize",
        "synthesize" if boolean(signals, "scope_ready") => "freeze",
        "freeze" if boolean(signals, "scope_locked") => "implement",
        "implement" if boolean(signals, "implementation_complete") => "verify",
        "verify"
            if boolean(signals, "verification_complete")
                && boolean(signals, "live_proof_required") =>
        {
            "release"
        }
        "verify" if boolean(signals, "verification_complete") => "complete",
        "release" if boolean(signals, "live_proof_complete") => "complete",
        _ => phase,
    }
}

fn boundary(signals: &Map<String, Value>) -> (&'static str, &'static str) {
    if boolean(signals, "new_task_requested") {
        return ("new_task", "the user explicitly requested a new task");
    }
    if boolean(signals, "fork_requested") {
        return ("fork", "the user explicitly requested a fork");
    }
    if [
        "primary_outcome_changed",
        "repository_changed",
        "permission_boundary_changed",
    ]
    .into_iter()
    .any(|name| boolean(signals, name))
    {
        return (
            "reconcile",
            "reconcile the changed scope or authority within the current task",
        );
    }
    if boolean(signals, "alternative_approach") {
        return (
            "stay",
            "revise the approach in the current task and reuse valid evidence",
        );
    }
    if boolean(signals, "side_question") {
        return (
            "side",
            "answer the side question, then resume the current task",
        );
    }
    if boolean(signals, "context_pressure") {
        return (
            "compact_packet",
            "context pressure is high but the primary outcome is unchanged",
        );
    }
    (
        "stay",
        "the current batch boundary still matches the primary outcome",
    )
}

pub fn assess(packet: &Value) -> Result<Value, ContractError> {
    let packet = packet
        .as_object()
        .ok_or_else(|| ContractError("input_not_object".to_owned()))?;
    if packet.get("kind").and_then(Value::as_str) != Some("groundline-batch-input")
        || packet.get("schema").and_then(Value::as_u64) != Some(1)
    {
        return Err(ContractError("unsupported_batch_input".to_owned()));
    }
    let phase = packet
        .get("phase")
        .and_then(Value::as_str)
        .filter(|value| PHASES.contains(value))
        .ok_or_else(|| ContractError("invalid_phase".to_owned()))?;
    let goal = packet
        .get("goal")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("missing_goal_or_signals".to_owned()))?;
    let signals = packet
        .get("signals")
        .and_then(Value::as_object)
        .ok_or_else(|| ContractError("missing_goal_or_signals".to_owned()))?;
    let goal_status = goal.get("status").and_then(Value::as_str).unwrap_or("none");
    if !GOAL_STATUSES.contains(&goal_status) {
        return Err(ContractError("invalid_goal_status".to_owned()));
    }

    let user_requested = boolean(goal, "user_requested");
    let objective_present = boolean(goal, "objective_present");
    let (status, mut goal_action) = if user_requested && !objective_present {
        ("BLOCKED", "request_objective")
    } else if user_requested && goal_status == "none" {
        ("PASS", "create")
    } else if matches!(goal_status, "active" | "paused") {
        ("PASS", "view")
    } else {
        ("PASS", "none")
    };

    let mut recommended_phase = next_phase(phase, signals);
    let (batch_boundary, boundary_reason) = boundary(signals);
    if matches!(batch_boundary, "reconcile" | "new_task" | "fork") {
        recommended_phase = "collect";
    }
    if status == "BLOCKED" {
        recommended_phase = phase;
    } else if recommended_phase == "complete" && goal_status == "active" {
        goal_action = "complete";
    }

    Ok(json!({
        "kind": "groundline-batch-assessment",
        "schema": 1,
        "status": status,
        "current_phase": phase,
        "recommended_phase": recommended_phase,
        "boundary": batch_boundary,
        "boundary_reason": boundary_reason,
        "goal": {
            "status": goal_status,
            "objective_present": objective_present,
            "user_requested": user_requested,
            "action": goal_action,
            "provider_owned": true,
        },
        "new_observation_count": non_negative_int(signals, "new_observations"),
        "mutation_performed": false,
        "settings_changed": false,
        "raw_content_emitted": false,
        "advisory_only": true,
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::assess;

    #[test]
    fn moves_a_frozen_scope_to_implementation() {
        let result = assess(&json!({
            "kind": "groundline-batch-input",
            "schema": 1,
            "phase": "freeze",
            "goal": {
                "status": "none",
                "objective_present": true,
                "user_requested": false
            },
            "signals": {"scope_locked": true, "new_observations": 2}
        }))
        .expect("valid packet");
        assert_eq!(result["status"], "PASS");
        assert_eq!(result["recommended_phase"], "implement");
        assert_eq!(result["new_observation_count"], 2);
    }

    #[test]
    fn outcome_change_reconciles_in_the_current_task() {
        let result = assess(&json!({
            "kind": "groundline-batch-input",
            "schema": 1,
            "phase": "collect",
            "goal": {},
            "signals": {"primary_outcome_changed": true}
        }))
        .expect("valid packet");
        assert_eq!(result["boundary"], "reconcile");
        assert_eq!(result["recommended_phase"], "collect");
    }

    #[test]
    fn requested_goal_without_objective_is_blocked() {
        let result = assess(&json!({
            "kind": "groundline-batch-input",
            "schema": 1,
            "phase": "collect",
            "goal": {"user_requested": true},
            "signals": {}
        }))
        .expect("valid packet");
        assert_eq!(result["status"], "BLOCKED");
        assert_eq!(result["goal"]["action"], "request_objective");
    }

    #[test]
    fn boundary_changes_cannot_complete_the_previous_goal() {
        for signal in [
            "primary_outcome_changed",
            "repository_changed",
            "permission_boundary_changed",
        ] {
            let result = assess(&json!({
                "kind": "groundline-batch-input", "schema": 1,
                "phase": "verify",
                "goal": {"status": "active", "objective_present": true},
                "signals": {signal: true, "verification_complete": true}
            }))
            .unwrap();
            assert_eq!(result["boundary"], "reconcile", "{signal}");
            assert_eq!(result["recommended_phase"], "collect", "{signal}");
            assert_eq!(result["goal"]["action"], "view", "{signal}");
        }
    }

    #[test]
    fn alternatives_questions_and_compaction_preserve_the_task() {
        for (signal, boundary) in [
            ("alternative_approach", "stay"),
            ("side_question", "side"),
            ("context_pressure", "compact_packet"),
        ] {
            let result = assess(&json!({
                "kind": "groundline-batch-input", "schema": 1,
                "phase": "implement", "goal": {}, "signals": {signal: true}
            }))
            .unwrap();
            assert_eq!(result["boundary"], boundary);
            assert_eq!(result["recommended_phase"], "implement");
            assert_eq!(result["goal"]["action"], "none");
            assert_eq!(result["mutation_performed"], false);
        }
    }

    #[test]
    fn new_tasks_and_forks_require_explicit_requests() {
        for (signal, boundary) in [
            ("new_task_requested", "new_task"),
            ("fork_requested", "fork"),
        ] {
            let result = assess(&json!({
                "kind": "groundline-batch-input", "schema": 1,
                "phase": "verify", "goal": {},
                "signals": {signal: true, "verification_complete": true}
            }))
            .unwrap();
            assert_eq!(result["boundary"], boundary);
            assert_eq!(result["recommended_phase"], "collect");
            assert_eq!(result["advisory_only"], true);
        }
    }

    #[test]
    fn missing_objective_cannot_recommend_completion() {
        let result = assess(&json!({
            "kind": "groundline-batch-input", "schema": 1, "phase": "verify",
            "goal": {"user_requested": true, "status": "active"},
            "signals": {"verification_complete": true}
        }))
        .unwrap();
        assert_eq!(result["status"], "BLOCKED");
        assert_eq!(result["recommended_phase"], "verify");
        assert_eq!(result["goal"]["action"], "request_objective");
    }
}
