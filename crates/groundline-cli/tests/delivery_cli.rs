use chrono::Utc;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn artifact(dir: &TempDir, name: &str, value: &Value) -> Value {
    let path = dir.path().join(name);
    let bytes = serde_json::to_vec(value).unwrap();
    fs::write(&path, &bytes).unwrap();
    json!({"artifact_path":path,"evidence_sha256":format!("{:x}",Sha256::digest(&bytes))})
}

fn manifest(dir: &TempDir) -> Value {
    let model = "gpt-6-sol";
    let effort = "medium";
    let proposal = artifact(
        dir,
        "proposal.json",
        &json!({"kind":"groundline-routing-proposal",
        "schema":2,"suggestion":{"model":model,"effort":effort}}),
    );
    // Point to the actual evidence once; no duplicate GroundLine observation
    // JSON contracts are required for requested/effective/verification.
    let native = artifact(
        dir,
        "native-result.json",
        &json!({"native":"fixture result"}),
    );
    let requested = &native;
    let effective = &native;
    let verification = &native;
    json!({"kind":"groundline-delivery-manifest","schema":2,"unit_hash":"a".repeat(64),
        "cohort_sha256":"b".repeat(64),"phase":"implementation",
        "completed_at_utc":Utc::now().to_rfc3339(),
        "recommendation":{"model":model,"effort":effort,
            "artifact_path":proposal["artifact_path"],"evidence_sha256":proposal["evidence_sha256"]},
        "requested":{"model":model,"effort":effort,
            "artifact_path":requested["artifact_path"],"evidence_sha256":requested["evidence_sha256"]},
        "effective":{"model":model,"effort":effort,
            "artifact_path":effective["artifact_path"],"evidence_sha256":effective["evidence_sha256"]},
        "verification":{"status":"verified","evidence_kind":"runtime_check","rework":false,
            "artifact_path":verification["artifact_path"],"evidence_sha256":verification["evidence_sha256"]},
        "resources":{"complete":true,"wall_duration_ms":1000,"entries":[
            {"owner":"root","unit_hash":"a".repeat(64),"response_hash":"c".repeat(64),
             "input_tokens":10,"cached_input_tokens":4,"output_tokens":7,
             "reasoning_output_tokens":3,"total_tokens":17}]}})
}

