#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::OnceCell;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
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

fn private_json(path: &Path, value: &Value) {
    private(path, &serde_json::to_vec_pretty(value).unwrap());
}

fn comparison(path: &Path) -> (Value, String) {
    let conditions: groundline_contracts::learning::conversation_work::ComparisonConditions =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    (
        serde_json::to_value(&conditions).unwrap(),
        conditions.fingerprint().unwrap(),
    )
}

fn executable() -> &'static Path {
    static COPY: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    let (_, executable) = COPY.get_or_init(|| {
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().canonicalize().unwrap().join("groundline");
        fs::copy(env!("CARGO_BIN_EXE_groundline"), &executable).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        (temporary, executable)
    });
    executable
}

struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    state: PathBuf,
    target: PathBuf,
    before: PathBuf,
    context: PathBuf,
    assistant: PathBuf,
    correction: PathBuf,
    criterion: PathBuf,
    verification: PathBuf,
    comparison: PathBuf,
    evidence: PathBuf,
    source_revision: PathBuf,
    input: PathBuf,
    review: Value,
    snapshot: OnceCell<Value>,
}

impl Fixture {
    fn new(project_dependent: bool) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        fs::set_permissions(&project, fs::Permissions::from_mode(0o755)).unwrap();
        let target = project.join("AGENTS.md");
        fs::write(&target, b"Old project guidance.\n").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
        let before = root.join("before.txt");
        private(&before, b"Old project guidance.\n");
        let context = root.join("context.txt");
        let assistant = root.join("assistant.txt");
        let correction = root.join("correction.txt");
        let criterion = root.join("criterion.txt");
        let verification = root.join("verification.txt");
        let comparison = root.join("comparison.txt");
        let evidence = root.join("evidence.txt");
        let source_revision = root.join("revision.txt");
        let review = json!({"kind":"groundline-learning-conversation-review-input","schema":1,
            "pattern_id":"owning-scope","feedback_ref_sha256":hash(b"native-feedback-reference"),
            "source_work_sha256":hash(b"native-original-work"),"interpretation":"assistant_error",
            "observed_scope":{"kind":"project","project_sha256":hash(project.as_os_str().as_encoded_bytes())},
            "project_dependent":project_dependent,"criterion_version":"v1",
            "criterion_sha256":private(&criterion,b"Keep each project's conditions local.\n"),
            "preceding_assistant_action_sha256":private(&assistant,b"PRIVATE_FAKE_ASSISTANT_TEXT"),
            "user_correction_sha256":private(&correction,b"PRIVATE_FAKE_CORRECTION_TEXT"),
            "context_evidence_sha256":private(&context,b"PRIVATE_FAKE_CONTEXT_TEXT"),
            "observed_at_utc":"2026-10-01T00:00:00Z","summary":"PRIVATE_FAKE_SUMMARY"});
        private(&verification, b"Actual guidance diff was checked.\n");
        private_json(
            &comparison,
            &json!({"schema":1,"model":"fixture-model","effort":"fixture-effort",
            "service_tier":"fixture-tier","runtime_family":"codex","runtime_version":"fixture-version",
            "tools_sha256":hash(b"relevant-tools"),"permissions_sha256":hash(b"scoped-permissions"),
            "task_parameters_sha256":hash(b"same-acceptance-parameters")}),
        );
        private(
            &evidence,
            b"Actual native instruction read or later work observation.\n",
        );
        private(&source_revision, b"Checked natural-work source revision.\n");
        let state = root.join("records");
        let input = root.join("input.json");
        Self {
            _temporary: temporary,
            root,
            project,
            state,
            target,
            before,
            context,
            assistant,
            correction,
            criterion,
            verification,
            comparison,
            evidence,
            source_revision,
            input,
            review,
            snapshot: OnceCell::new(),
        }
    }

    fn run(&self, command: &str, paths: &[(&str, &Path)], success: bool) -> Value {
        let mut process = Command::new(executable());
        process.current_dir(&self.root).args(["learning", command]);
        process.env("CODEX_HOME", self.root.join("codex-home"));
        for (flag, path) in paths {
            process.arg(flag).arg(path);
        }
        let output = process.output().unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.contains(self.root.to_str().unwrap()));
        assert!(!stdout.contains("PRIVATE_FAKE_"));
        serde_json::from_str(&stdout).unwrap()
    }

    fn review(&self, input: &Value, success: bool) -> Value {
        private_json(&self.input, input);
        self.run(
            "review-conversation",
            &[
                ("--input", &self.input),
                ("--state", &self.state),
                ("--context-evidence", &self.context),
                ("--assistant-evidence", &self.assistant),
                ("--correction-evidence", &self.correction),
                ("--criterion-evidence", &self.criterion),
            ],
            success,
        )
    }

    fn change_input(&self, review_hash: &Value) -> Value {
        self.change_input_in_scope(review_hash, self.review["observed_scope"].clone())
    }

    fn change_input_in_scope(&self, review_hash: &Value, scope: Value) -> Value {
        let snapshot = self.snapshot.get_or_init(|| {
            private_json(
                &self.input,
                &json!({"kind":"groundline-learning-conversation-capture-input",
                "schema":1,"review_sha256":review_hash,"scope":scope,
                "target_id":"project-guidance","target_kind":"agents_block"}),
            );
            let mut paths = vec![
                ("--input", self.input.as_path()),
                ("--target", self.target.as_path()),
                ("--before-output", self.before.as_path()),
                ("--state", self.state.as_path()),
            ];
            if scope["kind"] == "project" {
                paths.push(("--owner-project-root", self.project.as_path()));
            }
            let captured = self.run("capture-conversation-target", &paths, true);
            assert_eq!(
                captured["before_sha256"],
                hash(&fs::read(&self.target).unwrap())
            );
            // Native Codex edits the actual owning file only after capture.
            fs::write(
                &self.target,
                b"Keep project-local deployment criteria here.\n",
            )
            .unwrap();
            captured
        });
        let (conditions, fingerprint) = comparison(&self.comparison);
        json!({"kind":"groundline-learning-conversation-change-input","schema":2,
            "review_sha256":review_hash,"snapshot_sha256":snapshot["snapshot_sha256"],
            "target_binding_sha256":snapshot["target_binding_sha256"],"target":{"scope":scope,
                "target_id":"project-guidance","kind":"agents_block",
                "before_sha256":hash(&fs::read(&self.before).unwrap()),
                "after_sha256":hash(&fs::read(&self.target).unwrap())},
            "verification_evidence_sha256":hash(&fs::read(&self.verification).unwrap()),
            "comparison_context_sha256":fingerprint,"comparison_conditions":conditions,
            "applied_at_utc":chrono::Utc::now().to_rfc3339(),"transferability_review":null})
    }

    fn change(
        &self,
        input: &Value,
        root: Option<&Path>,
        target: &Path,
        scope_review: Option<&Path>,
        success: bool,
    ) -> Value {
        private_json(&self.input, input);
        let mut paths = vec![
            ("--input", self.input.as_path()),
            ("--state", self.state.as_path()),
            ("--target", target),
            ("--before", self.before.as_path()),
            ("--verification-evidence", self.verification.as_path()),
            ("--comparison-context-evidence", self.comparison.as_path()),
        ];
        if let Some(root) = root {
            paths.push(("--owner-project-root", root));
        }
        if let Some(path) = scope_review {
            paths.push(("--scope-review-evidence", path));
        }
        self.run("record-conversation-change", &paths, success)
    }

    fn activation(&self, change: &Value) -> Value {
        let (conditions, fingerprint) = comparison(&self.comparison);
        json!({"kind":"groundline-learning-conversation-activation-input","schema":2,
            "change_sha256":change,"target_after_sha256":hash(&fs::read(&self.target).unwrap()),
            "activation_kind":"instruction_read",
            "comparison_context_sha256":fingerprint,"comparison_conditions":conditions,
            "evidence_sha256":hash(&fs::read(&self.evidence).unwrap()),
            "activated_at_utc":chrono::Utc::now().to_rfc3339()})
    }

    fn observation(
        &self,
        change: &Value,
        work: &str,
        opportunity: Value,
        recurred: Value,
    ) -> Value {
        let (conditions, fingerprint) = comparison(&self.comparison);
        json!({"kind":"groundline-learning-conversation-observation-input","schema":2,
            "change_sha256":change,"source_work_sha256":hash(work.as_bytes()),
            "source_revision_sha256":hash(&fs::read(&self.source_revision).unwrap()),
            "target_after_sha256":hash(&fs::read(&self.target).unwrap()),
            "criterion_sha256":self.review["criterion_sha256"],"criterion_version":"v1",
            "comparison_context_sha256":fingerprint,"comparison_conditions":conditions,
            "opportunity":opportunity,"recurred":recurred,
            "evidence_sha256":hash(&fs::read(&self.evidence).unwrap()),
            "recurrence_evidence_sha256":null,"observed_at_utc":chrono::Utc::now().to_rfc3339()})
    }

    fn observe(&self, input: &Value, recurrence: Option<&Path>, success: bool) -> Value {
        private_json(&self.input, input);
        let mut paths = vec![
            ("--input", self.input.as_path()),
            ("--state", self.state.as_path()),
            ("--evidence", self.evidence.as_path()),
            ("--comparison-context-evidence", self.comparison.as_path()),
        ];
        if input["kind"] == "groundline-learning-conversation-observation-input" {
            paths.push(("--criterion-evidence", self.criterion.as_path()));
            paths.push(("--source-revision-evidence", self.source_revision.as_path()));
        }
        if let Some(path) = recurrence {
            paths.push(("--recurrence-evidence", path));
        }
        self.run("observe-conversation", &paths, success)
    }
}

