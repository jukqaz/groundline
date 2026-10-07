#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
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

fn observation_state_sha256(state: &Path) -> String {
    let metadata = fs::metadata(state).unwrap();
    let head: Value = serde_json::from_slice(&fs::read(state.join("head.json")).unwrap()).unwrap();
    let operations_path = state.join("operations");
    let mut operations = BTreeMap::new();
    if operations_path.exists() {
        for entry in fs::read_dir(operations_path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|v| v.to_str()) == Some("json") {
                operations.insert(
                    path.file_name().unwrap().to_str().unwrap().to_owned(),
                    hash(&fs::read(path).unwrap()),
                );
            }
        }
    }
    let observation = json!({"state_identity":[metadata.dev(),metadata.ino()],
        "baseline_sha256":head["baseline_sha256"],"bindings_sha256":head["bindings_sha256"],
        "registry_sha256":head["registry_sha256"],"operations":operations});
    let mut bytes = serde_json::to_vec_pretty(&observation).unwrap();
    bytes.push(b'\n');
    hash(&bytes)
}

fn run(root: &Path, args: &[&str], paths: &[(&str, &Path)], success: bool) -> Value {
    // Cargo may hard-link its build/deps executables on Linux. Exercise a
    // standalone installed file without relaxing the production pin checks.
    let installed = root.join("installed-groundline");
    if !installed.exists() {
        fs::copy(env!("CARGO_BIN_EXE_groundline"), &installed).unwrap();
        fs::set_permissions(&installed, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(fs::metadata(&installed).unwrap().nlink(), 1);
    }
    let mut command = Command::new(installed);
    command.current_dir(root).args(args);
    command.env("CODEX_HOME", root.join("codex-home"));
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
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains(root.to_str().unwrap()));
    serde_json::from_str(&stdout).unwrap()
}

impl Fixture {
    fn apply_independent_generation(&self, state: &Path, id: &str) -> Value {
        let proposal = self.root.join(format!("{id}-environment-proposal.json"));
        private_json(
            &proposal,
            &json!({"kind":"groundline-environment-proposal","schema":1,
            "proposal_id":id,"basis_revision":"baseline-1","source_revision":"source-1",
            "authority_ref":"user-request-1","changes":[{"target_id":"workflow","content":self.after}]}),
        );
        run(
            &self.root,
            &["environment", "plan"],
            &[("--state-dir", state), ("--proposal", &proposal)],
            true,
        );
        let applied = run(
            &self.root,
            &["environment", "apply", "--proposal-id", id],
            &[("--state-dir", state)],
            true,
        );
        assert_eq!(applied["status"], "APPLIED");
        applied
    }

