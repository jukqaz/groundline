use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

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

fn recommend_stdin(bytes: &[u8]) -> Output {
    let mut child = Command::new(groundline())
        .args(["efficiency", "recommend", "--audit", "-", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn weekly_review_runs_the_installed_command_contract_without_mutating_history() {
    use chrono::{Duration, Utc};
    let home = tempdir().unwrap();
    let sessions = home.path().join("sessions");
    fs::create_dir(&sessions).unwrap();
    let database = home.path().join("state_5.sqlite");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute_batch("CREATE TABLE threads (rollout_path TEXT, source TEXT, has_user_event INTEGER, updated_at INTEGER)").unwrap();
    let now = Utc::now();
    let mut expected = Vec::new();
    for n in 0..5 {
        let owner = format!("test-owner-{n}");
        let path = sessions.join(format!("test-{n}.jsonl"));
        let mut records = vec![
            json!({"timestamp":(now-Duration::seconds(40)).to_rfc3339(),"type":"session_meta","payload":{"id":owner,"originator":"codex_cli"}}),
        ];
        for turn in 1..=2 {
            let at = now - Duration::seconds(30 - turn * 5);
            records.push(json!({"timestamp":at.to_rfc3339(),"type":"event_msg","payload":{"type":"task_started","turn_id":format!("turn-{turn}")}}));
            records.push(json!({"timestamp":(at+Duration::seconds(1)).to_rfc3339(),"type":"token_usage_record","payload":{"thread_id":owner,"response_id":format!("response-{turn}"),"usage":{"input_tokens":9,"output_tokens":1,"total_tokens":10},"thread_token_usage":{"input_tokens":turn*9,"output_tokens":turn,"total_tokens":turn*10}}}));
            records.push(json!({"timestamp":(at+Duration::seconds(2)).to_rfc3339(),"type":"event_msg","payload":{"type":"task_complete","turn_id":format!("turn-{turn}")}}));
        }
        let contents = records
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&path, contents.as_bytes()).unwrap();
        expected.push((path.clone(), contents));
        connection
            .execute(
                "INSERT INTO threads VALUES (?1,'cli',1,?2)",
                rusqlite::params![path.to_str().unwrap(), now.timestamp()],
            )
            .unwrap();
    }
    drop(connection);
    let database_before = fs::read(&database).unwrap();
    let output = run(&[
        "audit",
        "weekly",
        "--review",
        "--codex-home",
        path_argument(home.path()),
        "--json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = parse_stdout(&output);
    assert_eq!(value["kind"], "groundline-codex-weekly-review");
    assert_eq!(
        value["execution"],
        json!({"audit_runs":1,"recommendation_runs":1,"audit_completed":true,"recommendation_completed":true})
    );
    assert_eq!(value["readout"]["completed_root_task_count"], 5);
    assert_eq!(value["readout"]["completed_turn_count"], 10);
    assert_eq!(
        value["audit"]["root"]["provider_reported_usage"]["total_tokens"],
        100
    );
    assert_eq!(value["recommendation"]["status"], "PASS");
    assert!(!String::from_utf8_lossy(&output.stdout).contains(path_argument(home.path())));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("test-owner"));
    assert_eq!(fs::read(&database).unwrap(), database_before);
    for (path, contents) in expected {
        assert_eq!(fs::read_to_string(path).unwrap(), contents);
    }
}

#[test]
fn recommendation_stdin_matches_a_file_without_creating_a_report_file() {
    let fixture = json!({
        "kind":"groundline-codex-weekly-audit","schema":1,"status":"PARTIAL",
        "scope":{"generated_at":"2026-09-14T00:00:00Z","completed_root_sample_count":6},
        "root":{"activity":{},"model_effort":{},"task_latency":{},"prompt_shape":{},"tools":{},"boundary_signals":{}},
        "raw_content_emitted":false,"private_paths_emitted":false,"thread_ids_emitted":false,
        "rollout_paths_emitted":false,"secret_value_printed":false
    });
    let bytes = serde_json::to_vec(&fixture).unwrap();
    let home = tempdir().unwrap();
    let file = home.path().join("weekly.json");
    fs::write(&file, &bytes).unwrap();
    let from_file = run(&[
        "efficiency",
        "recommend",
        "--audit",
        path_argument(&file),
        "--json",
    ]);
    let from_stdin = recommend_stdin(&bytes);
    assert!(from_file.status.success());
    assert!(from_stdin.status.success());
    assert_eq!(parse_stdout(&from_stdin), parse_stdout(&from_file));
    assert_eq!(
        parse_stdout(&from_stdin)["signals"]["short_message_ratio"],
        Value::Null
    );
}

#[test]
fn recommendation_stdin_rejects_empty_malformed_non_object_and_oversized_input_privately() {
    for bytes in [
        vec![],
        b"private-not-json".to_vec(),
        b"[]".to_vec(),
        vec![b' '; 2 * 1024 * 1024 + 1],
    ] {
        let output = recommend_stdin(&bytes);
        assert!(!output.status.success());
        assert_eq!(parse_stdout(&output)["status"], "FAIL");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-not-json"));
        assert!(output.stderr.is_empty());
    }
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
fn integrations_status_is_privacy_bounded_and_provider_honest() {
    let home = tempdir().expect("temporary directory");
    let state = home.path().join("groundline/insights/codex_app-desktop");
    fs::create_dir_all(state.join("outbox")).unwrap();
    fs::write(state.join("identity.json"), "private-collector-id").unwrap();
    fs::write(state.join("outbox/event.json"), "private-event").unwrap();

    let output = run(&[
        "integrations",
        "status",
        "insights",
        "--codex-home",
        path_argument(home.path()),
        "--json",
    ]);
    assert!(output.status.success());
    let result = parse_stdout(&output);
    let encoded = String::from_utf8_lossy(&output.stdout);
    assert_eq!(result["state_observed"], true);
    assert_eq!(result["pending_event_count"], 1);
    assert_eq!(
        result["plugin_installation_status"],
        "provider_check_required"
    );
    assert_eq!(result["hook_trust_status"], "provider_check_required");
    assert!(!encoded.contains("private-"));
    assert!(!encoded.contains(path_argument(home.path())));
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

#[test]
fn personal_cli_rejects_private_invalid_input_without_exposure_or_state_writes() {
    let root = tempdir().unwrap();
    let input = root.path().join("model.json");
    fs::write(&input, b"PRIVATE_INPUT_SENTINEL").unwrap();
    let output = run(&[
        "personal",
        "review",
        "--report",
        path_argument(&input),
        "--audit",
        path_argument(&input),
        "--model-evidence",
        path_argument(&input),
        "--catalog",
        path_argument(&input),
        "--state-dir",
        path_argument(root.path()),
        "--apply",
        "--json",
    ]);
    assert!(!output.status.success());
    let result = parse_stdout(&output);
    assert_eq!(result["error"], "personal_invalid_input");
    assert_eq!(result["raw_content_emitted"], false);
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!rendered.contains("PRIVATE_INPUT_SENTINEL"));
    assert!(!rendered.contains(path_argument(root.path())));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}