fn next(value: &Value) -> &Value {
    &value["conversation_work"]["chains"][0]["next_required_action"]
}

#[test]
fn one_complete_cli_cycle_uses_conditions_not_work_provenance_or_json_format() {
    let f = Fixture::new(true);
    let review = f.review(&f.review, true);
    assert_eq!(review["conversation_work"]["assessment_complete"], false);
    let change = f.change(
        &f.change_input(&review["record_sha256"]),
        Some(&f.project),
        &f.target,
        None,
        true,
    );
    let activated = f.observe(&f.activation(&change["record_sha256"]), None, true);
    assert_eq!(activated["conversation_work"]["assessment_complete"], false);
    // Formatting is not a changed condition. New work and source revision have
    // their own bindings; neither belongs in the condition document.
    let conditions: Value = serde_json::from_slice(&fs::read(&f.comparison).unwrap()).unwrap();
    private(&f.comparison, &serde_json::to_vec(&conditions).unwrap());
    private(
        &f.source_revision,
        b"Different real-work revision reference.\n",
    );
    let observation = f.observation(
        &change["record_sha256"],
        "independent followup work",
        json!(true),
        json!(false),
    );
    let result = f.observe(&observation, None, true);
    assert_eq!(next(&result), "no_recurrence_observed");
    assert_eq!(result["conversation_work"]["assessment_complete"], true);
    assert_eq!(result["conversation_work"]["comparison_complete"], true);
    assert_eq!(result["conversation_work"]["opportunity_count"], 1);
    assert_eq!(result["conversation_work"]["recurrence_rate"], 0.0);
    assert_eq!(
        result["conversation_work"]["causal_effect_established"],
        false
    );
    let repeated = f.observe(&observation, None, true);
    assert_eq!(repeated["status"], "UNCHANGED");
    assert_eq!(repeated["record_sha256"], result["record_sha256"]);
    let status = f.run("status", &[("--state", &f.state)], true);
    assert_eq!(status["conversation_work"]["assessment_complete"], true);
    assert_eq!(
        status["readiness"]["conversation_work"]["comparison_complete"],
        true
    );
    assert_eq!(status["conversation_work"]["observation_count"], 1);
}

