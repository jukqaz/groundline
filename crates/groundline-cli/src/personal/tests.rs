use super::*;
use tempfile::TempDir;

fn model() -> (ModelEvidence, Vec<u8>) {
    let catalog=serde_json::to_vec(&json!({"models":[{"slug":"future-model-2030","supported_reasoning_levels":[{"effort":"adaptive"}]}]})).unwrap();
    let e = ModelEvidence {
        kind: "groundline-model-evidence".into(),
        schema: 1,
        checked_at_utc: Utc::now().to_rfc3339(),
        runtime_version: "0.153.4".into(),
        runtime_family: "codex_app".into(),
        selected_model: "future-model-2030".into(),
        selected_effort: "adaptive".into(),
        latest_reference_model: "future-model-2030".into(),
        catalog_sha256: hash(&catalog),
        official_sources: vec![Source {
            applies_to_model: "future-model-2030".into(),
            url: "https://developers.openai.com/api/docs/guides/latest-model".into(),
            sha256: "a".repeat(64),
            checked_at_utc: Utc::now().to_rfc3339(),
        }],
        behavior_focus: vec![Rule::ApprovalContinuity, Rule::DiagnoseBeforeRetry],
    };
    (e, catalog)
}
fn sample(context: &str, offset: i64, key: &str) -> Sample {
    let start = Utc::now() - Duration::days(offset);
    Sample {
        kind: "groundline-outcome-sample".into(),
        schema: 2,
        period_start_utc: start.to_rfc3339(),
        period_end_utc: (start + Duration::hours(1)).to_rfc3339(),
        model_context_sha256: context.into(),
        guidance_sha256: hash(b""),
        comparison_context_sha256: hash(
            b"same non-trial instructions, tools, permissions and service tier",
        ),
        task_kind: "implementation".into(),
        scope_size: "medium".into(),
        activation_evidence: None,
        units: (0..10)
            .map(|i| Unit {
                unit_hash: hash(format!("{key}{i}").as_bytes()),
                started_at_utc: (start + Duration::minutes(i * 2)).to_rfc3339(),
                completed_at_utc: (start + Duration::minutes(i * 2 + 1)).to_rfc3339(),
                outcome: Outcome::Verified,
                evidence: Evidence::RuntimeCheck,
                rework: false,
                redundant_approval_count: 1,
                continuation_prompt_count: 1,
                repeated_call_count: 3,
                tool_call_count: 10,
                total_tokens: Some(1000),
                optimization_opportunities: None,
            })
            .collect(),
    }
}
fn activate(sample: &mut Sample) {
    sample.activation_evidence = Some(ActivationEvidence {
        instruction_load_sha256: hash(b"native instruction load evidence"),
        behavior_check_sha256: hash(b"native observed behavior evidence"),
        guidance_sha256: sample.guidance_sha256.clone(),
        observed_at_utc: (time(&sample.period_start_utc).unwrap() - Duration::seconds(1))
            .to_rfc3339(),
    });
}
fn trial() -> (TempDir, ModelEvidence, Vec<u8>, Sample) {
    let root = tempdir().unwrap();
    let (e, c) = model();
    let context = model_context(&e, &c, Utc::now()).unwrap();
    let baseline = sample(&context, 4, "before");
    apply(
        root.path(),
        Rule::ApprovalContinuity,
        &baseline,
        &context,
        Utc::now() - Duration::days(3),
    )
    .unwrap();
    let mut candidate = sample(&context, 2, "after");
    candidate.guidance_sha256 = hash(render(&[Rule::ApprovalContinuity]).as_bytes());
    activate(&mut candidate);
    for unit in &mut candidate.units {
        unit.redundant_approval_count = 0;
        unit.continuation_prompt_count = 0;
    }
    (root, e, c, candidate)
}
fn put(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn audit() -> Value {
    json!({"kind":"groundline-codex-weekly-audit","schema":1,"status":"PASS",
        "raw_content_emitted":false,"private_paths_emitted":false,"thread_ids_emitted":false,"rollout_paths_emitted":false,"secret_value_printed":false,
        "scope":{"generated_at":Utc::now().to_rfc3339(),"sample_sufficient":true,"completed_root_sample_count":10},
        "root":{"activity":{"user_messages_with_text":20},"model_effort":{"counts":{}},"task_latency":{},"prompt_shape":{"short_message_count":20,"broad_scope_message_count":20},
        "tools":{"call_count":100,"calls_in_exact_repeated_groups":20},"boundary_signals":{}}})
}
#[test]
fn current_native_catalog_accepts_future_models_but_rejects_stale_or_unofficial_evidence() {
    let (mut e, c) = model();
    assert!(model_context(&e, &c, Utc::now()).is_ok());
    e.selected_effort = "unknown".into();
    assert!(model_context(&e, &c, Utc::now()).is_err());
    e.selected_effort = "adaptive".into();
    e.checked_at_utc = (Utc::now() - Duration::days(2)).to_rfc3339();
    assert!(model_context(&e, &c, Utc::now()).is_err());
    e.checked_at_utc = Utc::now().to_rfc3339();
    e.official_sources[0].url = "https://developers.openai.com.evil.test/private".into();
    assert!(model_context(&e, &c, Utc::now()).is_err());
    e.official_sources[0].url = "https://user:secret@developers.openai.com/docs".into();
    assert!(model_context(&e, &c, Utc::now()).is_err());
    let (e, c) = model();
    assert!(model_context(&e, &[c, b" ".to_vec()].concat(), Utc::now()).is_err());
}

#[test]
fn failure_candidates_preserve_partial_evidence_for_astra_and_sol_without_applying() {
    for selected in ["gpt-6-astra", "gpt-5.6-sol"] {
        for native_failure in [false, true] {
            let root = tempdir().unwrap();
            let (mut e, _) = model();
            e.selected_model = selected.into();
            e.latest_reference_model = "gpt-6-astra".into();
            e.selected_effort = "xhigh".into();
            let c = serde_json::to_vec(&json!({"models": [
                {"slug":"gpt-6-astra","supported_reasoning_levels":[{"effort":"xhigh"}]},
                {"slug":"gpt-5.6-sol","supported_reasoning_levels":[{"effort":"xhigh"}]}
            ]}))
            .unwrap();
            e.catalog_sha256 = hash(&c);
            e.official_sources = ["gpt-6-astra", "gpt-5.6-sol"]
                .map(|name| Source {
                    applies_to_model: name.into(),
                    url: format!("https://developers.openai.com/api/docs/models/{name}"),
                    ..e.official_sources[0].clone()
                })
                .to_vec();
            let mut report = report_fixture();
            report["generated_at_utc"] =
                json!(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
            report["coverage"]["root_usage_missing_event_count"] = json!(1);
            report["coverage"]["root_usage_fallback_event_count"] = json!(1);
            report["collection_health"]["quarantined_event_count"] = json!(3);
            report["data_quality"]["status"] = json!("PARTIAL");
            report["data_quality"]["reason_codes"] = json!([
                "events_quarantined",
                "usage_fallback_present",
                "usage_missing"
            ]);
            report["comparison_readiness"]["reason_codes"] =
                json!(["comparison_baseline_not_included", "data_quality_not_pass"]);
            report["cohorts"]["model_effort_context_distribution"] = json!([
                {"model_family":"astra","effort":"xhigh","context_count":2},
                {"model_family":"sol","effort":"high","context_count":3}
            ]);
            let mut audit = audit();
            audit["root"]["tools"]["calls_in_exact_repeated_groups"] = json!(0);
            if native_failure {
                audit["root"]["tools"]["failure_signals"] = json!({"nonzero_exit":4});
            } else {
                let workflow = &mut report["weekly_metrics"]["workflow"];
                workflow["tool_call_count"] = json!(100);
                workflow["failure_signal_count"] = json!(4);
                workflow["failure_signal_rate"] = json!(0.04);
                workflow["repeated_call_rate"] = json!(0.0);
            }
            let report_path = root.path().join("report.json");
            let audit_path = root.path().join("audit.json");
            put(&report_path, &report);
            put(&audit_path, &audit);
            let out = review(
                ReviewInputs {
                    report: &report_path,
                    audit: &audit_path,
                    outcomes: None,
                    state_dir: Some(root.path()),
                    apply: true,
                },
                &e,
                &c,
            )
            .unwrap();
            assert_eq!(out["candidate"]["rule"], "diagnose_before_retry");
            assert_eq!(out["status"], "OBSERVE");
            assert_eq!(out["automatic_application_eligible"], false);
            assert_eq!(out["mutation_performed"], false);
            assert_eq!(out["insights"]["coverage"], report["coverage"]);
            assert_eq!(out["insights"]["data_quality"], report["data_quality"]);
            assert_eq!(out["insights"]["quarantined_event_count"], 3);
            assert_eq!(
                out["insights"]["model_effort_context_distribution"],
                report["cohorts"]["model_effort_context_distribution"]
            );
            assert_eq!(
                out["insights"]["model_performance_attribution_available"],
                false
            );
            assert!(!root.path().join("trial.json").exists());
            assert!(!root.path().join("personal-guidance.md").exists());
        }
    }
}

#[test]
fn native_failure_threshold_requires_a_nonempty_denominator() {
    let (e, _) = model();
    let mut audit = audit();
    audit["root"]["tools"]["calls_in_exact_repeated_groups"] = json!(0);
    audit["root"]["tools"]["failure_signals"] = json!({"nonzero_exit":3});
    assert_eq!(select_rule(&e, None, &audit), None);
    audit["root"]["tools"]["failure_signals"]["nonzero_exit"] = json!(4);
    assert_eq!(
        select_rule(&e, None, &audit),
        Some(Rule::DiagnoseBeforeRetry)
    );
    audit["root"]["tools"]["call_count"] = json!(0);
    assert_eq!(select_rule(&e, None, &audit), None);
}
#[test]
fn duplicate_unknown_and_misattributed_outcomes_cannot_become_verified_samples() {
    let mut s = sample(&"a".repeat(64), 4, "unit");
    validate_sample(&s, &"a".repeat(64), Utc::now()).unwrap();
    s.units.push(s.units[0].clone());
    assert!(validate_sample(&s, &"a".repeat(64), Utc::now()).is_err());
    s.units.pop();
    s.units[0].outcome = Outcome::Unknown;
    assert!(validate_sample(&s, &"a".repeat(64), Utc::now()).is_err());
    s.units[0].evidence = Evidence::Unobserved;
    validate_sample(&s, &"a".repeat(64), Utc::now()).unwrap();
    assert!(!sufficient(&s));
    assert!(validate_sample(&s, &"b".repeat(64), Utc::now()).is_err());
    let mut raw = serde_json::to_value(s).unwrap();
    raw["raw_prompt"] = json!("PRIVATE_SENTINEL");
    assert!(serde_json::from_value::<Sample>(raw).is_err());
}
#[test]
fn insufficient_outcomes_never_write_guidance_and_short_followups_do_not_trigger_a_rule() {
    let root = tempdir().unwrap();
    let (e, c) = model();
    let context = model_context(&e, &c, Utc::now()).unwrap();
    let mut report = report_fixture();
    report["generated_at_utc"] =
        json!(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    let report_path = root.path().join("report.json");
    put(&report_path, &report);
    let audit_path = root.path().join("audit.json");
    put(&audit_path, &audit());
    let out = review(
        ReviewInputs {
            report: &report_path,
            audit: &audit_path,
            outcomes: None,
            state_dir: Some(root.path()),
            apply: true,
        },
        &e,
        &c,
    )
    .unwrap();
    assert_eq!(out["status"], "OBSERVE");
    assert_eq!(out["mutation_performed"], false);
    assert!(!root.path().join("trial.json").exists());
    let mut only_short = audit();
    only_short["root"]["tools"] = json!({});
    assert_eq!(select_rule(&e, None, &only_short), None);
    let s = sample(&context, 4, "unit");
    let outcomes = root.path().join("outcomes.json");
    put(&outcomes, &serde_json::to_value(&s).unwrap());
    let out = review(
        ReviewInputs {
            report: &report_path,
            audit: &audit_path,
            outcomes: Some(&outcomes),
            state_dir: Some(root.path()),
            apply: true,
        },
        &e,
        &c,
    )
    .unwrap();
    assert_eq!(out["status"], "READY");
    assert_eq!(out["mutation_performed"], true);
    assert_eq!(out["trial"]["native_activation"], "UNVERIFIED");
    assert!(!out.to_string().contains(&s.units[0].unit_hash));
}
#[test]
fn comparable_observed_improvement_can_be_retained_without_claiming_causation() {
    let (root, e, c, s) = trial();
    let out = evaluate(root.path(), s, &e, &c).unwrap();
    assert_eq!(out["status"], "RETAINED");
    assert_eq!(out["causal_improvement_claimed"], false);
    assert_eq!(load_trial(root.path()).unwrap().status, "retained");
    run(Command::Rollback {
        state_dir: root.path().to_owned(),
        json: true,
    })
    .unwrap();
    assert_eq!(
        fs::read(root.path().join("personal-guidance.md")).unwrap(),
        b""
    );
}
#[test]
fn lower_token_use_does_not_override_quality_regression() {
    let (root, e, c, mut s) = trial();
    s.units[0].outcome = Outcome::Failed;
    for u in &mut s.units {
        u.total_tokens = Some(10);
    }
    let out = evaluate(root.path(), s, &e, &c).unwrap();
    assert_eq!(out["status"], "ROLLED_BACK");
    assert_eq!(out["reason_codes"], json!(["quality_regression"]));
    assert!(
        fs::read(root.path().join("personal-guidance.md"))
            .unwrap()
            .is_empty()
    );
}
#[test]
fn absent_activation_overlapping_periods_reused_ids_and_cohort_changes_are_inconclusive() {
    for case in 0..5 {
        let (root, e, c, mut s) = trial();
        let baseline = load_trial(root.path()).unwrap().baseline;
        match case {
            0 => s.activation_evidence = None,
            1 => {
                s.period_start_utc = baseline.period_start_utc.clone();
                activate(&mut s);
            }
            2 => s.units[0].unit_hash = baseline.units[0].unit_hash.clone(),
            3 => s.task_kind = "review".into(),
            _ => {
                s.units.pop();
            }
        }
        let before = fs::read(root.path().join("personal-guidance.md")).unwrap();
        let out = evaluate(root.path(), s, &e, &c).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert_eq!(
            fs::read(root.path().join("personal-guidance.md")).unwrap(),
            before
        );
        assert_eq!(load_trial(root.path()).unwrap().status, "pending");
    }
}
#[test]
fn user_edits_are_preserved_and_a_failed_candidate_is_not_repeated_on_the_same_units() {
    let (root, e, c, s) = trial();
    fs::write(root.path().join("personal-guidance.md"), "USER_EDIT").unwrap();
    assert!(evaluate(root.path(), s, &e, &c).is_err());
    assert!(
        run(Command::Rollback {
            state_dir: root.path().to_owned(),
            json: true
        })
        .is_err()
    );
    assert_eq!(
        fs::read_to_string(root.path().join("personal-guidance.md")).unwrap(),
        "USER_EDIT"
    );
    let (root, e, c, _) = trial();
    let t = load_trial(root.path()).unwrap();
    run(Command::Rollback {
        state_dir: root.path().to_owned(),
        json: true,
    })
    .unwrap();
    assert!(
        apply(
            root.path(),
            t.rule,
            &t.baseline,
            &model_context(&e, &c, Utc::now()).unwrap(),
            Utc::now()
        )
        .is_err()
    );
}

#[test]
fn another_candidate_does_not_allow_reusing_an_archived_baseline() {
    let (root, e, c, _) = trial();
    let first = load_trial(root.path()).unwrap();
    run(Command::Rollback {
        state_dir: root.path().to_owned(),
        json: true,
    })
    .unwrap();
    let context = model_context(&e, &c, Utc::now()).unwrap();
    let other = sample(&context, 2, "other-candidate");
    apply(
        root.path(),
        Rule::DiagnoseBeforeRetry,
        &other,
        &context,
        Utc::now(),
    )
    .unwrap();
    run(Command::Rollback {
        state_dir: root.path().to_owned(),
        json: true,
    })
    .unwrap();
    let before_trial = fs::read(root.path().join("trial.json")).unwrap();
    let before_guidance = fs::read(root.path().join("personal-guidance.md")).unwrap();
    let error = apply(
        root.path(),
        first.rule,
        &first.baseline,
        &context,
        Utc::now(),
    )
    .unwrap_err();
    assert_eq!(error.0, "personal_candidate_already_reviewed");
    assert_eq!(
        fs::read(root.path().join("trial.json")).unwrap(),
        before_trial
    );
    assert_eq!(
        fs::read(root.path().join("personal-guidance.md")).unwrap(),
        before_guidance
    );
    let mut partial_reuse = sample(&context, 1, "fresh");
    partial_reuse.units[9].unit_hash = first.baseline.units[0].unit_hash.clone();
    assert_eq!(
        apply(
            root.path(),
            first.rule,
            &partial_reuse,
            &context,
            Utc::now()
        )
        .unwrap_err()
        .0,
        "personal_candidate_already_reviewed"
    );
    let fresh = sample(&context, 1, "fresh");
    assert_eq!(
        apply(root.path(), first.rule, &fresh, &context, Utc::now()).unwrap()["state"],
        "pending"
    );
}
#[test]
fn invalid_archived_trials_are_preserved_and_rejected_before_any_state_write() {
    for case in ["hash", "json", "schema", "active"] {
        let (root, e, c, _) = trial();
        run(Command::Rollback {
            state_dir: root.path().to_owned(),
            json: true,
        })
        .unwrap();
        let before_trial = fs::read(root.path().join("trial.json")).unwrap();
        let mut archived: Value = serde_json::from_slice(&before_trial).unwrap();
        match case {
            "schema" => archived["schema"] = json!(3),
            "active" => archived["status"] = json!("pending"),
            _ => {}
        }
        let data = if case == "json" {
            b"PRIVATE_INVALID_ARCHIVE".to_vec()
        } else {
            serde_json::to_vec(&archived).unwrap()
        };
        let name_hash = if case == "hash" {
            hash(b"wrong")
        } else {
            hash(&data)
        };
        let path = root.path().join(format!("trial-{name_hash}.json"));
        atomic_write_private(&path, &data).unwrap();
        let before_count = fs::read_dir(root.path()).unwrap().count();
        let context = model_context(&e, &c, Utc::now()).unwrap();
        let fresh = sample(&context, 1, "fresh");
        let err = apply(
            root.path(),
            Rule::DiagnoseBeforeRetry,
            &fresh,
            &context,
            Utc::now(),
        )
        .unwrap_err();
        assert_eq!(
            err.0,
            if case == "hash" {
                "personal_archive_mismatch"
            } else if case == "schema" {
                "personal_unsupported_trial_schema_preserved_restore_with_matching_cli"
            } else {
                "personal_invalid_trial"
            }
        );
        assert_eq!(fs::read(&path).unwrap(), data);
        assert_eq!(
            fs::read(root.path().join("trial.json")).unwrap(),
            before_trial
        );
        assert!(
            fs::read(root.path().join("personal-guidance.md"))
                .unwrap()
                .is_empty()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), before_count);
    }
}
#[cfg(unix)]
#[test]
fn archived_trial_links_and_public_files_cannot_bypass_history_validation() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for case in ["symlink", "hardlink", "public"] {
        let (root, e, c, _) = trial();
        run(Command::Rollback {
            state_dir: root.path().to_owned(),
            json: true,
        })
        .unwrap();
        let path = root.path().join("trial.json");
        let data = fs::read(&path).unwrap();
        let archived = root.path().join(format!("trial-{}.json", hash(&data)));
        match case {
            "symlink" => symlink(&path, &archived).unwrap(),
            "hardlink" => {
                let target = root.path().join("external.json");
                atomic_write_private(&target, &data).unwrap();
                fs::hard_link(target, &archived).unwrap();
            }
            _ => {
                atomic_write_private(&archived, &data).unwrap();
                fs::set_permissions(&archived, fs::Permissions::from_mode(0o644)).unwrap();
            }
        }
        let context = model_context(&e, &c, Utc::now()).unwrap();
        let fresh = sample(&context, 1, "fresh");
        assert!(
            apply(
                root.path(),
                Rule::DiagnoseBeforeRetry,
                &fresh,
                &context,
                Utc::now()
            )
            .is_err()
        );
        assert_eq!(fs::read(path).unwrap(), data);
        assert!(
            fs::read(root.path().join("personal-guidance.md"))
                .unwrap()
                .is_empty()
        );
    }
}
#[test]
fn full_trial_history_rejects_new_writes_without_stranding_rollback() {
    for evaluating in [false, true] {
        let (root, e, c, s) = trial();
        if !evaluating {
            run(Command::Rollback {
                state_dir: root.path().to_owned(),
                json: true,
            })
            .unwrap();
        }
        let before_trial = fs::read(root.path().join("trial.json")).unwrap();
        let before_guidance = fs::read(root.path().join("personal-guidance.md")).unwrap();
        for i in fs::read_dir(root.path()).unwrap().count()..128 {
            atomic_write_private(&root.path().join(format!("owner-record-{i}")), b"").unwrap();
        }
        let err = if evaluating {
            evaluate(root.path(), s, &e, &c).unwrap_err()
        } else {
            let context = model_context(&e, &c, Utc::now()).unwrap();
            apply(
                root.path(),
                Rule::DiagnoseBeforeRetry,
                &sample(&context, 1, "fresh"),
                &context,
                Utc::now(),
            )
            .unwrap_err()
        };
        assert_eq!(err.0, "personal_state_archive_limit");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 128);
        assert_eq!(
            fs::read(root.path().join("trial.json")).unwrap(),
            before_trial
        );
        assert_eq!(
            fs::read(root.path().join("personal-guidance.md")).unwrap(),
            before_guidance
        );
        if evaluating {
            run(Command::Rollback {
                state_dir: root.path().to_owned(),
                json: true,
            })
            .unwrap();
            assert!(
                fs::read(root.path().join("personal-guidance.md"))
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
#[test]
fn initial_trial_reserves_space_for_its_lock_journal_and_guidance() {
    for entries in [125, 126, 128] {
        let root = tempdir().unwrap();
        for i in 0..entries {
            atomic_write_private(&root.path().join(format!("owner-record-{i}")), b"").unwrap();
        }
        let (e, c) = model();
        let context = model_context(&e, &c, Utc::now()).unwrap();
        let result = apply(
            root.path(),
            Rule::ApprovalContinuity,
            &sample(&context, 1, "fresh"),
            &context,
            Utc::now(),
        );
        if entries == 125 {
            result.unwrap();
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 128);
            run(Command::Rollback {
                state_dir: root.path().to_owned(),
                json: true,
            })
            .unwrap();
        } else {
            assert_eq!(result.unwrap_err().0, "personal_state_archive_limit");
            assert!(!root.path().join("trial.json").exists());
            assert!(!root.path().join("personal-guidance.md").exists());
            assert!(fs::read_dir(root.path()).unwrap().count() <= 128);
        }
    }
}
#[test]
fn existing_archive_does_not_require_another_slot_in_full_history() {
    let (root, e, c, _) = trial();
    run(Command::Rollback {
        state_dir: root.path().to_owned(),
        json: true,
    })
    .unwrap();
    let data = fs::read(root.path().join("trial.json")).unwrap();
    archive(root.path(), "trial", &data).unwrap();
    for i in fs::read_dir(root.path()).unwrap().count()..128 {
        atomic_write_private(&root.path().join(format!("owner-record-{i}")), b"").unwrap();
    }
    let context = model_context(&e, &c, Utc::now()).unwrap();
    apply(
        root.path(),
        Rule::DiagnoseBeforeRetry,
        &sample(&context, 1, "fresh"),
        &context,
        Utc::now(),
    )
    .unwrap();
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 128);
    run(Command::Rollback {
        state_dir: root.path().to_owned(),
        json: true,
    })
    .unwrap();
}
#[test]
fn interrupted_prepared_trial_can_be_restored_before_or_after_the_guidance_write() {
    for written in [false, true] {
        let (root, _, _, _) = trial();
        let mut t = load_trial(root.path()).unwrap();
        t.status = "prepared".into();
        save_trial(root.path(), &t).unwrap();
        if !written {
            atomic_write_private(&root.path().join("personal-guidance.md"), b"").unwrap();
        }
        run(Command::Rollback {
            state_dir: root.path().to_owned(),
            json: true,
        })
        .unwrap();
        assert!(
            fs::read(root.path().join("personal-guidance.md"))
                .unwrap()
                .is_empty()
        );
    }
}
#[cfg(unix)]
#[test]
fn symlink_hardlink_public_directory_and_concurrent_trial_writes_are_rejected() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (root, _, _, _) = trial();
    let guidance = root.path().join("personal-guidance.md");
    let target = root.path().join("target.md");
    fs::rename(&guidance, &target).unwrap();
    symlink(&target, &guidance).unwrap();
    assert!(current_rules(root.path()).is_err());
    fs::remove_file(&guidance).unwrap();
    fs::hard_link(&target, &guidance).unwrap();
    assert!(current_rules(root.path()).is_err());
    fs::remove_file(&guidance).unwrap();
    fs::rename(&target, &guidance).unwrap();
    let _lock = lock(root.path()).unwrap();
    assert!(lock(root.path()).is_err());
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(state_root(root.path()).is_err());
}

fn report_fixture() -> Value {
    json!({
        "schema_version": 3,
        "kind": "groundline-insights-weekly-report",
        "status": "PASS",
        "reason_code": "accepted",
        "generated_at_utc": "2026-08-27T00:00:00Z",
        "requested_days": 7,
        "source_contract": {
            "dataset": "basic_active",
            "time_basis": "utc",
            "metric_time_field": "period_end_or_generated_at",
            "freshness_time_field": "received_at",
            "roster_source": "enrolled_installation_registry",
            "analysis_mode": "descriptive_single_period",
            "query_set_version": 3,
            "basic_aggregate_only": true
        },
        "collection_health": {
            "enrolled_installation_count": 1,
            "metadata_known_installation_count": 1,
            "metadata_unknown_installation_count": 0,
            "observed_installation_count": 1,
            "reporting_installation_count": 1,
            "recent_installation_count": 1,
            "never_reported_installation_count": 0,
            "pending_initial_report_installation_count": 0,
            "overdue_never_reported_installation_count": 0,
            "stale_observed_installation_count": 0,
            "current_package_claim_installation_count": 1,
            "current_package_claim_unobserved_installation_count": 0,
            "current_observed_installation_count": 1,
            "current_reporting_installation_count": 1,
            "current_recent_installation_count": 1,
            "policy_latest_version": "0.18.0",
            "roster_status": "AVAILABLE",
            "latest_received_at_utc": "2026-08-27T00:00:00Z",
            "freshness_status": "FRESH",
            "freshness_threshold_hours": 48,
            "initial_report_grace_hours": 24,
            "stored_event_row_count": 2,
            "deduplicated_event_count": 2,
            "duplicate_event_row_count": 0,
            "ttl_expired_event_row_count": 0,
            "quarantined_event_count": 0,
            "delayed_delivery_event_count": 0,
            "overdue_delivery_event_count": 0,
            "clock_skew_event_count": 0,
            "delivery_delay_threshold_hours": 6,
            "delivery_overdue_threshold_hours": 24,
            "clock_skew_tolerance_minutes": 5
        },
        "coverage": {
            "event_count": 2,
            "eligible_root_count": 5,
            "selected_root_count": 5,
            "observed_root_count": 5,
            "completed_turn_count": 5,
            "unreadable_root_count": 0,
            "root_truncated_count": 0,
            "non_root_truncated_count": 0,
            "originator_unclassified_count": 0,
            "originator_source_fallback_count": 0,
            "root_usage_applicable_event_count": 2,
            "root_usage_missing_event_count": 0,
            "root_usage_fallback_event_count": 0,
            "delegated_usage_applicable_event_count": 0,
            "delegated_usage_missing_event_count": 0,
            "delegated_usage_fallback_event_count": 0,
            "guardian_usage_applicable_event_count": 0,
            "guardian_usage_missing_event_count": 0,
            "guardian_usage_fallback_event_count": 0,
            "guardian_incomplete_excluded_count": 0,
            "completed_root_coverage_applicable_event_count": 2,
            "completed_root_coverage_capable_event_count": 2,
            "completed_root_selection_coverage": 1.0,
            "latency_capable_event_count": 2,
            "boundary_count_capable_event_count": 2,
            "guardian_attribution_applicable_event_count": 0,
            "guardian_attribution_capable_event_count": 0,
            "component_nonpass_event_count": 0
        },
        "weekly_metrics": {
            "tokens": {
                "input": 100,
                "cached_input": 80,
                "non_cached_input": 20,
                "output": 10,
                "reasoning_output": 2,
                "total": 110,
                "delegated_total": 0,
                "guardian_total": 0
            },
            "workflow": {
                "compactions": 0,
                "compactions_per_observed_root": 0.0,
                "long_turn_count": 0,
                "long_turn_rate": 0.0,
                "exact_repeated_call_groups": 0,
                "calls_in_exact_repeated_groups": 0,
                "repeated_call_rate": null,
                "failure_signal_count": 0,
                "failure_signal_rate": null,
                "tool_call_count": 0,
                "user_messages_with_text": 0,
                "short_message_count": 0,
                "short_message_rate": null,
                "broad_scope_message_count": 0,
                "broad_scope_message_rate": null,
                "boundary_review_root_count": 0,
                "long_lived_root_count": 0
            },
            "verification": {
                "tool_call_count": 0,
                "success_count": 0,
                "failure_count": 0,
                "unresolved_count": 0,
                "outcome_coverage": null
            },
            "guardian": {
                "review_count": 0,
                "workspace_attributed_review_count": 0,
                "workspace_attribution_coverage": null
            }
        },
        "cohorts": {
            "event_distributions": {
                "schema_version": [{"value": "5", "event_count": 2}],
                "groundline_version": [{"value": "0.18.0", "event_count": 2}],
                "os_family": [{"value": "macos", "event_count": 2}],
                "runtime_family": [{"value": "codex_app", "event_count": 2}],
                "execution_mode": [{"value": "desktop", "event_count": 2}]
            },
            "installation_distributions": {
                "groundline_version": [{"value": "0.18.0", "installation_count": 1}],
                "os_family": [{"value": "macos", "installation_count": 1}],
                "runtime_family": [{"value": "codex_app", "installation_count": 1}],
                "execution_mode": [{"value": "desktop", "installation_count": 1}]
            },
            "model_effort_context_distribution": [],
            "model_effort_token_efficiency": {
                "status": "UNAVAILABLE",
                "reason_code": "token_usage_not_attributed_to_model_effort",
                "context_distribution_only": true
            }
        },
        "data_quality": {
            "status": "PASS",
            "reason_codes": [],
            "sample_sufficient_event_count": 2,
            "sample_insufficient_event_count": 0
        },
        "comparison_readiness": {
            "status": "INSUFFICIENT",
            "reason_codes": ["comparison_baseline_not_included"],
            "minimum_event_count": 2,
            "minimum_observed_root_count": 5
        }
    })
}

fn tempdir() -> std::io::Result<TempDir> {
    let root = tempfile::tempdir()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))?;
    }
    Ok(root)
}

