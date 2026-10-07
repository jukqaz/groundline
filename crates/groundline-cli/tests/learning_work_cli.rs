#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn private(path: &Path, bytes: &[u8]) -> String {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    hash(bytes)
}
fn json_file(path: &Path, value: &Value) -> String {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    private(path, &bytes)
}
fn private_tree(path: &Path, parent: &Path) {
    fs::create_dir_all(path).unwrap();
    for directory in path.ancestors().take_while(|path| *path != parent) {
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    }
}
fn executable() -> &'static Path {
    static INSTALLED: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    let (_, path) = INSTALLED.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.path().canonicalize().unwrap().join("groundline");
        fs::copy(env!("CARGO_BIN_EXE_groundline"), &path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        (directory, path)
    });
    path
}
fn run(root: &Path, args: &[&str], paths: &[(&str, &Path)], success: bool) -> Value {
    let mut command = Command::new(executable());
    command
        .current_dir(root)
        .env("CODEX_HOME", root.join("codex-home"))
        .args(args);
    for (flag, path) in paths {
        command.arg(flag).arg(path);
    }
    let output = command.output().unwrap();
    assert_eq!(
        output.status.success(),
        success,
        "args={args:?}, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains(root.to_str().unwrap()));
    assert!(!text.contains("PRIVATE-NATIVE-BODY"));
    serde_json::from_str(&text).unwrap()
}
fn observation_sha(state: &Path) -> String {
    let metadata = fs::metadata(state).unwrap();
    let head: Value = serde_json::from_slice(&fs::read(state.join("head.json")).unwrap()).unwrap();
    let operations_path = state.join("operations");
    let mut operations = BTreeMap::new();
    if operations_path.exists() {
        for entry in fs::read_dir(operations_path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|v| v == "json") {
                operations.insert(
                    path.file_name().unwrap().to_str().unwrap().to_owned(),
                    hash(&fs::read(path).unwrap()),
                );
            }
        }
    }
    let value = json!({"state_identity":[metadata.dev(),metadata.ino()],"baseline_sha256":head["baseline_sha256"],
        "bindings_sha256":head["bindings_sha256"],"registry_sha256":head["registry_sha256"],"operations":operations});
    let mut bytes = serde_json::to_vec_pretty(&value).unwrap();
    bytes.push(b'\n');
    hash(&bytes)
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
    environment: PathBuf,
    state: PathBuf,
    source: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let home = root.join("codex-home");
        let skills = root.join("skills");
        for path in [&home, &skills] {
            fs::create_dir(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let skill = b"---\nname: workflow\ndescription: fixture\n---\nDeclared workflow.\n";
        private(&skills.join("SKILL.md"), skill);
        let baseline = root.join("baseline.json");
        let bindings = root.join("bindings.json");
        json_file(
            &baseline,
            &json!({"kind":"groundline-environment-baseline","schema":1,
            "revision":"baseline-1","parent_revision":null,"source_revision":"source-1",
            "authority_revision":"authority-1","exception_revision":"device-1","authority_ref":"user-request-1",
            "managed_targets":[{"target_id":"workflow","kind":"skill_file","desired_sha256":hash(skill),"dependencies":[],"managed_block":null}]}),
        );
        json_file(
            &bindings,
            &json!({"kind":"groundline-environment-device-bindings","schema":1,
            "revision":"device-1","device_id":"fixture","roots":[{"root_id":"owner","path":skills,"aliases":[]}],
            "targets":[{"target_id":"workflow","root_id":"owner","relative_path":"SKILL.md"}]}),
        );
        let environment = root.join("environment");
        run(
            &root,
            &["environment", "register", "--json"],
            &[
                ("--state-dir", &environment),
                ("--baseline", &baseline),
                ("--bindings", &bindings),
            ],
            true,
        );
        run(
            &root,
            &[
                "learning",
                "configure",
                "--target",
                "workflow",
                "--device-id",
                "fixture",
            ],
            &[("--environment-state", &environment)],
            true,
        );
        run(&root, &["learning", "enable"], &[], true);
        let sessions = home.join("sessions");
        fs::create_dir(&sessions).unwrap();
        let source = sessions.join("rollout-fixture.jsonl");
        let start = chrono::Utc::now().to_rfc3339();
        let lines = [
            json!({"type":"session_meta","payload":{"id":"fixture-owner","instructions":"PRIVATE-NATIVE-BODY"}}),
            json!({"timestamp":start,"type":"turn_context","payload":{"turn_id":"fixture-turn","model":"gpt-6.1-sol","effort":"medium"}}),
            json!({"timestamp":start,"type":"event_msg","payload":{"type":"task_started","turn_id":"fixture-turn"}}),
        ];
        let bytes = lines
            .iter()
            .map(|v| format!("{}\n", serde_json::to_string(v).unwrap()))
            .collect::<String>();
        private(&source, bytes.as_bytes());
        let state = home.join("groundline/learning/records");
        let fixture = Self {
            _temp: temp,
            root,
            home,
            environment,
            state,
            source,
        };
        fixture.boundary(
            "start",
            "UserPromptSubmit",
            "fixture-owner",
            "fixture-turn",
            &start,
        );
        fixture
    }
    fn boundary(&self, id: &str, event: &str, owner: &str, turn: &str, at: &str) {
        let status = run(
            &self.root,
            &["environment", "status"],
            &[("--state-dir", &self.environment)],
            true,
        );
        let path = self
            .home
            .join("groundline/learning/boundaries")
            .join(format!("{id}.json"));
        json_file(
            &path,
            &json!({"kind":"groundline-learning-boundary","schema":1,"boundary_id":id,"event":event,
            "observed_at_utc":at,"session_hash":hash(format!("groundline-hook-session\0{owner}").as_bytes()),
            "turn_hash":hash(format!("groundline-hook-turn\0{turn}").as_bytes()),"model":null,"permission_mode":null,
            "payload_status":"complete","missing_fields":[],"native_evidence_sha256":hash(id.as_bytes()),
            "source_verified":false,"device_id":"fixture","native_activation":"UNVERIFIED",
            "targets":[{"target_id":"workflow","snapshot_code":"observed","skill_revision":hash(&fs::read(self.root.join("skills/SKILL.md")).unwrap()),
                "environment_revision":null,"source_revision":"source-1","baseline_sha256":status["common_change_ref"],
                "observation_state_sha256":observation_sha(&self.environment),"observed_at_utc":at}]}),
        );
    }
    fn start(&self, id: &str) -> Value {
        self.start_from_boundary(id, "start")
    }
    fn start_from_boundary(&self, id: &str, boundary_id: &str) -> Value {
        let path = self.root.join(format!("{id}-start.json"));
        json_file(
            &path,
            &json!({"kind":"groundline-learning-task-start-input","schema":1,
            "scope":{"unit_hash":hash(id.as_bytes()),"cohort_sha256":hash(b"cohort"),"phase":"implementation",
                "task_category":"code-change","criterion":{"sha256":hash(b"test-command-and-result"),"version":"v1"},
                "target_id":"workflow","runtime":{"family":"codex_cli","version":env!("CARGO_PKG_VERSION"),"evidence_sha256":hash(&fs::read(&self.source).unwrap())}},
            "boundary_id":boundary_id,"criterion_change_kind":"unknown","criterion_change_evidence_sha256":null}),
        );
        run(
            &self.root,
            &["learning", "task-start"],
            &[("--input", &path), ("--native-artifact", &self.source)],
            true,
        )
    }
    fn assessment(&self, start: &Value, status: &str) -> (Value, PathBuf, PathBuf) {
        let evidence = self.root.join("direct-result.txt");
        let digest = private(&evidence, b"Actual checked local test output: PASS\n");
        let path = self.root.join("assessment.json");
        json_file(
            &path,
            &json!({"kind":"groundline-learning-assessment-input","schema":1,"task_sha256":start["task_sha256"],
            "verification":{"status":status,"evidence_kind":if status == "unknown" {"unobserved"} else {"runtime_check"},
                "evidence_sha256":if status == "unknown" {Value::Null} else {json!(digest)},"rework":status == "failed"}}),
        );
        let paths = if status == "unknown" {
            vec![("--input", path.as_path())]
        } else {
            vec![
                ("--input", path.as_path()),
                ("--verification-evidence", evidence.as_path()),
            ]
        };
        (
            run(&self.root, &["learning", "assess"], &paths, true),
            path,
            evidence,
        )
    }
    fn capture_boundary(&self, event: &str) -> Value {
        let trigger = match event {
            "UserPromptSubmit" => "user_prompt_submit_hook",
            "Stop" => "stop_hook",
            _ => panic!("unsupported fixture event"),
        };
        let payload = serde_json::to_vec(&json!({"hook_event_name":event,
            "session_id":"fixture-owner","turn_id":"fixture-turn","model":"gpt-6.1-sol",
            "permission_mode":"default","prompt":"PRIVATE-NATIVE-BODY","transcript_path":self.source})).unwrap();
        let id =
            groundline_runtime::learning_boundary::capture(&self.home, trigger, payload.as_slice())
                .unwrap()
                .unwrap();
        serde_json::from_slice(
            &fs::read(
                self.home
                    .join("groundline/learning/boundaries")
                    .join(format!("{id}.json")),
            )
            .unwrap(),
        )
        .unwrap()
    }
    fn finish_native(&self, complete_cost: bool) -> String {
        let usage = if complete_cost {
            json!({"input_tokens":100,"cached_input_tokens":20,"output_tokens":5,"reasoning_output_tokens":2,"total_tokens":105})
        } else {
            json!({"total_tokens":105})
        };
        let response_at = chrono::Utc::now().to_rfc3339();
        let close_at = chrono::Utc::now().to_rfc3339();
        let lines = [
            json!({"timestamp":response_at,"type":"token_usage_record","payload":{"thread_id":"fixture-owner","turn_id":"fixture-turn",
                "response_id":"fixture-response","usage":usage,"turn_token_usage":usage,"thread_token_usage":usage}}),
            json!({"timestamp":close_at,"type":"event_msg","payload":{"type":"task_complete","turn_id":"fixture-turn"}}),
        ];
        let mut file = OpenOptions::new().append(true).open(&self.source).unwrap();
        for value in lines {
            writeln!(file, "{}", serde_json::to_string(&value).unwrap()).unwrap();
        }
        close_at
    }
    fn close(&self, complete_cost: bool) -> String {
        self.finish_native(complete_cost);
        let end_at = chrono::Utc::now().to_rfc3339();
        self.boundary("end", "Stop", "fixture-owner", "fixture-turn", &end_at);
        end_at
    }
    fn consume(&self) -> Value {
        run(&self.root, &["learning", "consume"], &[], true)
    }
    fn append_later_turn(&self) {
        let at = chrono::Utc::now().to_rfc3339();
        let usage = json!({"input_tokens":10000,"cached_input_tokens":2,"output_tokens":1000,"reasoning_output_tokens":50,"total_tokens":11000});
        let lines = [
            json!({"timestamp":at,"type":"turn_context","payload":{"turn_id":"later-turn","model":"gpt-6-astra","effort":"high"}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_started","turn_id":"later-turn"}}),
            json!({"timestamp":at,"type":"token_usage_record","payload":{"thread_id":"fixture-owner","turn_id":"later-turn",
                "response_id":"later-response","usage":usage,"turn_token_usage":usage}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_complete","turn_id":"later-turn"}}),
        ];
        let mut file = OpenOptions::new().append(true).open(&self.source).unwrap();
        for value in lines {
            writeln!(file, "{}", serde_json::to_string(&value).unwrap()).unwrap();
        }
    }
    fn records(&self, kind: &str) -> Vec<(PathBuf, Value)> {
        fs::read_dir(&self.state)
            .unwrap()
            .filter_map(|entry| {
                let path = entry.unwrap().path();
                if path.extension().is_none_or(|v| v != "json") {
                    return None;
                }
                let value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                (value["kind"] == kind).then_some((path, value))
            })
            .collect()
    }
    fn receipt(&self) -> Value {
        let outcome = self
            .records("groundline-learning-task-outcome")
            .pop()
            .unwrap()
            .1;
        serde_json::from_slice(
            &fs::read(
                self.home
                    .join("groundline/learning/deliveries")
                    .join(format!(
                        "{}.json",
                        outcome["receipt_sha256"].as_str().unwrap()
                    )),
            )
            .unwrap(),
        )
        .unwrap()
    }
}

#[test]
fn captured_stop_before_native_completion_links_once_after_actual_closure() {
    let fixture = Fixture::new();
    fs::remove_file(
        fixture
            .home
            .join("groundline/learning/boundaries/start.json"),
    )
    .unwrap();
    let start_boundary = fixture.capture_boundary("UserPromptSubmit");
    let started = fixture.start_from_boundary(
        "captured-end-unit",
        start_boundary["boundary_id"].as_str().unwrap(),
    );
    assert_eq!(started["native_boundary_matched"], true);
    let (assessment, _, evidence) = fixture.assessment(&started, "verified");
    assert_eq!(assessment["status"], "PENDING");
    let end_boundary = fixture.capture_boundary("Stop");
    let stop_at = end_boundary["observed_at_utc"].as_str().unwrap();
    let pending = fixture.consume();
    assert_eq!(pending["assessments"][0]["status"], "PENDING");
    assert!(
        fixture
            .records("groundline-learning-task-outcome")
            .is_empty()
    );

    // App dispatches Stop before persisting its task_complete event. A hook
    // cutoff alone would exclude the real closure permanently on every retry.
    let closed_at = fixture.finish_native(true);
    assert!(
        chrono::DateTime::parse_from_rfc3339(&closed_at).unwrap()
            > chrono::DateTime::parse_from_rfc3339(stop_at).unwrap()
    );
    let session_hash = hash(b"groundline-hook-session\0fixture-owner");
    let turn_hash = hash(b"groundline-hook-turn\0fixture-turn");
    let proof = groundline_runtime::learning_response::observe(
        &groundline_runtime::learning_response::TaskObservation {
            unit_hash: hash(b"captured-end-unit"),
            root: groundline_runtime::learning_response::NativeRef {
                path: fixture.source.clone(),
                session_hash,
                turn_hash,
            },
            children: vec![],
            owned_root_turn_hashes: vec![],
            start_utc: start_boundary["observed_at_utc"].as_str().unwrap().into(),
            end_utc: stop_at.into(),
        },
    )
    .unwrap();
    assert!(!proof.closure_observed);
    assert!(
        proof
            .missing
            .iter()
            .any(|v| v == "native_root_closure_missing")
    );

    fixture.append_later_turn();
    let archived = fixture.home.join("archived_sessions");
    private_tree(&archived, &fixture.root);
    fs::rename(&fixture.source, archived.join("rollout-fixture.jsonl")).unwrap();
    let consumed = fixture.consume();
    assert_eq!(
        consumed["assessments"][0]["status"], "FINALIZED",
        "{consumed}"
    );
    assert_eq!(consumed["archived_boundary_count"], 0);
    let outcome = fixture
        .records("groundline-learning-task-outcome")
        .pop()
        .unwrap()
        .1;
    assert_eq!(
        outcome["boundary_sha256"],
        groundline_contracts::learning::content_sha256(&end_boundary).unwrap()
    );
    let receipt = fixture.receipt();
    assert_eq!(receipt["completed_at_utc"], closed_at);
    assert_eq!(
        receipt["verification"]["evidence_sha256"],
        hash(&fs::read(evidence).unwrap())
    );
    assert_eq!(receipt["resources"]["entries"][0]["total_tokens"], 105);
    assert_eq!(receipt["resources"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(receipt["effective"]["model"], "gpt-6.1-sol");
    assert_eq!(receipt["effective"]["effort"], "medium");
    let saved_proof: Value = serde_json::from_slice(
        &fs::read(fixture.state.join("evidence").join(format!(
            "{}.artifact",
            outcome["native_response_proof_sha256"].as_str().unwrap()
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(saved_proof["proof"]["root_closed_at_utc"], closed_at);
    assert!(
        chrono::DateTime::parse_from_rfc3339(
            saved_proof["observation_cutoff_utc"].as_str().unwrap()
        )
        .unwrap()
            > chrono::DateTime::parse_from_rfc3339(&closed_at).unwrap()
    );
    assert_eq!(fixture.consume()["mutation_performed"], false);
    assert_eq!(fixture.records("groundline-learning-task-outcome").len(), 1);
    let patterns = run(&fixture.root, &["learning", "patterns"], &[], true);
    assert_eq!(patterns["readiness"]["totals"]["task_outcome_count"], 1);
    assert_eq!(
        patterns["readiness"]["totals"]["assessment_pending_connection_count"],
        0
    );
}

#[test]
fn assessment_waits_for_real_end_then_automatically_links_original_evidence_and_cost_once() {
    let fixture = Fixture::new();
    let started = fixture.start("automatic-unit");
    let (assessment, input, evidence) = fixture.assessment(&started, "verified");
    assert_eq!(assessment["status"], "PENDING");
    assert_eq!(fixture.consume()["assessments"][0]["status"], "PENDING");
    assert!(
        fixture
            .records("groundline-learning-task-outcome")
            .is_empty()
    );
    let replay = run(
        &fixture.root,
        &["learning", "assess"],
        &[("--input", &input), ("--verification-evidence", &evidence)],
        true,
    );
    assert_eq!(replay["submitted_at_utc"], assessment["submitted_at_utc"]);
    assert_eq!(replay["mutation_performed"], false);
    let end_at = fixture.close(true);
    let consumed = fixture.consume();
    assert_eq!(
        consumed["assessments"][0]["status"], "FINALIZED",
        "{consumed}"
    );
    let receipt = fixture.receipt();
    assert_eq!(receipt["completed_at_utc"], end_at);
    assert_eq!(
        receipt["verification"]["evidence_sha256"],
        hash(&fs::read(&evidence).unwrap())
    );
    assert_eq!(receipt["verification"]["status"], "verified");
    assert_eq!(receipt["effective"]["model"], "gpt-6.1-sol");
    assert_eq!(receipt["effective"]["effort"], "medium");
    assert_eq!(receipt["resources"]["complete"], true);
    assert_eq!(receipt["resources"]["entries"][0]["total_tokens"], 105);
    assert_eq!(fixture.records("groundline-learning-task-outcome").len(), 1);
    assert_eq!(fixture.records("groundline-learning-assessment").len(), 1);
    let again = fixture.consume();
    assert_eq!(again["mutation_performed"], false);
    let patterns = run(&fixture.root, &["learning", "patterns"], &[], true);
    assert_eq!(patterns["patterns"][0]["sample_count"], 1);
    assert_eq!(patterns["readiness"]["totals"]["task_outcome_count"], 1);
    assert_eq!(
        patterns["readiness"]["totals"]["assessment_pending_connection_count"],
        0
    );
    for (_, record) in fixture.records("groundline-learning-task-outcome") {
        assert!(
            !serde_json::to_string(&record)
                .unwrap()
                .contains(fixture.root.to_str().unwrap())
        );
    }
}

#[tokio::test]
async fn actual_core_worker_observes_closed_cost_without_llm_and_keeps_finalization_idempotent() {
    let fixture = Fixture::new();
    let started = fixture.start("actual-background-consumer");
    let (_, _, evidence) = fixture.assessment(&started, "verified");
    fixture.capture_boundary("Stop");
    fixture.finish_native(true);
    let original_source = fs::read(&fixture.source).unwrap();
    let original_evidence = fs::read(&evidence).unwrap();
    let worker = groundline_runtime::learning_boundary::consume(&fixture.home).await;
    assert_eq!(worker["result_code"], "consumed", "{worker}");
    assert_eq!(worker["core_status"], "CONSUMED");
    assert_eq!(worker["consume_attempt_count"], 1);
    assert_eq!(worker["pending_assessment_count"], 0);
    assert_eq!(worker["finalized_assessment_count"], 1);
    assert_eq!(worker["network_performed"], false);
    assert_eq!(
        groundline_runtime::learning_boundary::status(&fixture.home)["last_worker"],
        worker
    );
    let receipt = fixture.receipt();
    assert_eq!(receipt["resources"]["complete"], true);
    assert_eq!(receipt["resources"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(receipt["resources"]["entries"][0]["total_tokens"], 105);
    assert_eq!(receipt["effective"]["model"], "gpt-6.1-sol");
    assert_eq!(receipt["effective"]["effort"], "medium");
    assert_eq!(
        receipt["verification"]["evidence_sha256"],
        hash(&original_evidence)
    );
    assert_eq!(fs::read(&fixture.source).unwrap(), original_source);
    assert_eq!(fs::read(evidence).unwrap(), original_evidence);
    assert!(!fixture.home.join("groundline/insights").exists());
    assert!(!worker.to_string().contains(fixture.root.to_str().unwrap()));
    assert!(!worker.to_string().contains("fixture-owner"));
    fixture.append_later_turn();
    let again = groundline_runtime::learning_boundary::consume(&fixture.home).await;
    assert_eq!(again["result_code"], "consumed");
    assert_eq!(again["finalized_assessment_count"], 0);
    assert_eq!(fixture.records("groundline-learning-task-outcome").len(), 1);
    assert_eq!(fixture.receipt(), receipt);
    let patterns = run(&fixture.root, &["learning", "patterns"], &[], true);
    assert_eq!(patterns["readiness"]["totals"]["task_outcome_count"], 1);
    assert_eq!(
        patterns["readiness"]["totals"]["assessment_pending_connection_count"],
        0
    );
}

#[test]
fn stale_wrong_session_wrong_turn_and_hook_without_native_closure_remain_pending() {
    let fixture = Fixture::new();
    let started = fixture.start("pending-unit");
    fixture.boundary(
        "stale",
        "Stop",
        "fixture-owner",
        "fixture-turn",
        &chrono::Utc::now().to_rfc3339(),
    );
    fixture.assessment(&started, "unknown");
    for (id, owner, turn) in [
        ("wrong-session", "other-owner", "fixture-turn"),
        ("wrong-turn", "fixture-owner", "other-turn"),
    ] {
        fixture.boundary(id, "Stop", owner, turn, &chrono::Utc::now().to_rfc3339());
    }
    let consumed = fixture.consume();
    assert_eq!(consumed["assessments"][0]["status"], "PENDING");
    fixture.boundary(
        "hook-only",
        "Stop",
        "fixture-owner",
        "fixture-turn",
        &chrono::Utc::now().to_rfc3339(),
    );
    assert_eq!(fixture.consume()["assessments"][0]["status"], "PENDING");
    assert!(
        fixture
            .records("groundline-learning-task-outcome")
            .is_empty()
    );
    fixture.close(true);
    assert_eq!(fixture.consume()["assessments"][0]["status"], "FINALIZED");
}

#[test]
fn explicit_unknown_and_partial_cost_are_recorded_without_becoming_verified() {
    for status in ["unknown", "failed"] {
        let fixture = Fixture::new();
        let started = fixture.start(status);
        fixture.assessment(&started, status);
        fixture.close(false);
        let consumed = fixture.consume();
        assert_eq!(
            consumed["assessments"][0]["status"], "FINALIZED",
            "{consumed}"
        );
        let receipt = fixture.receipt();
        assert_eq!(receipt["verification"]["status"], status);
        assert_eq!(receipt["resources"]["complete"], false);
        assert!(receipt["resources"]["entries"][0]["input_tokens"].is_null());
        let patterns = run(&fixture.root, &["learning", "patterns"], &[], true);
        assert_eq!(patterns["patterns"][0]["resource_gap_count"], 1);
        assert_eq!(patterns["patterns"][0]["correction_unknown_count"], 1);
        let outcome = fixture
            .records("groundline-learning-task-outcome")
            .pop()
            .unwrap()
            .1;
        let proof_sha = outcome["native_response_proof_sha256"].as_str().unwrap();
        let proof_bytes = fs::read(
            fixture
                .state
                .join("evidence")
                .join(format!("{proof_sha}.artifact")),
        )
        .unwrap();
        assert_eq!(hash(&proof_bytes), proof_sha);
        let proof: Value = serde_json::from_slice(&proof_bytes).unwrap();
        assert!(
            proof["proof"]["missing"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "native_usage_counters_missing")
        );
    }
}

#[test]
fn assessment_rejects_changed_evidence_changed_input_and_supplied_clock() {
    let fixture = Fixture::new();
    let started = fixture.start("immutable-unit");
    let (_, input, evidence) = fixture.assessment(&started, "verified");
    private(&evidence, b"Changed test result.\n");
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "assess"],
            &[("--input", &input), ("--verification-evidence", &evidence)],
            false
        )["error"],
        "learning_assessment_evidence_mismatch"
    );
    let mut value: Value = serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
    value["verification"]["evidence_sha256"] = json!(hash(&fs::read(&evidence).unwrap()));
    json_file(&input, &value);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "assess"],
            &[("--input", &input), ("--verification-evidence", &evidence)],
            false
        )["error"],
        "learning_assessment_conflict"
    );
    value["submitted_at_utc"] = json!("2026-01-01T00:00:00Z");
    json_file(&input, &value);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "assess"],
            &[("--input", &input), ("--verification-evidence", &evidence)],
            false
        )["error"],
        "learning_invalid_assessment"
    );
}

#[test]
fn mutated_preserved_evidence_and_missing_receipt_prevent_verified_resume() {
    let fixture = Fixture::new();
    let started = fixture.start("guarded-resume");
    let (_, _, evidence) = fixture.assessment(&started, "verified");
    fixture.close(true);
    fixture.consume();
    for (path, _) in fixture
        .records("groundline-learning-task-outcome")
        .into_iter()
        .chain(fixture.records("groundline-learning-link"))
    {
        fs::remove_file(path).unwrap();
    }
    let hash = hash(&fs::read(&evidence).unwrap());
    private(
        &fixture
            .state
            .join("evidence")
            .join(format!("{hash}.artifact")),
        b"Changed saved evidence.\n",
    );
    let consumed = fixture.consume();
    assert_eq!(consumed["assessments"][0]["status"], "PENDING");
    assert!(
        fixture
            .records("groundline-learning-task-outcome")
            .is_empty()
    );
    assert_eq!(
        consumed["readiness"]["totals"]["assessment_pending_connection_count"],
        1
    );
}

#[test]
fn automatic_finalization_resumes_existing_request_without_duplicating_cost() {
    let fixture = Fixture::new();
    let started = fixture.start("resumed-unit");
    fixture.assessment(&started, "verified");
    fixture.close(true);
    fixture.consume();
    let outcome = fixture
        .records("groundline-learning-task-outcome")
        .pop()
        .unwrap();
    let original = outcome.1.clone();
    fs::remove_file(outcome.0).unwrap();
    let consumed = fixture.consume();
    assert_eq!(consumed["resumed_finalize_count"], 1);
    assert_eq!(
        fixture
            .records("groundline-learning-task-outcome")
            .pop()
            .unwrap()
            .1,
        original
    );
    assert_eq!(
        fixture
            .records("groundline-learning-finalize-request")
            .len(),
        1
    );
    assert_eq!(
        fixture.receipt()["resources"]["entries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn pending_assessment_retains_task_snapshot_start_and_evidence_during_compaction() {
    let fixture = Fixture::new();
    let started = fixture.start("retained-pending");
    let (assessment, _, evidence) = fixture.assessment(&started, "verified");
    for n in 0..401 {
        let value = json!({"kind":"groundline-learning-processing","schema":1,"input_sha256":hash(format!("unused-{n}").as_bytes()),
            "context_sha256":hash(b"context"),"status":"OBSERVED","evidence_refs":[],"reasons":[]});
        let mut bytes = serde_json::to_vec_pretty(&value).unwrap();
        bytes.push(b'\n');
        private(
            &fixture.state.join(format!(
                "{}.json",
                groundline_contracts::learning::content_sha256(&value).unwrap()
            )),
            &bytes,
        );
    }
    let consumed = fixture.consume();
    assert!(consumed["archived_record_count"].as_u64().unwrap() > 0);
    assert!(
        fixture
            .state
            .join(format!("{}.json", started["task_sha256"].as_str().unwrap()))
            .exists()
    );
    assert!(
        fixture
            .state
            .join(format!(
                "{}.json",
                assessment["assessment_sha256"].as_str().unwrap()
            ))
            .exists()
    );
    assert_eq!(fixture.records("groundline-learning-snapshot").len(), 1);
    assert_eq!(fixture.records("groundline-learning-boundary").len(), 1);
    let evidence_sha = hash(&fs::read(&evidence).unwrap());
    assert_eq!(
        fs::read(
            fixture
                .state
                .join("evidence")
                .join(format!("{evidence_sha}.artifact"))
        )
        .unwrap(),
        fs::read(evidence).unwrap()
    );
    fixture.close(true);
    assert_eq!(fixture.consume()["assessments"][0]["status"], "FINALIZED");
}

#[test]
fn old_canonical_session_is_found_before_sixty_four_unrelated_recent_prefixes() {
    let mut fixture = Fixture::new();
    let owner = "11111111-1111-4111-8111-111111111111";
    let original = fs::read_to_string(&fixture.source)
        .unwrap()
        .replace("fixture-owner", owner);
    let old = fixture
        .home
        .join("sessions")
        .join(format!("rollout-2026-09-30T18-24-07-{owner}.jsonl"));
    fs::remove_file(&fixture.source).unwrap();
    private(&old, original.as_bytes());
    fixture.source = old;
    let start_path = fixture
        .home
        .join("groundline/learning/boundaries/start.json");
    let start_boundary: Value = serde_json::from_slice(&fs::read(&start_path).unwrap()).unwrap();
    fixture.boundary(
        "start",
        "UserPromptSubmit",
        owner,
        "fixture-turn",
        start_boundary["observed_at_utc"].as_str().unwrap(),
    );
    let started = fixture.start("long-lived-session");
    fixture.assessment(&started, "unknown");
    fixture.close(true);
    let source = fs::read_to_string(&fixture.source)
        .unwrap()
        .replace("fixture-owner", owner);
    private(&fixture.source, source.as_bytes());
    fixture.boundary(
        "end",
        "Stop",
        owner,
        "fixture-turn",
        &chrono::Utc::now().to_rfc3339(),
    );
    for n in 0..70 {
        let id = uuid::Uuid::from_u128(0x22222222222242228222000000000000 + n);
        let path = fixture
            .home
            .join("sessions")
            .join(format!("rollout-2026-10-08T23-59-59-{id}.jsonl"));
        private(
            &path,
            format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\"}}}}\n").as_bytes(),
        );
    }
    let consumed = fixture.consume();
    assert_eq!(
        consumed["assessments"][0]["status"], "FINALIZED",
        "{consumed}"
    );
    assert_eq!(
        fixture.receipt()["resources"]["entries"][0]["total_tokens"],
        105
    );
}

#[test]
fn matching_filename_hint_is_not_accepted_as_native_identity() {
    let mut fixture = Fixture::new();
    let owner = "11111111-1111-4111-8111-111111111111";
    let original = fs::read_to_string(&fixture.source)
        .unwrap()
        .replace("fixture-owner", owner);
    private(&fixture.source, original.as_bytes());
    let start_path = fixture
        .home
        .join("groundline/learning/boundaries/start.json");
    let boundary: Value = serde_json::from_slice(&fs::read(start_path).unwrap()).unwrap();
    fixture.boundary(
        "start",
        "UserPromptSubmit",
        owner,
        "fixture-turn",
        boundary["observed_at_utc"].as_str().unwrap(),
    );
    let started = fixture.start("forged-filename");
    fixture.assessment(&started, "unknown");
    fixture.close(true);
    let original = fs::read_to_string(&fixture.source)
        .unwrap()
        .replace("fixture-owner", owner);
    private(&fixture.source, original.as_bytes());
    fixture.boundary(
        "end",
        "Stop",
        owner,
        "fixture-turn",
        &chrono::Utc::now().to_rfc3339(),
    );
    let external = fixture.root.join("retained-native.jsonl");
    fs::rename(&fixture.source, &external).unwrap();
    let forged = fixture
        .home
        .join("sessions")
        .join(format!("rollout-2026-09-30T18-24-07-{owner}.jsonl"));
    private(
        &forged,
        b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"forged-owner\"}}\n",
    );
    assert_eq!(fixture.consume()["assessments"][0]["status"], "PENDING");
    assert!(
        fixture
            .records("groundline-learning-task-outcome")
            .is_empty()
    );
    fixture.source = fixture.home.join("sessions/irregular-name.jsonl");
    fs::rename(external, &fixture.source).unwrap();
    assert_eq!(fixture.consume()["assessments"][0]["status"], "FINALIZED");
}

#[test]
fn separate_units_cannot_claim_the_same_native_response() {
    let fixture = Fixture::new();
    let first = fixture.start("first-owner");
    let second = fixture.start("second-owner");
    fixture.assessment(&first, "unknown");
    fixture.assessment(&second, "unknown");
    fixture.close(true);
    let consumed = fixture.consume();
    let rows = consumed["assessments"].as_array().unwrap();
    assert_eq!(
        rows.iter().filter(|v| v["status"] == "FINALIZED").count(),
        1,
        "{consumed}"
    );
    assert!(
        rows.iter()
            .any(|v| v["reason"] == "delivery_overlapping_response_ownership"),
        "{consumed}"
    );
    assert_eq!(fixture.records("groundline-learning-task-outcome").len(), 1);
}

#[test]
fn cold_assessment_retry_restores_completion_and_rejects_a_new_result() {
    let fixture = Fixture::new();
    let started = fixture.start("cold-completion");
    let (assessment, input, evidence) = fixture.assessment(&started, "verified");
    fixture.close(true);
    fixture.consume();
    let outcome = fixture
        .records("groundline-learning-task-outcome")
        .pop()
        .unwrap()
        .1;
    let task = fixture
        .records("groundline-learning-task-start")
        .pop()
        .unwrap()
        .1;
    let archived = fixture.state.join("archive");
    fs::create_dir(&archived).unwrap();
    fs::set_permissions(&archived, fs::Permissions::from_mode(0o700)).unwrap();
    let task_hash = started["task_sha256"].as_str().unwrap();
    let task_input = task["input_sha256"].as_str().unwrap();
    let indices = archived.join("inputs").join(&task_input[..2]);
    fs::create_dir_all(&indices).unwrap();
    for path in [archived.join("inputs"), indices.clone()] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    json_file(
        &indices.join(format!("{task_input}.json")),
        &json!({"kind":"groundline-learning-archive-input","schema":1,
        "input_sha256":task_input,"record_sha256":task_hash,"assessment_sha256":assessment["assessment_sha256"],
        "completion_sha256":groundline_contracts::learning::content_sha256(&outcome).unwrap()}),
    );
    let files: Vec<_> = fs::read_dir(&fixture.state)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|v| v == "json"))
        .collect();
    for path in files {
        let name = path.file_name().unwrap().to_str().unwrap();
        let shard = archived.join(&name[..2]);
        if !shard.exists() {
            fs::create_dir(&shard).unwrap();
            fs::set_permissions(&shard, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::rename(&path, shard.join(name)).unwrap();
    }
    let repeated = run(
        &fixture.root,
        &["learning", "assess"],
        &[("--input", &input), ("--verification-evidence", &evidence)],
        true,
    );
    assert_eq!(repeated["status"], "ALREADY_ASSESSED");
    assert_eq!(repeated["completion_pending"], false);
    let mut changed: Value = serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
    changed["verification"]["rework"] = json!(true);
    json_file(&input, &changed);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "assess"],
            &[("--input", &input), ("--verification-evidence", &evidence)],
            false
        )["error"],
        "learning_assessment_conflict"
    );
}

#[test]
fn interrupted_generated_publication_reuses_journaled_original_after_native_append() {
    // Unix permission failures inject real interruptions after the journal.
    // Root can bypass mode bits, so that environment needs another harness.
    if rustix::process::geteuid().as_raw() == 0 {
        return;
    }
    for fail_after_ownership in [false, true] {
        let fixture = Fixture::new();
        let started = fixture.start("interrupted-publication");
        fixture.assessment(&started, "verified");
        fixture.close(true);
        let deliveries = fixture.home.join("groundline/learning/deliveries");
        let unit = hash(b"interrupted-publication");
        let response = hash(b"groundline-native-response\0fixture-response");
        let units = deliveries.join("ownership/units");
        let unit_shard = units.join(&unit[..2]);
        let response_shard = deliveries.join("ownership/responses").join(&response[..2]);
        if fail_after_ownership {
            private_tree(&unit_shard, &deliveries);
            private_tree(&response_shard, &deliveries);
            fs::set_permissions(&deliveries, fs::Permissions::from_mode(0o500)).unwrap();
        } else {
            private_tree(&units, &deliveries);
            fs::set_permissions(&units, fs::Permissions::from_mode(0o500)).unwrap();
        }
        let interrupted = fixture.consume();
        assert_eq!(
            interrupted["assessments"][0]["status"], "PENDING",
            "{interrupted}"
        );
        let requests = fixture.records("groundline-learning-finalize-request");
        assert_eq!(
            requests.len(),
            1,
            "journal must precede ownership/live publication"
        );
        let request = &requests[0].1;
        let receipt_sha = request["receipt_sha256"].as_str().unwrap();
        let saved = fixture
            .state
            .join("evidence")
            .join(format!("{receipt_sha}.artifact"));
        let original = fs::read(&saved).unwrap();
        assert_eq!(hash(&original), receipt_sha);
        let live = deliveries.join(format!("{receipt_sha}.json"));
        assert!(!live.exists());
        assert_eq!(
            unit_shard.join(format!("{unit}.json")).exists(),
            fail_after_ownership
        );
        assert!(
            fixture
                .records("groundline-learning-task-outcome")
                .is_empty()
        );
        fs::set_permissions(&deliveries, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&units, fs::Permissions::from_mode(0o700)).unwrap();
        fixture.append_later_turn();
        assert_ne!(
            hash(&fs::read(&fixture.source).unwrap()),
            request["native"]["artifact_sha256"].as_str().unwrap()
        );
        let resumed = fixture.consume();
        assert_eq!(resumed["resumed_finalize_count"], 1, "{resumed}");
        assert_eq!(fs::read(live).unwrap(), original);
        let outcome = fixture
            .records("groundline-learning-task-outcome")
            .pop()
            .unwrap()
            .1;
        assert_eq!(outcome["receipt_sha256"], receipt_sha);
        assert_eq!(
            fixture.receipt()["resources"]["entries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            fixture.receipt()["resources"]["entries"][0]["total_tokens"],
            105
        );
        assert_eq!(fixture.consume()["mutation_performed"], false);
    }
}

#[test]
fn damaged_saved_receipt_is_pending_and_never_regenerated_from_new_native_history() {
    let fixture = Fixture::new();
    let started = fixture.start("damaged-receipt");
    fixture.assessment(&started, "verified");
    fixture.close(true);
    fixture.consume();
    let request = fixture
        .records("groundline-learning-finalize-request")
        .pop()
        .unwrap()
        .1;
    let receipt_sha = request["receipt_sha256"].as_str().unwrap();
    for (path, _) in fixture
        .records("groundline-learning-task-outcome")
        .into_iter()
        .chain(fixture.records("groundline-learning-link"))
    {
        fs::remove_file(path).unwrap();
    }
    fs::remove_file(
        fixture
            .home
            .join("groundline/learning/deliveries")
            .join(format!("{receipt_sha}.json")),
    )
    .unwrap();
    private(
        &fixture
            .state
            .join("evidence")
            .join(format!("{receipt_sha}.artifact")),
        b"Changed saved receipt.\n",
    );
    fixture.append_later_turn();
    let pending = fixture.consume();
    assert_eq!(pending["resumed_finalize_count"], 0);
    assert_eq!(pending["assessments"][0]["status"], "PENDING");
    assert!(
        fixture
            .records("groundline-learning-task-outcome")
            .is_empty()
    );
    assert_eq!(
        fixture
            .records("groundline-learning-finalize-request")
            .pop()
            .unwrap()
            .1,
        request
    );
}

#[test]
fn receipt_artifact_does_not_recover_manual_finalize_requests() {
    let fixture = Fixture::new();
    let started = fixture.start("manual-gap");
    fixture.assessment(&started, "verified");
    fixture.close(true);
    fixture.consume();
    for (path, _) in fixture
        .records("groundline-learning-task-outcome")
        .into_iter()
        .chain(fixture.records("groundline-learning-link"))
    {
        fs::remove_file(path).unwrap();
    }
    let (path, mut request) = fixture
        .records("groundline-learning-finalize-request")
        .pop()
        .unwrap();
    fs::remove_file(path).unwrap();
    request.as_object_mut().unwrap().remove("assessment_sha256");
    request
        .as_object_mut()
        .unwrap()
        .remove("native_response_proof_sha256");
    let receipt_sha = request["receipt_sha256"].as_str().unwrap();
    assert!(
        fixture
            .state
            .join("evidence")
            .join(format!("{receipt_sha}.artifact"))
            .exists()
    );
    fs::remove_file(
        fixture
            .home
            .join("groundline/learning/deliveries")
            .join(format!("{receipt_sha}.json")),
    )
    .unwrap();
    let request_hash = groundline_contracts::learning::content_sha256(&request).unwrap();
    json_file(
        &fixture.state.join(format!("{request_hash}.json")),
        &request,
    );
    fixture.append_later_turn();
    let pending = fixture.consume();
    assert_eq!(pending["resumed_finalize_count"], 0);
    assert_eq!(pending["assessments"][0]["status"], "PENDING");
    assert!(
        fixture
            .records("groundline-learning-task-outcome")
            .is_empty()
    );
}
