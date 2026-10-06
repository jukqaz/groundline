use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use tempfile::{TempDir, tempdir};

struct Fixture {
    root: TempDir,
    state: PathBuf,
    skill_root: PathBuf,
    baseline: Baseline,
    bindings: Bindings,
}
impl Fixture {
    fn new(before: Option<&str>, after: &str) -> Self {
        let root = tempdir().unwrap();
        let skill_root = root.path().join("skills");
        fs::create_dir(&skill_root).unwrap();
        if let Some(before) = before {
            fs::write(skill_root.join("SKILL.md"), before).unwrap();
        }
        let baseline = Baseline {
            kind: "groundline-environment-baseline".into(),
            schema: 1,
            revision: "rev-1".into(),
            parent_revision: None,
            source_revision: "official-1".into(),
            authority_revision: "authority-1".into(),
            exception_revision: "device-1".into(),
            authority_ref: "user-request-1".into(),
            managed_targets: vec![ManagedTarget {
                target_id: "skill".into(),
                kind: TargetKind::SkillFile,
                desired_sha256: hash(after.as_bytes()),
                dependencies: vec![],
                managed_block: None,
            }],
        };
        let bindings = Bindings {
            kind: "groundline-environment-device-bindings".into(),
            schema: 1,
            revision: "device-1".into(),
            device_id: "test-mac".into(),
            roots: vec![RootInput {
                root_id: "owner".into(),
                path: skill_root.clone(),
                aliases: vec![],
            }],
            targets: vec![TargetInput {
                target_id: "skill".into(),
                root_id: "owner".into(),
                relative_path: "SKILL.md".into(),
            }],
        };
        let state = root.path().join("environment-state");
        Self {
            root,
            state,
            skill_root,
            baseline,
            bindings,
        }
    }
    fn input<T: Serialize>(&self, name: &str, value: &T) -> PathBuf {
        let path = self.root.path().join(name);
        fs::write(&path, bytes(value).unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        path
    }
    fn register(&self) -> Result<Value, ContractError> {
        register(
            &self.state,
            &self.input("baseline.json", &self.baseline),
            &self.input("bindings.json", &self.bindings),
        )
    }
    fn propose(&self, id: &str, changes: Vec<(&str, &str)>) -> Result<Value, ContractError> {
        let p = Proposal {
            kind: "groundline-environment-proposal".into(),
            schema: 1,
            proposal_id: id.into(),
            basis_revision: self.baseline.revision.clone(),
            source_revision: self.baseline.source_revision.clone(),
            authority_ref: self.baseline.authority_ref.clone(),
            changes: changes
                .into_iter()
                .map(|(target_id, content)| Change {
                    target_id: target_id.into(),
                    content: content.into(),
                })
                .collect(),
        };
        plan(&self.state, &self.input("proposal.json", &p))
    }
    fn add(
        &mut self,
        target_id: &str,
        path: &str,
        before: Option<&str>,
        after: &str,
        deps: &[&str],
    ) {
        if let Some(before) = before {
            fs::write(self.skill_root.join(path), before).unwrap();
        }
        self.baseline.managed_targets.push(ManagedTarget {
            target_id: target_id.into(),
            kind: TargetKind::SkillFile,
            desired_sha256: hash(after.as_bytes()),
            dependencies: deps.iter().map(|d| (*d).into()).collect(),
            managed_block: None,
        });
        self.bindings.targets.push(TargetInput {
            target_id: target_id.into(),
            root_id: "owner".into(),
            relative_path: path.into(),
        });
    }
}

#[test]
fn no_op_has_no_operation_or_backup_and_reports_active_unknown() {
    let f = Fixture::new(Some("same\n"), "same\n");
    f.register().unwrap();
    let p = f.propose("same", vec![("skill", "same\n")]).unwrap();
    let raw = fs::read(f.state.join("plans/same.json")).unwrap();
    assert_eq!(p["plan_sha256"], hash(&raw));
    let result = apply(&f.state, "same").unwrap();
    assert_eq!(result["status"], "NO_CHANGE");
    assert!(result["operation_id"].is_null());
    assert!(!f.state.join("operations").exists());
    assert!(!f.state.join("backups").exists());
    let observed = inspect(&f.state).unwrap();
    assert_eq!(observed["disk_revision"], "rev-1");
    assert!(observed["active_revision"].is_null());
    assert_eq!(observed["native_activation"], "UNVERIFIED");
}

#[test]
fn successful_apply_and_repeat_then_rollback_keep_private_evidence() {
    let f = Fixture::new(Some("before\n"), "after\n");
    f.register().unwrap();
    f.propose("edit", vec![("skill", "after\n")]).unwrap();
    let applied = apply(&f.state, "edit").unwrap();
    assert_eq!(applied["status"], "APPLIED");
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "after\n"
    );
    let store = Store::open(&f.state, false).unwrap();
    let op = store.operation("op-edit").unwrap();
    validate_operation(&serde_json::to_value(&op).unwrap()).unwrap();
    assert!(op.entries[0].after_leaf.is_some());
    assert_eq!(
        apply(&f.state, "edit").unwrap()["status"],
        "ALREADY_RECORDED"
    );
    let rolled = rollback(&f.state, "op-edit").unwrap();
    assert_eq!(rolled["status"], "ROLLED_BACK");
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "before\n"
    );
    assert_eq!(
        fs::metadata(f.state.join("head.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(&f.state).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(
        !applied
            .to_string()
            .contains(&f.root.path().display().to_string())
    );
    assert!(!applied.to_string().contains("before\\n"));
}

#[test]
fn changed_original_prevents_apply_and_changed_post_file_prevents_rollback() {
    let f = Fixture::new(Some("before"), "after");
    f.register().unwrap();
    f.propose("edit", vec![("skill", "after")]).unwrap();
    fs::write(f.skill_root.join("SKILL.md"), "user").unwrap();
    assert!(
        apply(&f.state, "edit")
            .unwrap_err()
            .0
            .contains("target_changed")
    );
    assert!(!f.state.join("backups").exists());
    fs::write(f.skill_root.join("SKILL.md"), "before").unwrap();
    apply(&f.state, "edit").unwrap();
    fs::write(f.skill_root.join("SKILL.md"), "user edit").unwrap();
    let rolled = rollback(&f.state, "op-edit").unwrap();
    assert_eq!(rolled["status"], "PARTIAL");
    assert_eq!(rolled["entries"][0]["status"], "CONFLICT");
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "user edit"
    );
    assert!(rolled["rollback_operation_id"].is_null());
}

