//! A deterministic readout of one audit; never infer units or live runtime proof.
use serde_json::{Value, json};

fn count(audit: &Value, pointer: &str) -> Option<u64> {
    audit.pointer(pointer).and_then(Value::as_u64)
}
fn shown(value: Option<u64>) -> String {
    value
        .map(|n| n.to_string())
        .unwrap_or_else(|| "미관측".to_owned())
}

fn reasons(audit: &Value, pointer: &str, labels: &[(&str, &str)]) -> String {
    let Some(values) = audit.pointer(pointer).and_then(Value::as_object) else {
        return "사유 미집계".to_owned();
    };
    let parts = labels
        .iter()
        .filter_map(|(key, label)| {
            values
                .get(*key)
                .and_then(Value::as_u64)
                .filter(|n| *n > 0)
                .map(|n| format!("{label} {n}건"))
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "없음".to_owned()
    } else {
        parts.join(", ")
    }
}

fn availability(audit: &Value, key: &str) -> &'static str {
    match audit
        .pointer(&format!("/guardian/availability/{key}"))
        .and_then(Value::as_bool)
    {
        Some(true) => "관측",
        Some(false) => "미관측",
        None => "미확인",
    }
}

pub fn review(audit: Value) -> Result<Value, crate::ContractError> {
    if audit["kind"] != "groundline-codex-weekly-audit"
        || audit["schema"] != 1
        || [
            "raw_content_emitted",
            "private_paths_emitted",
            "thread_ids_emitted",
            "rollout_paths_emitted",
            "secret_value_printed",
        ]
        .iter()
        .any(|field| audit[*field] != false)
    {
        return Err(crate::ContractError(
            "unsafe_weekly_review_input".to_owned(),
        ));
    }
    let recommendation = crate::efficiency::recommend_weekly_optimization(&audit).unwrap_or_else(
        |_| json!({"status":"FAIL","reason_code":"weekly_recommendation_unavailable"}),
    );
    let roots = count(&audit, "/scope/completed_root_sample_count");
    let root = &audit["root"];
    let turns = count(root, "/task_latency/completed_count");
    let observed = count(root, "/provider_reported_usage/rollout_count_with_usage");
    let selected = count(root, "/coverage/rollout_count");
    let tokens = count(root, "/provider_reported_usage/total_tokens");
    let cumulative = count(root, "/provider_reported_usage/cumulative_rollout_count");
    let responses = count(root, "/provider_reported_usage/response_rollout_count");
    let last_usage = count(
        root,
        "/provider_reported_usage/last_usage_fallback_rollout_count",
    );
    let excluded = count(root, "/scope_exclusion_count");
    let issues = count(root, "/collection_issue_count");
    let success = count(root, "/tools/verification_success_count");
    let failure = count(root, "/tools/verification_failure_count");
    let unresolved = count(root, "/tools/verification_unresolved_count");
    let recovered = count(root, "/tools/verification_recovered_by_poll_count");
    let unclassified_commands = count(root, "/tools/unclassified_command_call_count");
    // These store-level failures never reached the root parser. Keep their
    // denominators separate instead of hiding them behind its zero issue count.
    let unreadable_roots = count(&audit, "/scope/unreadable_completed_root_count");
    let unreadable_delegated = count(&audit, "/scope/unreadable_delegated_count");
    let unreadable_guardian = count(&audit, "/scope/unreadable_guardian_count");
    let unclassified_roots = count(&audit, "/scope/originator_unclassified_excluded_root_count");
    let collection_coverage = format!(
        "별도 읽기 실패: 루트 {}개 · 위임 {}개 · Guardian {}개 · 실행 출처 미분류 {}개\n검증 호출 분류 범위 밖 명령 {}건 (검증 성공·실패·미확정 건수에 포함되지 않음)",
        shown(unreadable_roots),
        shown(unreadable_delegated),
        shown(unreadable_guardian),
        shown(unclassified_roots),
        shown(unclassified_commands)
    );
    let verification_reasons = reasons(
        root,
        "/tools/verification_unresolved_reasons",
        &[
            ("missing_call_id", "호출 식별자 없음"),
            ("missing_output", "응답 없음"),
            ("pending_completion", "실행 종료 미확인"),
            ("running_without_handle", "실행 식별자 없음"),
            ("status_metadata_unavailable", "종료 상태 없음"),
            ("mixed_batch_evidence", "일괄 결과 일부 미확인"),
            ("ambiguous_call_id", "호출 식별자 충돌"),
            ("ambiguous_process_handle", "실행 식별자 충돌"),
            ("conflicting_terminal_results", "종료 결과 충돌"),
            ("correlation_budget_exceeded", "연결 예산 초과"),
            ("unmatched_result_shape", "호출과 결과 형식 불일치"),
            ("unobserved_native_call", "출력되지 않은 호출"),
            ("verification_target_unavailable", "검증 대상 명령 미확인"),
        ],
    );
    let usage_reasons = reasons(
        root,
        "/provider_reported_usage/missing_usage_reasons",
        &[
            ("unsupported_ownership_boundary", "소유권 경계 미확인"),
            ("no_owned_usage_observed", "소유 사용량 원본 미관측"),
        ],
    );
    let guardian = format!(
        "결과 {} · 위험 {} · 추론 강도 {} · 작업공간 귀속 {}",
        availability(&audit, "outcomes"),
        availability(&audit, "risk_levels"),
        availability(&audit, "reviewer_effort"),
        availability(&audit, "workspace_attribution")
    );
    let recommendation_pass = recommendation["status"] == "PASS";
    let audit_status = match audit["status"].as_str() {
        Some("PASS") => "PASS",
        Some("PARTIAL") => "PARTIAL",
        Some("INSUFFICIENT_EVIDENCE") => "INSUFFICIENT_EVIDENCE",
        _ => "UNVERIFIED",
    };
    let source = match root
        .pointer("/provider_reported_usage/source")
        .and_then(Value::as_str)
    {
        Some("codex-cumulative-window-delta") => "공급자 누계의 기간 차이",
        Some("codex-response-usage-records") => "소유권이 확인된 공급자 응답 사용량",
        Some("codex-mixed-usage-sources" | "codex-window-delta-and-last-usage-fallback") => {
            "공급자 보고 사용량의 혼합 집계"
        }
        Some("codex-last-usage-events-summed-window") => "최근 사용량 이벤트의 대체 집계",
        _ => "출처 미확인",
    };
    let advice = if recommendation_pass
        && recommendation
            .pointer("/recommended_change/code")
            .and_then(Value::as_str)
            == Some("preserve_current_workflow")
    {
        "현재 작업 흐름 유지. 관련 증거가 바뀔 때 재평가하며 설정은 자동 변경하지 않습니다."
    } else if recommendation_pass {
        "별도 recommendation의 단일 권고와 근거를 확인하세요. 자동 적용은 하지 않습니다."
    } else {
        "추천 생성 실패. 확보한 감사 결과는 보존했습니다."
    };
    let report = format!(
        "주간 감사 {audit_status} · 추천 {}\n완료 루트 작업 표본 {}개 · 완료 턴 {}개\n사용량 관측 {}/{}개 rollout · {} 토큰 ({source}; 청구액 추정 아님)\n사용량 출처: 누계 {}개 · 소유 응답 {}개 · 최근 사용량 대체 {}개\n사용량 미관측 사유: {usage_reasons}\n알려진 범위 제외 {}건 · 읽힌 루트 내부 수집 문제 {}건\n{collection_coverage}\n검증: 성공 {}건 · 실패 {}건 · 미확정 {}건 · 후속 대기로 복구 {}건\n검증 미확정 사유: {verification_reasons}\nGuardian: {guardian}\n{advice}\n이 보고서는 설치 무결성, 실제 훅, worker, 서버 수신 또는 예약 트리거의 실행을 증명하지 않습니다.",
        if recommendation_pass { "PASS" } else { "FAIL" },
        shown(roots),
        shown(turns),
        shown(observed),
        shown(selected),
        shown(tokens),
        shown(cumulative),
        shown(responses),
        shown(last_usage),
        shown(excluded),
        shown(issues),
        shown(success),
        shown(failure),
        shown(unresolved),
        shown(recovered)
    );
    Ok(json!({
        "schema":1,"kind":"groundline-codex-weekly-review",
        "status":if recommendation_pass {audit_status} else {"PARTIAL"},
        "execution":{"audit_runs":1,"recommendation_runs":1,"audit_completed":true,"recommendation_completed":recommendation_pass},
        "readout":{
            "completed_root_task_count":roots,"completed_turn_count":turns,
            "usage_observed_rollouts":observed,"usage_selected_rollouts":selected,
            "scope_exclusion_count":excluded,"collection_issue_count":issues,
            "verification_success_count":success,"verification_failure_count":failure,"verification_unresolved_count":unresolved,
            "verification_recovered_by_poll_count":recovered,
            "unclassified_command_call_count":unclassified_commands,
            "unreadable_root_count":unreadable_roots,"unreadable_delegated_count":unreadable_delegated,
            "unreadable_guardian_count":unreadable_guardian,"originator_unclassified_root_count":unclassified_roots,
            "billing_inference_performed":false,"live_runtime_verified":false,
        },
        "report_ko":report,"audit":audit,"recommendation":recommendation,
        "mutation_performed":false,"raw_content_emitted":false,"private_paths_emitted":false,"thread_ids_emitted":false,"secret_value_printed":false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_turn_units_unknowns_and_failed_recommendation_evidence() {
        // Incomplete input deliberately fails recommendation but remains readable.
        let audit = json!({"kind":"groundline-codex-weekly-audit","schema":1,"raw_content_emitted":false,"private_paths_emitted":false,"thread_ids_emitted":false,"rollout_paths_emitted":false,"secret_value_printed":false,"status":"PARTIAL","scope":{"completed_root_sample_count":14},"root":{"activity":{"task_completed":128},"task_latency":{"completed_count":128},"coverage":{"rollout_count":14},"provider_reported_usage":{"rollout_count_with_usage":12,"source":"codex-mixed-usage-sources","total_tokens":100,"cumulative_rollout_count":11,"response_rollout_count":1,"last_usage_fallback_rollout_count":0},"scope_exclusion_count":2,"collection_issue_count":1}});
        let result = review(audit.clone()).unwrap();
        assert_eq!(result["audit"], audit);
        assert_eq!(result["execution"]["audit_runs"], 1);
        assert_eq!(result["execution"]["recommendation_runs"], 1);
        assert_eq!(result["recommendation"]["status"], "FAIL");
        assert_eq!(result["readout"]["completed_root_task_count"], 14);
        assert_eq!(result["readout"]["completed_turn_count"], 128);
        assert!(result["readout"]["verification_success_count"].is_null());
        assert!(result["readout"]["unreadable_root_count"].is_null());
        let text = result["report_ko"].as_str().unwrap();
        assert!(text.contains("완료 루트 작업 표본 14개 · 완료 턴 128개"));
        assert!(text.contains("혼합 집계"));
        assert!(!text.contains("정확 사용량"));
        assert!(text.contains("성공 미관측건"));
        let mut unsafe_audit = audit;
        unsafe_audit["raw_content_emitted"] = json!(true);
        assert!(review(unsafe_audit).is_err());
    }

    #[test]
    fn store_read_failures_and_unclassified_commands_are_not_zero_collection_proof() {
        let audit = json!({"kind":"groundline-codex-weekly-audit","schema":1,"raw_content_emitted":false,"private_paths_emitted":false,"thread_ids_emitted":false,"rollout_paths_emitted":false,"secret_value_printed":false,"status":"PARTIAL","scope":{"unreadable_completed_root_count":3,"unreadable_delegated_count":1,"unreadable_guardian_count":2,"originator_unclassified_excluded_root_count":4},"root":{"collection_issue_count":0,"tools":{"unclassified_command_call_count":6}}});
        let result = review(audit).unwrap();
        assert_eq!(result["readout"]["collection_issue_count"], 0);
        assert_eq!(result["readout"]["unreadable_root_count"], 3);
        assert_eq!(result["readout"]["unreadable_delegated_count"], 1);
        assert_eq!(result["readout"]["unreadable_guardian_count"], 2);
        assert_eq!(result["readout"]["originator_unclassified_root_count"], 4);
        assert_eq!(result["readout"]["unclassified_command_call_count"], 6);
        let text = result["report_ko"].as_str().unwrap();
        assert!(text.contains("읽힌 루트 내부 수집 문제 0건"));
        assert!(text.contains("별도 읽기 실패: 루트 3개 · 위임 1개 · Guardian 2개"));
        assert!(!text.contains("실제 수집 문제 0건"));
    }
}