#[test]
fn completed_review_with_changed_or_unknown_conditions_never_claims_comparison_success() {
    for unknown in [false, true] {
        let f = Fixture::new(true);
        let review = f.review(&f.review, true);
        if unknown {
            let mut conditions: Value =
                serde_json::from_slice(&fs::read(&f.comparison).unwrap()).unwrap();
            conditions["permissions_sha256"] = Value::Null;
            private_json(&f.comparison, &conditions);
        }
        let change = f.change(
            &f.change_input(&review["record_sha256"]),
            Some(&f.project),
            &f.target,
            None,
            true,
        );
        f.observe(&f.activation(&change["record_sha256"]), None, true);
        if !unknown {
            let mut conditions: Value =
                serde_json::from_slice(&fs::read(&f.comparison).unwrap()).unwrap();
            conditions["model"] = json!("different-fixture-model");
            private_json(&f.comparison, &conditions);
        }
        let observation = f.observation(
            &change["record_sha256"],
            "different or unobserved conditions",
            json!(true),
            json!(false),
        );
        let result = f.observe(&observation, None, true);
        assert_eq!(result["conversation_work"]["assessment_complete"], true);
        assert_eq!(result["conversation_work"]["comparison_complete"], false);
        assert_eq!(next(&result), "wait_comparable_followup");
        assert_eq!(result["conversation_work"]["opportunity_count"], 0);
        assert!(result["conversation_work"]["recurrence_rate"].is_null());
        assert_eq!(
            result["conversation_work"]["causal_effect_established"],
            false
        );
    }
}