#[test]
fn baseline_is_parent_cas_and_stale_plan_cannot_apply_or_rollback() {
    let mut f = Fixture::new(Some("before"), "after");
    f.register().unwrap();
    f.propose("edit", vec![("skill", "after")]).unwrap();
    f.baseline.revision = "rev-2".into();
    assert!(f.register().unwrap_err().0.contains("stale_parent"));
    f.baseline.parent_revision = Some("rev-1".into());
    f.register().unwrap();
    assert!(
        apply(&f.state, "edit")
            .unwrap_err()
            .0
            .contains("stale_basis")
    );
    f.baseline.revision = "rev-3".into();
    f.baseline.parent_revision = Some("rev-1".into());
    assert!(f.register().unwrap_err().0.contains("stale_parent"));
    assert_eq!(inspect(&f.state).unwrap()["desired_revision"], "rev-2");
}

#[test]
fn authority_and_exception_changes_reject_the_saved_plan() {
    for field in ["authority", "exception"] {
        let mut f = Fixture::new(Some("before"), "after");
        f.register().unwrap();
        f.propose("edit", vec![("skill", "after")]).unwrap();
        f.baseline.revision = "rev-2".into();
        f.baseline.parent_revision = Some("rev-1".into());
        if field == "authority" {
            f.baseline.authority_revision = "authority-2".into();
        } else {
            f.baseline.exception_revision = "device-2".into();
            f.bindings.revision = "device-2".into();
        }
        f.register().unwrap();
        assert!(
            apply(&f.state, "edit")
                .unwrap_err()
                .0
                .contains("stale_basis")
        );
    }
}

#[test]
fn rejects_leaf_symlink_hardlink_and_parent_symlink() {
    for kind in ["leaf", "hardlink", "parent"] {
        let mut f = Fixture::new(Some("before"), "after");
        if kind == "parent" {
            let outside = f.root.path().join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("SKILL.md"), "before").unwrap();
            symlink(&outside, f.skill_root.join("alias")).unwrap();
            f.bindings.targets[0].relative_path = "alias/SKILL.md".into();
        } else {
            let original = f.skill_root.join("SKILL.md");
            let alias = f.root.path().join("alias");
            if kind == "leaf" {
                fs::rename(&original, &alias).unwrap();
                symlink(&alias, &original).unwrap();
            } else {
                fs::hard_link(&original, &alias).unwrap();
            }
        }
        assert!(f.register().is_err(), "{kind}");
    }
}

