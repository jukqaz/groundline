#![cfg(unix)]
use chrono::Utc;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn argument(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn json_output(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    state: PathBuf,
    target: PathBuf,
    input: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let target = root.join("AGENTS.md");
        fs::write(&target, b"Original guidance.\n").unwrap();
        Self {
            state: root.join("records"),
            target,
            input: root.join("brief.json"),
            root,
            _temp: temp,
        }
    }
    fn write_input(&self, value: &Value) {
        fs::write(&self.input, serde_json::to_vec(value).unwrap()).unwrap();
        fs::set_permissions(&self.input, fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn brief(&self) -> Value {
        json!({"pattern_id":"cleanup-scope", "source_work_sha256":hash(b"original-work"),
        "context_sha256":hash(b"derived-context-reference"),"criterion_sha256":hash(b"owning-criterion"),
        "interpretation":"assistant_error","scope":"project","target_kind":"agents_block","transferability_sha256":null})
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_groundline"))
            .args(args)
            .env("CODEX_HOME", self.root.join("unused-codex-home"))
            .output()
            .unwrap()
    }
    fn capture(&self) -> Value {
        self.write_input(&self.brief());
        let out = self.run(&[
            "improve",
            "capture",
            "--input",
            argument(&self.input),
            "--target",
            argument(&self.target),
            "--owner-project-root",
            argument(&self.root),
            "--state",
            argument(&self.state),
        ]);
        assert!(out.status.success(), "{out:?}");
        json_output(&out)
    }
    fn record(&self, capture: &str, target: &Path) -> Output {
        self.run(&[
            "improve",
            "record",
            "--capture",
            capture,
            "--target",
            argument(target),
            "--owner-project-root",
            argument(&self.root),
            "--state",
            argument(&self.state),
        ])
    }
    fn change(&self) -> String {
        let capture = self.capture();
        fs::write(&self.target, b"Keep cleanup within the accepted scope.\n").unwrap();
        let out = self.record(capture["record_sha256"].as_str().unwrap(), &self.target);
        assert!(out.status.success(), "{out:?}");
        json_output(&out)["record_sha256"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn observe(
        &self,
        change: &str,
        work: &str,
        exposure: &str,
        opportunity: &str,
        behavior: &str,
    ) -> Output {
        self.write_input(&json!({"source_work_sha256":hash(work.as_bytes()),"evidence_sha256":hash(format!("evidence-{work}").as_bytes()),
            "target_after_sha256":hash(&fs::read(&self.target).unwrap()),"observed_at_utc":Utc::now().to_rfc3339(),
            "exposure":exposure,"opportunity":opportunity,"behavior":behavior}));
        self.run(&[
            "improve",
            "observe",
            "--change",
            change,
            "--input",
            argument(&self.input),
            "--state",
            argument(&self.state),
        ])
    }
    fn status(&self) -> Value {
        let out = self.run(&["improve", "status", "--state", argument(&self.state)]);
        assert!(out.status.success(), "{out:?}");
        json_output(&out)
    }
    fn files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files: Vec<_> = fs::read_dir(&self.state)
            .unwrap()
            .map(|p| {
                let p = p.unwrap().path();
                let bytes = fs::read(&p).unwrap();
                (p, bytes)
            })
            .collect();
        files.sort();
        files
    }
}

#[test]
fn compact_change_and_followup_work_without_model_comparison_or_settings() {
    let f = Fixture::new();
    let change = f.change();
    let before = fs::read(&f.target).unwrap();
    let observation = f.observe(&change, "independent-work", "observed", "present", "met");
    assert!(observation.status.success(), "{observation:?}");
    let snapshot = f.files();
    let duplicate = f.run(&[
        "improve",
        "observe",
        "--change",
        &change,
        "--input",
        argument(&f.input),
        "--state",
        argument(&f.state),
    ]);
    assert!(duplicate.status.success(), "{duplicate:?}");
    assert_eq!(json_output(&duplicate)["status"], "ALREADY_RECORDED");
    assert_eq!(f.files(), snapshot);
    let status = f.status();
    assert_eq!(status["changes"][0]["criterion_met_count"], 1);
    assert_eq!(status["changes"][0]["assessed_count"], 1);
    assert_eq!(status["causal_effect_verified"], false);
    assert_eq!(status["efficiency_improvement_verified"], false);
    assert_eq!(fs::read(&f.target).unwrap(), before);
    assert!(!f.root.join("unused-codex-home").exists());
}

#[test]
fn unknown_exposure_and_missing_opportunities_never_become_successes() {
    let f = Fixture::new();
    let change = f.change();
    for (work, exposure, opportunity, behavior) in [
        ("unknown-exposure", "unknown", "present", "met"),
        ("no-opportunity", "observed", "absent", "unknown"),
        ("recurrence", "observed", "present", "violated"),
    ] {
        let out = f.observe(&change, work, exposure, opportunity, behavior);
        assert!(out.status.success(), "{out:?}");
    }
    let status = f.status();
    let row = &status["changes"][0];
    assert_eq!(row["followup_count"], 3);
    assert_eq!(row["assessed_count"], 1);
    assert_eq!(row["criterion_met_count"], 0);
    assert_eq!(row["recurrence_count"], 1);
    assert_eq!(row["unassessed_count"], 2);
    assert_eq!(row["pattern_id"], "cleanup-scope");
    assert_eq!(
        row["opportunities"],
        json!({"present":2,"absent":1,"unknown":0})
    );
    assert_eq!(row["exposure_unknown_count"], 1);
}

#[test]
fn changed_requirements_and_exploration_exit_without_creating_state() {
    let f = Fixture::new();
    for interpretation in ["requirement_change", "exploration", "unknown"] {
        let mut brief = f.brief();
        brief["interpretation"] = json!(interpretation);
        f.write_input(&brief);
        let out = f.run(&[
            "improve",
            "capture",
            "--input",
            argument(&f.input),
            "--target",
            argument(&f.root.join("absent.md")),
            "--state",
            argument(&f.state),
        ]);
        assert!(out.status.success(), "{out:?}");
        assert_eq!(json_output(&out)["status"], "NO_CHANGE");
        assert!(!f.state.exists());
    }
}

#[test]
fn capture_and_change_retries_preserve_state_and_reject_other_targets() {
    let f = Fixture::new();
    let capture = f.capture();
    let snapshot = f.files();
    assert_eq!(f.capture()["status"], "ALREADY_RECORDED");
    assert_eq!(f.files(), snapshot);
    let id = capture["record_sha256"].as_str().unwrap();
    assert_eq!(json_output(&f.record(id, &f.target))["status"], "NO_CHANGE");
    assert_eq!(f.files(), snapshot);
    let other_root = f.root.join("other");
    fs::create_dir(&other_root).unwrap();
    let other = other_root.join("AGENTS.md");
    fs::write(&other, fs::read(&f.target).unwrap()).unwrap();
    assert!(!f.record(id, &other).status.success());
    assert_eq!(f.files(), snapshot);
    fs::write(&f.target, b"New guidance.\n").unwrap();
    assert!(f.record(id, &f.target).status.success());
    let snapshot = f.files();
    assert_eq!(
        json_output(&f.record(id, &f.target))["status"],
        "ALREADY_RECORDED"
    );
    assert_eq!(f.files(), snapshot);
    fs::write(&f.target, b"Later user edit.\n").unwrap();
    assert!(!f.record(id, &f.target).status.success());
    assert_eq!(fs::read(&f.target).unwrap(), b"Later user edit.\n");
    assert_eq!(f.files(), snapshot);
}

#[test]
fn global_promotion_requires_review_and_project_targets_cannot_escape() {
    let f = Fixture::new();
    let mut brief = f.brief();
    brief["scope"] = json!("common");
    f.write_input(&brief);
    let out = f.run(&[
        "improve",
        "capture",
        "--input",
        argument(&f.input),
        "--target",
        argument(&f.target),
        "--state",
        argument(&f.state),
    ]);
    assert!(!out.status.success());
    assert!(!f.state.exists());
    f.write_input(&f.brief());
    let unrelated = f.root.join("other");
    fs::create_dir(&unrelated).unwrap();
    let out = f.run(&[
        "improve",
        "capture",
        "--input",
        argument(&f.input),
        "--target",
        argument(&f.target),
        "--owner-project-root",
        argument(&unrelated),
        "--state",
        argument(&f.state),
    ]);
    assert!(!out.status.success());
    assert!(!f.state.exists());
}

#[test]
fn symlink_hardlink_and_public_inputs_are_rejected_without_target_changes() {
    let f = Fixture::new();
    f.write_input(&f.brief());
    let link = f.root.join("linked.md");
    symlink(&f.target, &link).unwrap();
    let mut brief = f.brief();
    brief["target_kind"] = json!("skill");
    f.write_input(&brief);
    let out = f.run(&[
        "improve",
        "capture",
        "--input",
        argument(&f.input),
        "--target",
        argument(&link),
        "--owner-project-root",
        argument(&f.root),
        "--state",
        argument(&f.state),
    ]);
    assert!(!out.status.success());
    assert!(!f.state.exists());
    fs::remove_file(&link).unwrap();
    fs::hard_link(&f.target, &link).unwrap();
    let out = f.run(&[
        "improve",
        "capture",
        "--input",
        argument(&f.input),
        "--target",
        argument(&link),
        "--owner-project-root",
        argument(&f.root),
        "--state",
        argument(&f.state),
    ]);
    assert!(!out.status.success());
    assert!(!f.state.exists());
    fs::remove_file(&link).unwrap();
    fs::set_permissions(&f.input, fs::Permissions::from_mode(0o644)).unwrap();
    let out = f.run(&[
        "improve",
        "capture",
        "--input",
        argument(&f.input),
        "--target",
        argument(&f.target),
        "--owner-project-root",
        argument(&f.root),
        "--state",
        argument(&f.state),
    ]);
    assert!(!out.status.success());
    assert!(!f.state.exists());
    assert_eq!(fs::read(&f.target).unwrap(), b"Original guidance.\n");
}

#[test]
fn same_work_conflicting_followups_and_tampered_state_fail_privately() {
    let f = Fixture::new();
    let change = f.change();
    let snapshot = f.files();
    let out = f.observe(&change, "original-work", "observed", "present", "met");
    assert!(!out.status.success());
    assert_eq!(f.files(), snapshot);
    assert!(
        f.observe(&change, "independent-work", "observed", "present", "met")
            .status
            .success()
    );
    let snapshot = f.files();
    let out = f.observe(
        &change,
        "independent-work",
        "observed",
        "present",
        "violated",
    );
    assert!(!out.status.success());
    assert_eq!(f.files(), snapshot);
    assert!(!String::from_utf8_lossy(&out.stdout).contains(argument(&f.root)));
    let path = f.state.join(format!("{}.json", hash(b"wrong-digest")));
    fs::write(&path, b"{\"private\":\"PRIVATE_SENTINEL\"}").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let out = f.run(&["improve", "status", "--state", argument(&f.state)]);
    assert!(!out.status.success());
    assert!(!String::from_utf8_lossy(&out.stdout).contains("PRIVATE_SENTINEL"));
    assert_eq!(
        fs::read(&path).unwrap(),
        b"{\"private\":\"PRIVATE_SENTINEL\"}"
    );
}

#[test]
fn historical_conversation_record_is_read_without_lock_creation_or_migration() {
    let f = Fixture::new();
    fs::create_dir(&f.state).unwrap();
    fs::set_permissions(&f.state, fs::Permissions::from_mode(0o700)).unwrap();
    let input = json!({"kind":"groundline-learning-conversation-review-input","schema":1,
        "pattern_id":"old-pattern","feedback_ref_sha256":hash(b"feedback"),"source_work_sha256":hash(b"old-work"),
        "interpretation":"assistant_error","observed_scope":{"kind":"common"},"project_dependent":false,
        "criterion_sha256":hash(b"criterion"),"criterion_version":"criterion-1",
        "preceding_assistant_action_sha256":hash(b"action"),"user_correction_sha256":hash(b"correction"),
        "context_evidence_sha256":hash(b"context"),"observed_at_utc":"2026-10-01T00:00:00Z","summary":null});
    let old = groundline_contracts::learning::conversation_work::build_record(
        &input,
        "2026-10-01T00:00:01Z",
    )
    .unwrap();
    let name = groundline_contracts::learning::content_sha256(&old).unwrap();
    let path = f.state.join(format!("{name}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&old).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let snapshot = f.files();
    let status = f.status();
    assert_eq!(status["historical_record_count"], 1);
    assert_eq!(f.files(), snapshot);
    let out = f.run(&["learning", "status", "--state", argument(&f.state)]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(f.files(), snapshot);
    assert!(!f.state.join(".learning.lock").exists());
    let before = fs::read(&path).unwrap();
    let metadata = fs::metadata(&path).unwrap();
    f.change();
    assert_eq!(f.status()["historical_record_count"], 1);
    assert_eq!(fs::read(&path).unwrap(), before);
    use std::os::unix::fs::MetadataExt;
    assert_eq!(fs::metadata(&path).unwrap().ino(), metadata.ino());
}

#[test]
fn future_and_contradictory_observations_and_conflicting_writer_are_read_only_failures() {
    let f = Fixture::new();
    let change = f.change();
    let snapshot = f.files();
    let out = f.observe(&change, "later-work", "observed", "absent", "met");
    assert!(!out.status.success());
    assert_eq!(f.files(), snapshot);
    f.write_input(&json!({"source_work_sha256":hash(b"later-work"),"evidence_sha256":hash(b"later-evidence"),
        "target_after_sha256":hash(&fs::read(&f.target).unwrap()),"observed_at_utc":(Utc::now()+chrono::Duration::hours(1)).to_rfc3339(),
        "exposure":"observed","opportunity":"present","behavior":"met"}));
    let out = f.run(&[
        "improve",
        "observe",
        "--change",
        &change,
        "--input",
        argument(&f.input),
        "--state",
        argument(&f.state),
    ]);
    assert!(!out.status.success());
    assert_eq!(f.files(), snapshot);
    let lock = fs::File::open(f.state.join(".learning.lock")).unwrap();
    lock.try_lock().unwrap();
    let out = f.run(&["improve", "status", "--state", argument(&f.state)]);
    assert!(!out.status.success());
    assert_eq!(f.files(), snapshot);
    lock.unlock().unwrap();
}

#[test]
fn unsupported_state_layout_is_preserved_and_missing_writer_lock_is_rejected() {
    let f = Fixture::new();
    let change = f.change();
    let snapshot = f.files();
    let extra = f.state.join("unsupported-archive");
    fs::create_dir(&extra).unwrap();
    let out = f.run(&["learning", "status", "--state", argument(&f.state)]);
    assert!(!out.status.success());
    assert!(extra.exists());
    fs::remove_dir(&extra).unwrap();
    assert_eq!(f.files(), snapshot);
    fs::remove_file(f.state.join(".learning.lock")).unwrap();
    let snapshot = f.files();
    let out = f.observe(&change, "later-work", "observed", "present", "met");
    assert!(!out.status.success());
    assert_eq!(f.files(), snapshot);
    assert!(!f.state.join(".learning.lock").exists());
}
