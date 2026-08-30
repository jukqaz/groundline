use std::process::{Command, Output};

use serde_json::Value;

const DEPLOYMENT_ENV: &[&str] = &[
    "GROUNDLINE_TRUENAS_URI",
    "GROUNDLINE_TRUENAS_USERNAME",
    "GROUNDLINE_TRUENAS_API_KEY",
    "GROUNDLINE_INSIGHTS_ENROLLMENT_TOKEN",
    "GROUNDLINE_TRUENAS_APP_NAME",
    "GROUNDLINE_INSIGHTS_API_HEALTH_URL",
    "GROUNDLINE_INSIGHTS_GRAFANA_HEALTH_URL",
    "GROUNDLINE_INSIGHTS_ACCESS_URL",
];

fn controller() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_groundline-deploy"));
    for name in DEPLOYMENT_ENV {
        command.env_remove(name);
    }
    command
}

fn receipt(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("controller must emit one JSON receipt")
}

#[test]
fn commands_are_explicit_and_the_removed_flat_interface_is_rejected() {
    let help = controller().arg("--help").output().unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("preflight"));
    assert!(help.contains("apply"));
    assert!(!help.contains("deploy-insights"));

    let removed = controller().arg("--image").arg("invalid").output().unwrap();
    assert!(!removed.status.success());
}

#[test]
fn apply_rejects_an_invalid_image_before_reading_secrets() {
    let output = controller()
        .args([
            "apply",
            "--image",
            "invalid",
            "--expected-current-config-sha256",
            &"a".repeat(64),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let receipt = receipt(&output);
    assert_eq!(receipt["kind"], "groundline-insights-deployment-apply");
    assert_eq!(receipt["status"], "FAIL");
    assert_eq!(receipt["phase"], "input");
    assert_eq!(receipt["mutation_started"], false);
    assert_eq!(receipt["secret_value_printed"], false);
}

#[test]
fn preflight_fails_closed_with_a_redacted_receipt_when_inputs_are_missing() {
    let output = controller().args(["preflight", "--json"]).output().unwrap();
    assert!(!output.status.success());
    let receipt = receipt(&output);
    assert_eq!(receipt["kind"], "groundline-insights-deployment-preflight");
    assert_eq!(receipt["reason"], "missing_deployment_input");
    assert_eq!(receipt["mutation_started"], false);
    assert_eq!(receipt["rollback"], "not_required");
    assert_eq!(receipt["configuration_printed"], false);
    assert_eq!(receipt["private_url_printed"], false);
    assert_eq!(receipt["secret_value_printed"], false);
}

#[test]
fn apply_rejects_a_malformed_preflight_hash_before_connecting() {
    let image = format!(
        "ghcr.io/jukqaz/groundline-insights-api@sha256:{}",
        "a".repeat(64)
    );
    let output = controller()
        .args([
            "apply",
            "--image",
            &image,
            "--expected-current-config-sha256",
            "not-a-sha256",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let receipt = receipt(&output);
    assert_eq!(receipt["phase"], "input");
    assert_eq!(receipt["reason"], "invalid_runtime_configuration");
    assert_eq!(receipt["mutation_started"], false);
}
