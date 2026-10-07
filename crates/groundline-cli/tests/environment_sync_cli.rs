#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write_json(path: &Path, value: &Value) -> String {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(path, &bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    hash(&bytes)
}
fn arg(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn run(root: &Path, arguments: &[&str], expected_error: Option<&str>) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .current_dir(root)
        .arg("environment")
        .args(arguments)
        .arg("--json")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains(arg(root)), "private path emitted");
    assert!(
        !stdout.contains("Outside device"),
        "private content emitted"
    );
    let result: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "stdout={stdout}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    if let Some(error) = expected_error {
        assert!(!output.status.success(), "{result}");
        assert_eq!(result["error"], error);
    } else {
        // PARTIAL is a valid recorded rollback result and has a nonzero CLI exit.
        assert!(
            output.status.success() || result["status"] == "PARTIAL",
            "{result}"
        );
    }
    assert_eq!(result["raw_content_emitted"], false);
    assert_eq!(result["private_paths_emitted"], false);
    result
}

struct Device {
    root: PathBuf,
    state: PathBuf,
    bindings: PathBuf,
    workflow: PathBuf,
    reference: PathBuf,
    agents: PathBuf,
    before_agents: String,
}
impl Device {
    fn new(root: &Path, id: &str, exception: &str) -> Self {
        let root = root.join(id);
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let skills = root.join("skills");
        fs::create_dir(&skills).unwrap();
        fs::set_permissions(&skills, fs::Permissions::from_mode(0o700)).unwrap();
        let workflow = skills.join("SKILL.md");
        fs::write(&workflow, format!("Before {id}\n")).unwrap();
        let reference = skills.join("reference.md");
        let agents = root.join("AGENTS.md");
        let before_agents = format!(
            "Outside device {id} private instructions\n<!-- managed:start -->\nBefore {id}\n<!-- managed:end -->\nOutside device {id} private tail\n"
        );
        fs::write(&agents, &before_agents).unwrap();
        let bindings = root.join("bindings.json");
        write_json(
            &bindings,
            &json!({"kind":"groundline-environment-device-bindings","schema":1,
            "revision":exception,"device_id":id,
            "roots":[{"root_id":"owner","path":root,"aliases":[]}],
            "targets":[{"target_id":"workflow","root_id":"owner","relative_path":"skills/SKILL.md"},
                {"target_id":"reference","root_id":"owner","relative_path":"skills/reference.md"},
                {"target_id":"instructions","root_id":"owner","relative_path":"AGENTS.md"}]}),
        );
        Self {
            state: root.join("PRIVATE_STATE"),
            root,
            bindings,
            workflow,
            reference,
            agents,
            before_agents,
        }
    }
    fn seed(&self, revision: &str, parent: Option<&str>, version: u8, exception: &str) {
        let content = |id| format!("{id} v{version}\n");
        let baseline = json!({"kind":"groundline-environment-baseline","schema":1,"revision":revision,
            "parent_revision":parent,"source_revision":format!("source-{version}"),"authority_revision":"authority-1",
            "exception_revision":exception,"authority_ref":"explicit-owner-approval",
            "managed_targets":[{"target_id":"workflow","kind":"skill_file","desired_sha256":hash(content("Workflow").as_bytes()),"dependencies":["reference"],"managed_block":null},
                {"target_id":"reference","kind":"skill_file","desired_sha256":hash(content("Reference").as_bytes()),"dependencies":[],"managed_block":null},
                {"target_id":"instructions","kind":"agents_block","desired_sha256":hash(content("Instructions").as_bytes()),"dependencies":[],"managed_block":{"start_marker":"<!-- managed:start -->","end_marker":"<!-- managed:end -->"}}]});
        write_json(&self.root.join("baseline.json"), &baseline);
        let mut bindings: Value =
            serde_json::from_slice(&fs::read(&self.bindings).unwrap()).unwrap();
        bindings["revision"] = json!(exception);
        write_json(&self.bindings, &bindings);
        write_json(
            &self.root.join("proposal.json"),
            &json!({"kind":"groundline-environment-proposal","schema":1,
            "proposal_id":format!("export-{version}"),"basis_revision":revision,"source_revision":format!("source-{version}"),
            "authority_ref":"explicit-owner-approval","changes":[{"target_id":"workflow","content":content("Workflow")},
                {"target_id":"reference","content":content("Reference")},{"target_id":"instructions","content":content("Instructions")}]}),
        );
        run(
            &self.root,
            &[
                "register",
                "--state-dir",
                "PRIVATE_STATE",
                "--baseline",
                "baseline.json",
                "--bindings",
                "bindings.json",
            ],
            None,
        );
    }
    fn export(&self, name: &str, explicit: bool) -> (PathBuf, String) {
        let output = self.root.join(name);
        let mut args = vec!["export", "--state-dir", "PRIVATE_STATE", "--output", name];
        if explicit {
            args.extend(["--proposal", "proposal.json"]);
        }
        let result = run(&self.root, &args, None);
        (output, result["bundle_sha256"].as_str().unwrap().into())
    }
    fn import(
        &self,
        bundle: &Path,
        digest: &str,
        expected: Option<(&str, &str)>,
        error: Option<&str>,
    ) -> Value {
        let mut args = vec![
            "import",
            "--state-dir",
            arg(&self.state),
            "--bundle",
            arg(bundle),
            "--bundle-sha256",
            digest,
            "--bindings",
            arg(&self.bindings),
            "--authority-ref",
            "explicit-owner-approval",
        ];
        if let Some((revision, exception)) = expected {
            args.extend([
                "--expected-revision",
                revision,
                "--expected-exception-revision",
                exception,
            ]);
        } else {
            args.push("--new-device");
        }
        run(&self.root, &args, error)
    }
    fn plan(&self, bundle: &Path, digest: &str, id: &str) -> Value {
        run(
            &self.root,
            &[
                "plan-bundle",
                "--state-dir",
                arg(&self.state),
                "--bundle",
                arg(bundle),
                "--bundle-sha256",
                digest,
                "--proposal-id",
                id,
            ],
            None,
        )
    }
    fn apply(&self, id: &str, error: Option<&str>) -> Value {
        run(
            &self.root,
            &[
                "apply",
                "--state-dir",
                arg(&self.state),
                "--proposal-id",
                id,
            ],
            error,
        )
    }
    fn rollback(&self, id: &str) -> Value {
        run(
            &self.root,
            &[
                "rollback",
                "--state-dir",
                arg(&self.state),
                "--operation-id",
                id,
            ],
            None,
        )
    }
}
fn private_root() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    temp
}

