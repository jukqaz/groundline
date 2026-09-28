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
        "schema":1,"suggestion":{"model":model,"effort":effort}}),
    );
    let requested = artifact(
        dir,
        "requested.json",
        &json!({"kind":"groundline-selection-request",
        "schema":1,"model":model,"effort":effort,"source":"operator_supplied"}),
    );
    let effective = artifact(
        dir,
        "effective.json",
        &json!({"kind":"groundline-native-selection-observation",
        "schema":1,"model":model,"effort":effort,"source":"operator_supplied"}),
    );
    let verification = artifact(
        dir,
        "verification.json",
        &json!({"kind":"groundline-delivery-verification-observation",
        "schema":1,"status":"verified","evidence_kind":"runtime_check","source":"operator_supplied"}),
    );
    json!({"kind":"groundline-delivery-manifest","schema":1,"unit_hash":"a".repeat(64),
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
fn rejects_mismatched_digest_and_observation_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let mut fixture = manifest(&dir);
    fixture["effective"]["evidence_sha256"] = json!("0".repeat(64));
    let bad_digest = run(&dir, &fixture);
    assert!(!bad_digest.status.success());
    assert!(!dir.path().join("receipt.json").exists());
    assert!(!String::from_utf8_lossy(&bad_digest.stderr).contains(dir.path().to_str().unwrap()));
    fixture = manifest(&dir);
    fixture["effective"]["model"] = json!("gpt-6-luna");
    let bad_effective = run(&dir, &fixture);
    assert!(!bad_effective.status.success());
    assert!(!dir.path().join("receipt.json").exists());
    fixture = manifest(&dir);
    fixture["verification"]["status"] = json!("failed");
    let bad_verification = run(&dir, &fixture);
    assert!(!bad_verification.status.success());
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