#[test]
fn comparison_contract_rejects_work_identity_and_retired_writer_inputs() {
    let f = Fixture::new(true);
    let review = f.review(&f.review, true);
    let change_input = f.change_input(&review["record_sha256"]);
    let change = f.change(&change_input, Some(&f.project), &f.target, None, true);
    let mut activation = f.activation(&change["record_sha256"]);
    activation["schema"] = json!(1);
    activation
        .as_object_mut()
        .unwrap()
        .remove("comparison_conditions");
    assert_eq!(
        f.observe(&activation, None, false)["error"],
        "learning_unsupported_conversation_comparison_schema"
    );
    let valid_activation = f.activation(&change["record_sha256"]);
    let mut conditions: Value = serde_json::from_slice(&fs::read(&f.comparison).unwrap()).unwrap();
    conditions["native_work_sha256"] = json!(hash(b"work identity belongs elsewhere"));
    private_json(&f.comparison, &conditions);
    assert_eq!(
        f.observe(&valid_activation, None, false)["error"],
        "learning_invalid_comparison_conditions"
    );
}

#[test]
fn justified_no_change_is_a_completed_cycle_without_a_trial_or_effect_claim() {
    let f = Fixture::new(true);
    let mut input = f.review.clone();
    input["interpretation"] = json!("iteration");
    let result = f.review(&input, true);
    assert_eq!(next(&result), "no_change");
    assert_eq!(result["conversation_work"]["assessment_complete"], true);
    assert_eq!(result["conversation_work"]["change_count"], 0);
    assert_eq!(
        result["conversation_work"]["causal_effect_established"],
        false
    );
}

#[test]
fn actual_change_activation_and_natural_followup_are_separate_from_delivery() {
    let f = Fixture::new(true);
    let review = f.review(&f.review, true);
    assert_eq!(next(&review), "apply_scoped_change");
    assert_eq!(
        f.review(&f.review, true)["record_sha256"],
        review["record_sha256"]
    );
    let change = f.change(
        &f.change_input(&review["record_sha256"]),
        Some(&f.project),
        &f.target,
        None,
        true,
    );
    assert_eq!(next(&change), "verify_activation");
    assert_eq!(change["conversation_work"]["activation_count"], 0);
    let activation = f.activation(&change["record_sha256"]);
    let activated = f.observe(&activation, None, true);
    assert_eq!(next(&activated), "wait_natural_opportunity");
    assert_eq!(activated["conversation_work"]["opportunity_count"], 0);
    let unknown = f.observation(
        &change["record_sha256"],
        "unassessed work",
        Value::Null,
        Value::Null,
    );
    assert_eq!(
        next(&f.observe(&unknown, None, true)),
        "wait_context_evidence"
    );
    let unrelated = f.observation(
        &change["record_sha256"],
        "no opportunity",
        json!(false),
        Value::Null,
    );
    assert_eq!(
        f.observe(&unrelated, None, true)["conversation_work"]["opportunity_count"],
        0
    );
    let followup = f.observation(
        &change["record_sha256"],
        "real new work",
        json!(true),
        json!(false),
    );
    let observed = f.observe(&followup, None, true);
    assert_eq!(observed["conversation_work"]["opportunity_count"], 1);
    assert!(observed["conversation_work"]["recurrence_rate"].is_null());
    let repeat = f.observe(&followup, None, true);
    assert_eq!(repeat["status"], "UNCHANGED");
    assert_eq!(repeat["conversation_work"]["opportunity_count"], 1);
    let status = f.run("status", &[("--state", &f.state)], true);
    assert_eq!(status["readiness"]["totals"]["task_outcome_count"], 0);
    assert_eq!(
        status["readiness"]["conversation_work"]["opportunity_count"],
        1
    );
    assert_eq!(
        status["readiness"]["efficiency_improvement_verified"],
        false
    );
}