#[test]
fn a_known_model_cannot_borrow_another_models_guidance() {
    let (mut e, c) = model();
    e.official_sources[0].applies_to_model = "different-model".into();
    assert!(model_context(&e, &c, Utc::now()).is_err());
}
#[test]
fn lower_intervention_does_not_hide_higher_resource_use() {
    let (root, e, c, mut s) = trial();
    for u in &mut s.units {
        u.total_tokens = Some(2000);
    }
    let out = evaluate(root.path(), s, &e, &c).unwrap();
    assert_eq!(out["status"], "ROLLED_BACK");
    assert_eq!(out["reason_codes"], json!(["resource_regression"]));
}

#[test]
fn changed_nontrial_settings_cannot_be_retained_as_guidance_improvement() {
    let (root, e, c, mut s) = trial();
    s.comparison_context_sha256 = hash(b"changed tools or tier");
    let out = evaluate(root.path(), s, &e, &c).unwrap();
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert_eq!(out["reason_codes"], json!(["cohort_mismatch"]));
    assert_eq!(load_trial(root.path()).unwrap().status, "pending");
}

#[test]
fn aggregate_signals_alone_cannot_authorize_a_trial() {
    for source in ["repetition", "native_failure", "insights_failure"] {
        let root = tempdir().unwrap();
        let (e, c) = model();
        let context = model_context(&e, &c, Utc::now()).unwrap();
        let mut report = report_fixture();
        report["generated_at_utc"] =
            json!(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let mut audit = audit();
        if source != "repetition" {
            audit["root"]["tools"]["calls_in_exact_repeated_groups"] = json!(0);
        }
        if source == "native_failure" {
            audit["root"]["tools"]["failure_signals"] = json!({"nonzero_exit":4});
        } else if source == "insights_failure" {
            let workflow = &mut report["weekly_metrics"]["workflow"];
            workflow["tool_call_count"] = json!(100);
            workflow["failure_signal_count"] = json!(4);
            workflow["failure_signal_rate"] = json!(0.04);
            workflow["repeated_call_rate"] = json!(0.0);
        }
        put(&root.path().join("report.json"), &report);
        put(&root.path().join("audit.json"), &audit);
        let mut s = sample(&context, 4, "baseline");
        for u in &mut s.units {
            u.redundant_approval_count = 0;
            u.continuation_prompt_count = 0;
            u.repeated_call_count = 0;
        }
        put(
            &root.path().join("outcomes.json"),
            &serde_json::to_value(s).unwrap(),
        );
        let out = review(
            ReviewInputs {
                report: &root.path().join("report.json"),
                audit: &root.path().join("audit.json"),
                outcomes: Some(&root.path().join("outcomes.json")),
                state_dir: Some(root.path()),
                apply: true,
            },
            &e,
            &c,
        )
        .unwrap();
        assert_eq!(out["status"], "OBSERVE");
        assert_eq!(
            out["reason_codes"],
            json!(["candidate_signal_missing_in_outcomes"])
        );
        assert_eq!(out["mutation_performed"], false);
        assert!(!root.path().join("trial.json").exists());
    }
}

#[test]
#[cfg(unix)]
fn dangling_archive_links_are_preserved_and_rejected() {
    use std::os::unix::fs::symlink;
    let root = tempdir().unwrap();
    let data = b"private archive";
    let path = root.path().join(format!("trial-{}.json", hash(data)));
    symlink(root.path().join("missing"), &path).unwrap();
    assert!(archive(root.path(), "trial", data).is_err());
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
}

#[test]
fn rollback_recovers_every_durable_write_boundary_and_repeats_without_mutation() {
    for fail_at in 1..=3 {
        for after_write in [false, true] {
            let (root, _, _, _) = trial();
            let mut t = load_trial(root.path()).unwrap();
            let mut writes = 0;
            let err = restore_with_writer(root.path(), &mut t, |path, contents| {
                writes += 1;
                if writes == fail_at && !after_write {
                    return Err(error("injected_interruption"));
                }
                atomic_write_private(path, contents).unwrap();
                if writes == fail_at {
                    return Err(error("injected_interruption"));
                }
                Ok(())
            })
            .unwrap_err();
            assert_eq!(err.0, "personal_injected_interruption");
            let rollback = || {
                run(Command::Rollback {
                    state_dir: root.path().to_owned(),
                    json: true,
                })
                .unwrap()
            };
            assert_eq!(rollback()["status"], "ROLLED_BACK");
            assert_eq!(load_trial(root.path()).unwrap().status, "rolled_back");
            assert_eq!(current_rules(root.path()).unwrap(), Vec::<Rule>::new());
            let before = fs::read(root.path().join("trial.json")).unwrap();
            assert_eq!(rollback()["mutation_performed"], false);
            assert_eq!(fs::read(root.path().join("trial.json")).unwrap(), before);
        }
    }
}

#[test]
fn recovery_intent_is_required_and_owner_edits_still_block_restore() {
    for phase in ["pending", "restoring", "rolled_back"] {
        let (root, _, _, _) = trial();
        let mut t = load_trial(root.path()).unwrap();
        t.status = phase.into();
        save_trial(root.path(), &t).unwrap();
        // Even exact generated before-guidance is not proof of an interrupted
        // rollback when the durable journal has no recovery intent.
        let owner = if phase == "pending" {
            b"".as_slice()
        } else {
            b"OWNER_EDIT".as_slice()
        };
        atomic_write_private(&root.path().join("personal-guidance.md"), owner).unwrap();
        let before = fs::read(root.path().join("trial.json")).unwrap();
        assert_eq!(
            run(Command::Rollback {
                state_dir: root.path().to_owned(),
                json: true
            })
            .unwrap_err()
            .0,
            "personal_user_edited_guidance_preserved"
        );
        assert_eq!(
            fs::read(root.path().join("personal-guidance.md")).unwrap(),
            owner
        );
        assert_eq!(fs::read(root.path().join("trial.json")).unwrap(), before);
    }
    let (root, _, _, _) = trial();
    let mut t = load_trial(root.path()).unwrap();
    let err = restore_with_writer(root.path(), &mut t, |path, contents| {
        atomic_write_private(path, contents).unwrap();
        atomic_write_private(&root.path().join("personal-guidance.md"), b"OWNER_EDIT").unwrap();
        Ok(())
    })
    .unwrap_err();
    assert_eq!(err.0, "personal_user_edited_guidance_preserved");
    assert_eq!(load_trial(root.path()).unwrap().status, "restoring");
}

#[test]
fn interrupted_private_replacements_do_not_strand_a_full_history() {
    for name in [
        ".trial.json.99999.0.tmp".to_owned(),
        ".personal-guidance.md.99999.1.tmp".into(),
        format!(".evaluation-{}.json.99999.2.tmp", "a".repeat(64)),
    ] {
        let (root, _, _, _) = trial();
        for i in fs::read_dir(root.path()).unwrap().count()..MAX_STATE_FILES {
            atomic_write_private(&root.path().join(format!("owner-record-{i}")), b"").unwrap();
        }
        let temporary = root.path().join(name);
        atomic_write_private(&temporary, b"partial interrupted bytes").unwrap();
        run(Command::Rollback {
            state_dir: root.path().to_owned(),
            json: true,
        })
        .unwrap();
        assert_eq!(load_trial(root.path()).unwrap().status, "rolled_back");
        assert_eq!(fs::read(temporary).unwrap(), b"partial interrupted bytes");
        assert_eq!(
            fs::read_dir(root.path()).unwrap().count(),
            MAX_STATE_FILES + 1
        );
    }
}

#[test]
fn temporary_allowance_is_bounded_and_does_not_hide_unknown_files() {
    for name in [
        ".unrelated.99999.0.tmp",
        ".trial.json.invalid.0.tmp",
        ".trial.json.0.0.tmp",
        ".trial.json.99999.00.tmp",
        ".trial-invalid.json.99999.0.tmp",
    ] {
        let root = tempdir().unwrap();
        for i in 0..MAX_STATE_FILES {
            atomic_write_private(&root.path().join(format!("owner-record-{i}")), b"").unwrap();
        }
        let path = root.path().join(name);
        atomic_write_private(&path, b"owner file").unwrap();
        assert_eq!(
            state_root(root.path()).unwrap_err().0,
            "personal_state_archive_limit"
        );
        assert_eq!(fs::read(path).unwrap(), b"owner file");
    }
    let root = tempdir().unwrap();
    for i in 0..=MAX_INTERRUPTED_WRITES {
        atomic_write_private(&root.path().join(format!(".trial.json.99999.{i}.tmp")), b"").unwrap();
    }
    assert_eq!(
        state_root(root.path()).unwrap_err().0,
        "personal_interrupted_write_limit"
    );
    assert_eq!(
        fs::read_dir(root.path()).unwrap().count(),
        MAX_INTERRUPTED_WRITES + 1
    );
}

#[cfg(unix)]
#[test]
fn temporary_allowance_rejects_links_and_public_files_without_deleting_them() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for kind in ["symlink", "hardlink", "public"] {
        let root = tempdir().unwrap();
        let target = root.path().join("owner-file");
        atomic_write_private(&target, b"owner bytes").unwrap();
        let temporary = root.path().join(".trial.json.99999.0.tmp");
        match kind {
            "symlink" => symlink(&target, &temporary).unwrap(),
            "hardlink" => fs::hard_link(&target, &temporary).unwrap(),
            _ => {
                atomic_write_private(&temporary, b"owner bytes").unwrap();
                fs::set_permissions(&temporary, fs::Permissions::from_mode(0o644)).unwrap();
            }
        }
        assert!(state_root(root.path()).is_err());
        assert!(fs::symlink_metadata(temporary).is_ok());
        assert_eq!(fs::read(target).unwrap(), b"owner bytes");
    }
}

