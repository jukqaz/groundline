use super::*;
use std::collections::BTreeMap;
use tempfile::TempDir;

fn fixture(state: &str, before: bool) -> TempDir {
    let root = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    }
    let start = Utc::now() - Duration::days(200);
    let before_rules = vec![Rule::ApprovalContinuity];
    let trial = Trial {
        kind: "groundline-personal-trial".into(),
        schema: 2,
        status: state.into(),
        applied_at_utc: (start + Duration::days(1)).to_rfc3339(),
        rule: Rule::EvidenceReuse,
        before_rules: before_rules.clone(),
        after_rules: vec![Rule::ApprovalContinuity, Rule::EvidenceReuse],
        baseline: Sample {
            kind: "groundline-outcome-sample".into(),
            schema: 2,
            period_start_utc: start.to_rfc3339(),
            period_end_utc: (start + Duration::hours(1)).to_rfc3339(),
            model_context_sha256: hash(b"old execution context"),
            guidance_sha256: hash(render(&before_rules).as_bytes()),
            comparison_context_sha256: hash(b"old comparison context"),
            task_kind: "implementation".into(),
            scope_size: "small".into(),
            activation_evidence: Some(ActivationEvidence {
                instruction_load_sha256: hash(b"old load observation"),
                behavior_check_sha256: hash(b"old behavior observation"),
                guidance_sha256: hash(render(&before_rules).as_bytes()),
                observed_at_utc: (start - Duration::seconds(1)).to_rfc3339(),
            }),
            units: vec![Unit {
                unit_hash: hash(b"old unit"),
                started_at_utc: start.to_rfc3339(),
                completed_at_utc: (start + Duration::minutes(1)).to_rfc3339(),
                outcome: Outcome::Verified,
                evidence: Evidence::RuntimeCheck,
                rework: false,
                redundant_approval_count: 0,
                continuation_prompt_count: 0,
                repeated_call_count: 0,
                tool_call_count: 1,
                total_tokens: Some(100),
                optimization_opportunities: Some(vec![OpportunityEvidence {
                    kind: OpportunityKind::EvidenceReuse,
                    evidence_sha256: hash(b"old reuse observation"),
                    eligible_count: 1,
                }]),
            }],
        },
        model_context_sha256: hash(b"old execution context"),
    };
    save_trial(root.path(), &trial).unwrap();
    atomic_write_private(
        &root.path().join("personal-guidance.md"),
        render(if before {
            &trial.before_rules
        } else {
            &trial.after_rules
        })
        .as_bytes(),
    )
    .unwrap();
    root
}
fn rollback(root: &Path) -> Result<Value, ContractError> {
    run(Command::Rollback {
        state_dir: root.to_owned(),
        json: true,
    })
}
fn snapshot(root: &Path) -> BTreeMap<std::ffi::OsString, Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), fs::read(entry.path()).unwrap())
        })
        .collect()
}

#[test]
fn generated_instruction_bytes_remain_compatible_with_existing_state() {
    let digests = [
        "5d884ee3971a92e414b1bb2f9c054a3295424d4346726b91fa6ecb11150c5a52",
        "c9ae329901b622a88891a2b0249f4d8bca6e352cb15510f9c8c48b6bcd254f5b",
        "a8ed3376d892ab54da418ce753526c1034338db6e232be85b03e280f5e31400a",
        "426a898ef8387894cff4c2b0f049e319ef7226f5ca207516209b08bb141dff33",
        "1f791b3aadbc2ad01305d7ef747020e6a18780cbd55e8cb821c8ea2e427a2ff0",
    ];
    for (rule, expected) in Rule::ALL.into_iter().zip(digests) {
        assert_eq!(hash(render(&[rule]).as_bytes()), expected);
    }
    assert_eq!(
        hash(render(&Rule::ALL).as_bytes()),
        "e69b2c951ebcadbff686ecf6aabb78954d891b925dce1196703087faae545e86"
    );
}

#[test]
fn status_reads_old_state_without_fresh_model_evidence_or_a_new_lock() {
    let root = fixture("pending", false);
    let before = snapshot(root.path());
    let out = run(Command::Status {
        state_dir: root.path().to_owned(),
        json: true,
    })
    .unwrap();
    assert_eq!(out["kind"], "groundline-personal-recovery");
    assert_eq!(out["trial_state"], "pending");
    assert_eq!(out["rollback_available"], true);
    assert_eq!(out["mutation_performed"], false);
    assert_eq!(out["native_activation"], "UNVERIFIED");
    assert_eq!(snapshot(root.path()), before);
    assert!(!root.path().join(".lock").exists());
    assert!(
        !out.to_string()
            .contains(&root.path().to_string_lossy().to_string())
    );
}