#[test]
fn common_change_tracks_different_device_plans_and_operations_without_copying_effect() {
    let temp = private_root();
    let root = temp.path().canonicalize().unwrap();
    let a = Device::new(&root, "device-a", "exception-a1");
    let b = Device::new(&root, "device-b", "exception-b7");
    a.seed("baseline-1", None, 1, "exception-a1");
    let (bundle, digest) = a.export("bundle.json", true);
    b.import(&bundle, &digest, None, None);
    let a_plan = a.plan(&bundle, &digest, "a1");
    let b_plan = b.plan(&bundle, &digest, "b1");
    assert_eq!(a_plan["common_change_ref"], b_plan["common_change_ref"]);
    assert_ne!(a_plan["plan_sha256"], b_plan["plan_sha256"]);
    assert_ne!(a_plan["device_sha256"], b_plan["device_sha256"]);
    a.apply("a1", None);
    b.apply("b1", None);
    for device in [&a, &b] {
        let status = run(
            &device.root,
            &["status", "--state-dir", arg(&device.state)],
            None,
        );
        assert_eq!(status["common_change_ref"], a_plan["common_change_ref"]);
        assert_eq!(status["other_devices_verified"], false);
        assert_eq!(status["unlinked_plan_count"], 0);
        let changes = status["changes"].as_array().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0]["common_change_ref"], a_plan["common_change_ref"]);
        assert_eq!(changes[0]["bundle"]["bundle_sha256"], digest);
        let operations = changes[0]["operations"].as_array().unwrap();
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0]["status"], "APPLIED");
        assert_eq!(operations[0]["native_activation"], "UNVERIFIED");
        assert_eq!(operations[0]["effect_verified"], false);
        assert!(operations[0]["evaluations"].as_array().unwrap().is_empty());
        let repeated = run(
            &device.root,
            &["status", "--state-dir", arg(&device.state)],
            None,
        );
        assert_eq!(repeated, status);
    }
}