fn run(dir: &TempDir, manifest: &Value) -> std::process::Output {
    let input = dir.path().join("manifest.json");
    let output = dir.path().join("receipt.json");
    fs::write(&input, serde_json::to_vec(manifest).unwrap()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_groundline"))
        .args(["efficiency", "record-delivery", "--input"])
        .arg(input)
        .arg("--output")
        .arg(output)
        .arg("--json")
        .output()
        .unwrap()
}

#[test]
fn writes_private_path_free_receipt_once() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = manifest(&dir);
    let first = run(&dir, &fixture);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let saved: Value =
        serde_json::from_slice(&fs::read(dir.path().join("receipt.json")).unwrap()).unwrap();
    assert_eq!(saved["kind"], "groundline-delivery-receipt");
    assert_eq!(saved["activation_verified"], false);
    assert_eq!(saved["observed_selection_matches_requested"], true);
    assert_eq!(saved["verification"]["authenticity_verified"], false);
    let report: Value = serde_json::from_slice(&first.stdout).unwrap();
    let saved_bytes = fs::read(dir.path().join("receipt.json")).unwrap();
    assert_eq!(report["kind"], "groundline-delivery-recording");
    assert_eq!(report["status"], "PASS");
    assert_eq!(
        report["receipt_sha256"],
        format!("{:x}", Sha256::digest(&saved_bytes))
    );
    assert_eq!(report["phase"], "implementation");
    assert_eq!(report["outcome"], "verified");
    assert_eq!(report["mutation_performed"], true);
    assert_eq!(report["network_performed"], false);
    assert_eq!(report["raw_content_emitted"], false);
    assert_eq!(report["private_paths_emitted"], false);
    assert!(report.get("resources").is_none());
    assert_eq!(report.as_object().unwrap().len(), 12);
    assert!(!String::from_utf8_lossy(&first.stdout).contains(dir.path().to_str().unwrap()));
    assert!(!saved.to_string().contains(dir.path().to_str().unwrap()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(dir.path().join("receipt.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    let second = run(&dir, &fixture);
    assert!(!second.status.success());
    assert_eq!(
        saved,
        serde_json::from_slice::<Value>(&fs::read(dir.path().join("receipt.json")).unwrap())
            .unwrap()
    );
}

#[test]
fn accepts_relative_output_filename_in_current_directory() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = manifest(&dir);
    fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&fixture).unwrap(),
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .current_dir(dir.path())
        .args([
            "efficiency",
            "record-delivery",
            "--input",
            "manifest.json",
            "--output",
            "receipt.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    let bytes = fs::read(dir.path().join("receipt.json")).unwrap();
    assert_eq!(
        report["receipt_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
}

#[test]
fn matching_failed_observation_is_saved_as_failure_with_complete_resources() {
    let dir = tempfile::tempdir().unwrap();
    let mut fixture = manifest(&dir);
    let verification = artifact(
        &dir,
        "failed.json",
        &json!({
            "kind":"groundline-delivery-verification-observation", "schema":1,
            "status":"failed", "evidence_kind":"runtime_check", "source":"operator_supplied"
        }),
    );
    fixture["verification"]["status"] = json!("failed");
    fixture["verification"]["rework"] = json!(true);
    fixture["verification"]["artifact_path"] = verification["artifact_path"].clone();
    fixture["verification"]["evidence_sha256"] = verification["evidence_sha256"].clone();
    let output = run(&dir, &fixture);
    assert!(output.status.success());
    let saved: Value =
        serde_json::from_slice(&fs::read(dir.path().join("receipt.json")).unwrap()).unwrap();
    let outcome = groundline_contracts::delivery::outcome_from_receipt(&saved)
        .unwrap()
        .unwrap();
    assert_eq!(outcome["outcome"], "failed");
    assert_eq!(outcome["rework"], true);
    assert_eq!(outcome["owned_total_tokens"], 17);
    assert_eq!(outcome["owned_resources_complete"], true);
}

#[test]
fn rejects_mismatched_digest_proposal_and_derived_claims_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let mut fixture = manifest(&dir);
    fixture["effective"]["evidence_sha256"] = json!("0".repeat(64));
    let bad_digest = run(&dir, &fixture);
    assert!(!bad_digest.status.success());
    assert!(!dir.path().join("receipt.json").exists());
    assert!(!String::from_utf8_lossy(&bad_digest.stderr).contains(dir.path().to_str().unwrap()));
    fixture = manifest(&dir);
    fixture["recommendation"]["model"] = json!("gpt-6-luna");
    let bad_proposal = run(&dir, &fixture);
    assert!(!bad_proposal.status.success());
    assert!(!dir.path().join("receipt.json").exists());
    fixture = manifest(&dir);
    fixture["schema"] = json!(1);
    assert!(!run(&dir, &fixture).status.success());
    assert!(!dir.path().join("receipt.json").exists());
    fixture = manifest(&dir);
    fixture["activation_verified"] = json!(true);
    let forged_activation = run(&dir, &fixture);
    assert!(!forged_activation.status.success());
    assert!(!dir.path().join("receipt.json").exists());
}

#[test]
fn records_requested_effective_mismatch_as_observation_without_activation_proof() {
    let dir = tempfile::tempdir().unwrap();
    let mut fixture = manifest(&dir);
    let actual = artifact(
        &dir,
        "actual-effective.json",
        &json!({"kind":"groundline-native-selection-observation","schema":1,
            "model":"gpt-6-luna","effort":"low","source":"operator_supplied"}),
    );
    fixture["effective"] = json!({"model":"gpt-6-luna","effort":"low",
        "artifact_path":actual["artifact_path"],"evidence_sha256":actual["evidence_sha256"]});
    let result = run(&dir, &fixture);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let saved: Value =
        serde_json::from_slice(&fs::read(dir.path().join("receipt.json")).unwrap()).unwrap();
    assert_eq!(saved["requested"]["model"], "gpt-6-sol");
    assert_eq!(saved["effective"]["model"], "gpt-6-luna");
    assert_eq!(saved["observed_selection_matches_requested"], false);
    assert_eq!(saved["activation_verified"], false);
}

#[cfg(unix)]
#[test]
fn rejects_existing_symlink_without_changing_target() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let fixture = manifest(&dir);
    let target = dir.path().join("target.json");
    fs::write(&target, b"keep").unwrap();
    symlink(&target, dir.path().join("receipt.json")).unwrap();
    let result = run(&dir, &fixture);
    assert!(!result.status.success());
    assert_eq!(fs::read(target).unwrap(), b"keep");
}

fn recorded_receipt(dir: &TempDir) -> Value {
    let result = run(dir, &manifest(dir));
    assert!(result.status.success());
    serde_json::from_slice(&fs::read(dir.path().join("receipt.json")).unwrap()).unwrap()
}

fn summarize(directory: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_groundline"))
        .args(["efficiency", "delivery-summary", "--deliveries"])
        .arg(directory)
        .arg("--json")
        .output()
        .unwrap()
}

#[test]
fn summary_links_actual_selection_failure_and_missing_measurements_without_changing_receipts() {
    let dir = tempfile::tempdir().unwrap();
    let first = recorded_receipt(&dir);
    let receipts = dir.path().join("retained-private-receipts");
    fs::create_dir(&receipts).unwrap();
    let mut second = first.clone();
    second["unit_hash"] = json!("d".repeat(64));
    second["resources"]["entries"][0]["unit_hash"] = json!("d".repeat(64));
    second["resources"]["entries"][0]["response_hash"] = json!("e".repeat(64));
    second["effective"]["model"] = json!("gpt-6-luna");
    second["effective"]["effort"] = json!("low");
    second["observed_selection_matches_requested"] = json!(false);
    second["verification"]["status"] = json!("failed");
    second["verification"]["rework"] = json!(true);
    second["resources"]["complete"] = json!(false);
    second["resources"]["entries"][0]["total_tokens"] = Value::Null;
    second["completed_at_utc"] = json!((Utc::now() - chrono::Duration::days(60)).to_rfc3339());
    second["cohort_sha256"] = json!("f".repeat(64));
    let mut third = first.clone();
    third["unit_hash"] = json!("1".repeat(64));
    third["phase"] = json!("deployment");
    third["effective"] = Value::Null;
    third["requested"] = Value::Null;
    third["observed_selection_matches_requested"] = Value::Null;
    third["verification"]["status"] = json!("unknown");
    third["verification"]["evidence_kind"] = json!("unobserved");
    third["resources"]["complete"] = json!(false);
    third["resources"]["entries"] = json!([]);
    third["resources"]["wall_duration_ms"] = Value::Null;
    let fixtures = [first, second, third];
    let mut before = Vec::new();
    for (index, value) in fixtures.iter().enumerate() {
        let path = receipts.join(format!("private-task-{index}.json"));
        let bytes = serde_json::to_vec(value).unwrap();
        fs::write(&path, &bytes).unwrap();
        before.push((path, bytes));
    }
    let result = summarize(&receipts);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let summary: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(summary["kind"], "groundline-delivery-summary");
    assert_eq!(
        summary["scope"],
        "all_retained_receipts_without_comparison_filters"
    );
    assert_eq!(summary["overall"]["delivery_count"], 3);
    assert_eq!(summary["overall"]["verified_count"], 1);
    assert_eq!(summary["overall"]["failed_count"], 1);
    assert_eq!(summary["overall"]["unknown_count"], 1);
    assert_eq!(summary["overall"]["rework_count"], 1);
    assert_eq!(summary["overall"]["recommendation_only_count"], 1);
    assert_eq!(
        summary["overall"]["selection_relationships"]["requested_to_effective"]["mismatched_count"],
        1
    );
    assert_eq!(
        summary["overall"]["resources"]["all_owners"]["total_tokens"]["known_sum"],
        17
    );
    assert_eq!(
        summary["overall"]["resources"]["all_owners"]["total_tokens"]["missing_count"],
        1
    );
    assert_eq!(
        summary["evidence_scope"]["optimization_demonstrated"],
        false
    );
    assert_eq!(summary["evidence_scope"]["activation_verified"], false);
    assert_eq!(summary["mutation_performed"], false);
    assert_eq!(summary["network_performed"], false);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(!stdout.contains(dir.path().to_str().unwrap()));
    assert!(!stdout.contains("private-task-"));
    assert!(!stdout.contains(&"a".repeat(64)));
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn summary_rejects_duplicate_deliveries_overlap_and_raw_fields_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let first = recorded_receipt(&dir);
    let receipts = dir.path().join("receipts");
    fs::create_dir(&receipts).unwrap();
    let one = serde_json::to_vec(&first).unwrap();
    fs::write(receipts.join("one.json"), &one).unwrap();
    let mut second = first.clone();
    for (error, value) in [
        ("delivery_summary_duplicate_delivery", first.clone()),
        ("delivery_summary_overlapping_response_ownership", {
            second["unit_hash"] = json!("d".repeat(64));
            second["resources"]["entries"][0]["unit_hash"] = json!("d".repeat(64));
            second.clone()
        }),
        ("delivery_summary_invalid_receipt", {
            second["private_path"] = json!(dir.path());
            second
        }),
    ] {
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(receipts.join("two.json"), &bytes).unwrap();
        let result = summarize(&receipts);
        assert!(!result.status.success());
        let failure: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(failure["error"], error);
        assert_eq!(failure["mutation_performed"], false);
        assert!(!String::from_utf8_lossy(&result.stdout).contains(dir.path().to_str().unwrap()));
        assert_eq!(fs::read(receipts.join("one.json")).unwrap(), one);
        assert_eq!(fs::read(receipts.join("two.json")).unwrap(), bytes);
    }
}

#[test]
fn summary_is_nonrecursive_and_bounds_all_directory_entries() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("unread-invalid.json"), b"invalid").unwrap();
    let result = summarize(dir.path());
    assert!(result.status.success());
    let summary: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(summary["status"], "EMPTY");
    assert!(summary["overall"]["resources"]["all_owners"]["total_tokens"]["known_sum"].is_null());
    for index in 0..1000 {
        fs::write(dir.path().join(format!("unread-{index}.txt")), b"ignored").unwrap();
    }
    let result = summarize(dir.path());
    assert!(!result.status.success());
    let failure: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(failure["error"], "delivery_summary_too_many_entries");
}

#[cfg(unix)]
#[test]
fn summary_rejects_receipt_and_directory_symlinks_without_changing_targets() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let first = recorded_receipt(&dir);
    let receipts = dir.path().join("receipts");
    fs::create_dir(&receipts).unwrap();
    let bytes = serde_json::to_vec(&first).unwrap();
    let target = dir.path().join("target.json");
    fs::write(&target, &bytes).unwrap();
    symlink(&target, receipts.join("linked.json")).unwrap();
    let result = summarize(&receipts);
    assert!(!result.status.success());
    let failure: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(failure["error"], "delivery_summary_invalid_receipt_file");
    assert_eq!(fs::read(&target).unwrap(), bytes);
    let linked = dir.path().join("linked-directory");
    symlink(&receipts, &linked).unwrap();
    let result = summarize(&linked);
    assert!(!result.status.success());
    let failure: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(failure["error"], "delivery_summary_invalid_directory");
}

#[test]
fn summary_bounds_receipt_bytes_and_total_directory_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let first = recorded_receipt(&dir);
    let receipts = dir.path().join("receipts");
    fs::create_dir(&receipts).unwrap();
    fs::write(
        receipts.join("oversized.json"),
        vec![b' '; 2 * 1024 * 1024 + 1],
    )
    .unwrap();
    let result = summarize(&receipts);
    assert!(!result.status.success());
    let failure: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(failure["error"], "delivery_summary_invalid_receipt_file");
    fs::remove_file(receipts.join("oversized.json")).unwrap();
    for index in 1..=9 {
        let mut receipt = first.clone();
        let unit = format!("{index:064x}");
        receipt["unit_hash"] = json!(unit);
        receipt["resources"]["entries"][0]["unit_hash"] = json!(unit);
        receipt["resources"]["entries"][0]["response_hash"] = json!(format!("{:064x}", index + 10));
        let mut bytes = serde_json::to_vec(&receipt).unwrap();
        bytes.resize(2 * 1024 * 1024, b' ');
        fs::write(receipts.join(format!("receipt-{index}.json")), bytes).unwrap();
    }
    let result = summarize(&receipts);
    assert!(!result.status.success());
    let failure: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(failure["error"], "delivery_summary_directory_too_large");
    assert_eq!(failure["mutation_performed"], false);
    assert_eq!(fs::read_dir(&receipts).unwrap().count(), 9);
}