#[test]
fn rollback_preserves_baseline_and_archives_and_is_idempotent() {
    for state in ["pending", "retained"] {
        let root = fixture(state, false);
        let original: Value =
            serde_json::from_slice(&fs::read(root.path().join("trial.json")).unwrap()).unwrap();
        let archive = root
            .path()
            .join(format!("trial-{}.json", hash(b"existing archive")));
        atomic_write_private(&archive, b"existing archive").unwrap();
        let out = rollback(root.path()).unwrap();
        assert_eq!(out["status"], "ROLLED_BACK");
        assert_eq!(out["mutation_performed"], true);
        assert_eq!(
            current_rules(root.path()).unwrap(),
            vec![Rule::ApprovalContinuity]
        );
        let mut restored: Value =
            serde_json::from_slice(&fs::read(root.path().join("trial.json")).unwrap()).unwrap();
        assert_eq!(restored["status"], "rolled_back");
        restored["status"] = json!(state);
        assert_eq!(restored, original);
        assert_eq!(fs::read(archive).unwrap(), b"existing archive");
        let completed = snapshot(root.path());
        assert_eq!(rollback(root.path()).unwrap()["mutation_performed"], false);
        assert_eq!(snapshot(root.path()), completed);
        assert_eq!(status(root.path()).unwrap()["rollback_needed"], false);
    }
}

#[test]
fn prepared_and_restoring_journals_restore_before_or_after_guidance_write() {
    for state in ["prepared", "restoring"] {
        for before in [false, true] {
            let root = fixture(state, before);
            assert_eq!(status(root.path()).unwrap()["rollback_available"], true);
            rollback(root.path()).unwrap();
            assert_eq!(load_trial(root.path()).unwrap().status, "rolled_back");
            assert_eq!(
                current_rules(root.path()).unwrap(),
                vec![Rule::ApprovalContinuity]
            );
        }
    }
}

#[test]
fn interrupted_rollback_can_resume_after_each_failed_write() {
    for failed_write in 0..3 {
        let root = fixture("pending", false);
        let mut trial = load_trial(root.path()).unwrap();
        let mut writes = 0;
        let result = restore_with_writer(root.path(), &mut trial, |path, bytes| {
            let fail = writes == failed_write;
            writes += 1;
            if fail {
                return Err(error("injected_write_failure"));
            }
            atomic_write_private(path, bytes).map_err(|_| error("restore_write_failed"))
        });
        assert_eq!(result.unwrap_err().0, "personal_injected_write_failure");
        assert_eq!(
            load_trial(root.path()).unwrap().status,
            if failed_write == 0 {
                "pending"
            } else {
                "restoring"
            }
        );
        rollback(root.path()).unwrap();
        assert_eq!(
            current_rules(root.path()).unwrap(),
            vec![Rule::ApprovalContinuity]
        );
    }
}

#[test]
fn owner_edits_and_unexpected_generated_guidance_are_preserved() {
    for state in ["pending", "prepared", "restoring", "rolled_back"] {
        for content in [
            "owner modification".to_owned(),
            render(&[Rule::DiagnoseBeforeRetry]),
        ] {
            let root = fixture(state, false);
            atomic_write_private(
                &root.path().join("personal-guidance.md"),
                content.as_bytes(),
            )
            .unwrap();
            let journal = fs::read(root.path().join("trial.json")).unwrap();
            let observed = status(root.path()).unwrap();
            assert_eq!(observed["status"], "BLOCKED");
            assert_eq!(observed["rollback_available"], false);
            assert_eq!(
                rollback(root.path()).unwrap_err().0,
                "personal_user_edited_guidance_preserved"
            );
            assert_eq!(
                fs::read(root.path().join("personal-guidance.md")).unwrap(),
                content.as_bytes()
            );
            assert_eq!(fs::read(root.path().join("trial.json")).unwrap(), journal);
        }
    }
}

#[test]
fn owner_edit_after_restoring_intent_is_not_overwritten() {
    let root = fixture("pending", false);
    let mut trial = load_trial(root.path()).unwrap();
    let err = restore_with_writer(root.path(), &mut trial, |path, bytes| {
        atomic_write_private(path, bytes).unwrap();
        atomic_write_private(
            &root.path().join("personal-guidance.md"),
            b"owner edit during restore",
        )
        .unwrap();
        Ok(())
    })
    .unwrap_err();
    assert_eq!(err.0, "personal_user_edited_guidance_preserved");
    assert_eq!(load_trial(root.path()).unwrap().status, "restoring");
    assert_eq!(
        fs::read(root.path().join("personal-guidance.md")).unwrap(),
        b"owner edit during restore"
    );
}

