#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

const URL: &str = "https://developers.openai.com/blog/eval-skills.md";

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn source(id: &str, targets: &[&str]) -> Value {
    json!({"source_id":id,"url":URL,"model":null,"runtime":null,"affected_target_ids":targets})
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let fixture = Self { _temp: temp, root };
        fixture.manifest(vec![source("skills", &["workflow"])]);
        fixture.snapshot(vec![fixture.observation("skills", "# Reviewed text\n", 1)]);
        fixture
    }

    fn manifest(&self, sources: Vec<Value>) {
        write_json(
            &self.root.join("manifest.json"),
            &json!({
            "kind":"groundline-official-source-manifest","schema":1,"sources":sources}),
        );
    }

    fn observation(&self, id: &str, content: &str, day: u8) -> Value {
        json!({"source_id":id,"url":URL,"observed_at_utc":format!("2026-01-{day:02}T00:00:00Z"),"content":content})
    }

    fn snapshot(&self, snapshots: Vec<Value>) {
        write_json(
            &self.root.join("snapshot.json"),
            &json!({
            "kind":"groundline-official-source-snapshots","schema":1,"snapshots":snapshots}),
        );
    }

    fn run(&self, success: bool) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
            .current_dir(&self.root)
            .args([
                "sources",
                "check",
                "--manifest",
                "manifest.json",
                "--snapshot",
                "snapshot.json",
                "--state-dir",
                "source-state",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "stderr={} stdout={}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.contains(self.root.to_str().unwrap()));
        assert!(!stdout.contains("Reviewed text"));
        serde_json::from_str(&stdout).unwrap()
    }

    fn cache(&self) -> Vec<u8> {
        fs::read(self.root.join("source-state/cache.json")).unwrap()
    }
}