    fn continuous(mut self) -> Self {
        let home = self.root.join("codex-home");
        fs::create_dir(&home).unwrap();
        fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
        let configured = run(
            &self.root,
            &[
                "learning",
                "configure",
                "--target",
                "workflow",
                "--device-id",
                "fixture",
            ],
            &[
                ("--codex-home", &home),
                ("--environment-state", &self.environment),
            ],
            true,
        );
        assert_eq!(configured["enabled"], false);
        self.learning = home.join("groundline/learning/records");
        self.deliveries = home.join("groundline/learning/deliveries");
        assert_eq!(
            fs::metadata(&self.learning).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(home.join("groundline/learning/profile.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        run(
            &self.root,
            &["learning", "enable"],
            &[("--codex-home", &home)],
            true,
        );
        self
    }

    fn native_artifact(&self, turn: &str) -> PathBuf {
        let path = self.root.join(format!("{turn}-native.jsonl"));
        let lines = [
            json!({"type":"session_meta","payload":{"id":"fixture-owner","originator":"codex_cli"}}),
            json!({"type":"turn_context","payload":{"turn_id":turn,"model":"gpt-6.1-sol","effort":"medium"}}),
        ];
        let bytes = lines
            .iter()
            .map(|v| serde_json::to_string(v).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        path
    }

    fn boundary(&self, id: &str, event: &str, turn: &str) -> PathBuf {
        let status = run(
            &self.root,
            &["environment", "status"],
            &[("--state-dir", &self.environment)],
            true,
        );
        let path = self
            .root
            .join("codex-home/groundline/learning/boundaries")
            .join(format!("{id}.json"));
        let skill = fs::read(self.root.join("skills/SKILL.md")).unwrap();
        private_json(
            &path,
            &json!({"kind":"groundline-learning-boundary","schema":1,"boundary_id":id,"event":event,
            "observed_at_utc":chrono::Utc::now().to_rfc3339(),"session_hash":hash(b"groundline-hook-session\0fixture-owner"),
            "turn_hash":hash(format!("groundline-hook-turn\0{turn}").as_bytes()),"model":null,"permission_mode":null,
            "payload_status":"complete","missing_fields":[],"native_evidence_sha256":hash(id.as_bytes()),"source_verified":false,
            "device_id":"fixture","native_activation":"UNVERIFIED","targets":[{"target_id":"workflow","snapshot_code":"observed",
                "skill_revision":hash(&skill),"environment_revision":null,"source_revision":"source-1",
                "observation_state_sha256":observation_state_sha256(&self.environment),
                "baseline_sha256":status["common_change_ref"],"observed_at_utc":chrono::Utc::now().to_rfc3339()}]}),
        );
        path
    }

    fn start_task(&self, id: &str, version: &str) -> (Value, PathBuf, PathBuf) {
        let artifact = self.native_artifact(id);
        self.boundary(&format!("{id}-start"), "UserPromptSubmit", id);
        let input = self.root.join(format!("{id}-start-input.json"));
        private_json(
            &input,
            &json!({"kind":"groundline-learning-task-start-input","schema":1,
            "scope":{"unit_hash":hash(id.as_bytes()),"cohort_sha256":hash(b"cohort"),"phase":"implementation","task_category":"code-change",
                "criterion":{"sha256":hash(b"defined-test-command"),"version":version},"target_id":"workflow",
                "runtime":{"family":"codex_cli","version":"fixture-1","evidence_sha256":hash(&fs::read(&artifact).unwrap())}},
            "boundary_id":format!("{id}-start"),"criterion_change_kind":"unknown","criterion_change_evidence_sha256":null}),
        );
        let result = run(
            &self.root,
            &["learning", "task-start"],
            &[("--input", &input), ("--native-artifact", &artifact)],
            true,
        );
        (result, input, artifact)
    }

    fn finalize_task(&self, id: &str, start: &Value, artifact: &Path) -> (Value, PathBuf, PathBuf) {
        self.boundary(&format!("{id}-end"), "Stop", id);
        let receipt_in = self.receipt(id, &chrono::Utc::now().to_rfc3339());
        let receipt = self.root.join(format!("{id}-original-receipt.json"));
        fs::rename(receipt_in, &receipt).unwrap();
        let input = self.root.join(format!("{id}-finalize-input.json"));
        private_json(
            &input,
            &json!({"kind":"groundline-learning-task-finalize-input","schema":1,
            "task_sha256":start["task_sha256"],"boundary_id":format!("{id}-end"),"correction_kind":"unknown","correction_evidence_sha256":null}),
        );
        let result = run(
            &self.root,
            &["learning", "finalize"],
            &[
                ("--input", &input),
                ("--receipt", &receipt),
                ("--native-artifact", artifact),
            ],
            true,
        );
        (result, input, receipt)
    }

    fn trial(&self, proposal_sha: &str, plan_sha: &str) {
        let evidence = self.root.join("trial-approval.txt");
        fs::write(
            &evidence,
            b"Allow this fixture scope and restore its exact original bytes.",
        )
        .unwrap();
        fs::set_permissions(&evidence, fs::Permissions::from_mode(0o600)).unwrap();
        let input = self.root.join("trial-authorization.json");
        private_json(
            &input,
            &json!({"kind":"groundline-learning-trial-authorization","schema":1,"proposal_sha256":proposal_sha,
            "plan_sha256":plan_sha,"scope_sha256":hash(b"workflow"),"rollback_sha256":hash(self.before.as_bytes()),
            "authority_ref":"user-request-1","evidence_sha256":hash(&fs::read(&evidence).unwrap())}),
        );
        run(
            &self.root,
            &["learning", "authorize-trial"],
            &[
                ("--input", &input),
                ("--evidence", &evidence),
                ("--state", &self.learning),
            ],
            true,
        );
    }
}

#[test]
fn continuous_learning_links_reconciles_retries_and_resumes_without_inventing_quality() {
    let fixture = Fixture::new().continuous();
    let (before, input, artifact) = fixture.start_task("natural-before", "v1");
    let repeated = run(
        &fixture.root,
        &["learning", "task-start"],
        &[("--input", &input), ("--native-artifact", &artifact)],
        true,
    );
    assert_eq!(repeated["task_sha256"], before["task_sha256"]);
    assert_eq!(repeated["status"], "ALREADY_STARTED");
    let (finished, final_input, receipt) =
        fixture.finalize_task("natural-before", &before, &artifact);
    assert_eq!(finished["status"], "FINALIZED");
    let bytes = fs::read(&receipt).unwrap();
    let copied = fixture.deliveries.join(format!("{}.json", hash(&bytes)));
    assert_eq!(fs::read(copied).unwrap(), bytes);
    let replay = run(
        &fixture.root,
        &["learning", "finalize"],
        &[
            ("--input", &final_input),
            ("--receipt", &receipt),
            ("--native-artifact", &artifact),
        ],
        true,
    );
    assert_eq!(replay["status"], "ALREADY_FINALIZED");
    assert_eq!(replay["mutation_performed"], false);
    let before_link = finished["link_sha256"].as_str().unwrap();
    let (proposal_sha, plan_sha) = fixture.propose(before_link);
    let pending = run(&fixture.root, &["learning", "consume"], &[], true);
    assert_eq!(pending["reconciliations"][0]["status"], "PENDING");
    assert_eq!(pending["quality_inferred_from_boundaries"], false);
    assert_eq!(
        run(
            &fixture.root,
            &["environment", "apply", "--proposal-id", "workflow-change"],
            &[("--state-dir", &fixture.environment)],
            false
        )["error"],
        "learning_application_intent_required"
    );
    fixture.trial(&proposal_sha, &plan_sha);
    let applied = run(
        &fixture.root,
        &[
            "environment",
            "apply",
            "--proposal-id",
            "workflow-change",
            "--intent",
            "trial",
        ],
        &[
            ("--state-dir", &fixture.environment),
            ("--learning-state", &fixture.learning),
        ],
        true,
    );
    assert_eq!(applied["status"], "APPLIED");
    let (after, _, after_artifact) = fixture.start_task("natural-after", "v1");
    let (followup, _, _) = fixture.finalize_task("natural-after", &after, &after_artifact);
    assert_eq!(
        followup["reconciliation"]["reconciliations"][0]["status"],
        "INCONCLUSIVE"
    );
    let status = fixture.status(&[&fixture.environment.join("operations").join(format!(
        "{}.json",
        applied["operation_id"].as_str().unwrap()
    ))]);
    assert_eq!(status["records"]["linked"], 2);
    assert_eq!(
        status["candidates"][0]["evaluations"][0]["status"],
        "INCONCLUSIVE"
    );
    assert_eq!(
        status["candidates"][0]["analysis_attempt_resources"]["complete"],
        false
    );
    let retried = run(&fixture.root, &["learning", "consume"], &[], true);
    assert_eq!(retried["reconciliations"][0]["duplicate_suppressed"], true);
    let decision = fixture.root.join("unsupported-adoption.json");
    private_json(
        &decision,
        &json!({"kind":"groundline-learning-decision-input","schema":1,
        "proposal_id":"workflow-change","proposal_sha256":proposal_sha,"decision":"adopt","reason":"Explicit declaration is not measured acceptance",
        "evidence_refs":[followup["link_sha256"]],"evaluation_sha256":status["candidates"][0]["evaluations"][0]["evaluation_sha256"],
        "previous_decision_sha256":null}),
    );
    run(
        &fixture.root,
        &["learning", "decide"],
        &[("--input", &decision), ("--state", &fixture.learning)],
        true,
    );
    assert_eq!(
        run(
            &fixture.root,
            &[
                "environment",
                "apply",
                "--proposal-id",
                "workflow-change",
                "--intent",
                "adoption"
            ],
            &[
                ("--state-dir", &fixture.environment),
                ("--learning-state", &fixture.learning)
            ],
            false
        )["error"],
        "learning_adoption_evaluation_inconclusive_or_regressed"
    );
    let lost = fixture.learning.join(format!(
        "{}.json",
        followup["outcome_sha256"].as_str().unwrap()
    ));
    fs::remove_file(&lost).unwrap(); // Interruption after request/link publication.
    let resumed = run(&fixture.root, &["learning", "consume"], &[], true);
    assert_eq!(resumed["resumed_finalize_count"], 1);
    assert!(lost.exists());
    let (_, unrelated_input, unrelated_artifact) = fixture.start_task("unrelated-latest", "v1");
    let mut unrelated: Value =
        serde_json::from_slice(&fs::read(&unrelated_input).unwrap()).unwrap();
    unrelated["scope"]["unit_hash"] = json!(hash(b"different-cohort"));
    unrelated["scope"]["cohort_sha256"] = json!(hash(b"other-cohort"));
    private_json(&unrelated_input, &unrelated);
    let unrelated = run(
        &fixture.root,
        &["learning", "task-start"],
        &[
            ("--input", &unrelated_input),
            ("--native-artifact", &unrelated_artifact),
        ],
        true,
    );
    let end = fixture.boundary("unrelated-end", "Stop", "unrelated-latest");
    let unrelated_receipt = fixture.receipt("different-cohort", &chrono::Utc::now().to_rfc3339());
    let mut value: Value = serde_json::from_slice(&fs::read(&unrelated_receipt).unwrap()).unwrap();
    value["cohort_sha256"] = json!(hash(b"other-cohort"));
    private_json(&unrelated_receipt, &value);
    let unrelated_final = fixture.root.join("unrelated-final.json");
    private_json(
        &unrelated_final,
        &json!({"kind":"groundline-learning-task-finalize-input","schema":1,
        "task_sha256":unrelated["task_sha256"],"boundary_id":end.file_stem().unwrap().to_str().unwrap(),
        "correction_kind":"unknown","correction_evidence_sha256":null}),
    );
    let unrelated = run(
        &fixture.root,
        &["learning", "finalize"],
        &[
            ("--input", &unrelated_final),
            ("--receipt", &unrelated_receipt),
            ("--native-artifact", &unrelated_artifact),
        ],
        true,
    );
    assert_eq!(
        unrelated["reconciliation"]["reconciliations"][0]["duplicate_suppressed"],
        true
    );
    assert!(
        unrelated["reconciliation"]["reconciliations"][0]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&followup["link_sha256"])
    );
    let patterns = run(&fixture.root, &["learning", "patterns"], &[], true);
    assert!(
        patterns["patterns"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["outcome_unknown_count"] == v["sample_count"])
    );
    assert_eq!(patterns["historical_quality_coverage_complete"], false);
    assert_eq!(patterns["automatic_candidate_generation"], false);
}

#[test]
fn continuous_learning_rejects_changed_boundary_criterion_receipt_and_false_correction() {
    let fixture = Fixture::new().continuous();
    let (start, input, artifact) = fixture.start_task("scoped", "v1");
    let mut scope: Value = serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
    fixture.boundary("changed-start", "UserPromptSubmit", "scoped");
    scope["scope"]["unit_hash"] = json!(hash(b"second-unit"));
    scope["boundary_id"] = json!("changed-start");
    private_json(&input, &scope);
    fs::write(fixture.root.join("skills/SKILL.md"), fixture.after).unwrap();
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "task-start"],
            &[("--input", &input), ("--native-artifact", &artifact)],
            false
        )["error"],
        "learning_boundary_environment_changed"
    );
    fs::write(fixture.root.join("skills/SKILL.md"), fixture.before).unwrap();
    scope["scope"]["criterion"]["sha256"] = json!(hash(b"changed-criterion"));
    private_json(&input, &scope);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "task-start"],
            &[("--input", &input), ("--native-artifact", &artifact)],
            false
        )["error"],
        "learning_criterion_change_requires_version_and_correction"
    );
    let end = fixture.boundary("scoped-end", "Stop", "scoped");
    let final_input = fixture.root.join("invalid-final.json");
    let value = json!({"kind":"groundline-learning-task-finalize-input","schema":1,"task_sha256":start["task_sha256"],
        "boundary_id":"scoped-end","correction_kind":"unknown","correction_evidence_sha256":null});
    private_json(&final_input, &value);
    let receipt = fixture.receipt("different-unit", &chrono::Utc::now().to_rfc3339());
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "finalize"],
            &[
                ("--input", &final_input),
                ("--receipt", &receipt),
                ("--native-artifact", &artifact)
            ],
            false
        )["error"],
        "learning_snapshot_receipt_mismatch"
    );
    assert!(end.exists());
    assert!(fixture.status(&[])["records"]["linked"].is_null());
    let mut acceptance = value.clone();
    acceptance["correction_kind"] = json!("acceptance");
    acceptance["correction_evidence_sha256"] = json!(hash(b"silence"));
    private_json(&final_input, &acceptance);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "finalize"],
            &[("--input", &final_input), ("--receipt", &receipt)],
            false
        )["error"],
        "learning_correction_evidence_required_or_mismatched"
    );
    let mut override_time = value;
    override_time["completed_at_utc"] = json!("2026-01-01T00:00:00Z");
    private_json(&final_input, &override_time);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "finalize"],
            &[("--input", &final_input), ("--receipt", &receipt)],
            false
        )["error"],
        "learning_invalid_task_finalize"
    );
}