#[test]
fn evidence_mismatch_private_permissions_and_changed_logical_input_are_rejected() {
    let f = Fixture::new(true);
    for field in [
        "context_evidence_sha256",
        "preceding_assistant_action_sha256",
        "user_correction_sha256",
        "criterion_sha256",
    ] {
        let mut invalid = f.review.clone();
        invalid[field] = json!(hash(b"different"));
        assert_eq!(
            f.review(&invalid, false)["error"],
            "learning_conversation_evidence_mismatch"
        );
    }
    assert!(!f.state.exists());
    fs::set_permissions(&f.context, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        f.review(&f.review, false)["error"],
        "learning_private_regular_input_required"
    );
    fs::set_permissions(&f.context, fs::Permissions::from_mode(0o600)).unwrap();
    let original = f.review(&f.review, true);
    let original_bytes = fs::read(f.state.join(format!(
        "{}.json",
        original["record_sha256"].as_str().unwrap()
    )))
    .unwrap();
    let mut invalid = f.review.clone();
    invalid["interpretation"] = json!("direction_change");
    assert_eq!(
        f.review(&invalid, false)["error"],
        "learning_conversation_duplicate_conflict"
    );
    assert_eq!(
        fs::read(f.state.join(format!(
            "{}.json",
            original["record_sha256"].as_str().unwrap()
        )))
        .unwrap(),
        original_bytes
    );
    assert_eq!(
        f.run("status", &[("--state", &f.state)], true)["conversation_work"]["review_count"],
        1
    );
}

#[test]
fn owning_project_root_target_snapshots_and_symlinks_are_checked() {
    let f = Fixture::new(true);
    let review = f.review(&f.review, true);
    let mut input = f.change_input(&review["record_sha256"]);
    let other = f.root.join("other-project");
    fs::create_dir(&other).unwrap();
    fs::set_permissions(&other, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        f.change(&input, Some(&other), &f.target, None, false)["error"],
        "learning_conversation_project_root_mismatch"
    );
    assert_eq!(
        f.change(&input, None, &f.target, None, false)["error"],
        "learning_conversation_project_root_required"
    );
    input["target"]["before_sha256"] = json!(hash(b"wrong snapshot"));
    assert_eq!(
        f.change(&input, Some(&f.project), &f.target, None, false)["error"],
        "learning_conversation_target_digest_mismatch"
    );
    input = f.change_input(&review["record_sha256"]);
    let linked = f.project.join("linked.md");
    symlink(&f.target, &linked).unwrap();
    assert_eq!(
        f.change(&input, Some(&f.project), &linked, None, false)["error"],
        "learning_conversation_target_binding_mismatch"
    );
    let linked_directory = f.project.join("linked-directory");
    symlink(&other, &linked_directory).unwrap();
    let outsider = other.join("AGENTS.md");
    fs::write(&outsider, fs::read(&f.target).unwrap()).unwrap();
    assert_eq!(
        f.change(
            &input,
            Some(&f.project),
            &linked_directory.join("AGENTS.md"),
            None,
            false
        )["error"],
        "learning_conversation_target_binding_mismatch"
    );
    assert_eq!(
        f.run("status", &[("--state", &f.state)], true)["conversation_work"]["change_count"],
        0
    );
}