#[test]
fn status_keeps_old_operation_after_common_baseline_advances() {
    let temp = private_root();
    let root = temp.path().canonicalize().unwrap();
    let a = Device::new(&root, "device-a", "exception-a1");
    let b = Device::new(&root, "device-b", "exception-b7");
    a.seed("baseline-1", None, 1, "exception-a1");
    let (bundle, digest) = a.export("bundle1.json", true);
    b.import(&bundle, &digest, None, None);
    let old_plan = b.plan(&bundle, &digest, "b1");
    b.apply("b1", None);
    a.seed("baseline-2", Some("baseline-1"), 2, "exception-a1");
    let (bundle2, digest2) = a.export("bundle2.json", true);
    b.import(
        &bundle2,
        &digest2,
        Some(("baseline-1", "exception-b7")),
        None,
    );
    let status = run(&b.root, &["status", "--state-dir", arg(&b.state)], None);
    assert_eq!(status["desired_revision"], "baseline-2");
    assert_ne!(status["common_change_ref"], old_plan["common_change_ref"]);
    assert_eq!(
        status["changes"][0]["common_change_ref"],
        old_plan["common_change_ref"]
    );
    assert_eq!(status["changes"][0]["operations"][0]["status"], "APPLIED");
    assert_eq!(
        status["changes"][0]["operations"][0]["effect_verified"],
        false
    );
    b.apply("b1", Some("environment_stale_basis_or_authority"));
    assert_eq!(fs::read_to_string(&b.workflow).unwrap(), "Workflow v1\n");
}

#[test]
fn interrupted_tracking_never_exposes_an_applicable_plan_and_retry_completes() {
    for sidecar in ["change-links", "bundle-links"] {
        let temp = private_root();
        let root = temp.path().canonicalize().unwrap();
        let a = Device::new(&root, "device-a", "exception-a1");
        a.seed("baseline-1", None, 1, "exception-a1");
        let (bundle, digest) = a.export("bundle.json", true);
        let directory = a.state.join(sidecar);
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o500)).unwrap();
        fs::create_dir(a.state.join("plans")).unwrap();
        fs::set_permissions(a.state.join("plans"), fs::Permissions::from_mode(0o700)).unwrap();
        run(
            &a.root,
            &[
                "plan-bundle",
                "--state-dir",
                arg(&a.state),
                "--bundle",
                arg(&bundle),
                "--bundle-sha256",
                &digest,
                "--proposal-id",
                "a1",
            ],
            Some("environment_owner_or_link_contract"),
        );
        assert!(!a.state.join("plans/a1.json").exists());
        a.apply("a1", Some("environment_plan_missing"));
        assert_eq!(
            fs::read_to_string(&a.workflow).unwrap(),
            "Before device-a\n"
        );
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
        a.plan(&bundle, &digest, "a1");
        a.apply("a1", None);
        assert_eq!(fs::read_to_string(&a.workflow).unwrap(), "Workflow v1\n");
    }
}

#[test]
fn altered_common_change_link_fails_without_changing_targets() {
    let temp = private_root();
    let root = temp.path().canonicalize().unwrap();
    let a = Device::new(&root, "device-a", "exception-a1");
    a.seed("baseline-1", None, 1, "exception-a1");
    let (bundle, digest) = a.export("bundle.json", true);
    a.plan(&bundle, &digest, "a1");
    let path = a.state.join("change-links/a1.json");
    let mut link: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    link["common_change_ref"] = json!("0".repeat(64));
    write_json(&path, &link);
    run(
        &a.root,
        &["status", "--state-dir", arg(&a.state)],
        Some("environment_change_link_binding_mismatch"),
    );
    a.apply("a1", Some("environment_change_link_binding_mismatch"));
    assert_eq!(
        fs::read_to_string(&a.workflow).unwrap(),
        "Before device-a\n"
    );
}