#[test]
fn continuous_learning_rejects_a_boundary_from_another_environment_with_identical_workflow_bytes() {
    let fixture = Fixture::new().continuous();
    let applied_a = fixture.apply_independent_generation(&fixture.environment, "generation-a");
    let artifact = fixture.native_artifact("state-switch");
    let boundary_path = fixture.boundary("state-a-start", "UserPromptSubmit", "state-switch");
    let old: Value = serde_json::from_slice(&fs::read(&boundary_path).unwrap()).unwrap();
    let input = fixture.root.join("state-switch-input.json");
    private_json(
        &input,
        &json!({"kind":"groundline-learning-task-start-input","schema":1,
        "scope":{"unit_hash":hash(b"state-switch"),"cohort_sha256":hash(b"cohort"),"phase":"implementation",
            "task_category":"code-change","criterion":{"sha256":hash(b"defined-test-command"),"version":"v1"},
            "target_id":"workflow","runtime":null},"boundary_id":"state-a-start",
        "criterion_change_kind":"unknown","criterion_change_evidence_sha256":null}),
    );
    // Register B against its own original bytes, then really apply a distinct B
    // plan so the final workflow bytes equal A's old boundary observation.
    fs::write(fixture.root.join("skills/SKILL.md"), fixture.before).unwrap();
    let environment_b = fixture.root.join("environment-b");
    run(
        &fixture.root,
        &["environment", "register"],
        &[
            ("--state-dir", &environment_b),
            ("--baseline", &fixture.root.join("baseline.json")),
            ("--bindings", &fixture.root.join("bindings.json")),
        ],
        true,
    );
    let applied_b = fixture.apply_independent_generation(&environment_b, "generation-b");
    assert_ne!(applied_a["plan_sha256"], applied_b["plan_sha256"]);
    assert_eq!(
        old["targets"][0]["skill_revision"],
        hash(&fs::read(fixture.root.join("skills/SKILL.md")).unwrap())
    );
    let status_b = run(
        &fixture.root,
        &["environment", "status"],
        &[("--state-dir", &environment_b)],
        true,
    );
    assert_eq!(
        old["targets"][0]["baseline_sha256"],
        status_b["common_change_ref"]
    );
    assert_ne!(
        old["targets"][0]["observation_state_sha256"],
        observation_state_sha256(&environment_b)
    );
    run(
        &fixture.root,
        &[
            "learning",
            "configure",
            "--target",
            "workflow",
            "--device-id",
            "fixture",
        ],
        &[("--environment-state", &environment_b)],
        true,
    );
    run(&fixture.root, &["learning", "enable"], &[], true);
    let observation = fixture.root.join("state-b-observation.json");
    private_json(
        &observation,
        &json!({"kind":"groundline-learning-capture-input","schema":1,
        "unit_hash":hash(b"state-b-probe"),"cohort_sha256":hash(b"cohort"),"phase":"implementation","runtime":null}),
    );
    let capture_b = run(
        &fixture.root,
        &["learning", "capture", "--target", "workflow"],
        &[
            ("--environment-state", &environment_b),
            ("--state", &fixture.learning),
            ("--observation", &observation),
        ],
        true,
    );
    assert_eq!(capture_b["environment_revision"], applied_b["plan_sha256"]);
    assert_eq!(
        capture_b["source_revision"],
        old["targets"][0]["source_revision"]
    );
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "task-start"],
            &[("--input", &input), ("--native-artifact", &artifact)],
            false
        )["error"],
        "learning_boundary_environment_changed"
    );
    assert_eq!(fs::read(&boundary_path).unwrap(), {
        let mut bytes = serde_json::to_vec_pretty(&old).unwrap();
        bytes.push(b'\n');
        bytes
    });
}