fn review_sample(
    root: &Path,
    s: &Sample,
    e: &ModelEvidence,
    c: &[u8],
    state: Option<&Path>,
    apply: bool,
) -> Value {
    let mut report = report_fixture();
    report["generated_at_utc"] =
        json!(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    put(&root.join("report.json"), &report);
    put(&root.join("audit.json"), &audit());
    put(
        &root.join("outcomes.json"),
        &serde_json::to_value(s).unwrap(),
    );
    review(
        ReviewInputs {
            report: &root.join("report.json"),
            audit: &root.join("audit.json"),
            outcomes: Some(&root.join("outcomes.json")),
            state_dir: state,
            apply,
        },
        e,
        c,
    )
    .unwrap()
}

#[test]
fn review_selects_the_next_eligible_rule_and_matches_locked_application() {
    let (root, e, c, s) = trial();
    evaluate(root.path(), s, &e, &c).unwrap();
    let inputs = tempdir().unwrap();
    let context = model_context(&e, &c, Utc::now()).unwrap();
    let mut fresh = sample(&context, 1, "fresh");
    fresh.guidance_sha256 = hash(render(&[Rule::ApprovalContinuity]).as_bytes());
    activate(&mut fresh);
    let before = fs::read(root.path().join("trial.json")).unwrap();
    let out = review_sample(inputs.path(), &fresh, &e, &c, Some(root.path()), false);
    assert_eq!(out["status"], "READY");
    assert_eq!(out["candidate"]["rule"], "diagnose_before_retry");
    assert_eq!(out["state_preflight_checked"], true);
    assert_eq!(out["automatic_application_eligible"], true);
    assert_eq!(fs::read(root.path().join("trial.json")).unwrap(), before);
    assert_eq!(
        review_sample(inputs.path(), &fresh, &e, &c, Some(root.path()), true)["mutation_performed"],
        true
    );
    // A changed state between review and application cannot bypass the lock's
    // second eligibility check.
    assert_eq!(
        apply(
            root.path(),
            Rule::DiagnoseBeforeRetry,
            &fresh,
            &context,
            Utc::now()
        )
        .unwrap_err()
        .0,
        "personal_trial_already_active"
    );
    let mut both = load_trial(root.path()).unwrap();
    both.status = "retained".into();
    save_trial(root.path(), &both).unwrap();
    fresh.guidance_sha256 = hash(render(&both.after_rules).as_bytes());
    activate(&mut fresh);
    for unit in &mut fresh.units {
        unit.unit_hash = hash(unit.unit_hash.as_bytes());
    }
    let out = review_sample(inputs.path(), &fresh, &e, &c, Some(root.path()), false);
    assert_eq!(out["status"], "OBSERVE");
    assert_eq!(out["automatic_application_eligible"], false);
    assert_eq!(out["candidate"], Value::Null);
}

#[test]
fn review_reports_blocked_state_and_never_authorizes_an_unchecked_directory() {
    for phase in ["pending", "restoring", "retained", "rolled_back"] {
        let (root, e, c, _) = trial();
        let mut t = load_trial(root.path()).unwrap();
        t.status = phase.into();
        save_trial(root.path(), &t).unwrap();
        let mut sample = t.baseline.clone();
        let mut e = e.clone();
        e.behavior_focus = vec![Rule::ApprovalContinuity];
        if phase == "rolled_back" {
            atomic_write_private(&root.path().join("personal-guidance.md"), b"").unwrap();
        } else {
            sample.guidance_sha256 = hash(render(&t.after_rules).as_bytes());
            activate(&mut sample);
            if phase == "retained" {
                for unit in &mut sample.units {
                    unit.unit_hash = hash(unit.unit_hash.as_bytes());
                }
            }
        }
        let inputs = tempdir().unwrap();
        let out = review_sample(inputs.path(), &sample, &e, &c, Some(root.path()), false);
        assert_eq!(out["status"], "OBSERVE");
        assert_eq!(out["automatic_application_eligible"], false);
        let expected = match phase {
            "retained" => "candidate_already_applied",
            "rolled_back" => "candidate_already_reviewed",
            _ => "trial_already_active",
        };
        assert!(
            out["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!(expected))
        );
    }
    let (e, c) = model();
    let s = sample(&model_context(&e, &c, Utc::now()).unwrap(), 1, "fresh");
    let inputs = tempdir().unwrap();
    let out = review_sample(inputs.path(), &s, &e, &c, None, false);
    assert_eq!(out["status"], "OBSERVE");
    assert_eq!(out["state_preflight_checked"], false);
    assert_eq!(out["automatic_application_eligible"], false);
}

fn observe_opportunity(sample: &mut Sample, rule: Rule) {
    for unit in &mut sample.units {
        unit.optimization_opportunities = Some(vec![OpportunityEvidence {
            kind: rule.opportunity().unwrap(),
            evidence_sha256: hash(format!("observed eligibility {}", unit.unit_hash).as_bytes()),
            eligible_count: 1,
        }]);
    }
}

fn optimization_trial(rule: Rule) -> (TempDir, ModelEvidence, Vec<u8>, Sample) {
    let root = tempdir().unwrap();
    let (mut evidence, catalog) = model();
    evidence.behavior_focus = vec![rule];
    let context = model_context(&evidence, &catalog, Utc::now()).unwrap();
    let mut baseline = sample(&context, 4, "optimizer-before");
    observe_opportunity(&mut baseline, rule);
    apply(
        root.path(),
        rule,
        &baseline,
        &context,
        Utc::now() - Duration::days(3),
    )
    .unwrap();
    let mut candidate = sample(&context, 2, "optimizer-after");
    candidate.guidance_sha256 = hash(render(&[rule]).as_bytes());
    activate(&mut candidate);
    for unit in &mut candidate.units {
        // The task and intervention counts stay identical: only the resource
        // named by the rule's primary objective improves.
        unit.optimization_opportunities = Some(Vec::new());
        match rule {
            Rule::EvidenceReuse | Rule::JustInTimeContext => unit.total_tokens = Some(800),
            Rule::BoundedParallelReads => {
                unit.completed_at_utc =
                    (time(&unit.started_at_utc).unwrap() + Duration::seconds(40)).to_rfc3339();
            }
            _ => unreachable!(),
        }
    }
    (root, evidence, catalog, candidate)
}

#[test]
fn optimization_rules_use_direct_opportunities_not_effort_or_aggregate_tool_counts() {
    let (mut evidence, catalog) = model();
    evidence.behavior_focus = Rule::ALL.to_vec();
    let context = model_context(&evidence, &catalog, Utc::now()).unwrap();
    for rule in [
        Rule::EvidenceReuse,
        Rule::JustInTimeContext,
        Rule::BoundedParallelReads,
    ] {
        evidence.behavior_focus = vec![rule];
        let mut baseline = sample(&context, 1, "direct-opportunity");
        assert_eq!(select_rule(&evidence, Some(&baseline), &audit()), None);
        assert_eq!(
            strategy_eligibility(rule, Some(&baseline))["observed_eligible_opportunity_count"],
            Value::Null
        );
        for unit in &mut baseline.units {
            unit.optimization_opportunities = Some(Vec::new());
        }
        assert_eq!(select_rule(&evidence, Some(&baseline), &audit()), None);
        assert_eq!(
            strategy_eligibility(rule, Some(&baseline))["observed_eligible_opportunity_count"],
            0
        );
        observe_opportunity(&mut baseline, rule);
        assert_eq!(
            select_rule(&evidence, Some(&baseline), &audit()),
            Some(rule)
        );
        let inputs = tempdir().unwrap();
        let state = tempdir().unwrap();
        let reviewed = review_sample(
            inputs.path(),
            &baseline,
            &evidence,
            &catalog,
            Some(state.path()),
            true,
        );
        assert_eq!(reviewed["status"], "READY");
        assert_eq!(
            reviewed["candidate"]["primary_metric"],
            rule.primary_metric()
        );
        assert_eq!(reviewed["trial"]["native_activation"], "UNVERIFIED");
        assert_eq!(
            reviewed["strategy_eligibility"][0]["observed_eligible_opportunity_count"],
            10
        );
    }
}

#[test]
fn token_or_time_only_improvement_can_retain_each_optimization_strategy() {
    for rule in [
        Rule::EvidenceReuse,
        Rule::JustInTimeContext,
        Rule::BoundedParallelReads,
    ] {
        let (root, evidence, catalog, candidate) = optimization_trial(rule);
        let out = evaluate(root.path(), candidate, &evidence, &catalog).unwrap();
        assert_eq!(out["status"], "RETAINED");
        assert_eq!(out["primary_metric"], rule.primary_metric());
        assert_eq!(out["native_activation"], "OPERATOR_EVIDENCED");
        assert_eq!(out["activation_independently_verified"], false);
        assert_eq!(out["causal_improvement_claimed"], false);
        assert_eq!(
            out["baseline"]["redundant_approvals_per_unit"],
            out["candidate"]["redundant_approvals_per_unit"]
        );
        assert_eq!(
            out["baseline"]["repeated_calls_per_unit"],
            out["candidate"]["repeated_calls_per_unit"]
        );
    }
}

#[test]
fn verified_delivery_metrics_include_failed_resources_and_keep_unknown_or_zero_denominators_null() {
    let mut observed = sample(&hash(b"context"), 1, "denominator");
    observed.units[0].outcome = Outcome::Failed;
    let measured = metrics(&observed);
    assert_eq!(measured["verified_delivery_count"], 9);
    assert_eq!(
        measured["owned_tokens_per_verified_delivery"],
        json!(10_000.0 / 9.0)
    );
    assert_eq!(
        measured["wall_duration_ms_per_verified_delivery"],
        json!(600_000.0 / 9.0)
    );
    observed.units[0].total_tokens = None;
    assert_eq!(
        metrics(&observed)["owned_tokens_per_verified_delivery"],
        Value::Null
    );
    assert_eq!(
        metrics(&observed)["owned_token_coverage"]["unobserved_unit_count"],
        1
    );
    for unit in &mut observed.units {
        unit.outcome = Outcome::Failed;
        unit.total_tokens = Some(1000);
    }
    assert_eq!(
        metrics(&observed)["owned_tokens_per_verified_delivery"],
        Value::Null
    );
    assert_eq!(
        metrics(&observed)["wall_duration_ms_per_verified_delivery"],
        Value::Null
    );
}

#[test]
fn missing_owned_usage_blocks_optimization_application_and_evaluation_without_mutation() {
    for rule in [
        Rule::EvidenceReuse,
        Rule::JustInTimeContext,
        Rule::BoundedParallelReads,
    ] {
        let (root, evidence, catalog, mut candidate) = optimization_trial(rule);
        let mut baseline = load_trial(root.path()).unwrap().baseline;
        baseline.units[0].total_tokens = None;
        let inputs = tempdir().unwrap();
        let unused = tempdir().unwrap();
        let out = review_sample(
            inputs.path(),
            &baseline,
            &evidence,
            &catalog,
            Some(unused.path()),
            true,
        );
        assert_eq!(out["status"], "OBSERVE");
        assert_eq!(out["mutation_performed"], false);
        assert!(
            out["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!("owned_token_baseline_incomplete"))
        );
        candidate.units[0].total_tokens = None;
        let before = fs::read(root.path().join("trial.json")).unwrap();
        let out = evaluate(root.path(), candidate, &evidence, &catalog).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert_eq!(out["mutation_performed"], false);
        assert_eq!(out["candidate"]["tokens_per_unit"], Value::Null);
        assert_eq!(fs::read(root.path().join("trial.json")).unwrap(), before);
    }
    let (root, evidence, catalog, candidate) = optimization_trial(Rule::EvidenceReuse);
    let mut trial = load_trial(root.path()).unwrap();
    trial.baseline.units[0].total_tokens = None;
    save_trial(root.path(), &trial).unwrap();
    let out = evaluate(root.path(), candidate, &evidence, &catalog).unwrap();
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert_eq!(
        out["baseline"]["owned_tokens_per_verified_delivery"],
        Value::Null
    );
}

#[test]
fn invalid_opportunity_evidence_and_retired_samples_are_rejected() {
    let context = hash(b"context");
    for invalid in ["digest", "zero", "unbounded", "duplicate", "retired"] {
        let mut observed = sample(&context, 1, "invalid-opportunity");
        observe_opportunity(&mut observed, Rule::EvidenceReuse);
        let opportunities = observed.units[0]
            .optimization_opportunities
            .as_mut()
            .unwrap();
        match invalid {
            "digest" => opportunities[0].evidence_sha256 = "not-a-digest".into(),
            "zero" => opportunities[0].eligible_count = 0,
            "unbounded" => opportunities[0].eligible_count = 10_001,
            "duplicate" => opportunities.push(opportunities[0].clone()),
            _ => observed.schema = 1,
        }
        assert!(validate_sample(&observed, &context, Utc::now()).is_err());
    }
    let (root, _, _, _) = optimization_trial(Rule::EvidenceReuse);
    let mut trial = serde_json::to_value(load_trial(root.path()).unwrap()).unwrap();
    trial["schema"] = json!(1);
    trial["baseline"]["schema"] = json!(1);
    trial["baseline"]
        .as_object_mut()
        .unwrap()
        .remove("activation_evidence");
    trial["baseline"]["activation_verified"] = json!(true);
    atomic_write_private(
        &root.path().join("trial.json"),
        &serde_json::to_vec(&trial).unwrap(),
    )
    .unwrap();
    let before = fs::read(root.path().join("trial.json")).unwrap();
    let before_guidance = fs::read(root.path().join("personal-guidance.md")).unwrap();
    assert_eq!(
        run(Command::Rollback {
            state_dir: root.path().into(),
            json: true
        })
        .unwrap_err()
        .0,
        "personal_unsupported_trial_schema_preserved_restore_with_matching_cli"
    );
    assert_eq!(fs::read(root.path().join("trial.json")).unwrap(), before);
    assert_eq!(
        fs::read(root.path().join("personal-guidance.md")).unwrap(),
        before_guidance
    );
}

#[test]
fn activation_requires_load_and_behavior_evidence_before_the_trial_sample() {
    let (root, evidence, catalog, candidate) = optimization_trial(Rule::EvidenceReuse);
    let context = model_context(&evidence, &catalog, Utc::now()).unwrap();
    for invalid in ["load", "behavior", "guidance", "equal_sample_start", "late"] {
        let mut sample = candidate.clone();
        let activation = sample.activation_evidence.as_mut().unwrap();
        match invalid {
            "load" => activation.instruction_load_sha256.clear(),
            "behavior" => activation.behavior_check_sha256.clear(),
            "guidance" => activation.guidance_sha256 = hash(b"another guidance"),
            "equal_sample_start" => activation.observed_at_utc = sample.period_start_utc.clone(),
            _ => {
                activation.observed_at_utc =
                    (time(&sample.period_start_utc).unwrap() + Duration::seconds(1)).to_rfc3339()
            }
        }
        assert!(validate_sample(&sample, &context, Utc::now()).is_err());
    }
    let mut absent = candidate.clone();
    absent.activation_evidence = None;
    let out = evaluate(root.path(), absent, &evidence, &catalog).unwrap();
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert_eq!(out["native_activation"], "UNVERIFIED");
    let applied = time(&load_trial(root.path()).unwrap().applied_at_utc).unwrap();
    for offset in [-1, 0] {
        let mut pretrial = candidate.clone();
        pretrial
            .activation_evidence
            .as_mut()
            .unwrap()
            .observed_at_utc = (applied + Duration::seconds(offset)).to_rfc3339();
        let out = evaluate(root.path(), pretrial, &evidence, &catalog).unwrap();
        assert_eq!(out["status"], "INCONCLUSIVE");
        assert_eq!(out["native_activation"], "UNVERIFIED");
    }
}

#[test]
fn optimization_resource_or_quality_regressions_roll_back_and_preserve_owner_edits() {
    for regression in ["quality", "tokens", "time", "intervention", "no_benefit"] {
        let (root, evidence, catalog, mut candidate) = optimization_trial(Rule::JustInTimeContext);
        match regression {
            "quality" => candidate.units[0].outcome = Outcome::Failed,
            "tokens" => candidate.units[0].total_tokens = Some(100_000),
            "time" => {
                candidate.units[0].completed_at_utc =
                    (time(&candidate.units[0].started_at_utc).unwrap() + Duration::seconds(90))
                        .to_rfc3339()
            }
            "intervention" => candidate.units[0].redundant_approval_count += 1,
            _ => {
                for unit in &mut candidate.units {
                    unit.total_tokens = Some(1000);
                }
            }
        }
        let out = evaluate(root.path(), candidate, &evidence, &catalog).unwrap();
        assert_eq!(out["status"], "ROLLED_BACK");
        assert_eq!(
            fs::read(root.path().join("personal-guidance.md")).unwrap(),
            b""
        );
    }
    let (root, evidence, catalog, candidate) = optimization_trial(Rule::EvidenceReuse);
    atomic_write_private(&root.path().join("personal-guidance.md"), b"owner changes").unwrap();
    assert!(evaluate(root.path(), candidate, &evidence, &catalog).is_err());
    assert_eq!(
        fs::read(root.path().join("personal-guidance.md")).unwrap(),
        b"owner changes"
    );
}

#[test]
fn all_five_generated_rules_and_large_trial_subsets_round_trip_safely() {
    let root = tempdir().unwrap();
    for bits in 0..(1 << Rule::ALL.len()) {
        let rules: Vec<_> = Rule::ALL
            .into_iter()
            .enumerate()
            .filter(|(index, _)| bits & (1 << index) != 0)
            .map(|(_, rule)| rule)
            .collect();
        atomic_write_private(
            &root.path().join("personal-guidance.md"),
            render(&rules).as_bytes(),
        )
        .unwrap();
        assert_eq!(current_rules(root.path()).unwrap(), rules);
    }
    let (root, _, _, _) = optimization_trial(Rule::BoundedParallelReads);
    let mut trial = load_trial(root.path()).unwrap();
    trial.before_rules = Rule::ALL[..4].to_vec();
    trial.after_rules = Rule::ALL.to_vec();
    trial.baseline.guidance_sha256 = hash(render(&trial.before_rules).as_bytes());
    activate(&mut trial.baseline);
    assert!(parse_trial(&serde_json::to_vec(&trial).unwrap()).is_ok());
    trial.before_rules.swap(0, 1);
    assert!(parse_trial(&serde_json::to_vec(&trial).unwrap()).is_err());
}

#[test]
fn evaluation_rechecks_the_stored_baseline_opportunity_evidence() {
    let (root, evidence, catalog, candidate) = optimization_trial(Rule::EvidenceReuse);
    let mut trial = load_trial(root.path()).unwrap();
    for unit in &mut trial.baseline.units {
        unit.optimization_opportunities = None;
    }
    save_trial(root.path(), &trial).unwrap();
    let before = fs::read(root.path().join("trial.json")).unwrap();
    let out = evaluate(root.path(), candidate, &evidence, &catalog).unwrap();
    assert_eq!(out["status"], "INCONCLUSIVE");
    assert!(
        out["reason_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("baseline_candidate_signal_missing"))
    );
    assert_eq!(fs::read(root.path().join("trial.json")).unwrap(), before);
}