#[test]
fn two_devices_round_trip_preserves_local_exceptions_user_edits_and_partial_rollback() {
    let temp = private_root();
    let root = temp.path().canonicalize().unwrap();
    let a = Device::new(&root, "device-a", "exception-a1");
    let b = Device::new(&root, "device-b", "exception-b7");
    a.seed("baseline-1", None, 1, "exception-a1");
    let (bundle1, digest1) = a.export("bundle1.json", true);
    let inspect = run(
        &b.root,
        &[
            "inspect-bundle",
            "--bundle",
            arg(&bundle1),
            "--bundle-sha256",
            &digest1,
            "--bindings",
            "bindings.json",
        ],
        None,
    );
    assert_eq!(inspect["transition"], "NEW_DEVICE");
    assert_eq!(inspect["exception_revision"], "exception-b7");
    assert_eq!(
        b.import(&bundle1, &digest1, None, None)["targets_written"],
        false
    );
    assert_eq!(
        fs::read_to_string(&b.workflow).unwrap(),
        "Before device-b\n"
    );
    b.import(
        &bundle1,
        &digest1,
        None,
        Some("environment_import_device_already_registered"),
    );
    let plan = b.plan(&bundle1, &digest1, "b1");
    let applied = b.apply("b1", None);
    assert_eq!(applied["status"], "APPLIED");
    assert_eq!(applied["plan_sha256"], plan["plan_sha256"]);
    assert_eq!(applied["native_activation"], "UNVERIFIED");
    assert_eq!(fs::read_to_string(&b.workflow).unwrap(), "Workflow v1\n");
    assert!(
        fs::read_to_string(&b.agents)
            .unwrap()
            .contains("Outside device device-b private instructions")
    );
    let (roundtrip, roundtrip_digest) = b.export("roundtrip.json", false);
    assert_eq!(roundtrip_digest, digest1);
    assert_eq!(fs::read(&roundtrip).unwrap(), fs::read(&bundle1).unwrap());
    a.import(
        &roundtrip,
        &roundtrip_digest,
        Some(("baseline-1", "exception-a1")),
        None,
    );
    let rollback = b.rollback(applied["operation_id"].as_str().unwrap());
    assert_eq!(rollback["status"], "ROLLED_BACK");
    assert_eq!(fs::read_to_string(&b.agents).unwrap(), b.before_agents);
    assert!(!b.reference.exists());
    a.plan(&bundle1, &digest1, "a1");
    a.apply("a1", None);
    a.seed("baseline-2", Some("baseline-1"), 2, "exception-a2");
    let (bundle2, digest2) = a.export("bundle2.json", true);
    fs::write(&b.workflow, "User edit before import\n").unwrap();
    let imported = b.import(
        &bundle2,
        &digest2,
        Some(("baseline-1", "exception-b7")),
        None,
    );
    assert_eq!(imported["exception_revision"], "exception-b7");
    assert_eq!(
        fs::read_to_string(&b.workflow).unwrap(),
        "User edit before import\n"
    );
    b.import(
        &bundle2,
        &digest2,
        Some(("baseline-1", "exception-b7")),
        Some("environment_stale_import_basis"),
    );
    b.plan(&bundle2, &digest2, "b2-stale");
    fs::write(&b.workflow, "User edit after plan\n").unwrap();
    b.apply("b2-stale", Some("environment_target_changed_since_plan"));
    assert_eq!(
        fs::read_to_string(&b.workflow).unwrap(),
        "User edit after plan\n"
    );
    b.plan(&bundle2, &digest2, "b2");
    let applied = b.apply("b2", None);
    fs::write(&b.workflow, "User edit after apply\n").unwrap();
    let partial = b.rollback(applied["operation_id"].as_str().unwrap());
    assert_eq!(partial["status"], "PARTIAL");
    assert_eq!(
        fs::read_to_string(&b.workflow).unwrap(),
        "User edit after apply\n"
    );
    assert_eq!(fs::read_to_string(&b.reference).unwrap(), "Reference v2\n");
    assert_eq!(fs::read_to_string(&b.agents).unwrap(), b.before_agents);
    let (roundtrip2, roundtrip2_digest) = b.export("roundtrip2.json", false);
    assert_eq!(roundtrip2_digest, digest2);
    assert_eq!(fs::read(&roundtrip2).unwrap(), fs::read(&bundle2).unwrap());
    b.import(
        &bundle1,
        &digest1,
        Some(("baseline-2", "exception-b7")),
        Some("environment_stale_bundle_revision"),
    );
    let c = Device::new(&root, "device-c", "exception-c1");
    assert_eq!(
        c.import(&bundle2, &digest2, None, None)["desired_revision"],
        "baseline-2"
    );
}