#[test]
fn parent_rename_recreation_same_bytes_and_late_hardlink_are_conflicts() {
    for kind in ["parent", "hardlink"] {
        let f = Fixture::new(Some("before"), "after");
        f.register().unwrap();
        f.propose("edit", vec![("skill", "after")]).unwrap();
        if kind == "parent" {
            fs::rename(&f.skill_root, f.root.path().join("old-skills")).unwrap();
            fs::create_dir(&f.skill_root).unwrap();
            fs::write(f.skill_root.join("SKILL.md"), "before").unwrap();
        } else {
            fs::hard_link(
                f.skill_root.join("SKILL.md"),
                f.root.path().join("extra-link"),
            )
            .unwrap();
        }
        assert!(apply(&f.state, "edit").is_err());
        assert_eq!(
            fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
            "before"
        );
    }
}

#[test]
fn aliases_are_bound_and_duplicate_parent_leaf_is_rejected() {
    let mut f = Fixture::new(Some("before"), "after");
    let alias = f.root.path().join("skill-alias");
    symlink(&f.skill_root, &alias).unwrap();
    f.bindings.roots[0].aliases.push(alias.clone());
    f.register().unwrap();
    f.propose("edit", vec![("skill", "after")]).unwrap();
    fs::remove_file(&alias).unwrap();
    let other = f.root.path().join("other-skills");
    fs::create_dir(&other).unwrap();
    symlink(&other, &alias).unwrap();
    assert!(apply(&f.state, "edit").is_err());
    let mut duplicate = Fixture::new(Some("before"), "after");
    duplicate.add("alias", "other.md", None, "after", &[]);
    duplicate.bindings.targets[1].relative_path = "SKILL.md".into();
    assert!(
        duplicate
            .register()
            .unwrap_err()
            .0
            .contains("duplicate_parent_leaf")
    );
}

#[test]
fn new_leaf_uses_no_clobber_and_rollback_proves_the_same_leaf() {
    let f = Fixture::new(None, "new");
    f.register().unwrap();
    f.propose("new", vec![("skill", "new")]).unwrap();
    fs::write(f.skill_root.join("SKILL.md"), "other writer").unwrap();
    assert!(apply(&f.state, "new").is_err());
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "other writer"
    );
    fs::remove_file(f.skill_root.join("SKILL.md")).unwrap();
    apply(&f.state, "new").unwrap();
    let op = Store::open(&f.state, false)
        .unwrap()
        .operation("op-new")
        .unwrap();
    let _held_original = fs::File::open(f.skill_root.join("SKILL.md")).unwrap();
    fs::remove_file(f.skill_root.join("SKILL.md")).unwrap();
    fs::write(f.skill_root.join("SKILL.md"), "new").unwrap();
    let actual = files::canonical_dir(&f.skill_root, false).unwrap();
    let r = files::read_at(&actual.file, Path::new("SKILL.md"), false, files::MAX_BYTES)
        .unwrap()
        .unwrap();
    assert_ne!(Some(r.binding), op.entries[0].after_leaf);
    assert_eq!(
        rollback(&f.state, "op-new").unwrap()["entries"][0]["status"],
        "CONFLICT"
    );
    assert!(f.skill_root.join("SKILL.md").exists());
}

#[test]
fn direct_descriptor_candidate_no_replace_rejects_concurrent_new_leaf() {
    let f = Fixture::new(None, "new");
    let parent = files::canonical_dir(&f.skill_root, false).unwrap();
    let mut candidate = Candidate::new(&parent, b"new", 0o600).unwrap();
    fs::write(f.skill_root.join("SKILL.md"), "concurrent").unwrap();
    assert!(candidate.commit(&parent, "SKILL.md", false).is_err());
    assert!(!candidate.renamed());
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "concurrent"
    );
}

