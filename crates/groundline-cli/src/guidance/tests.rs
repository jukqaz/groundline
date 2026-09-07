use super::*;
use tempfile::{TempDir, tempdir};

fn fixture() -> (TempDir, PathBuf, PathBuf, PathBuf) {
    let temp = tempdir().unwrap();
    let root = temp.path().join("skills");
    fs::create_dir(&root).unwrap();
    add_skill(&root, "example", "example");
    let profile = temp.path().join("profile.json");
    save(
        &profile,
        &json!({"kind":"groundline-guidance-profile","schema":1,"roots":{"personal":root},"sources":{}}),
    );
    let baseline = temp.path().join("baseline.json");
    snapshot(&profile, &baseline, false).unwrap();
    (temp, root, profile, baseline)
}
fn save(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn document(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn add_skill(root: &Path, directory: &str, name: &str) {
    let dir = root.join(directory);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!(
            "---\nname: {name}\ndescription: |\n  A focused task.\n---\nprivate-instructions\n"
        ),
    )
    .unwrap();
}
fn source(profile: &Path, checkout: &Path) {
    let mut value = document(profile);
    value["sources"] = json!({"personal/example":{"checkout":checkout,"repository":"https://github.com/example/skills","declared_revision":"a".repeat(40)}});
    save(profile, &value);
}

#[test]
fn inventory_does_not_claim_a_baseline_or_upstream_freshness() {
    let (_temp, _root, profile, baseline) = fixture();
    assert_eq!(
        audit(&profile, None, false).unwrap()["status"],
        "NOT_BASELINED"
    );
    let matched = audit(&profile, Some(&baseline), false).unwrap();
    assert_eq!(matched["status"], "PASS");
    assert_eq!(matched["upstream_unverified_count"], 1);
    assert_eq!(matched["latest_upstream_verified"], false);
    assert_eq!(matched["scripts_executed"], false);
    assert_eq!(matched["mutation_performed"], false);
    assert!(!matched.to_string().contains("private-instructions"));
}
#[test]
fn discovers_new_removed_and_renamed_skills_on_every_audit() {
    let (_temp, root, profile, baseline) = fixture();
    add_skill(&root, "new-skill", "new-skill");
    let added = audit(&profile, Some(&baseline), false).unwrap();
    assert_eq!(added["installed_counts"]["added"], 1);
    assert_eq!(added["skill_count"], 2);
    fs::remove_file(root.join("example/SKILL.md")).unwrap();
    let removed = audit(&profile, Some(&baseline), false).unwrap();
    assert_eq!(removed["installed_counts"]["removed"], 1);
    assert_eq!(removed["status"], "REVIEW_REQUIRED");
    add_skill(&root, "example", "renamed");
    assert_eq!(
        audit(&profile, Some(&baseline), false).unwrap()["installed_counts"]["modified"],
        1
    );
}
#[test]
fn file_deltas_and_removal_of_all_skills_are_visible() {
    let (_temp, root, profile, baseline) = fixture();
    fs::write(root.join("example/helper.sh"), "exit 99\n").unwrap();
    assert_eq!(
        audit(&profile, Some(&baseline), false).unwrap()["skills"][0]["installed_delta"]["added"],
        1
    );
    let next = baseline.with_file_name("next.json");
    snapshot(&profile, &next, false).unwrap();
    fs::remove_file(root.join("example/helper.sh")).unwrap();
    assert_eq!(
        audit(&profile, Some(&next), false).unwrap()["skills"][0]["installed_delta"]["removed"],
        1
    );
    fs::remove_file(root.join("example/SKILL.md")).unwrap();
    let empty = audit(&profile, Some(&baseline), false).unwrap();
    assert_eq!(empty["skill_count"], 0);
    assert_eq!(empty["installed_counts"]["removed"], 1);
}
#[test]
fn path_free_baselines_are_deterministic_and_portable() {
    let (_temp, root, profile, baseline) = fixture();
    let host = tempdir().unwrap();
    let other_root = host.path().join("different/location");
    add_skill(&other_root, "example", "example");
    let other_profile = host.path().join("profile.json");
    save(
        &other_profile,
        &json!({"kind":"groundline-guidance-profile","schema":1,"roots":{"personal":other_root}}),
    );
    let other_baseline = host.path().join("baseline.json");
    snapshot(&other_profile, &other_baseline, false).unwrap();
    assert_eq!(
        fs::read(&baseline).unwrap(),
        fs::read(&other_baseline).unwrap()
    );
    let text = fs::read_to_string(&baseline).unwrap();
    assert!(!text.contains(root.to_str().unwrap()));
    assert!(!text.contains("installed_path"));
    assert_eq!(
        audit(&other_profile, Some(&baseline), false).unwrap()["status"],
        "PASS"
    );
    assert_eq!(
        audit(&profile, Some(&other_baseline), false).unwrap()["status"],
        "PASS"
    );
}
#[test]
fn snapshot_is_new_only_private_and_outside_managed_roots() {
    let (_temp, root, profile, baseline) = fixture();
    let old = fs::read(&baseline).unwrap();
    assert!(snapshot(&profile, &baseline, false).is_err());
    assert!(snapshot(&profile, &root.join("new.json"), false).is_err());
    assert!(!root.join("new.json").exists());
    assert_eq!(fs::read(&baseline).unwrap(), old);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(baseline).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn upstream_refresh_is_captured_without_manual_hash_edits() {
    let (temp, root, profile, _baseline) = fixture();
    let upstream = temp.path().join("upstream");
    add_skill(&upstream, "example", "example");
    let checkout = upstream.join("example");
    source(&profile, &checkout);
    let first = temp.path().join("first-source.json");
    snapshot(&profile, &first, true).unwrap();
    fs::write(checkout.join("new.sh"), "never-execute\n").unwrap();
    let changed = audit(&profile, Some(&first), true).unwrap();
    assert_eq!(changed["skills"][0]["upstream_status"], "changed");
    assert_eq!(changed["skills"][0]["installed_status"], "unchanged");
    assert_eq!(changed["skills"][0]["local_differs_from_upstream"], true);
    assert!(!root.join("example/new.sh").exists());
    let next = temp.path().join("next-source.json");
    snapshot(&profile, &next, true).unwrap();
    assert_eq!(
        audit(&profile, Some(&next), true).unwrap()["skills"][0]["upstream_status"],
        "unchanged"
    );
    assert!(snapshot(&profile, &checkout.join("bad.json"), true).is_err());
}
#[test]
fn changed_provenance_and_removed_sources_require_review() {
    let (temp, _root, profile, _baseline) = fixture();
    let upstream = temp.path().join("upstream");
    add_skill(&upstream, "example", "example");
    source(&profile, &upstream.join("example"));
    let baseline = temp.path().join("source.json");
    snapshot(&profile, &baseline, true).unwrap();
    let mut data = document(&profile);
    data["sources"]["personal/example"]["declared_revision"] = json!("b".repeat(40));
    save(&profile, &data);
    assert_eq!(
        audit(&profile, Some(&baseline), true).unwrap()["status"],
        "REVIEW_REQUIRED"
    );
    data["sources"] = json!({});
    save(&profile, &data);
    assert_eq!(
        audit(&profile, Some(&baseline), true).unwrap()["skills"][0]["upstream_status"],
        "source_removed"
    );
}
#[test]
fn upstream_is_optional_but_a_requested_missing_checkout_fails() {
    let (temp, _root, profile, baseline) = fixture();
    source(&profile, &temp.path().join("missing"));
    assert_eq!(
        audit(&profile, Some(&baseline), false).unwrap()["status"],
        "PASS"
    );
    assert!(audit(&profile, Some(&baseline), true).is_err());
}

#[test]
fn adding_upstream_checks_does_not_claim_an_existing_source_baseline() {
    let (temp, _root, profile, baseline) = fixture();
    let upstream = temp.path().join("upstream");
    add_skill(&upstream, "example", "example");
    source(&profile, &upstream.join("example"));
    let result = audit(&profile, Some(&baseline), true).unwrap();
    assert_eq!(result["status"], "NOT_BASELINED");
    assert_eq!(result["upstream_unbaselined_count"], 1);
}

#[test]
fn duplicate_map_keys_cannot_silently_replace_a_root() {
    let (_temp, root, profile, _baseline) = fixture();
    let root = serde_json::to_string(&root).unwrap();
    fs::write(&profile, format!("{{\"kind\":\"groundline-guidance-profile\",\"schema\":1,\"roots\":{{\"personal\":{root},\"personal\":{root}}}}}")).unwrap();
    assert!(load_profile(&profile).is_err());
}
#[test]
fn duplicates_and_orphan_sources_are_reported_not_deleted() {
    let (temp, root, profile, baseline) = fixture();
    add_skill(&root, "another", "example");
    let result = audit(&profile, Some(&baseline), false).unwrap();
    assert_eq!(result["duplicate_name_count"], 1);
    assert_eq!(result["status"], "REVIEW_REQUIRED");
    source(&profile, &temp.path().join("upstream"));
    fs::remove_file(root.join("example/SKILL.md")).unwrap();
    assert_eq!(
        audit(&profile, Some(&baseline), false).unwrap()["orphan_source_count"],
        1
    );
    assert!(snapshot(&profile, &temp.path().join("bad.json"), false).is_err());
    assert!(root.join("another/SKILL.md").exists());
}
#[test]
fn legacy_schemas_unknown_fields_and_versions_have_no_fallback() {
    let (_temp, _root, profile, baseline) = fixture();
    let original = document(&profile);
    for value in [
        json!({"schema_version":1,"automatic_updates":false,"skills":[]}),
        {
            let mut d = original.clone();
            d["unexpected"] = json!(true);
            d
        },
        {
            let mut d = original.clone();
            d["schema"] = json!(2);
            d
        },
    ] {
        save(&profile, &value);
        assert!(audit(&profile, Some(&baseline), false).is_err());
    }
    save(&profile, &original);
    let mut b = document(&baseline);
    b["skills"]["personal/example"]["installed_path"] = json!("must-not-be-accepted");
    save(&baseline, &b);
    assert!(audit(&profile, Some(&baseline), false).is_err());
}
#[test]
fn overlapping_roots_and_wrong_baseline_scope_are_rejected() {
    let (_temp, root, profile, baseline) = fixture();
    let mut data = document(&profile);
    data["roots"]["duplicate"] = json!(root);
    save(&profile, &data);
    assert!(load_profile(&profile).is_err());
    data["roots"] = json!({"other":root});
    save(&profile, &data);
    assert!(audit(&profile, Some(&baseline), false).is_err());
    for path in ["../escape", "/absolute", "C:/private", "a\\b", "a/./b"] {
        assert!(!safe_relative(path));
    }
}
#[test]
fn provenance_cannot_copy_credentials_or_local_urls_into_baselines() {
    for value in [
        "https://token@github.com/example/skills",
        "file:///private/path",
        "https://github.com/x?token=secret",
        "https://github.com/x#secret",
    ] {
        assert!(!valid_repository(value));
    }
    assert!(valid_repository("https://github.com/example/skills"));
    assert!(!valid_revision(&Some("stable".into())));
    assert!(valid_revision(&None));
}
#[test]
fn crlf_ui_policy_and_invalid_yaml_share_one_metadata_scanner() {
    let (_temp, root, profile, baseline) = fixture();
    let entry = root.join("example/SKILL.md");
    let original = fs::read_to_string(&entry).unwrap();
    fs::write(&entry, original.replace('\n', "\r\n")).unwrap();
    let agents = root.join("example/agents");
    fs::create_dir(&agents).unwrap();
    fs::write(
        agents.join("openai.yaml"),
        "policy:\n  allow_implicit_invocation: false\n",
    )
    .unwrap();
    assert_eq!(
        audit(&profile, Some(&baseline), false).unwrap()["skills"][0]["allow_implicit_invocation"],
        false
    );
    for header in [
        "name: example\ndescription: []",
        "name: example\nname: bad\ndescription: x",
        "name: example\ndescription: ''",
    ] {
        fs::write(&entry, format!("---\n{header}\n---\nbody\n")).unwrap();
        assert!(audit(&profile, Some(&baseline), false).is_err());
    }
    fs::write(&entry, original).unwrap();
    fs::write(
        agents.join("openai.yaml"),
        "policy:\n  allow_implicit_invocation: [false]\n",
    )
    .unwrap();
    assert!(audit(&profile, Some(&baseline), false).is_err());
}
#[test]
fn file_contracts_enforce_paths_budgets_and_secret_file_boundaries() {
    let (_temp, root, profile, baseline) = fixture();
    let mut value = document(&baseline);
    value["skills"]["personal/example"]["files"]["../outside"] = json!("a".repeat(64));
    save(&baseline, &value);
    assert!(load_baseline(&baseline).is_err());
    assert!(
        inspect(
            &root.join("example"),
            &mut Budget {
                entries: files::MAX_ENTRIES,
                bytes: 0
            }
        )
        .is_err()
    );
    assert!(
        inspect(
            &root.join("example"),
            &mut Budget {
                entries: 0,
                bytes: 128 * 1024 * 1024
            }
        )
        .is_err()
    );
    fs::write(root.join("example/.env"), "never-read-secret").unwrap();
    assert!(audit(&profile, None, false).is_err());
}
#[cfg(unix)]
#[test]
fn symlinks_and_sockets_are_rejected_before_reads() {
    use std::os::unix::{fs::symlink, net::UnixListener};
    let (temp, root, profile, baseline) = fixture();
    let link = root.join("example/link");
    symlink(&profile, &link).unwrap();
    assert!(audit(&profile, Some(&baseline), false).is_err());
    fs::remove_file(&link).unwrap();
    let socket = root.join("example/socket");
    let _listener = UnixListener::bind(&socket).unwrap();
    assert!(audit(&profile, None, false).is_err());
    fs::remove_file(&socket).unwrap();
    let linked_profile = temp.path().join("linked.json");
    symlink(&profile, &linked_profile).unwrap();
    assert!(audit(&linked_profile, None, false).is_err());
    assert!(snapshot(&profile, &linked_profile, false).is_err());
}
