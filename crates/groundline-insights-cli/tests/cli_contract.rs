use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

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

fn run_stdin(arguments: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(groundline())
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn learning_profile(home: &Path) -> std::path::PathBuf {
    let canonical = fs::canonicalize(home).unwrap();
    let directory = home.join("groundline/learning");
    fs::create_dir_all(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let input = json!({"kind":"groundline-learning-profile","schema":1,"enabled":true,
        "environment_state":canonical.join("environment"),"learning_state":canonical.join("records"),
        "deliveries":canonical.join("deliveries"),"target_ids":["sample-skill"],"device_id":"device",
        "core_executable":canonical.join("missing-core"),"core_sha256":"0".repeat(64)});
    groundline_runtime::local_file::atomic_write_private(
        &directory.join("profile.json"),
        &serde_json::to_vec(&input).unwrap(),
    )
    .unwrap();
    directory
}

#[test]
fn native_stdin_learning_capture_is_independent_of_owner_collection_consent() {
    let home = tempdir().unwrap();
    let directory = learning_profile(home.path());
    let input = serde_json::to_vec(&json!({"hook_event_name":"UserPromptSubmit",
        "session_id":"native-parent-session","turn_id":"native-turn","model":"gpt-6.1-sol",
        "permission_mode":"default","prompt":"PRIVATE_PROMPT_SENTINEL","cwd":"/PRIVATE_PATH_SENTINEL",
        "transcript_path":"/PRIVATE_TRANSCRIPT_SENTINEL"})).unwrap();
    let output = run_stdin(
        &[
            "checkpoint",
            "user_prompt_submit_hook",
            "--codex-home",
            path_argument(home.path()),
        ],
        &input,
    );
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let files: Vec<_> = fs::read_dir(directory.join("boundaries"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let saved = fs::read_to_string(&files[0]).unwrap();
    assert!(!saved.contains("PRIVATE_"));
    assert!(!saved.contains("native-parent-session"));
    assert!(!saved.contains("native-turn"));
    let boundary: Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(boundary["event"], "UserPromptSubmit");
    assert_eq!(boundary["payload_status"], "complete");
    assert_eq!(boundary["source_verified"], false);
    assert_eq!(boundary["native_activation"], "UNVERIFIED");
    assert!(!home.path().join("groundline/insights").exists());
    let report = parse_stdout(&run(&[
        "worker",
        "status",
        "--codex-home",
        path_argument(home.path()),
    ]));
    assert_eq!(report["collection_enabled"], false);
    assert_eq!(report["learning_boundaries"]["state"], "enabled");
    assert_eq!(report["learning_boundaries"]["boundary_count"], 1);
    assert!(!report.to_string().contains("PRIVATE_"));
    assert!(!report.to_string().contains(path_argument(home.path())));
}

#[test]
fn boundary_failure_does_not_suppress_existing_activity_wakeup_capture() {
    let home = tempdir().unwrap();
    let directory = learning_profile(home.path());
    let profile = directory.join("profile.json");
    fs::set_permissions(&profile, fs::Permissions::from_mode(0o644)).unwrap();
    let activity = groundline_runtime::insights::state_directory(home.path()).unwrap();
    let policy = activity.join("owner-auto-policy.json");
    groundline_runtime::local_file::atomic_write_private(&policy,&serde_json::to_vec(&json!({
        "schema_version":1,"kind":"groundline-insights-owner-auto-policy","status":"active",
        "automatic_activity_checkpoints":true,"collection_scope":"all_activity","diagnostic_enabled":false,
        "trigger_mode":"native_hook_checkpoints","updated_at_utc":"2026-10-07T00:00:00Z"})).unwrap()).unwrap();
    let output = run_stdin(
        &[
            "checkpoint",
            "user_prompt_submit_hook",
            "--codex-home",
            path_argument(home.path()),
        ],
        b"{\"prompt\":\"PRIVATE_INPUT_SENTINEL\"}",
    );
    assert!(output.status.success());
    assert!(
        activity
            .join("hook-captures/user_prompt_submit_hook.json")
            .is_file()
    );
    assert!(!directory.join("boundaries").exists());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
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
fn server_check_is_read_only_and_unconfigured_is_a_nonblocking_skip() {
    let home = tempdir().unwrap();
    let result = run(&[
        "worker",
        "check-server",
        "--codex-home",
        path_argument(home.path()),
        "--json",
    ]);
    assert_eq!(result.status.code(), Some(0));
    let report = parse_stdout(&result);
    assert_eq!(report["status"], "NOT_CONFIGURED");
    assert_eq!(report["result_code"], "owner_profile_not_configured");
    assert_eq!(report["network_attempted"], false);
    assert_eq!(report["mutation_performed"], false);
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);

    let profile = home.path().join("groundline/insights/owner-profile.json");
    groundline_runtime::local_file::atomic_write_private(&profile, b"PRIVATE_INVALID_PROFILE")
        .unwrap();
    let before = (
        fs::read(&profile).unwrap(),
        fs::metadata(&profile).unwrap().modified().unwrap(),
    );
    let result = run(&[
        "worker",
        "check-server",
        "--codex-home",
        path_argument(home.path()),
        "--json",
    ]);
    assert_eq!(result.status.code(), Some(1));
    let report = parse_stdout(&result);
    assert_eq!(report["status"], "FAIL");
    assert_eq!(report["result_code"], "invalid_owner_profile");
    assert_eq!(report["network_attempted"], false);
    assert_eq!(report["mutation_performed"], false);
    assert_eq!(
        (
            fs::read(&profile).unwrap(),
            fs::metadata(&profile).unwrap().modified().unwrap()
        ),
        before
    );
    assert_eq!(fs::read_dir(profile.parent().unwrap()).unwrap().count(), 1);
    let emitted = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!emitted.contains("PRIVATE_INVALID_PROFILE"));
    assert!(!emitted.contains(path_argument(home.path())));
}

#[test]
fn setup_reports_missing_actions_and_never_enables_collection_implicitly() {
    let home = tempdir().unwrap();
    let result = run(&["setup", "--codex-home", path_argument(home.path())]);
    assert_eq!(result.status.code(), Some(2));
    let report = parse_stdout(&result);
    assert_eq!(report["status"], "ACTION_REQUIRED");
    assert_eq!(report["stages"]["collection_consented"], false);
    assert_eq!(report["stages"]["connection_verified_now"], false);
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn setup_keeps_one_connection_and_preserves_identity_on_repeated_enable() {
    let home = tempdir().unwrap();
    let private = tempdir().unwrap();
    let token = private.path().join("token");
    let secret = "PRIVATE_SETUP_SECRET_MUST_NOT_APPEAR_123456789";
    groundline_runtime::local_file::atomic_write_private(&token, secret.as_bytes()).unwrap();
    let arguments = [
        "setup",
        "--codex-home",
        path_argument(home.path()),
        "--endpoint",
        "https://insights.example.com",
        "--enrollment-token-file",
        path_argument(&token),
    ];
    let result = run(&arguments);
    assert_eq!(result.status.code(), Some(2), "{result:?}");
    let report = parse_stdout(&result);
    assert_eq!(report["stages"]["connection_configured"], true);
    assert_eq!(report["stages"]["collection_consented"], false);
    let mut enabled = arguments.to_vec();
    enabled.push("--enable");
    let result = run(&enabled);
    assert_eq!(result.status.code(), Some(2), "{result:?}");
    assert_eq!(
        parse_stdout(&result)["stages"]["collection_consented"],
        true
    );
    let directory = groundline_runtime::insights::state_directory(home.path()).unwrap();
    let paths = [
        directory.join("identity.json"),
        directory.join("consent.json"),
        directory.join("owner-auto-policy.json"),
        home.path().join("groundline/insights/owner-profile.json"),
    ];
    let before = paths.each_ref().map(|p| {
        (
            fs::read(p).unwrap(),
            fs::metadata(p).unwrap().modified().unwrap(),
        )
    });
    let result = run(&enabled);
    let report = parse_stdout(&result);
    assert_eq!(report["configuration_changed"], false);
    assert_eq!(report["collection_enabled_now"], false);
    assert_eq!(report["stages"]["recent_delivery_confirmed"], false);
    assert_eq!(
        paths.each_ref().map(|p| (
            fs::read(p).unwrap(),
            fs::metadata(p).unwrap().modified().unwrap()
        )),
        before
    );
    let emitted = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!emitted.contains(secret));
    assert!(!emitted.contains("insights.example.com"));
    let conflict: Vec<_> = arguments
        .iter()
        .map(|s| {
            if *s == "https://insights.example.com" {
                "https://changed.example.com"
            } else {
                *s
            }
        })
        .collect();
    let result = run(&conflict);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert_eq!(
        parse_stdout(&result)["result_code"],
        "setup_connection_change_requires_review"
    );
    assert_eq!(
        paths.each_ref().map(|p| (
            fs::read(p).unwrap(),
            fs::metadata(p).unwrap().modified().unwrap()
        )),
        before
    );
}

#[cfg(unix)]
#[test]
fn setup_rejects_world_readable_secrets_before_writing_state() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempdir().unwrap();
    let token = home.path().join("token");
    fs::write(&token, "PRIVATE_SECRET_123456789012345678901234567890").unwrap();
    fs::set_permissions(&token, fs::Permissions::from_mode(0o644)).unwrap();
    let result = run(&[
        "setup",
        "--codex-home",
        path_argument(home.path()),
        "--endpoint",
        "https://insights.example.com",
        "--enrollment-token-file",
        path_argument(&token),
        "--enable",
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 1);
}

#[test]
fn rejected_event_validation_never_echoes_input() {
    let root = tempdir().unwrap();
    let input = root.path().join("event.json");
    fs::write(&input, br#"{"private":"PRIVATE_EVENT_SENTINEL"}"#).unwrap();
    let output = run(&[
        "insights",
        "validate-event",
        "--input",
        path_argument(&input),
        "--json",
    ]);
    assert!(!output.status.success());
    assert_eq!(parse_stdout(&output)["status"], "FAIL");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_EVENT_SENTINEL"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_EVENT_SENTINEL"));
}

#[test]
fn unsupported_runtime_environment_never_creates_checkpoint_state() {
    for (name, value) in [
        ("GROUNDLINE_RUNTIME_FAMILY", "claude_code"),
        ("GROUNDLINE_RUNTIME_FAMILY", "hermes"),
        ("GROUNDLINE_RUNTIME_FAMILY", "gemini"),
        ("GROUNDLINE_RUNTIME_FAMILY", ""),
        ("GROUNDLINE_EXECUTION_MODE", "unsupported"),
        ("CODEX_INTERNAL_ORIGINATOR_OVERRIDE", "noncodex-desktop"),
    ] {
        let home = tempdir().unwrap();
        let output = Command::new(groundline())
            .args([
                "checkpoint",
                "session_start_hook",
                "--codex-home",
                path_argument(home.path()),
            ])
            .env_remove("GROUNDLINE_RUNTIME_FAMILY")
            .env_remove("GROUNDLINE_EXECUTION_MODE")
            .env_remove("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")
            .env(name, value)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{name}={value}");
        assert!(fs::read_dir(home.path()).unwrap().next().is_none());
    }
}

#[test]
fn doctor_uses_native_store_discovery_without_model_configuration_or_executables() {
    let home = tempdir().unwrap();
    let empty_path = home.path().join("empty-path");
    fs::create_dir(&empty_path).unwrap();
    let config = b"INVALID TOML [ PRIVATE_CONFIG_SENTINEL";
    fs::write(home.path().join("config.toml"), config).unwrap();
    // Doctor proves presence only, not SQLite schema validity or live delivery.
    let database = home.path().join("state_42.sqlite");
    groundline_runtime::local_file::atomic_write_private(&database, b"presence-fixture").unwrap();
    assert!(groundline_runtime::local_file::owned_by_current_user(
        &fs::File::open(&database).unwrap()
    ));
    let inspect = || {
        let output = Command::new(groundline())
            .args([
                "doctor",
                "--codex-home",
                path_argument(home.path()),
                "--json",
            ])
            .env("PATH", &empty_path)
            .env("OPENCODEX_HOME", home.path().join("removed-proxy"))
            .env("OPENAI_BASE_URL", "http://127.0.0.1:1")
            .output()
            .unwrap();
        assert!(output.status.success(), "stderr={:?}", output.stderr);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_CONFIG_SENTINEL"));
        parse_stdout(&output)
    };
    let result = inspect();
    assert_eq!(result["codex_state_store_present"], true);
    assert_eq!(result["collection_source"], "native_codex_state");
    for field in [
        "inference_proxy_required",
        "model_catalog_required",
        "core_plugin_required",
        "network_performed",
        "mutation_performed",
    ] {
        assert_eq!(result[field], false);
    }
    groundline_runtime::local_file::atomic_write_private(&home.path().join("state_43.sqlite"), &[])
        .unwrap();
    assert_eq!(inspect()["codex_state_store_present"], false);
    assert_eq!(fs::read(home.path().join("config.toml")).unwrap(), config);
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
fn enable_rejects_unsupported_state_with_a_private_nonzero_receipt() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/groundline-insights/references/owner-profile.example.json");
    let profile = fs::read_to_string(example)
        .unwrap()
        .replace("REPLACE_ME", &"e".repeat(32));
    for file in [
        "consent.json",
        "owner-auto-policy.json",
        "owner-auto-status.json",
    ] {
        let home = tempdir().unwrap();
        groundline_runtime::insights_state::configure_profile(home.path(), profile.as_bytes())
            .unwrap();
        groundline_runtime::insights_state::enable(home.path()).unwrap();
        let directory = groundline_runtime::insights::state_directory(home.path()).unwrap();
        let unsupported = br#"{"schema_version":0,"private_value":"PRIVATE_SENTINEL"}"#;
        groundline_runtime::local_file::atomic_write_private(&directory.join(file), unsupported)
            .unwrap();
        let files = [
            "consent.json",
            "owner-auto-policy.json",
            "owner-auto-status.json",
            "identity.json",
        ];
        let before = files.map(|name| fs::read(directory.join(name)).ok());
        let output = run(&[
            "worker",
            "enable",
            "--codex-home",
            path_argument(home.path()),
        ]);
        assert_eq!(output.status.code(), Some(1));
        let result = parse_stdout(&output);
        assert_eq!(result["status"], "FAIL");
        assert_eq!(result["result_code"], "unsupported_local_state");
        assert_eq!(result["network_performed"], false);
        assert_eq!(result["private_paths_emitted"], false);
        assert_eq!(result["secret_value_printed"], false);
        let emitted = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!emitted.contains("PRIVATE_SENTINEL"));
        assert!(!emitted.contains(path_argument(home.path())));
        assert_eq!(
            files.map(|name| fs::read(directory.join(name)).ok()),
            before
        );
        // Explicit revocation must remain possible even when activation is blocked.
        let disable = run(&[
            "worker",
            "disable",
            "--codex-home",
            path_argument(home.path()),
        ]);
        assert!(disable.status.success());
        assert!(!groundline_runtime::insights_state::checkpoint_enabled(home.path()).unwrap());
    }
}

#[test]
fn owner_report_requires_an_explicit_administrative_token_file() {
    let output = run(&["insights", "fetch-report", "--json"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--admin-token-file"), "stderr={stderr}");
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
        br#"{"hooks":{"SessionStart":{"command":"groundline-insights checkpoint"},"UserPromptSubmit":{"command":"groundline-insights checkpoint"},"Stop":{"command":"groundline-insights checkpoint"},"PostCompact":{"command":"groundline-insights checkpoint"},"SessionEnd":{"command":"groundline-insights checkpoint"}}}"#,
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
    assert_eq!(result["hook_event_count"], 5);
    let hook_path = root.path().join("hooks/hooks.json");
    let original_hooks = fs::read(&hook_path).unwrap();
    let mut four_hooks: Value = serde_json::from_slice(&original_hooks).unwrap();
    four_hooks["hooks"]
        .as_object_mut()
        .unwrap()
        .remove("UserPromptSubmit");
    fs::write(&hook_path, serde_json::to_vec(&four_hooks).unwrap()).unwrap();
    let incomplete_hooks = run(&[
        "provider-smoke",
        "--plugin-root",
        path_argument(root.path()),
        "--require-installed",
        "--json",
    ]);
    assert!(!incomplete_hooks.status.success());
    assert_eq!(
        parse_stdout(&incomplete_hooks)["error"],
        "invalid_hook_manifest"
    );
    fs::write(&hook_path, original_hooks).unwrap();

    let checksum_path = binary
        .parent()
        .unwrap()
        .join(format!("{executable}.sha256"));
    fs::write(&checksum_path, format!("{checksum}  {executable}\r\n")).unwrap();
    let crlf = run(&[
        "provider-smoke",
        "--plugin-root",
        path_argument(root.path()),
        "--require-installed",
        "--json",
    ]);
    assert!(crlf.status.success(), "{:?}", crlf.stdout);
    assert_eq!(parse_stdout(&crlf)["artifact_verified"], true);
    fs::write(&binary, b"changed-native-groundline-fixture").unwrap();
    let changed = run(&[
        "provider-smoke",
        "--plugin-root",
        path_argument(root.path()),
        "--require-installed",
        "--json",
    ]);
    assert!(!changed.status.success());
    assert_eq!(parse_stdout(&changed)["error"], "invalid_artifact_checksum");
}