#[test]
fn continuous_learning_missing_observation_state_keeps_applied_environment_unknown() {
    let fixture = Fixture::new().continuous();
    fixture.apply_independent_generation(&fixture.environment, "current-generation");
    for snapshot_code in ["observed", "target_missing"] {
        let artifact = fixture.native_artifact(snapshot_code);
        let path = fixture.boundary(
            &format!("{snapshot_code}-start"),
            "UserPromptSubmit",
            snapshot_code,
        );
        let mut boundary: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        boundary["targets"][0]["snapshot_code"] = json!(snapshot_code);
        boundary["targets"][0]
            .as_object_mut()
            .unwrap()
            .remove("observation_state_sha256");
        private_json(&path, &boundary);
        let input = fixture.root.join(format!("{snapshot_code}-input.json"));
        private_json(
            &input,
            &json!({"kind":"groundline-learning-task-start-input","schema":1,
            "scope":{"unit_hash":hash(snapshot_code.as_bytes()),"cohort_sha256":hash(b"cohort"),"phase":"implementation",
                "task_category":"code-change","criterion":{"sha256":hash(b"defined-test-command"),"version":"v1"},
                "target_id":"workflow","runtime":null},"boundary_id":format!("{snapshot_code}-start"),
            "criterion_change_kind":"unknown","criterion_change_evidence_sha256":null}),
        );
        let started = run(
            &fixture.root,
            &["learning", "task-start"],
            &[("--input", &input), ("--native-artifact", &artifact)],
            true,
        );
        let task: Value = serde_json::from_slice(
            &fs::read(
                fixture
                    .learning
                    .join(format!("{}.json", started["task_sha256"].as_str().unwrap())),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(task["boundary_environment_matched"], false);
        let snapshot: Value = serde_json::from_slice(
            &fs::read(fixture.learning.join(format!(
                "{}.json",
                task["snapshot_sha256"].as_str().unwrap()
            )))
            .unwrap(),
        )
        .unwrap();
        for field in ["environment_revision", "source_revision", "skill"] {
            assert!(snapshot[field].is_null());
        }
    }
}

#[test]
fn continuous_learning_hold_blocks_trial_and_adoption_but_allows_rollback() {
    let fixture = Fixture::new().continuous();
    let (start, _, artifact) = fixture.start_task("held-before", "v1");
    let (done, _, _) = fixture.finalize_task("held-before", &start, &artifact);
    let link = done["link_sha256"].as_str().unwrap();
    let (proposal_sha, plan_sha) = fixture.propose(link);
    fixture.trial(&proposal_sha, &plan_sha);
    let applied = run(
        &fixture.root,
        &[
            "environment",
            "apply",
            "--proposal-id",
            "workflow-change",
            "--intent",
            "trial",
        ],
        &[
            ("--state-dir", &fixture.environment),
            ("--learning-state", &fixture.learning),
        ],
        true,
    );
    let decision = fixture.root.join("hold.json");
    private_json(
        &decision,
        &json!({"kind":"groundline-learning-decision-input","schema":1,"proposal_id":"workflow-change","proposal_sha256":proposal_sha,
        "decision":"hold","reason":"A further direct result is required","evidence_refs":[link],"evaluation_sha256":null,"previous_decision_sha256":null}),
    );
    run(
        &fixture.root,
        &["learning", "decide"],
        &[("--input", &decision), ("--state", &fixture.learning)],
        true,
    );
    for intent in ["trial", "adoption"] {
        assert_eq!(
            run(
                &fixture.root,
                &[
                    "environment",
                    "apply",
                    "--proposal-id",
                    "workflow-change",
                    "--intent",
                    intent
                ],
                &[
                    ("--state-dir", &fixture.environment),
                    ("--learning-state", &fixture.learning)
                ],
                false
            )["error"],
            "learning_application_decision_blocked"
        );
    }
    assert_eq!(
        run(&fixture.root, &["learning", "consume"], &[], true)["reconciliations"][0]["status"],
        "BLOCKED"
    );
    let rollback = run(
        &fixture.root,
        &[
            "environment",
            "rollback",
            "--operation-id",
            applied["operation_id"].as_str().unwrap(),
        ],
        &[("--state-dir", &fixture.environment)],
        true,
    );
    assert_eq!(rollback["status"], "ROLLED_BACK");
    assert_eq!(
        fs::read_to_string(fixture.root.join("skills/SKILL.md")).unwrap(),
        fixture.before
    );
}

#[test]
fn continuous_learning_archives_boundary_queue_and_retains_exact_native_references() {
    let fixture = Fixture::new().continuous();
    let artifact = fixture.native_artifact("queued");
    let path = fixture.boundary("queued-start", "UserPromptSubmit", "queued");
    let original = fs::read(&path).unwrap();
    let readout = run(
        &fixture.root,
        &["learning", "boundaries"],
        &[("--native-artifact", &artifact)],
        true,
    );
    assert_eq!(readout["archived_boundary_count"], 1);
    assert_eq!(readout["mutation_performed"], true);
    assert_eq!(readout["boundaries"][0]["boundary_id"], "queued-start");
    assert_eq!(readout["boundaries"][0]["native_boundary_matched"], true);
    assert!(!path.exists());
    let archive = fixture
        .root
        .join("codex-home/groundline/learning/boundaries/archive")
        .join(&hash(b"queued-start")[..2])
        .join("queued-start.json");
    assert_eq!(fs::read(&archive).unwrap(), original);
    let repeated = run(
        &fixture.root,
        &["learning", "boundaries"],
        &[("--native-artifact", &artifact)],
        true,
    );
    assert_eq!(repeated["mutation_performed"], false);
    assert_eq!(repeated["boundaries"], readout["boundaries"]);
    let other = fixture.native_artifact("other");
    assert!(
        run(
            &fixture.root,
            &["learning", "boundaries"],
            &[("--native-artifact", &other)],
            true
        )["boundaries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut changed: Value = serde_json::from_slice(&original).unwrap();
    changed["payload_status"] = json!("missing_fields");
    private_json(&path, &changed);
    assert_eq!(
        run(&fixture.root, &["learning", "consume"], &[], false)["error"],
        "learning_boundary_conflict"
    );
    assert_eq!(fs::read(&archive).unwrap(), original);
}

#[test]
fn continuous_learning_boundary_lookup_has_one_total_byte_budget() {
    let fixture = Fixture::new().continuous();
    let artifact = fixture.native_artifact("budget");
    fixture.boundary("budget-start", "UserPromptSubmit", "budget");
    run(
        &fixture.root,
        &["learning", "boundaries"],
        &[("--native-artifact", &artifact)],
        true,
    );
    let index_root = fixture
        .root
        .join("codex-home/groundline/learning/boundaries/archive/references")
        .join(hash(b"groundline-hook-session\0fixture-owner"))
        .join(hash(b"groundline-hook-turn\0budget"));
    let shard = index_root.join(&hash(b"budget-start")[..2]);
    let reference: Value =
        serde_json::from_slice(&fs::read(shard.join("budget-start.json")).unwrap()).unwrap();
    for n in 0..5 {
        let mut padded = reference.clone();
        padded["padding"] = json!("x".repeat(1024 * 1024));
        private_json(&shard.join(format!("padded-{n}.json")), &padded);
    }
    let readout = run(
        &fixture.root,
        &["learning", "boundaries"],
        &[("--native-artifact", &artifact)],
        true,
    );
    assert_eq!(readout["coverage_complete"], false);
    assert_eq!(readout["mutation_performed"], false);
    assert!(readout["lookup_budget"]["bytes_read"].as_u64().unwrap() <= 4 * 1024 * 1024);
    assert!(
        readout["lookup_budget"]["entries_visited"]
            .as_u64()
            .unwrap()
            <= 512
    );
}

#[test]
fn continuous_learning_archives_records_and_preserves_retry_and_unit_ownership() {
    let fixture = Fixture::new().continuous();
    let (start, start_input, artifact) = fixture.start_task("durable-old", "v1");
    let (finished, final_input, receipt_path) =
        fixture.finalize_task("durable-old", &start, &artifact);
    let rows: Vec<Value> = fs::read_dir(&fixture.learning)
        .unwrap()
        .filter_map(|e| {
            let p = e.unwrap().path();
            (p.extension().and_then(|v| v.to_str()) == Some("json"))
                .then(|| serde_json::from_slice(&fs::read(p).unwrap()).unwrap())
        })
        .collect();
    let record = |kind: &str| rows.iter().find(|v| v["kind"] == kind).unwrap().clone();
    let save = |value: &Value| {
        let digest = groundline_contracts::learning::content_sha256(value).unwrap();
        private_json(&fixture.learning.join(format!("{digest}.json")), value);
        digest
    };
    // A deterministic storage fixture supplies more than the 32 retained tasks;
    // these cloned records do not claim new native work or quality observations.
    for n in 0..33 {
        let unit = hash(format!("archive-fixture-{n}").as_bytes());
        let completed = chrono::Utc::now().to_rfc3339();
        let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        receipt["unit_hash"] = json!(unit);
        receipt["completed_at_utc"] = json!(completed);
        let bytes = serde_json::to_vec_pretty(&receipt).unwrap();
        let receipt_hash = hash(&bytes);
        fs::write(
            fixture.deliveries.join(format!("{receipt_hash}.json")),
            &bytes,
        )
        .unwrap();
        fs::set_permissions(
            fixture.deliveries.join(format!("{receipt_hash}.json")),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let mut snapshot = record("groundline-learning-snapshot");
        snapshot["unit_hash"] = json!(unit);
        snapshot["captured_at_utc"] = json!(completed);
        snapshot["capture_input_sha256"] = json!(hash(format!("capture-{n}").as_bytes()));
        let snapshot_hash = save(&snapshot);
        let mut task = record("groundline-learning-task-start");
        task["scope"]["unit_hash"] = json!(unit);
        task["input_sha256"] = json!(hash(format!("start-{n}").as_bytes()));
        task["snapshot_sha256"] = json!(snapshot_hash);
        let task_hash = save(&task);
        let mut link = record("groundline-learning-link");
        link["unit_hash"] = json!(unit);
        link["receipt_sha256"] = json!(receipt_hash);
        link["completed_at_utc"] = json!(completed);
        let link_hash = save(&link);
        let input_hash = hash(format!("finalize-{n}").as_bytes());
        let mut request = record("groundline-learning-finalize-request");
        request["input"]["task_sha256"] = json!(task_hash);
        request["input_sha256"] = json!(input_hash);
        request["receipt_sha256"] = json!(receipt_hash);
        save(&request);
        let mut outcome = record("groundline-learning-task-outcome");
        outcome["task_sha256"] = json!(task_hash);
        outcome["input_sha256"] = json!(input_hash);
        outcome["link_sha256"] = json!(link_hash);
        outcome["receipt_sha256"] = json!(receipt_hash);
        save(&outcome);
    }
    for n in 0..240 {
        save(&json!({"kind":"groundline-learning-processing","schema":1,
            "input_sha256":hash(format!("unreferenced-{n}").as_bytes()),"context_sha256":hash(b"context"),
            "status":"OBSERVED","evidence_refs":[],"reasons":[]}));
    }
    let original_task = fixture
        .learning
        .join(format!("{}.json", start["task_sha256"].as_str().unwrap()));
    let task_bytes = fs::read(&original_task).unwrap();
    let consumed = run(&fixture.root, &["learning", "consume"], &[], true);
    assert!(consumed["archived_record_count"].as_u64().unwrap() > 0);
    assert_eq!(consumed["mutation_performed"], true);
    assert_eq!(
        consumed["capacity"]["historical_quality_coverage_complete"],
        false
    );
    assert!(!original_task.exists());
    let task_hash = start["task_sha256"].as_str().unwrap();
    let archived = fixture
        .learning
        .join("archive")
        .join(&task_hash[..2])
        .join(format!("{task_hash}.json"));
    assert_eq!(fs::read(&archived).unwrap(), task_bytes);
    let replay = run(
        &fixture.root,
        &["learning", "task-start"],
        &[("--input", &start_input), ("--native-artifact", &artifact)],
        true,
    );
    assert_eq!(replay["task_sha256"], start["task_sha256"]);
    assert_eq!(replay["archived"], true);
    let replay = run(
        &fixture.root,
        &["learning", "finalize"],
        &[
            ("--input", &final_input),
            ("--receipt", &receipt_path),
            ("--native-artifact", &artifact),
        ],
        true,
    );
    assert_eq!(replay["outcome_sha256"], finished["outcome_sha256"]);
    assert_eq!(replay["mutation_performed"], false);
    let mut changed: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    changed["verification"]["evidence_sha256"] = json!(hash(b"changed-receipt"));
    private_json(&receipt_path, &changed);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "finalize"],
            &[
                ("--input", &final_input),
                ("--receipt", &receipt_path),
                ("--native-artifact", &artifact)
            ],
            false
        )["error"],
        "learning_task_outcome_conflict"
    );
    assert_eq!(fs::read(&archived).unwrap(), task_bytes);
    let ledger = fixture
        .deliveries
        .join("ownership/units")
        .join(&hash(b"durable-old")[..2])
        .join(format!("{}.json", hash(b"durable-old")));
    let owned: Value = serde_json::from_slice(&fs::read(ledger).unwrap()).unwrap();
    assert_eq!(
        owned["receipt_sha256"],
        record("groundline-learning-task-outcome")["receipt_sha256"]
    );
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    environment: PathBuf,
    learning: PathBuf,
    deliveries: PathBuf,
    before: &'static str,
    after: &'static str,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let skills = root.join("skills");
        let deliveries = root.join("deliveries");
        for directory in [&skills, &deliveries] {
            fs::create_dir(directory).unwrap();
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let before = "---\nname: workflow\ndescription: fixture\n---\nBefore.\n";
        let after = "---\nname: workflow\ndescription: fixture\n---\nAfter.\n";
        fs::write(skills.join("SKILL.md"), before).unwrap();
        let baseline = root.join("baseline.json");
        let bindings = root.join("bindings.json");
        private_json(
            &baseline,
            &json!({"kind":"groundline-environment-baseline","schema":1,
            "revision":"baseline-1","parent_revision":null,"source_revision":"source-1",
            "authority_revision":"authority-1","exception_revision":"device-1","authority_ref":"user-request-1",
            "managed_targets":[{"target_id":"workflow","kind":"skill_file","desired_sha256":hash(after.as_bytes()),
                "dependencies":[],"managed_block":null}]}),
        );
        private_json(
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
        let learning = root.join("learning");
        Self {
            _temp: temp,
            root,
            environment,
            learning,
            deliveries,
            before,
            after,
        }
    }

    fn capture(&self, id: &str) -> Value {
        let observation = self.root.join(format!("{id}-observation.json"));
        private_json(
            &observation,
            &json!({"kind":"groundline-learning-capture-input","schema":1,
            "unit_hash":hash(id.as_bytes()),"cohort_sha256":hash(b"cohort"),"phase":"implementation","runtime":null}),
        );
        run(
            &self.root,
            &["learning", "capture", "--target", "workflow"],
            &[
                ("--environment-state", &self.environment),
                ("--observation", &observation),
                ("--state", &self.learning),
            ],
            true,
        )
    }

    fn receipt(&self, id: &str, completed_at_utc: &str) -> PathBuf {
        let path = self.deliveries.join(format!("{id}.json"));
        private_json(
            &path,
            &json!({"kind":"groundline-delivery-receipt","schema":1,
            "unit_hash":hash(id.as_bytes()),"cohort_sha256":hash(b"cohort"),"phase":"implementation",
            "completed_at_utc":completed_at_utc,"recommendation":null,"requested":null,"effective":null,
            "verification":{"status":"unknown","evidence_kind":"unobserved","evidence_sha256":hash(id.as_bytes()),
                "rework":false,"authenticity_verified":false},
            "resources":{"complete":false,"wall_duration_ms":null,"entries":[]},
            "activation_verified":false,"observed_selection_matches_requested":null}),
        );
        path
    }

    fn finish(&self, id: &str, captured: &Value) -> String {
        let receipt = self.receipt(id, &chrono::Utc::now().to_rfc3339());
        let draft = self.root.join(format!("{id}-link.json"));
        let snapshot = captured["snapshot_sha256"].as_str().unwrap();
        let prepared = run(
            &self.root,
            &["learning", "prepare", "--snapshot", snapshot],
            &[
                ("--receipt", &receipt),
                ("--state", &self.learning),
                ("--output", &draft),
            ],
            true,
        );
        assert_eq!(prepared["status"], "PREPARED");
        assert_eq!(prepared["correction_kind"], "unknown");
        assert_eq!(
            run(
                &self.root,
                &["learning", "prepare", "--snapshot", snapshot],
                &[
                    ("--receipt", &receipt),
                    ("--state", &self.learning),
                    ("--output", &draft)
                ],
                true
            )["status"],
            "ALREADY_PREPARED"
        );
        let input: Value = serde_json::from_slice(&fs::read(&draft).unwrap()).unwrap();
        assert_eq!(input["observed_at_utc"], captured["captured_at_utc"]);
        assert_eq!(
            input["environment_revision"],
            captured["environment_revision"]
        );
        assert!(input.get("resources").is_none());
        let linked = run(
            &self.root,
            &["learning", "link-outcome"],
            &[
                ("--input", &draft),
                ("--receipt", &receipt),
                ("--state", &self.learning),
            ],
            true,
        );
        assert_eq!(linked["status"], "LINKED");
        linked["link_sha256"].as_str().unwrap().to_owned()
    }

    fn propose(&self, evidence: &str) -> (String, String) {
        let proposal = self.root.join("patch.json");
        private_json(
            &proposal,
            &json!({"kind":"groundline-environment-proposal","schema":1,
            "proposal_id":"workflow-change","basis_revision":"baseline-1","source_revision":"source-1",
            "authority_ref":"user-request-1","changes":[{"target_id":"workflow","content":self.after}]}),
        );
        let plan = run(
            &self.root,
            &["environment", "plan", "--json"],
            &[
                ("--state-dir", &self.environment),
                ("--proposal", &proposal),
            ],
            true,
        );
        let plan_sha = plan["plan_sha256"].as_str().unwrap().to_owned();
        let candidate = self.root.join("candidate.json");
        private_json(
            &candidate,
            &json!({"kind":"groundline-learning-proposal-input","schema":1,
            "proposal_id":"workflow-change","evidence_refs":[evidence],"scope_sha256":hash(b"workflow"),
            "basis_revision":"baseline-1","source_revision":"source-1","source":null,
            "plan_sha256":plan_sha,"status":"candidate","reason":"Observe one bounded transition",
            "target":{"target_id":"workflow","kind":"skill","change":"modify",
                "before_sha256":hash(self.before.as_bytes()),"after_sha256":hash(self.after.as_bytes()),
                "intended_trigger":"Same work","hypothesis":"Reduce friction","expected_result":"Direct outcome",
                "falsification":"More rework","authority_ref":"user-request-1","rollback_ref":hash(self.before.as_bytes())},
            "analysis":null}),
        );
        let proposed = run(
            &self.root,
            &["learning", "propose"],
            &[("--input", &candidate), ("--state", &self.learning)],
            true,
        );
        assert_eq!(proposed["status"], "PROPOSED");
        assert_eq!(proposed["analysis_resources_complete"], false);
        assert_eq!(
            run(
                &self.root,
                &["learning", "propose"],
                &[("--input", &candidate), ("--state", &self.learning)],
                true
            )["status"],
            "DUPLICATE_SUPPRESSED"
        );
        (
            proposed["proposal_sha256"].as_str().unwrap().to_owned(),
            plan_sha,
        )
    }

    fn apply(&self) -> PathBuf {
        let applied = run(
            &self.root,
            &[
                "environment",
                "apply",
                "--proposal-id",
                "workflow-change",
                "--json",
            ],
            &[("--state-dir", &self.environment)],
            true,
        );
        assert_eq!(applied["status"], "APPLIED");
        self.environment.join("operations").join(format!(
            "{}.json",
            applied["operation_id"].as_str().unwrap()
        ))
    }

    fn status(&self, operations: &[&Path]) -> Value {
        let mut paths = vec![("--state", self.learning.as_path())];
        paths.extend(operations.iter().map(|p| ("--operation", *p)));
        run(&self.root, &["learning", "status"], &paths, true)
    }
}

#[test]
fn learning_capture_prepare_evaluate_decide_keeps_application_and_effects_separate() {
    let fixture = Fixture::new();
    let before = fixture.capture("before");
    assert_eq!(before["environment_revision"], Value::Null);
    assert_eq!(before["skill"]["revision"], hash(fixture.before.as_bytes()));
    let repeated = fixture.capture("before");
    assert_eq!(repeated["status"], "ALREADY_CAPTURED");
    assert_eq!(repeated["snapshot_sha256"], before["snapshot_sha256"]);
    let before_link = fixture.finish("before", &before);
    let (proposal_sha, plan_sha) = fixture.propose(&before_link);
    assert_eq!(
        fixture.status(&[])["candidates"][0]["application"]["state"],
        "UNOBSERVED"
    );
    let decision_path = fixture.root.join("decision.json");
    let mut decision = json!({"kind":"groundline-learning-decision-input","schema":1,
        "proposal_id":"workflow-change","proposal_sha256":proposal_sha,"decision":"adopt",
        "reason":"Explicit small trial","evidence_refs":[before_link],
        "evaluation_sha256":hash(b"missing-evaluation"),"previous_decision_sha256":null});
    private_json(&decision_path, &decision);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "decide"],
            &[("--input", &decision_path), ("--state", &fixture.learning)],
            false
        )["error"],
        "learning_decision_evidence_unlinked"
    );
    decision["decision"] = json!("hold");
    decision["evaluation_sha256"] = Value::Null;
    private_json(&decision_path, &decision);
    let hold = run(
        &fixture.root,
        &["learning", "decide"],
        &[("--input", &decision_path), ("--state", &fixture.learning)],
        true,
    );
    assert_eq!(hold["status"], "DECIDED");
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "decide"],
            &[("--input", &decision_path), ("--state", &fixture.learning)],
            true
        )["status"],
        "ALREADY_DECIDED"
    );
    decision["decision"] = json!("reject");
    private_json(&decision_path, &decision);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "decide"],
            &[("--input", &decision_path), ("--state", &fixture.learning)],
            false
        )["error"],
        "learning_decision_history_conflict"
    );
    let operation = fixture.apply();
    let status = fixture.status(&[&operation, &operation]);
    assert_eq!(status["operation_observation_count"], 1);
    assert_eq!(status["candidates"][0]["application"]["state"], "APPLIED");
    assert_eq!(status["candidates"][0]["evaluation_pending"], true);
    assert_eq!(
        status["candidates"][0]["analysis_attempt_resources"]["complete"],
        false
    );
    assert!(
        status["candidates"][0]["analysis_attempt_resources"]["additional_owned_total_tokens"]
            .is_null()
    );
    let after = fixture.capture("after");
    assert_eq!(after["environment_revision"], plan_sha);
    assert_eq!(after["skill"]["revision"], hash(fixture.after.as_bytes()));
    let after_link = fixture.finish("after", &after);
    let evaluation = fixture.root.join("evaluation.json");
    let mut evaluation_input = json!({"kind":"groundline-learning-evaluation-input","schema":1,
        "proposal_id":"workflow-change","baseline_revision":"unobserved-initial-generation",
        "proposal_revision":plan_sha,"baseline_refs":[before_link],"followup_refs":[hash(b"unlinked")]});
    private_json(&evaluation, &evaluation_input);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "evaluate"],
            &[
                ("--input", &evaluation),
                ("--deliveries", &fixture.deliveries),
                ("--operation", &operation),
                ("--state", &fixture.learning)
            ],
            false
        )["error"],
        "learning_evaluation_link_missing"
    );
    assert!(
        fixture.status(&[&operation])["candidates"][0]["evaluations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    evaluation_input["followup_refs"] = json!([after_link]);
    private_json(&evaluation, &evaluation_input);
    let result = run(
        &fixture.root,
        &["learning", "evaluate"],
        &[
            ("--input", &evaluation),
            ("--deliveries", &fixture.deliveries),
            ("--operation", &operation),
            ("--state", &fixture.learning),
        ],
        true,
    );
    assert_eq!(result["status"], "INCONCLUSIVE");
    assert!(
        result["comparison_reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("revision_mismatch"))
    );
    assert_eq!(result["analysis_attempt_resources"]["complete"], false);
    assert!(result["analysis_attempt_resources"]["additional_owned_total_tokens"].is_null());
    assert_eq!(result["causal_effect_verified"], false);
    decision["decision"] = json!("adopt");
    decision["evaluation_sha256"] = result["evaluation_sha256"].clone();
    decision["previous_decision_sha256"] = hold["decision_sha256"].clone();
    decision["evidence_refs"] = json!([before_link, after_link, result["evaluation_sha256"]]);
    private_json(&decision_path, &decision);
    let adopted = run(
        &fixture.root,
        &["learning", "decide"],
        &[("--input", &decision_path), ("--state", &fixture.learning)],
        true,
    );
    assert_eq!(adopted["status"], "DECIDED");
    let status = fixture.status(&[&operation]);
    assert_eq!(status["records"]["captured"], 2);
    assert_eq!(status["records"]["linked"], 2);
    assert_eq!(status["records"]["decision_adopt"], 1);
    assert_eq!(
        status["candidates"][0]["declared_decision"]["decision"],
        "adopt"
    );
    assert_eq!(
        status["candidates"][0]["evaluations"][0]["status"],
        "INCONCLUSIVE"
    );
    assert_eq!(status["candidates"][0]["evaluation_pending"], true);
    assert_eq!(status["analysis_resources"]["unobserved_analysis_count"], 1);
    assert_eq!(
        fixture.status(&[])["candidates"][0]["application"]["state"],
        "UNOBSERVED"
    );
    let operation_id = operation.file_stem().unwrap().to_str().unwrap();
    let rolled = run(
        &fixture.root,
        &[
            "environment",
            "rollback",
            "--operation-id",
            operation_id,
            "--json",
        ],
        &[("--state-dir", &fixture.environment)],
        true,
    );
    assert_eq!(rolled["status"], "ROLLED_BACK");
    let rollback = fixture
        .environment
        .join("operations")
        .join(format!("{}.json", rolled["operation_id"].as_str().unwrap()));
    assert_eq!(
        fixture.status(&[&operation, &rollback])["candidates"][0]["application"]["state"],
        "ROLLED_BACK"
    );
    let restored = fixture.capture("restored");
    assert!(restored["environment_revision"].is_null());
    assert_eq!(
        restored["skill"]["revision"],
        hash(fixture.before.as_bytes())
    );
}

#[test]
fn learning_prepare_rejects_late_capture_wrong_receipt_and_output_conflict_without_overwrite() {
    let fixture = Fixture::new();
    let snapshot = fixture.capture("task");
    let reference = snapshot["snapshot_sha256"].as_str().unwrap();
    let draft = fixture.root.join("draft.json");
    let old = fixture.receipt("task", "2026-10-01T12:00:00Z");
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "prepare", "--snapshot", reference],
            &[
                ("--receipt", &old),
                ("--state", &fixture.learning),
                ("--output", &draft)
            ],
            false
        )["error"],
        "learning_snapshot_after_delivery"
    );
    assert!(!draft.exists());
    let other = fixture.receipt("other", &chrono::Utc::now().to_rfc3339());
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "prepare", "--snapshot", reference],
            &[
                ("--receipt", &other),
                ("--state", &fixture.learning),
                ("--output", &draft)
            ],
            false
        )["error"],
        "learning_snapshot_receipt_mismatch"
    );
    let receipt = fixture.receipt("task", &chrono::Utc::now().to_rfc3339());
    private_json(&draft, &json!({"private_user_content":"preserve"}));
    let original = fs::read(&draft).unwrap();
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "prepare", "--snapshot", reference],
            &[
                ("--receipt", &receipt),
                ("--state", &fixture.learning),
                ("--output", &draft)
            ],
            false
        )["error"],
        "learning_draft_conflict"
    );
    assert_eq!(fs::read(&draft).unwrap(), original);
    fs::remove_file(&draft).unwrap();
    run(
        &fixture.root,
        &["learning", "prepare", "--snapshot", reference],
        &[
            ("--receipt", &receipt),
            ("--state", &fixture.learning),
            ("--output", &draft),
        ],
        true,
    );
    let mut altered: Value = serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
    altered["unit_hash"] = json!(hash(b"altered"));
    private_json(&receipt, &altered);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "link-outcome"],
            &[
                ("--input", &draft),
                ("--receipt", &receipt),
                ("--state", &fixture.learning)
            ],
            false
        )["error"],
        "learning_receipt_link_mismatch"
    );
    assert_eq!(fixture.status(&[])["records"]["captured"], 1);
    assert!(fixture.status(&[])["records"]["linked"].is_null());
}

