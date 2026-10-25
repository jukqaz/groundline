use serde_json::Value;
use std::fs;
use std::process::Command;

#[test]
fn incomplete_observation_is_non_success_and_does_not_claim_execution() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.toml");
    fs::write(&config, "model = 'PRIVATE_MODEL'\n").unwrap();
    let before = fs::read(&config).unwrap();
    let missing = root.path().join("PRIVATE_MISSING_CLI");
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .arg("environment-observe")
        .arg("--app-cli")
        .arg(&missing)
        .arg("--path-cli")
        .arg(&missing)
        .arg("--codex-home")
        .arg(root.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["schema"], 2);
    assert_eq!(result["metadata_status"], "PARTIAL");
    assert_eq!(result["package_comparison"]["status"], "NOT_REQUESTED");
    assert_eq!(
        result["verification_summary"]["plugin_tool_exposure"],
        "NOT_OBSERVED"
    );
    assert_eq!(
        result["verification_summary"]["plugin_invocation"],
        "NOT_OBSERVED"
    );
    assert_eq!(
        result["verification_summary"]["plugin_outcome"],
        "NOT_VERIFIED"
    );
    assert_eq!(result["model_calls_requested"], 0);
    assert_eq!(result["mutation_performed"], false);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_"));
    assert_eq!(fs::read(config).unwrap(), before);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn incomplete_comparison_arguments_are_rejected_by_cli() {
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .args([
            "environment-observe",
            "--app-cli",
            "unused",
            "--path-cli",
            "unused",
            "--codex-home",
            "unused",
            "--source-plugin-root",
            "unused",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn traversal_is_rejected_before_starting_native_clients() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("config.toml"), "# keep\n").unwrap();
    let source = root.path().join("source");
    let installed = root.path().join("installed");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&installed).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .args([
            "environment-observe",
            "--app-cli",
            "unused",
            "--path-cli",
            "unused",
        ])
        .arg("--codex-home")
        .arg(root.path())
        .arg("--source-plugin-root")
        .arg(&source)
        .arg("--installed-plugin-root")
        .arg(&installed)
        .args(["--compare-file", "../PRIVATE_ESCAPE"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_"));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("environment_observation_comparison_relative_files_required")
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
}