#[test]
fn baseline_unchanged_and_changed_only_request_affected_guidance_review() {
    let f = Fixture::new();
    let initial = f.run(true);
    assert_eq!(initial["observations"][0]["status"], "baseline_created");
    assert_eq!(initial["observations"][0]["reason"], "first_observation");
    assert_eq!(initial["observations"][0]["review_required"], false);
    assert_eq!(initial["observations"][0]["affected_target_ids"], json!([]));
    assert_eq!(initial["observations"][0]["authenticity_verified"], false);
    assert_eq!(
        initial["observations"][0]["observation"]["affected_target_ids"],
        json!(["workflow"])
    );
    assert_eq!(initial["network_fetch_performed"], false);
    assert_eq!(initial["model_calls"], 0);
    assert_eq!(initial["guidance_changed"], false);
    assert_eq!(
        initial["observations"][0]["snapshot_sha256"],
        initial["snapshot_sha256"]
    );
    let before = f.cache();
    f.snapshot(vec![f.observation("skills", "# Reviewed text\n", 2)]);
    let same = f.run(true);
    assert_eq!(same["observations"][0]["status"], "unchanged");
    assert_eq!(same["observations"][0]["analysis_skipped"], true);
    assert_eq!(same["cache_updated"], true);
    assert_eq!(same["observations"][0]["cache_content_updated"], false);
    assert_eq!(same["observations"][0]["cache_metadata_updated"], true);
    assert_eq!(
        same["observations"][0]["previous_snapshot_sha256"],
        initial["snapshot_sha256"]
    );
    let watermark = f.cache();
    assert_ne!(watermark, before);
    let repeated = f.run(true);
    assert_eq!(repeated["cache_updated"], false);
    assert_eq!(repeated["observations"][0]["cache_metadata_updated"], false);
    assert_eq!(f.cache(), watermark);
    f.snapshot(vec![f.observation(
        "skills",
        "# Changed official text\n",
        3,
    )]);
    let changed = f.run(true);
    assert_eq!(changed["status"], "REVIEW_REQUIRED");
    assert_eq!(changed["observations"][0]["status"], "changed");
    assert_eq!(
        changed["observations"][0]["affected_target_ids"],
        json!(["workflow"])
    );
    assert_eq!(
        changed["observations"][0]["content_sha256"],
        format!("{:x}", Sha256::digest(b"# Changed official text\n"))
    );
    assert_eq!(changed["analysis_performed"], false);
    assert_eq!(changed["recommendation_inferred"], false);
    assert_ne!(f.cache(), before);
    let state = f.root.join("source-state");
    assert_eq!(
        fs::metadata(&state).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(state.join("cache.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn missing_stale_html_and_oversized_snapshot_preserve_previous_cache() {
    let f = Fixture::new();
    f.snapshot(vec![f.observation("skills", "current", 3)]);
    f.run(true);
    let cache = f.cache();
    for (snapshots, reason) in [
        (vec![], "snapshot_unavailable"),
        (
            vec![f.observation("skills", "older content", 2)],
            "stale_snapshot",
        ),
        (
            vec![f.observation("skills", "<!doctype html><html>navigation</html>", 4)],
            "text_snapshot_required",
        ),
        (
            vec![f.observation("skills", &"x".repeat(512 * 1024 + 1), 4)],
            "content_size_limit",
        ),
    ] {
        f.snapshot(snapshots);
        let result = f.run(true);
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["observations"][0]["status"], "unavailable");
        assert_eq!(result["observations"][0]["reason"], reason);
        assert_eq!(result["observations"][0]["previous_state_preserved"], true);
        assert_eq!(result["cache_updated"], false);
        assert_eq!(f.cache(), cache);
    }
}

#[test]
fn fresh_unchanged_watermark_rejects_an_older_differing_snapshot() {
    let f = Fixture::new();
    f.snapshot(vec![f.observation("skills", "content A", 1)]);
    f.run(true);
    let initial: Value = serde_json::from_slice(&f.cache()).unwrap();
    f.snapshot(vec![f.observation("skills", "content A", 5)]);
    let same = f.run(true);
    assert_eq!(same["observations"][0]["status"], "unchanged");
    assert_eq!(same["observations"][0]["analysis_skipped"], true);
    let current = f.cache();
    let watermark: Value = serde_json::from_slice(&current).unwrap();
    assert_eq!(
        initial["entries"]["skills"]["content"],
        watermark["entries"]["skills"]["content"]
    );
    assert_eq!(
        watermark["entries"]["skills"]["latest_observed_at_utc"],
        "2026-01-05T00:00:00Z"
    );
    f.snapshot(vec![f.observation("skills", "content B", 4)]);
    let stale = f.run(true);
    assert_eq!(stale["observations"][0]["status"], "unavailable");
    assert_eq!(stale["observations"][0]["reason"], "stale_snapshot");
    assert_eq!(stale["cache_updated"], false);
    assert_eq!(f.cache(), current);
}

#[test]
fn same_url_mapping_change_preserves_freshness_and_rejects_same_time_conflicts() {
    let f = Fixture::new();
    f.snapshot(vec![f.observation("skills", "content A", 3)]);
    f.run(true);
    let current = f.cache();
    f.manifest(vec![source("skills", &["another-guidance"])]);
    for (day, content, reason) in [
        (1, "content A", "stale_snapshot"),
        (2, "content B", "stale_snapshot"),
        (3, "content B", "conflicting_snapshot_time"),
    ] {
        f.snapshot(vec![f.observation("skills", content, day)]);
        let rejected = f.run(true);
        assert_eq!(rejected["observations"][0]["status"], "unavailable");
        assert_eq!(rejected["observations"][0]["reason"], reason);
        assert_eq!(rejected["observations"][0]["review_required"], false);
        assert_eq!(rejected["cache_updated"], false);
        assert_eq!(f.cache(), current);
    }
    // A new target association may establish its baseline at the existing
    // watermark, provided it agrees with the observed content at that time.
    f.snapshot(vec![f.observation("skills", "content A", 3)]);
    let mapped = f.run(true);
    assert_eq!(mapped["observations"][0]["status"], "baseline_created");
    assert_eq!(
        mapped["observations"][0]["reason"],
        "source_mapping_changed"
    );
    let remapped = f.cache();
    f.snapshot(vec![f.observation("skills", "content B", 2)]);
    assert_eq!(f.run(true)["observations"][0]["reason"], "stale_snapshot");
    assert_eq!(f.cache(), remapped);
}

#[test]
fn different_url_establishes_a_baseline_without_comparing_unrelated_documents() {
    let f = Fixture::new();
    f.snapshot(vec![f.observation("skills", "content A", 3)]);
    f.run(true);
    let next_url = "https://learn.chatgpt.com/docs/hooks.md";
    let mut next_source = source("skills", &["workflow"]);
    next_source["url"] = json!(next_url);
    f.manifest(vec![next_source]);
    let mut next_snapshot = f.observation("skills", "different document", 1);
    next_snapshot["url"] = json!(next_url);
    f.snapshot(vec![next_snapshot]);
    let next = f.run(true);
    assert_eq!(next["observations"][0]["status"], "baseline_created");
    assert_eq!(next["observations"][0]["review_required"], false);
    assert_eq!(next["observations"][0]["observed_url"], next_url);
    assert_eq!(
        next["observations"][0]["latest_observed_at_utc"],
        "2026-01-01T00:00:00Z"
    );
}

#[test]
fn url_mapping_and_record_bounds_are_rejected_before_cache_mutation() {
    let f = Fixture::new();
    f.run(true);
    let cache = f.cache();
    for url in [
        "https://developers.openai.com.evil.invalid/x",
        "http://developers.openai.com/x",
        "https://user@developers.openai.com/x",
        "https://developers.openai.com:8443/x",
        "https://developers.openai.com/x?token=private",
        "https://developers.openai.com/x#fragment",
    ] {
        let mut entry = source("skills", &["workflow"]);
        entry["url"] = json!(url);
        f.manifest(vec![entry]);
        let result = f.run(false);
        assert_eq!(result["error"], "sources_invalid_official_url");
        assert_eq!(f.cache(), cache);
    }
    let mut entry = source("skills", &["workflow"]);
    entry.as_object_mut().unwrap().remove("model");
    f.manifest(vec![entry]);
    assert_eq!(f.run(false)["error"], "sources_explicit_mapping_required");
    f.manifest(
        (0..33)
            .map(|i| source(&format!("source-{i}"), &["workflow"]))
            .collect(),
    );
    assert_eq!(f.run(false)["error"], "sources_invalid_manifest");
    f.manifest(vec![
        source("skills", &["workflow"]),
        source("skills", &["workflow"]),
    ]);
    assert_eq!(f.run(false)["error"], "sources_duplicate_source_id");
    assert_eq!(f.cache(), cache);
}

#[test]
fn changed_mapping_creates_new_baseline_without_guessing_a_guidance_change() {
    let f = Fixture::new();
    f.run(true);
    f.manifest(vec![source("skills", &["another-guidance"])]);
    f.snapshot(vec![f.observation("skills", "different content", 2)]);
    let result = f.run(true);
    assert_eq!(result["observations"][0]["status"], "baseline_created");
    assert_eq!(
        result["observations"][0]["reason"],
        "source_mapping_changed"
    );
    assert_eq!(result["observations"][0]["review_required"], false);
    assert_eq!(result["guidance_changed"], false);
}

#[test]
fn corrupt_or_linked_private_inputs_are_preserved_and_rejected() {
    let f = Fixture::new();
    f.run(true);
    let cache = f.root.join("source-state/cache.json");
    fs::write(&cache, b"{corrupt").unwrap();
    assert_eq!(f.run(false)["error"], "sources_invalid_cache");
    assert_eq!(fs::read(&cache).unwrap(), b"{corrupt");
    fs::remove_file(&cache).unwrap();
    symlink(f.root.join("snapshot.json"), &cache).unwrap();
    assert_eq!(f.run(false)["error"], "sources_invalid_cache_file");
    fs::remove_file(&cache).unwrap();
    fs::rename(
        f.root.join("snapshot.json"),
        f.root.join("real-snapshot.json"),
    )
    .unwrap();
    symlink("real-snapshot.json", f.root.join("snapshot.json")).unwrap();
    assert_eq!(f.run(false)["error"], "sources_input_unavailable");
    assert!(f.root.join("real-snapshot.json").is_file());
}

#[test]
fn mixed_sources_keep_failed_baseline_and_review_only_changed_target() {
    let f = Fixture::new();
    f.manifest(vec![
        source("skills", &["workflow"]),
        source("runtime", &["permissions"]),
    ]);
    f.snapshot(vec![
        f.observation("skills", "first", 1),
        f.observation("runtime", "second", 1),
    ]);
    f.run(true);
    let cache: Value = serde_json::from_slice(&f.cache()).unwrap();
    f.snapshot(vec![f.observation("skills", "changed", 2)]);
    let result = f.run(true);
    assert_eq!(result["status"], "PARTIAL");
    assert_eq!(
        result["observations"][0]["affected_target_ids"],
        json!(["workflow"])
    );
    assert_eq!(result["observations"][1]["status"], "unavailable");
    let after: Value = serde_json::from_slice(&f.cache()).unwrap();
    assert_eq!(cache["entries"]["runtime"], after["entries"]["runtime"]);
}

#[test]
fn manifest_and_snapshot_files_have_independent_bounds_and_private_permissions() {
    let f = Fixture::new();
    f.run(true);
    let cache = f.cache();
    fs::write(f.root.join("manifest.json"), vec![b' '; 64 * 1024 + 1]).unwrap();
    assert_eq!(
        f.run(false)["error"],
        "sources_private_regular_input_required"
    );
    f.manifest(vec![source("skills", &["workflow"])]);
    fs::write(
        f.root.join("snapshot.json"),
        vec![b' '; 4 * 1024 * 1024 + 1],
    )
    .unwrap();
    assert_eq!(
        f.run(false)["error"],
        "sources_private_regular_input_required"
    );
    f.snapshot(vec![f.observation("skills", "replacement", 2)]);
    fs::set_permissions(
        f.root.join("snapshot.json"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(
        f.run(false)["error"],
        "sources_private_regular_input_required"
    );
    assert_eq!(f.cache(), cache);
}

#[test]
fn future_public_model_mapping_is_observation_without_availability_inference() {
    let f = Fixture::new();
    let mut future = source("skills", &["workflow"]);
    future["model"] = json!("gpt-7-sol");
    future["runtime"] = json!({"family":"codex_app","version":"0.200.0",
        "evidence_sha256":"a".repeat(64)});
    f.manifest(vec![future.clone()]);
    let result = f.run(true);
    assert_eq!(result["observations"][0]["status"], "baseline_created");
    assert_eq!(
        result["observations"][0]["observation"]["model"],
        "gpt-7-sol"
    );
    assert_eq!(result["recommendation_inferred"], false);
    assert_eq!(result["native_availability_verified"], false);
    assert_eq!(result["selection_inferred"], false);
    assert_eq!(result["native_settings_changed"], false);
    let cache = f.cache();
    for model in [
        "custom-private-model",
        "sol",
        "other",
        "private-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    ] {
        future["model"] = json!(model);
        f.manifest(vec![future.clone()]);
        assert_eq!(f.run(false)["error"], "sources_invalid_source_mapping");
        assert_eq!(f.cache(), cache);
    }
}