#[test]
fn pending_trial_with_prematurely_restored_guidance_remains_blocked() {
    let root = fixture("pending", true);
    assert_eq!(status(root.path()).unwrap()["rollback_available"], false);
    assert_eq!(
        rollback(root.path()).unwrap_err().0,
        "personal_user_edited_guidance_preserved"
    );
}

#[test]
fn unsupported_or_invalid_trials_are_not_migrated_or_deleted() {
    for invalid in ["schema", "rules", "baseline", "extra", "outcome"] {
        let root = fixture("pending", false);
        let mut trial: Value =
            serde_json::from_slice(&fs::read(root.path().join("trial.json")).unwrap()).unwrap();
        match invalid {
            "schema" => trial["schema"] = json!(1),
            "rules" => trial["after_rules"] = json!(["evidence_reuse"]),
            "baseline" => trial["baseline"]["guidance_sha256"] = json!(hash(b"wrong guidance")),
            "outcome" => trial["baseline"]["units"][0]["evidence"] = json!("unobserved"),
            _ => trial["unrecognized"] = json!(true),
        }
        atomic_write_private(
            &root.path().join("trial.json"),
            &serde_json::to_vec(&trial).unwrap(),
        )
        .unwrap();
        let before = snapshot(root.path());
        let err = status(root.path()).unwrap_err();
        if invalid == "schema" {
            assert_eq!(
                err.0,
                "personal_unsupported_trial_schema_preserved_restore_with_matching_cli"
            );
        }
        assert_eq!(snapshot(root.path()), before);
        assert!(rollback(root.path()).is_err());
        assert_eq!(
            fs::read(root.path().join("trial.json")).unwrap(),
            before[std::ffi::OsStr::new("trial.json")]
        );
    }
}

#[test]
fn concurrent_rollback_is_rejected_and_full_history_still_allows_recovery() {
    let root = fixture("pending", false);
    let held = lock(root.path()).unwrap();
    assert_eq!(rollback(root.path()).unwrap_err().0, "personal_state_busy");
    drop(held);
    for index in fs::read_dir(root.path()).unwrap().count()..MAX_STATE_FILES {
        atomic_write_private(
            &root.path().join(format!("owner-{index}")),
            b"private archive",
        )
        .unwrap();
    }
    let leftover = root.path().join(".personal-guidance.md.12345.1.tmp");
    atomic_write_private(&leftover, b"interrupted bytes").unwrap();
    rollback(root.path()).unwrap();
    assert_eq!(fs::read(leftover).unwrap(), b"interrupted bytes");
    assert_eq!(
        fs::read_dir(root.path()).unwrap().count(),
        MAX_STATE_FILES + 1
    );
}

#[test]
fn excessive_interrupted_writes_are_preserved_for_owner_inspection() {
    let root = fixture("pending", false);
    for index in 0..=MAX_INTERRUPTED_WRITES {
        atomic_write_private(
            &root.path().join(format!(".trial.json.12345.{index}.tmp")),
            b"partial bytes",
        )
        .unwrap();
    }
    let before = snapshot(root.path());
    assert_eq!(
        status(root.path()).unwrap_err().0,
        "personal_interrupted_write_limit"
    );
    assert_eq!(
        rollback(root.path()).unwrap_err().0,
        "personal_interrupted_write_limit"
    );
    assert_eq!(snapshot(root.path()), before);
}

#[cfg(unix)]
#[test]
fn unsafe_permissions_links_and_git_directories_are_rejected() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for name in ["trial.json", "personal-guidance.md"] {
        for kind in ["symlink", "hardlink", "public"] {
            let root = fixture("pending", false);
            let path = root.path().join(name);
            let original = fs::read(&path).unwrap();
            if kind == "public" {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            } else {
                let target = root.path().join("untouched-original");
                fs::rename(&path, &target).unwrap();
                if kind == "symlink" {
                    symlink(&target, &path).unwrap();
                } else {
                    fs::hard_link(&target, &path).unwrap();
                }
            }
            assert!(status(root.path()).is_err());
            assert!(rollback(root.path()).is_err());
            assert_eq!(fs::read(&path).unwrap(), original);
        }
    }
    let root = fixture("pending", false);
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        status(root.path()).unwrap_err().0,
        "personal_private_state_required"
    );
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir(root.path().join(".git")).unwrap();
    assert_eq!(
        status(root.path()).unwrap_err().0,
        "personal_private_state_outside_git_required"
    );
}