#[test]
fn common_changes_require_positive_evidence_and_project_neutral_origin() {
    let f = Fixture::new(true);
    let review = f.review(&f.review, true);
    let mut change = f.change_input_in_scope(&review["record_sha256"], json!({"kind":"common"}));
    let neutral = f.root.join("neutral-review.txt");
    let neutral_hash = private(&neutral, b"Reviewed applicability beyond this project.\n");
    change["transferability_review"] =
        json!({"project_neutral":true,"evidence_sha256":neutral_hash});
    assert_eq!(
        f.change(&change, None, &f.target, None, false)["error"],
        "learning_conversation_evidence_required"
    );
    assert_eq!(
        f.change(&change, None, &f.target, Some(&neutral), false)["error"],
        "learning_conversation_common_scope_unreviewed"
    );
    let f = Fixture::new(false);
    let review = f.review(&f.review, true);
    let mut change = f.change_input_in_scope(&review["record_sha256"], json!({"kind":"common"}));
    let neutral = f.root.join("neutral-review.txt");
    let neutral_hash = private(&neutral, b"Reviewed project-neutral instruction.\n");
    change["transferability_review"] =
        json!({"project_neutral":false,"evidence_sha256":neutral_hash});
    assert_eq!(
        f.change(&change, None, &f.target, Some(&neutral), false)["error"],
        "learning_conversation_common_scope_unreviewed"
    );
    change["transferability_review"]["project_neutral"] = json!(true);
    assert_eq!(
        next(&f.change(&change, None, &f.target, Some(&neutral), true)),
        "verify_activation"
    );
}

#[test]
fn recurrence_exact_bytes_and_comparable_conditions_control_followup_readout() {
    let f = Fixture::new(true);
    let review = f.review(&f.review, true);
    let change = f.change(
        &f.change_input(&review["record_sha256"]),
        Some(&f.project),
        &f.target,
        None,
        true,
    );
    let mut activation = f.activation(&change["record_sha256"]);
    activation["activated_at_utc"] = json!("2026-10-01T00:00:00Z");
    assert_eq!(
        f.observe(&activation, None, false)["error"],
        "learning_conversation_activation_mismatch"
    );
    f.observe(&f.activation(&change["record_sha256"]), None, true);
    let mut followup = f.observation(
        &change["record_sha256"],
        "regression work",
        json!(true),
        json!(true),
    );
    followup["recurrence_evidence_sha256"] = json!(hash(&fs::read(&f.correction).unwrap()));
    assert_eq!(
        f.observe(&followup, None, false)["error"],
        "learning_conversation_evidence_required"
    );
    let out = f.observe(&followup, Some(&f.correction), true);
    assert_eq!(next(&out), "review_recurrence");
    assert_eq!(out["conversation_work"]["recurrence_rate"], 1.0);
    let mut changed = f.observation(
        &change["record_sha256"],
        "changed criterion work",
        json!(true),
        json!(false),
    );
    changed["criterion_version"] = json!("v2");
    let out = f.observe(&changed, None, true);
    assert_eq!(out["conversation_work"]["inconclusive_count"], 1);
    assert!(out["conversation_work"]["recurrence_rate"].is_null());
    assert_eq!(out["conversation_work"]["causal_effect_established"], false);
}