#[test]
fn interrupted_prepared_operation_recovers_before_after_or_conflict() {
    for actual in ["before", "after", "user edit"] {
        let f = Fixture::new(Some("before"), "after");
        f.register().unwrap();
        f.propose("edit", vec![("skill", "after")]).unwrap();
        let store = Store::open(&f.state, false).unwrap();
        let (plan, digest) = store.plan("edit").unwrap();
        let mut op = operation_from_plan(&plan, &digest);
        let name = "op-edit-skill.backup";
        store
            .artifacts("backups", true)
            .unwrap()
            .write(name, b"before", false)
            .unwrap();
        op.entries[0].backup_ref = Some(name.into());
        store.save_operation(&op, false).unwrap();
        fs::write(f.skill_root.join("SKILL.md"), actual).unwrap();
        let result = recover(&f.state, "op-edit").unwrap();
        let status = match actual {
            "before" => "DISK_OBSERVED_BEFORE",
            "after" => "DISK_OBSERVED_AFTER_ONLY",
            _ => "CONFLICT",
        };
        assert_eq!(result["entries"][0]["status"], status);
        assert_ne!(result["status"], "APPLIED");
        assert_eq!(
            fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
            actual
        );
        assert_eq!(result["native_activation"], "UNVERIFIED");
    }
}

#[test]
fn partial_rollback_keeps_transitive_dependencies_of_an_edited_consumer() {
    let mut f = Fixture::new(Some("old skill"), "new skill");
    f.baseline.managed_targets[0].dependencies = vec!["reference".into()];
    f.add(
        "reference",
        "reference.md",
        Some("old ref"),
        "new ref",
        &["nested"],
    );
    f.add("nested", "nested.md", None, "new nested", &[]);
    f.add(
        "independent",
        "independent.md",
        Some("old independent"),
        "new independent",
        &[],
    );
    f.register().unwrap();
    f.propose(
        "bundle",
        vec![
            ("skill", "new skill"),
            ("reference", "new ref"),
            ("nested", "new nested"),
            ("independent", "new independent"),
        ],
    )
    .unwrap();
    assert_eq!(apply(&f.state, "bundle").unwrap()["status"], "APPLIED");
    fs::write(
        f.skill_root.join("SKILL.md"),
        "user skill still consumes reference",
    )
    .unwrap();
    let result = rollback(&f.state, "op-bundle").unwrap();
    assert_eq!(result["status"], "PARTIAL");
    assert_eq!(
        fs::read_to_string(f.skill_root.join("reference.md")).unwrap(),
        "new ref"
    );
    assert_eq!(
        fs::read_to_string(f.skill_root.join("nested.md")).unwrap(),
        "new nested"
    );
    assert_eq!(
        fs::read_to_string(f.skill_root.join("independent.md")).unwrap(),
        "old independent"
    );
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "user skill still consumes reference"
    );
    let statuses: BTreeMap<_, _> = result["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["target_id"].as_str().unwrap(),
                e["status"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(statuses["reference"], "RETAINED_DEPENDENCY");
    assert_eq!(statuses["nested"], "RETAINED_DEPENDENCY");
}

#[test]
fn shared_dependency_is_retained_for_an_unmodified_consumer_outside_operation() {
    let mut f = Fixture::new(Some("same skill"), "same skill");
    f.baseline.managed_targets[0].dependencies = vec!["reference".into()];
    f.add("reference", "reference.md", Some("old ref"), "new ref", &[]);
    f.register().unwrap();
    f.propose("ref", vec![("reference", "new ref")]).unwrap();
    apply(&f.state, "ref").unwrap();
    let result = rollback(&f.state, "op-ref").unwrap();
    assert_eq!(result["entries"][0]["status"], "RETAINED_DEPENDENCY");
    assert_eq!(
        fs::read_to_string(f.skill_root.join("reference.md")).unwrap(),
        "new ref"
    );
    assert!(result["rollback_operation_id"].is_null());
}

#[test]
fn agents_managed_block_preserves_all_other_bytes() {
    let mut f = Fixture::new(None, "ignored");
    let body = "Managed guidance.\n";
    let before =
        "Owner first line.\n<!-- managed:start -->\nOld.\n<!-- managed:end -->\nOwner last line.\n";
    let agents = f.skill_root.join("AGENTS.md");
    fs::write(&agents, before).unwrap();
    f.baseline.managed_targets[0].kind = TargetKind::AgentsBlock;
    f.baseline.managed_targets[0].desired_sha256 = hash(body.as_bytes());
    f.baseline.managed_targets[0].managed_block = Some(Block {
        start_marker: "<!-- managed:start -->".into(),
        end_marker: "<!-- managed:end -->".into(),
    });
    f.bindings.targets[0].relative_path = "AGENTS.md".into();
    f.register().unwrap();
    f.propose("agents", vec![("skill", body)]).unwrap();
    apply(&f.state, "agents").unwrap();
    assert_eq!(
        fs::read_to_string(&agents).unwrap(),
        before.replace("Old.\n", body)
    );
    rollback(&f.state, "op-agents").unwrap();
    assert_eq!(fs::read_to_string(&agents).unwrap(), before);
}

#[test]
fn unsafe_state_links_and_unknown_or_protected_input_fail_closed() {
    let f = Fixture::new(Some("before"), "after");
    let mut baseline = serde_json::to_value(&f.baseline).unwrap();
    baseline["secret"] = json!("unrecognized");
    let path = f.input("unknown.json", &baseline);
    assert!(register(&f.state, &path, &f.input("bindings.json", &f.bindings)).is_err());
    let mut protected = Fixture::new(Some("before"), "after");
    protected.bindings.targets[0].relative_path = "config.toml".into();
    fs::write(protected.skill_root.join("config.toml"), "before").unwrap();
    assert!(
        protected
            .register()
            .unwrap_err()
            .0
            .contains("protected_target")
    );
    f.register().unwrap();
    let head = f.state.join("head.json");
    fs::hard_link(&head, f.root.path().join("head-copy")).unwrap();
    assert!(inspect(&f.state).is_err());
}

#[test]
fn prepare_failures_leave_target_unchanged_and_preserve_completed_backup() {
    for name in ["op-edit-skill.backup", "op-edit.json"] {
        let f = Fixture::new(Some("before"), "after");
        f.register().unwrap();
        f.propose("edit", vec![("skill", "after")]).unwrap();
        files::inject(files::FaultPoint::StateWrite, name, 0);
        assert!(apply(&f.state, "edit").is_err());
        assert_eq!(
            fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
            "before"
        );
        if name == "op-edit.json" {
            assert_eq!(
                fs::read_to_string(f.state.join("backups/op-edit-skill.backup")).unwrap(),
                "before"
            );
        }
    }
}

#[test]
fn candidate_sync_failure_is_unapplied_but_journal_is_recoverable() {
    let f = Fixture::new(Some("before"), "after");
    f.register().unwrap();
    f.propose("edit", vec![("skill", "after")]).unwrap();
    files::inject(files::FaultPoint::CandidateSync, "", 0);
    let result = apply(&f.state, "edit").unwrap();
    assert_eq!(result["status"], "PARTIAL");
    assert_eq!(result["mutation_performed"], false);
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "before"
    );
    assert_eq!(
        recover(&f.state, "op-edit").unwrap()["entries"][0]["status"],
        "DISK_OBSERVED_BEFORE"
    );
}

