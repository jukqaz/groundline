#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
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

fn run(root: &Path, args: &[&str], paths: &[(&str, &Path)], success: bool) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_groundline"));
    command.current_dir(root).args(args);
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
