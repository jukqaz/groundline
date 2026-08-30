use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn groundline() -> &'static str {
    env!("CARGO_BIN_EXE_groundline-insights")
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
fn tailnet_command_is_privacy_bounded_on_the_native_host() {
    let output = run(&["tailnet-status", "--json"]);
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    let result = parse_stdout(&output);
    let serialized = String::from_utf8_lossy(&output.stdout);

    assert_eq!(result["network_performed"], false);
    assert_eq!(result["private_values_emitted"], false);
    assert_eq!(result["probe_method"], "local_cli_only");
    assert!(result["tailnet_reason_code"].is_string());
    if let Ok(current_directory) = std::env::current_dir()
        && let Some(current_directory) = current_directory.to_str()
    {
        assert!(!serialized.contains(current_directory));
    }
}

#[test]
fn fresh_install_is_inert_until_owner_configuration_and_enablement() {
    let home = tempdir().expect("temporary Codex home");
    let home_arg = path_argument(home.path());
    let output = run(&["worker", "status", "--codex-home", home_arg]);
    assert!(output.status.success(), "stderr={:?}", output.stderr);
    let result = parse_stdout(&output);
    assert_eq!(result["kind"], "groundline-insights-worker-status");
    assert_eq!(result["schema"], 2);
    assert_eq!(result["status"], "PASS");
    assert_eq!(result["collection_state"], "disabled");
    assert_eq!(result["collection_enabled"], false);
    assert_eq!(result["ready_to_collect"], false);

    let checkpoint = run(&["checkpoint", "session_start_hook", "--codex-home", home_arg]);
    assert!(
        checkpoint.status.success(),
        "stderr={:?}",
        checkpoint.stderr
    );
    assert!(!home.path().join("groundline/insights").exists());
    let invalid_checkpoint = run(&["checkpoint", "invalid_hook", "--codex-home", home_arg]);
    assert!(!invalid_checkpoint.status.success());
    assert!(!home.path().join("groundline/insights").exists());

    let enable = run(&["worker", "enable", "--codex-home", home_arg]);
    assert!(!enable.status.success());
    let enable = parse_stdout(&enable);
    assert_eq!(enable["kind"], "groundline-insights-worker-error");
    assert_eq!(enable["schema"], 1);
    assert_eq!(enable["network_performed"], false);
    assert_eq!(enable["mutation_performed"], false);
    assert_eq!(enable["private_paths_emitted"], false);
    assert_eq!(enable["secret_value_printed"], false);
    assert_eq!(enable["result_code"], "invalid_owner_profile");

    let example = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/groundline-insights/references/owner-profile.example.json");
    let example = fs::read_to_string(example).expect("read packaged owner profile example");
    assert!(example.contains("REPLACE_ME"));
    let configured = example.replace("REPLACE_ME", &"e".repeat(32));
    let input = home.path().join("owner-profile.input.json");
    fs::write(&input, configured).expect("write private test input");
    let configure = run(&[
        "worker",
        "configure",
        "--input",
        path_argument(&input),
        "--codex-home",
        home_arg,
    ]);
    assert!(configure.status.success(), "stderr={:?}", configure.stderr);
    let configure = parse_stdout(&configure);
    assert_eq!(configure["status"], "PASS");
    assert_eq!(configure["owner_profile_configured"], true);
    assert_eq!(configure["enrollment_credential_configured"], true);
    assert_eq!(configure["endpoint_emitted"], false);
    assert!(!configure.to_string().contains("100.64"));

    let enable = run(&["worker", "enable", "--codex-home", home_arg]);
    assert!(enable.status.success(), "stderr={:?}", enable.stderr);
    assert_eq!(parse_stdout(&enable)["enabled"], true);
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
            "name":"groundline-insights",
            "version":env!("CARGO_PKG_VERSION")
        }))
        .unwrap(),
    )
    .unwrap();
    fs::create_dir(root.path().join("hooks")).unwrap();
    fs::write(
        root.path().join("hooks/hooks.json"),
        br#"{"hooks":{"SessionStart":{"command":"groundline-insights checkpoint"},"Stop":{"command":"groundline-insights checkpoint"},"PostCompact":{"command":"groundline-insights checkpoint"},"SessionEnd":{"command":"groundline-insights checkpoint"}}}"#,
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
}