#[test]
fn target_sync_or_receipt_failure_after_rename_is_partial_and_backup_survives() {
    for point in [
        files::FaultPoint::TargetDirectorySync,
        files::FaultPoint::StateWrite,
    ] {
        let f = Fixture::new(Some("before"), "after");
        f.register().unwrap();
        f.propose("edit", vec![("skill", "after")]).unwrap();
        if point == files::FaultPoint::StateWrite {
            files::inject(point, "op-edit.json", 1);
        } else {
            files::inject(point, "SKILL.md", 0);
        }
        let result = apply(&f.state, "edit").unwrap();
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["mutation_performed"], true);
        assert_eq!(
            fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
            "after"
        );
        assert_eq!(
            fs::read_to_string(f.state.join("backups/op-edit-skill.backup")).unwrap(),
            "before"
        );
        let recovered = recover(&f.state, "op-edit").unwrap();
        assert_eq!(
            recovered["entries"][0]["status"],
            "DISK_OBSERVED_AFTER_ONLY"
        );
        assert_eq!(recovered["status"], "PARTIAL");
        assert_eq!(
            rollback(&f.state, "op-edit").unwrap()["status"],
            "ROLLED_BACK"
        );
        assert_eq!(
            fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
            "before"
        );
    }
}

#[test]
fn stale_desired_revision_also_blocks_a_previously_applied_rollback() {
    let mut f = Fixture::new(Some("before"), "after");
    f.register().unwrap();
    f.propose("edit", vec![("skill", "after")]).unwrap();
    apply(&f.state, "edit").unwrap();
    f.baseline.revision = "rev-2".into();
    f.baseline.parent_revision = Some("rev-1".into());
    f.baseline.managed_targets[0].desired_sha256 = hash(b"new desired");
    f.register().unwrap();
    assert!(
        rollback(&f.state, "op-edit")
            .unwrap_err()
            .0
            .contains("stale_basis")
    );
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "after"
    );
}