#[test]
fn records_child_selection_from_checked_artifact_without_root_inheritance() {
    let dir = tempfile::tempdir().unwrap();
    let mut fixture = manifest(&dir);
    let native = artifact(&dir, "child-native.json", &json!({"native":"child result"}));
    let mut child = fixture["resources"]["entries"][0].clone();
    child["owner"] = json!("child");
    child["unit_hash"] = json!("d".repeat(64));
    child["response_hash"] = json!("e".repeat(64));
    child["effective"] = json!({"model":"gpt-6-luna","effort":"max",
        "artifact_path":native["artifact_path"],"evidence_sha256":native["evidence_sha256"]});
    fixture["resources"]["entries"]
        .as_array_mut()
        .unwrap()
        .push(child);
    let output = run(&dir, &fixture);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let saved: Value =
        serde_json::from_slice(&fs::read(dir.path().join("receipt.json")).unwrap()).unwrap();
    assert!(saved["resources"]["entries"][0]["effective"].is_null());
    assert_eq!(
        saved["resources"]["entries"][1]["effective"]["model"],
        "gpt-6-luna"
    );
    assert!(
        saved["resources"]["entries"][1]["effective"]
            .get("artifact_path")
            .is_none()
    );
    let report = groundline_contracts::delivery::summarize_receipts(&[saved]).unwrap();
    assert_eq!(
        report["evidence_scope"]["child_model_effort_attribution_available"],
        true
    );
    assert_eq!(
        report["evidence_scope"]["child_model_effort_attribution_complete"],
        true
    );
    assert_eq!(
        report["overall"]["resources"]["all_owners"]["total_tokens"]["known_sum"],
        34
    );
    let groups = report["overall"]["resources"]["by_owner_and_effective_selection"]
        .as_array()
        .unwrap();
    assert!(
        groups
            .iter()
            .any(|g| g["owner"] == "child" && g["effective"]["effort"] == "max")
    );
    assert!(
        groups
            .iter()
            .any(|g| g["owner"] == "root" && g["effective"].is_null())
    );
    assert_eq!(report["evidence_scope"]["activation_verified"], false);
}

#[test]
fn total_evidence_budget_is_checked_before_a_receipt_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let mut fixture = manifest(&dir);
    let evidence = dir.path().join("native-evidence.log");
    let bytes = vec![b'x'; 2 * 1024 * 1024];
    fs::write(&evidence, &bytes).unwrap();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let entries = fixture["resources"]["entries"].as_array_mut().unwrap();
    for index in 0..8 {
        let mut child = entries[0].clone();
        child["owner"] = json!("child");
        child["unit_hash"] = json!(format!("{:064x}", index + 100));
        child["response_hash"] = json!(format!("{:064x}", index + 200));
        child["effective"] = json!({"model":"gpt-6-sol","effort":"high",
            "artifact_path":evidence,"evidence_sha256":hash});
        entries.push(child);
    }
    let result = run(&dir, &fixture);
    assert!(!result.status.success());
    assert!(!dir.path().join("receipt.json").exists());
    let error: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(error["error"], "delivery_invalid_input_file");
    assert_eq!(fs::metadata(evidence).unwrap().len(), bytes.len() as u64);
}