#[test]
fn learning_capture_checks_explicit_native_evidence_and_rejects_revision_overrides() {
    let fixture = Fixture::new();
    let native = fixture.root.join("native.json");
    let digest = private_json(&native, &json!({"runtime":"fixture-only"}));
    let observation = fixture.root.join("observation.json");
    let mut input = json!({"kind":"groundline-learning-capture-input","schema":1,
        "unit_hash":hash(b"native-task"),"cohort_sha256":hash(b"cohort"),"phase":"implementation",
        "runtime":{"family":"codex_app","version":"1.2.3","evidence_sha256":digest}});
    private_json(&observation, &input);
    let paths = [
        ("--environment-state", fixture.environment.as_path()),
        ("--observation", observation.as_path()),
        ("--state", fixture.learning.as_path()),
    ];
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "capture", "--target", "workflow"],
            &paths,
            false
        )["error"],
        "learning_native_evidence_required"
    );
    private_json(&native, &json!({"runtime":"different"}));
    let mut paths = paths.to_vec();
    paths.push(("--native-evidence", &native));
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "capture", "--target", "workflow"],
            &paths,
            false
        )["error"],
        "learning_native_evidence_mismatch"
    );
    private_json(&native, &json!({"runtime":"fixture-only"}));
    let captured = run(
        &fixture.root,
        &["learning", "capture", "--target", "workflow"],
        &paths,
        true,
    );
    assert_eq!(captured["runtime_observed"], true);
    assert_eq!(captured["native_evidence_authenticity_verified"], false);
    assert_eq!(captured["native_activation"], "UNVERIFIED");
    input["source_revision"] = json!("invented-source");
    private_json(&observation, &input);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "capture", "--target", "workflow"],
            &paths,
            false
        )["error"],
        "learning_invalid_capture_input"
    );
    assert_eq!(fixture.status(&[])["records"]["captured"], 1);
}

#[test]
fn learning_conflicting_operation_observations_fail_before_changing_learning_state() {
    let fixture = Fixture::new();
    let before = fixture.capture("task");
    let reference = fixture.finish("task", &before);
    fixture.propose(&reference);
    let operation = fixture.apply();
    let mut conflicting: Value = serde_json::from_slice(&fs::read(&operation).unwrap()).unwrap();
    conflicting["source_revision"] = json!("unrelated-source");
    let other = fixture.root.join("conflicting-operation.json");
    private_json(&other, &conflicting);
    assert_eq!(
        run(
            &fixture.root,
            &["learning", "status"],
            &[
                ("--state", &fixture.learning),
                ("--operation", &operation),
                ("--operation", &other)
            ],
            false
        )["error"],
        "learning_operation_observation_conflict"
    );
    let status = fixture.status(&[&other]);
    assert_eq!(
        status["candidates"][0]["application"]["state"],
        "UNOBSERVED"
    );
    assert_eq!(
        status["candidates"][0]["application"]["unmatched_operation_count"],
        1
    );
    assert_eq!(
        fixture.status(&[&operation])["candidates"][0]["application"]["state"],
        "APPLIED"
    );
}