#[test]
fn unchanged_consumer_in_the_same_plan_retains_the_shared_dependency() {
    let mut f = Fixture::new(Some("same skill"), "same skill");
    f.baseline.managed_targets[0].dependencies = vec!["reference".into()];
    f.add("reference", "reference.md", Some("old ref"), "new ref", &[]);
    f.register().unwrap();
    f.propose(
        "ref",
        vec![("skill", "same skill"), ("reference", "new ref")],
    )
    .unwrap();
    apply(&f.state, "ref").unwrap();
    let result = rollback(&f.state, "op-ref").unwrap();
    let statuses: BTreeMap<_, _> = result["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["target_id"].as_str().unwrap(),
                e["status"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(statuses["reference"], "RETAINED_DEPENDENCY");
    assert_eq!(
        fs::read_to_string(f.skill_root.join("reference.md")).unwrap(),
        "new ref"
    );
}

#[test]
fn altered_operation_content_cannot_authorize_a_different_rollback() {
    let f = Fixture::new(Some("before"), "after");
    f.register().unwrap();
    f.propose("edit", vec![("skill", "after")]).unwrap();
    apply(&f.state, "edit").unwrap();
    let store = Store::open(&f.state, false).unwrap();
    let mut op = store.operation("op-edit").unwrap();
    op.entries[0].before_sha256 = Some(hash(b"different private backup"));
    let backup_name = op.entries[0].backup_ref.clone().unwrap();
    store
        .artifacts("backups", false)
        .unwrap()
        .write(&backup_name, b"different private backup", true)
        .unwrap();
    store.save_operation(&op, true).unwrap();
    assert!(
        rollback(&f.state, "op-edit")
            .unwrap_err()
            .0
            .contains("operation_plan_entry_mismatch")
    );
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "after"
    );
}

#[test]
fn a_new_external_consumer_created_during_rollback_retains_its_dependency_closure() {
    let mut f = Fixture::new(Some("old skill"), "new skill");
    f.baseline.managed_targets[0].dependencies = vec!["reference".into()];
    f.add(
        "reference",
        "reference.md",
        Some("old ref"),
        "new ref",
        &["nested"],
    );
    f.add("nested", "nested.md", None, "new nested", &[]);
    f.add("external", "late.md", None, "late consumer", &["reference"]);
    f.register().unwrap();
    f.propose(
        "bundle",
        vec![
            ("skill", "new skill"),
            ("reference", "new ref"),
            ("nested", "new nested"),
        ],
    )
    .unwrap();
    apply(&f.state, "bundle").unwrap();
    let late = f.skill_root.join("late.md");
    AFTER_ROLLBACK_PREPARE.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            fs::write(late, "late consumer").unwrap();
        }))
    });
    let result = rollback(&f.state, "op-bundle").unwrap();
    assert_eq!(result["status"], "PARTIAL");
    assert_eq!(
        fs::read_to_string(f.skill_root.join("SKILL.md")).unwrap(),
        "old skill"
    );
    assert_eq!(
        fs::read_to_string(f.skill_root.join("reference.md")).unwrap(),
        "new ref"
    );
    assert_eq!(
        fs::read_to_string(f.skill_root.join("nested.md")).unwrap(),
        "new nested"
    );
    assert_eq!(
        fs::read_to_string(f.skill_root.join("late.md")).unwrap(),
        "late consumer"
    );
}

#[test]
fn skill_file_cannot_claim_agents_using_case_insensitive_names() {
    for name in ["agents.md", "AgEnTs.Md"] {
        let mut f = Fixture::new(None, "after");
        f.bindings.targets[0].relative_path = name.into();
        fs::write(f.skill_root.join(name), "before").unwrap();
        assert!(f.register().unwrap_err().0.contains("unsupported_target"));
    }
}
