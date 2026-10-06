#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn private_json(path: &Path, value: &Value) -> String {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(path, &bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    hash(&bytes)
}

fn run(root: &Path, args: &[&str], paths: &[(&str, &Path)]) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_groundline"));
    command.args(args);
    for (flag, path) in paths {
        command.arg(flag).arg(path);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "args={args:?}, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains(root.to_str().unwrap()));
    serde_json::from_str(&stdout).unwrap()
}

fn link(
    root: &Path,
    deliveries: &Path,
    learning: &Path,
    id: &str,
    environment_revision: &str,
    skill_revision: &str,
) -> String {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let unit = hash(id.as_bytes());
    let receipt = json!({
        "kind":"groundline-delivery-receipt", "schema":1,
        "unit_hash":unit, "cohort_sha256":"b".repeat(64),
        "phase":"implementation", "completed_at_utc":timestamp,
        "recommendation":null, "requested":null, "effective":null,
        "verification":{"status":"unknown", "evidence_kind":"unobserved",
            "evidence_sha256":"c".repeat(64), "rework":false,
            "authenticity_verified":false},
        "resources":{"complete":false, "wall_duration_ms":null, "entries":[]},
        "activation_verified":false, "observed_selection_matches_requested":null
    });
    let receipt_path = deliveries.join(format!("{id}.json"));
    let receipt_sha256 = private_json(&receipt_path, &receipt);
    let input = root.join(format!("{id}-link.json"));
    private_json(
        &input,
        &json!({
            "kind":"groundline-learning-link-input", "schema":1,
            "receipt_sha256":receipt_sha256, "unit_hash":unit,
            "cohort_sha256":receipt["cohort_sha256"], "phase":"implementation",
            "completed_at_utc":timestamp, "observed_at_utc":timestamp,
            "environment_revision":environment_revision,
            "skill":{"target_id":"workflow", "revision":skill_revision},
            "source_revision":"source-1", "runtime":null,
            "correction_kind":"unknown", "correction_evidence_sha256":null
        }),
    );
    run(
        root,
        &["learning", "link-outcome"],
        &[
            ("--input", &input),
            ("--receipt", &receipt_path),
            ("--state", learning),
        ],
    )["link_sha256"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn applied_plan_links_real_operation_and_preserves_unobserved_effects() {
    // Resolve macOS /var before using component-bound private evidence reads.
    let temp = tempfile::tempdir().unwrap();
    let root: PathBuf = temp.path().canonicalize().unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let skills = root.join("skills");
    let deliveries = root.join("deliveries");
    for directory in [&skills, &deliveries] {
        fs::create_dir(directory).unwrap();
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let target = skills.join("SKILL.md");
    let before = "---\nname: workflow\ndescription: fixture\n---\nBefore.\n";
    let after = "---\nname: workflow\ndescription: fixture\n---\nAfter.\n";
    fs::write(&target, before).unwrap();
    let before_sha256 = hash(before.as_bytes());
    let after_sha256 = hash(after.as_bytes());
    let state = root.join("environment-state");
    let learning = root.join("learning-state");
    let baseline = root.join("baseline.json");
    let bindings = root.join("bindings.json");
    private_json(
        &baseline,
        &json!({
            "kind":"groundline-environment-baseline", "schema":1,
            "revision":"baseline-1", "parent_revision":null,
            "source_revision":"source-1", "authority_revision":"authority-1",
            "exception_revision":"device-1", "authority_ref":"user-request-1",
            "managed_targets":[{"target_id":"workflow", "kind":"skill_file",
                "desired_sha256":after_sha256, "dependencies":[], "managed_block":null}]
        }),
    );
    private_json(
        &bindings,
        &json!({
            "kind":"groundline-environment-device-bindings", "schema":1,
            "revision":"device-1", "device_id":"fixture",
            "roots":[{"root_id":"owner", "path":skills, "aliases":[]}],
            "targets":[{"target_id":"workflow", "root_id":"owner", "relative_path":"SKILL.md"}]
        }),
    );
    run(
        &root,
        &["environment", "register", "--json"],
        &[
            ("--state-dir", &state),
            ("--baseline", &baseline),
            ("--bindings", &bindings),
        ],
    );
    let baseline_link = link(
        &root,
        &deliveries,
        &learning,
        "before",
        "previous-applied-revision",
        &before_sha256,
    );
    let patch = root.join("patch.json");
    private_json(
        &patch,
        &json!({
            "kind":"groundline-environment-proposal", "schema":1,
            "proposal_id":"workflow-change", "basis_revision":"baseline-1",
            "source_revision":"source-1", "authority_ref":"user-request-1",
            "changes":[{"target_id":"workflow", "content":after}]
        }),
    );
    let plan = run(
        &root,
        &["environment", "plan", "--json"],
        &[("--state-dir", &state), ("--proposal", &patch)],
    );
    let plan_sha256 = plan["plan_sha256"].as_str().unwrap();
    assert_eq!(
        plan_sha256,
        hash(&fs::read(state.join("plans/workflow-change.json")).unwrap())
    );
    let candidate = root.join("candidate.json");
    let mut candidate_value = json!({
        "kind":"groundline-learning-proposal-input", "schema":1,
        "proposal_id":"workflow-change", "evidence_refs":[baseline_link],
        "scope_sha256":hash(b"workflow"), "basis_revision":"baseline-1",
        "source_revision":"source-1", "source":null,
        "plan_sha256":plan_sha256, "status":"candidate", "reason":"Fixture transition",
        "target":{"target_id":"workflow", "kind":"skill", "change":"modify",
            "before_sha256":before_sha256, "after_sha256":after_sha256,
            "intended_trigger":"Same recurring workflow", "hypothesis":"Remove friction",
            "expected_result":"Direct comparable outcome", "falsification":"More rework",
            "authority_ref":"user-request-1", "rollback_ref":hash(before.as_bytes())},
        "analysis":null
    });
    private_json(&candidate, &candidate_value);
    assert_eq!(
        run(
            &root,
            &["learning", "propose"],
            &[("--input", &candidate), ("--state", &learning)]
        )["status"],
        "PROPOSED"
    );
    candidate_value["proposal_id"] = json!("different-name");
    candidate_value["plan_sha256"] = json!("d".repeat(64));
    private_json(&candidate, &candidate_value);
    assert_eq!(
        run(
            &root,
            &["learning", "propose"],
            &[("--input", &candidate), ("--state", &learning)]
        )["status"],
        "DUPLICATE_SUPPRESSED"
    );
    let applied = run(
        &root,
        &[
            "environment",
            "apply",
            "--proposal-id",
            "workflow-change",
            "--json",
        ],
        &[("--state-dir", &state)],
    );
    assert_eq!(applied["status"], "APPLIED");
    assert_eq!(fs::read_to_string(&target).unwrap(), after);
    assert_eq!(
        run(
            &root,
            &[
                "environment",
                "apply",
                "--proposal-id",
                "workflow-change",
                "--json"
            ],
            &[("--state-dir", &state)]
        )["status"],
        "ALREADY_RECORDED"
    );
    let operation_id = applied["operation_id"].as_str().unwrap();
    let operation = state
        .join("operations")
        .join(format!("{operation_id}.json"));
    let followup_link = link(
        &root,
        &deliveries,
        &learning,
        "after",
        plan_sha256,
        &after_sha256,
    );
    let evaluation = root.join("evaluation.json");
    private_json(
        &evaluation,
        &json!({
            "kind":"groundline-learning-evaluation-input", "schema":1,
            "proposal_id":"workflow-change", "baseline_revision":"previous-applied-revision",
            "proposal_revision":plan_sha256, "baseline_refs":[baseline_link],
            "followup_refs":[followup_link]
        }),
    );
    let result = run(
        &root,
        &["learning", "evaluate"],
        &[
            ("--input", &evaluation),
            ("--deliveries", &deliveries),
            ("--state", &learning),
            ("--operation", &operation),
        ],
    );
    assert_eq!(result["status"], "INCONCLUSIVE");
    assert_eq!(result["causal_effect_verified"], false);
    assert_eq!(result["efficiency_improvement_verified"], false);
    assert_eq!(result["native_activation"], "UNVERIFIED");
    for reason in [
        "revision_mismatch",
        "operation_not_applied_to_proposal",
        "target_revision_missing_or_mismatched",
        "source_observation_missing",
        "source_transition_not_declared",
        "candidate_source_revision_mismatch",
    ] {
        assert!(
            !result["comparison_reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == reason)
        );
    }
    assert!(
        result["comparison_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "analysis_resources_unknown")
    );
    let rolled = run(
        &root,
        &[
            "environment",
            "rollback",
            "--operation-id",
            operation_id,
            "--json",
        ],
        &[("--state-dir", &state)],
    );
    assert_eq!(rolled["status"], "ROLLED_BACK");
    assert_eq!(fs::read_to_string(&target).unwrap(), before);
    assert_eq!(
        fs::metadata(operation).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(!result.to_string().contains("Before."));
    assert!(!result.to_string().contains("After."));
}