#[test]
fn unrelated_private_records_survive_and_pre_activation_work_is_not_effect_proof() {
    let f = Fixture::new(true);
    fs::create_dir(&f.state).unwrap();
    fs::set_permissions(&f.state, fs::Permissions::from_mode(0o700)).unwrap();
    let snapshot = json!({"kind":"groundline-learning-snapshot","schema":1,
        "captured_at_utc":"2026-10-01T00:00:00Z","unit_hash":hash(b"unrelated-unit"),
        "cohort_sha256":hash(b"unrelated-cohort"),"phase":"implementation",
        "environment_revision":null,"source_revision":null,"skill":null,"runtime":null,
        "environment_provenance_sha256":hash(b"unrelated-environment"),
        "capture_input_sha256":hash(b"unrelated-capture"),"native_activation":"UNVERIFIED"});
    let existing = f.state.join(format!(
        "{}.json",
        groundline_contracts::learning::content_sha256(&snapshot).unwrap()
    ));
    private_json(&existing, &snapshot);
    let original = fs::read(&existing).unwrap();
    let review = f.review(&f.review, true);
    let change = f.change(
        &f.change_input(&review["record_sha256"]),
        Some(&f.project),
        &f.target,
        None,
        true,
    );
    let mut observation = f.observation(
        &change["record_sha256"],
        "before activation",
        json!(true),
        json!(false),
    );
    observation["observed_at_utc"] = json!("2026-10-01T01:30:00Z");
    let out = f.observe(&observation, None, true);
    assert_eq!(out["conversation_work"]["opportunity_count"], 0);
    assert_eq!(
        out["conversation_work"]["recorded_opportunities"]["present_count"],
        1
    );
    assert_eq!(out["conversation_work"]["inconclusive_count"], 1);
    f.observe(&f.activation(&change["record_sha256"]), None, true);
    let status = f.run("status", &[("--state", &f.state)], true);
    assert_eq!(status["conversation_work"]["opportunity_count"], 0);
    assert_eq!(
        status["conversation_work"]["recorded_opportunities"]["present_count"],
        1
    );
    assert_eq!(
        status["readiness"]["conversation_work"]["recorded_opportunities"],
        status["conversation_work"]["recorded_opportunities"]
    );
    assert_eq!(status["conversation_work"]["inconclusive_count"], 1);
    assert_eq!(status["readiness"]["standalone_capture_count"], 1);
    assert_eq!(fs::read(&existing).unwrap(), original);
    assert!(
        !f.root
            .join("codex-home/groundline/learning/profile.json")
            .exists()
    );
}

#[test]
fn real_capture_rejects_manufactured_before_and_another_identical_target() {
    let f = Fixture::new(true);
    let review = f.review(&f.review, true);
    let input = f.change_input(&review["record_sha256"]);
    let other = f.project.join("OTHER.md");
    fs::write(&other, fs::read(&f.target).unwrap()).unwrap();
    assert_eq!(
        f.change(&input, Some(&f.project), &other, None, false)["error"],
        "learning_conversation_target_binding_mismatch"
    );
    let mut forged = input.clone();
    let fake_before = private(&f.before, b"Caller-made invented earlier contents.\n");
    forged["target"]["before_sha256"] = json!(fake_before);
    assert_eq!(
        f.change(&forged, Some(&f.project), &f.target, None, false)["error"],
        "learning_conversation_snapshot_mismatch"
    );
    private(&f.before, b"Old project guidance.\n");
    forged = input.clone();
    forged["snapshot_sha256"] = json!(hash(b"not a captured record"));
    assert_eq!(
        f.change(&forged, Some(&f.project), &f.target, None, false)["error"],
        "learning_conversation_snapshot_reference_missing"
    );
    // The same declared path replaced by a symlink still fails after binding.
    fs::rename(&f.target, &other).unwrap();
    symlink(&other, &f.target).unwrap();
    assert_eq!(
        f.change(&input, Some(&f.project), &f.target, None, false)["error"],
        "learning_invalid_conversation_target"
    );
}