#[test]
fn bundle_is_allowlisted_digest_bound_and_never_clobbers_existing_output() {
    let temp = private_root();
    let root = temp.path().canonicalize().unwrap();
    let a = Device::new(&root, "device-a", "exception-a1");
    a.seed("baseline-1", None, 1, "exception-a1");
    let (bundle, digest) = a.export("bundle.json", true);
    let raw = fs::read_to_string(&bundle).unwrap();
    for forbidden in [
        arg(&root),
        "Outside device",
        "exception-a1",
        "device-a",
        "canonical_path",
        "bindings_sha256",
        "registry_sha256",
        "before_content",
        "backup_ref",
        "auth.json",
        "config.toml",
    ] {
        assert!(!raw.contains(forbidden), "exported {forbidden}");
    }
    assert_eq!(
        fs::metadata(&bundle).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(a.export("bundle.json", true).1, digest);
    let b = Device::new(&root, "device-b", "exception-b1");
    fs::write(a.root.join("occupied.json"), "Preserve private original").unwrap();
    fs::set_permissions(
        a.root.join("occupied.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    run(
        &a.root,
        &[
            "export",
            "--state-dir",
            "PRIVATE_STATE",
            "--proposal",
            "proposal.json",
            "--output",
            "occupied.json",
        ],
        Some("environment_artifact_id_conflict"),
    );
    assert_eq!(
        fs::read_to_string(a.root.join("occupied.json")).unwrap(),
        "Preserve private original"
    );
    let mut value: Value = serde_json::from_str(&raw).unwrap();
    value["payload"]["contents"][0]["content"] = json!("Tampered\n");
    let tampered = a.root.join("tampered.json");
    let tampered_digest = write_json(&tampered, &value);
    b.import(
        &tampered,
        &digest,
        None,
        Some("environment_bundle_digest_mismatch"),
    );
    b.import(
        &tampered,
        &tampered_digest,
        None,
        Some("environment_bundle_payload_digest_mismatch"),
    );
    let mut payload = serde_json::to_vec_pretty(&value["payload"]).unwrap();
    payload.push(b'\n');
    value["payload_sha256"] = json!(hash(&payload));
    let tampered_digest = write_json(&tampered, &value);
    b.import(
        &tampered,
        &tampered_digest,
        None,
        Some("environment_bundle_content_digest_mismatch"),
    );
    value = serde_json::from_str(&raw).unwrap();
    value["payload"]["lineage"]["revisions"][0]["baseline"]["bindings"] = json!({"path":"/secret"});
    let tampered_digest = write_json(&tampered, &value);
    b.import(
        &tampered,
        &tampered_digest,
        None,
        Some("environment_invalid_document"),
    );
    assert!(!b.state.exists());
    assert_eq!(
        fs::read_to_string(&b.workflow).unwrap(),
        "Before device-b\n"
    );
}

#[test]
fn bundle_and_target_paths_reject_links_traversal_and_mismatched_device_bindings() {
    let temp = private_root();
    let root = temp.path().canonicalize().unwrap();
    let a = Device::new(&root, "device-a", "exception-a1");
    a.seed("baseline-1", None, 1, "exception-a1");
    let (bundle, digest) = a.export("bundle.json", true);
    let b = Device::new(&root, "device-b", "exception-b1");
    let linked = b.root.join("linked.json");
    symlink(&bundle, &linked).unwrap();
    b.import(
        &linked,
        &digest,
        None,
        Some("environment_leaf_link_or_open"),
    );
    fs::remove_file(&linked).unwrap();
    fs::hard_link(&bundle, &linked).unwrap();
    b.import(
        &linked,
        &digest,
        None,
        Some("environment_owner_or_link_contract"),
    );
    fs::remove_file(&linked).unwrap();
    let alias = b.root.join("transport");
    symlink(&a.root, &alias).unwrap();
    b.import(
        &alias.join("bundle.json"),
        &digest,
        None,
        Some("environment_directory_link_or_missing"),
    );
    fs::remove_file(&alias).unwrap();
    fs::create_dir(&alias).unwrap();
    b.import(
        &alias.join("../linked.json"),
        &digest,
        None,
        Some("environment_invalid_path"),
    );
    let mut bindings: Value = serde_json::from_slice(&fs::read(&b.bindings).unwrap()).unwrap();
    let original = bindings.clone();
    bindings["targets"][0]["relative_path"] = json!("../skills/SKILL.md");
    write_json(&b.bindings, &bindings);
    b.import(
        &bundle,
        &digest,
        None,
        Some("environment_invalid_relative_path"),
    );
    write_json(&b.bindings, &original);
    fs::rename(b.root.join("skills"), b.root.join("real-skills")).unwrap();
    symlink(b.root.join("real-skills"), b.root.join("skills")).unwrap();
    b.import(
        &bundle,
        &digest,
        None,
        Some("environment_parent_link_or_missing"),
    );
    fs::remove_file(b.root.join("skills")).unwrap();
    fs::rename(b.root.join("real-skills"), b.root.join("skills")).unwrap();
    b.import(&bundle, &digest, None, None);
    let mut bindings = original;
    bindings["device_id"] = json!("different-device");
    write_json(&b.bindings, &bindings);
    b.import(
        &bundle,
        &digest,
        Some(("baseline-1", "exception-b1")),
        Some("environment_import_device_binding_mismatch"),
    );
}

#[test]
fn divergent_lineage_and_host_revision_conflicts_require_replanning() {
    let temp = private_root();
    let root = temp.path().canonicalize().unwrap();
    let a = Device::new(&root, "device-a", "exception-a1");
    let b = Device::new(&root, "device-b", "exception-b1");
    a.seed("baseline-1", None, 1, "exception-a1");
    let (initial, initial_digest) = a.export("initial.json", true);
    b.import(&initial, &initial_digest, None, None);
    a.seed("baseline-2", Some("baseline-1"), 2, "exception-a2");
    let (accepted, accepted_digest) = a.export("accepted.json", true);
    b.plan(&initial, &initial_digest, "old-plan");
    b.import(
        &accepted,
        &accepted_digest,
        Some(("baseline-1", "exception-b1")),
        None,
    );
    b.apply("old-plan", Some("environment_stale_basis_or_authority"));
    b.import(
        &accepted,
        &accepted_digest,
        Some(("baseline-2", "wrong-exception")),
        Some("environment_stale_import_basis"),
    );
    b.seed("baseline-other", Some("baseline-2"), 3, "exception-b2");
    let (other, other_digest) = b.export("other.json", true);
    a.seed("baseline-3", Some("baseline-2"), 3, "exception-a2");
    a.import(
        &other,
        &other_digest,
        Some(("baseline-3", "exception-a2")),
        Some("environment_bundle_lineage_conflict"),
    );
    assert_eq!(
        run(&a.root, &["inspect", "--state-dir", "PRIVATE_STATE"], None)["desired_revision"],
        "baseline-3"
    );
    assert_eq!(
        fs::read_to_string(&a.workflow).unwrap(),
        "Before device-a\n"
    );
}
