use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn groundline() -> &'static str {
    env!("CARGO_BIN_EXE_groundline")
}

fn run(arguments: &[&str]) -> Output {
    Command::new(groundline())
        .args(arguments)
        .output()
        .expect("execute GroundLine test binary")
}

fn parse_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "expected JSON stdout: {error}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn path_argument(path: &Path) -> &str {
    path.to_str().expect("UTF-8 temporary path")
}

#[test]
fn platform_command_reports_the_native_packaging_contract() {
    let output = run(&["platform", "--json"]);
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    let result = parse_stdout(&output);

    assert_eq!(result["status"], "PASS");
    assert_eq!(result["mutation_performed"], false);
    assert_eq!(result["schema"], 1);
    assert!(
        result["packaged_binary"]
            .as_str()
            .is_some_and(|path| path.starts_with("bin/") || path.starts_with("bin\\"))
    );
}

#[test]
fn efficiency_batch_runs_through_the_real_cli() {
    let root = tempdir().expect("temporary directory");
    let input = root.path().join("batch.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
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
        .expect("fixture JSON"),
    )
    .expect("fixture file");

    let output = run(&[
        "efficiency",
        "batch",
        "--input",
        path_argument(&input),
        "--json",
    ]);
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    let result = parse_stdout(&output);

    assert_eq!(result["kind"], "groundline-batch-assessment");
    assert_eq!(result["recommended_phase"], "implement");
    assert_eq!(result["new_observation_count"], 2);
    assert_eq!(result["mutation_performed"], false);
    assert!(!String::from_utf8_lossy(&output.stdout).contains(path_argument(&input)));
}

#[test]
fn invalid_input_fails_without_emitting_paths_or_content() {
    let root = tempdir().expect("temporary directory");
    let input = root.path().join("invalid.json");
    fs::write(&input, b"not-json-and-private").expect("fixture file");

    let output = run(&[
        "efficiency",
        "batch",
        "--input",
        path_argument(&input),
        "--json",
    ]);
    assert!(!output.status.success());
    let result = parse_stdout(&output);
    let serialized = String::from_utf8_lossy(&output.stdout);

    assert_eq!(result["status"], "FAIL");
    assert_eq!(result["mutation_performed"], false);
    assert_eq!(result["raw_content_emitted"], false);
    assert!(!serialized.contains(path_argument(&input)));
    assert!(!serialized.contains("not-json-and-private"));
}

#[test]
fn project_audit_reports_worktree_include_without_configuration_content() {
    let root = tempdir().expect("temporary directory");
    fs::write(root.path().join("AGENTS.md"), "do-not-emit-this").unwrap();
    fs::write(root.path().join(".worktreeinclude"), ".env.local").unwrap();
    let output = run(&[
        "project-audit",
        "--repo",
        path_argument(root.path()),
        "--json",
    ]);
    assert!(output.status.success());
    let result = parse_stdout(&output);
    let encoded = String::from_utf8_lossy(&output.stdout);
    assert_eq!(result["worktree_include_present"], true);
    assert_eq!(result["surface_counts"]["guidance"], 1);
    assert!(!encoded.contains("do-not-emit-this"));
    assert!(!encoded.contains(path_argument(root.path())));
}

#[test]
fn provider_smoke_verifies_one_native_binary_package() {
    let root = tempdir().expect("temporary directory");
    let platform = run(&["platform", "--json"]);
    let platform = parse_stdout(&platform);
    let target = platform["target"].as_str().unwrap();
    let packaged = platform["packaged_binary"].as_str().unwrap();
    let binary = root.path().join(packaged);
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    let binary_bytes = b"native-groundline-fixture";
    fs::write(&binary, binary_bytes).unwrap();
    let executable = binary.file_name().unwrap().to_str().unwrap();
    let checksum = format!("{:x}", Sha256::digest(binary_bytes));
    fs::write(
        binary
            .parent()
            .unwrap()
            .join(format!("{executable}.sha256")),
        format!("{checksum}  {executable}\n"),
    )
    .unwrap();
    fs::write(
        binary.parent().unwrap().join("manifest.json"),
        serde_json::to_vec(&json!({
            "schema_version":1,
            "kind":"groundline-binary-artifact",
            "groundline_version":env!("CARGO_PKG_VERSION"),
            "target":target,
            "executable":executable,
            "size_bytes":binary_bytes.len(),
            "sha256":checksum,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::create_dir_all(root.path().join(".codex-plugin")).unwrap();
    fs::write(
        root.path().join(".codex-plugin/plugin.json"),
        serde_json::to_vec(&json!({
            "name":"groundline",
            "version":env!("CARGO_PKG_VERSION")
        }))
        .unwrap(),
    )
    .unwrap();
    let output = run(&[
        "provider-smoke",
        "--plugin-root",
        path_argument(root.path()),
        "--require-installed",
        "--json",
    ]);
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    let result = parse_stdout(&output);
    assert_eq!(result["status"], "PASS");
    assert_eq!(result["artifact_verified"], true);
    assert_eq!(result["python_runtime_required"], false);
    assert_eq!(result["hook_event_count"], 0);
    assert_eq!(result["network_capability_present"], false);
}

#[test]
fn provider_smoke_rejects_an_owner_hook_manifest() {
    let root = tempdir().expect("temporary directory");
    fs::create_dir_all(root.path().join(".codex-plugin")).unwrap();
    fs::write(
        root.path().join(".codex-plugin/plugin.json"),
        serde_json::to_vec(&json!({
            "name":"groundline",
            "version":env!("CARGO_PKG_VERSION")
        }))
        .unwrap(),
    )
    .unwrap();
    fs::create_dir(root.path().join("hooks")).unwrap();
    fs::write(root.path().join("hooks/hooks.json"), b"{}").unwrap();

    let output = run(&[
        "provider-smoke",
        "--plugin-root",
        path_argument(root.path()),
        "--json",
    ]);
    assert!(!output.status.success());
    assert_eq!(parse_stdout(&output)["error"], "owner_hook_not_allowed");
}