#[test]
fn completed_history_cools_with_immutable_retry_and_pending_work_preserved() {
    let f = Fixture::new(true);
    fs::create_dir(&f.state).unwrap();
    fs::set_permissions(&f.state, fs::Permissions::from_mode(0o700)).unwrap();
    let mut old = Vec::new();
    for index in 0..440 {
        let mut input = f.review.clone();
        input["pattern_id"] = json!(format!("old-{index}"));
        input["feedback_ref_sha256"] = json!(hash(format!("old-feedback-{index}").as_bytes()));
        input["interpretation"] = json!("iteration");
        let record = groundline_contracts::learning::conversation_work::build_record(
            &input,
            "2026-10-02T00:00:00Z",
        )
        .unwrap();
        let digest = groundline_contracts::learning::content_sha256(&record).unwrap();
        let path = f.state.join(format!("{digest}.json"));
        private_json(&path, &record);
        old.push((input, digest, fs::read(path).unwrap()));
    }
    let pending = f.review(&f.review, true);
    assert_eq!(pending["archive"]["archived_record_count_this_write"], 408);
    assert_eq!(pending["conversation_work"]["review_count"], 33);
    assert_eq!(pending["archive"]["historical_coverage_complete"], false);
    assert!(pending["archive"]["archived_history_count"].is_null());
    assert!(
        f.state
            .join(format!(
                "{}.json",
                pending["record_sha256"].as_str().unwrap()
            ))
            .exists()
    );
    let (input, digest, bytes) = old
        .into_iter()
        .find(|(_, digest, _)| {
            f.state
                .join("archive")
                .join(&digest[..2])
                .join(format!("{digest}.json"))
                .exists()
        })
        .unwrap();
    let archived = f
        .state
        .join("archive")
        .join(&digest[..2])
        .join(format!("{digest}.json"));
    assert_eq!(fs::read(&archived).unwrap(), bytes);
    assert_eq!(
        fs::metadata(&archived).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let retried = f.review(&input, true);
    assert_eq!(retried["status"], "UNCHANGED");
    assert_eq!(retried["record_sha256"], digest);
    assert_eq!(retried["archive"]["referenced_archive_record_count"], 1);
    let mut changed = input;
    changed["interpretation"] = json!("assistant_error");
    assert_eq!(
        f.review(&changed, false)["error"],
        "learning_conversation_duplicate_conflict"
    );
    assert_eq!(fs::read(&archived).unwrap(), bytes);
    assert_eq!(
        f.run("status", &[("--state", &f.state)], true)["conversation_work"]["review_count"],
        33
    );
}

#[test]
fn later_recurrence_restores_cold_activation_and_original_followup_closure() {
    let f = Fixture::new(true);
    let review = f.review(&f.review, true);
    let change = f.change(
        &f.change_input(&review["record_sha256"]),
        Some(&f.project),
        &f.target,
        None,
        true,
    );
    f.observe(&f.activation(&change["record_sha256"]), None, true);
    let successful = f.observation(
        &change["record_sha256"],
        "first natural work",
        json!(true),
        json!(false),
    );
    assert_eq!(
        next(&f.observe(&successful, None, true)),
        "no_recurrence_observed"
    );
    for index in 0..400 {
        let mut input = f.review.clone();
        input["pattern_id"] = json!(format!("finished-{index}"));
        input["feedback_ref_sha256"] = json!(hash(format!("finished-feedback-{index}").as_bytes()));
        input["interpretation"] = json!("iteration");
        let record = groundline_contracts::learning::conversation_work::build_record(
            &input,
            &chrono::Utc::now().to_rfc3339(),
        )
        .unwrap();
        let digest = groundline_contracts::learning::content_sha256(&record).unwrap();
        private_json(&f.state.join(format!("{digest}.json")), &record);
    }
    let mut new_review = f.review.clone();
    new_review["pattern_id"] = json!("another-pending-pattern");
    new_review["feedback_ref_sha256"] = json!(hash(b"another pending feedback"));
    let maintained = f.review(&new_review, true);
    assert_eq!(
        maintained["archive"]["archived_record_count_this_write"],
        373
    );
    assert!(
        !f.state
            .join(format!(
                "{}.json",
                change["record_sha256"].as_str().unwrap()
            ))
            .exists()
    );
    let mut recurrence = f.observation(
        &change["record_sha256"],
        "later real regression",
        json!(true),
        json!(true),
    );
    recurrence["recurrence_evidence_sha256"] = json!(hash(&fs::read(&f.correction).unwrap()));
    let observed = f.observe(&recurrence, Some(&f.correction), true);
    assert_eq!(observed["conversation_work"]["opportunity_count"], 2);
    assert_eq!(observed["conversation_work"]["recurrence_count"], 1);
    let status = f.run("status", &[("--state", &f.state)], true);
    assert_eq!(status["conversation_work"]["activation_count"], 1);
    assert_eq!(status["conversation_work"]["opportunity_count"], 2);
    assert_eq!(status["conversation_work"]["recurrence_count"], 1);
    assert_eq!(status["archive"]["referenced_archive_record_count"], 5);
}
