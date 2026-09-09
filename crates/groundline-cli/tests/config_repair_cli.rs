use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::{TempDir, tempdir};

mod common;

struct Fixture {
    root: TempDir,
    config: PathBuf,
    catalog: PathBuf,
    backup: PathBuf,
}

impl Fixture {
    fn new(config_text: &str) -> Self {
        let root = tempdir().unwrap();
        let config = root.path().join("config.toml");
        let catalog = root.path().join("models.json");
        let backup = root.path().join("before.toml");
        common::write_owned_config(&config, config_text.as_bytes());
        fs::write(
            &catalog,
            serde_json::to_vec(&json!({"models":[{
                "slug":"gpt-6-astra", "default_reasoning_level":"low",
                "supported_reasoning_levels":[{"effort":"low"},{"effort":"ultra"}],
                "support_verbosity":true
            }]}))
            .unwrap(),
        )
        .unwrap();
        Self {
            root,
            config,
            catalog,
            backup,
        }
    }

    fn run(&self, extra: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
            .arg("config-repair")
            .arg("--config")
            .arg(&self.config)
            .arg("--catalog")
            .arg(&self.catalog)
            .args(extra)
            .output()
            .unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        for private in ["PRIVATE_SENTINEL", self.root.path().to_str().unwrap()] {
            assert!(!text.contains(private));
            assert!(!String::from_utf8_lossy(&output.stderr).contains(private));
        }
        (
            output.status.code().unwrap(),
            serde_json::from_str(&text).unwrap(),
        )
    }

    fn apply(&self, preview: &Value, extra: &[&str]) -> (i32, Value) {
        let mut args = vec![
            "--apply",
            "--expect-plan",
            preview["plan_sha256"]
                .as_str()
                .unwrap_or_else(|| panic!("preview did not produce a plan: {preview}")),
            "--backup",
            self.backup.to_str().unwrap(),
        ];
        args.extend(extra);
        self.run(&args)
    }
}

#[test]
fn preview_apply_backup_and_second_application_are_bounded_and_private() {
    let original = "# preserve\nmodel='gpt-6-astra'\nmodel_reasoning_effort='ultra'\nservice_tier='fast'\nmodel_context_window=0\n[unrelated]\nsecret='PRIVATE_SENTINEL'\n";
    let fixture = Fixture::new(original);
    let (code, preview) = fixture.run(&[]);
    assert_eq!(code, 0, "{preview}");
    assert_eq!(preview["status"], "READY");
    assert_eq!(preview["mutation_performed"], false);
    assert_eq!(fs::read_dir(fixture.root.path()).unwrap().count(), 2);
    assert_eq!(fs::read_to_string(&fixture.config).unwrap(), original);
    let (code, applied) = fixture.apply(&preview, &[]);
    assert_eq!(code, 0);
    assert_eq!(applied["status"], "PASS");
    assert_eq!(applied["configuration_changed"], true);
    assert_eq!(applied["backup_written"], true);
    assert_eq!(applied["file_verified"], true);
    assert_eq!(applied["effective_runtime_verified"], false);
    assert_eq!(fs::read_to_string(&fixture.backup).unwrap(), original);
    assert_eq!(
        fs::read_to_string(&fixture.config).unwrap(),
        original.replace("model_context_window=0\n", "")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for file in [&fixture.config, &fixture.backup] {
            assert_eq!(
                fs::metadata(file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    // A stale apply must never replay, even if the old backup still exists.
    assert_eq!(fixture.apply(&preview, &[]).0, 1);
    let (_, no_op) = fixture.run(&[]);
    assert_eq!(no_op["removed_keys"], json!([]));
    let (_, second) = fixture.apply(&no_op, &[]);
    assert_eq!(second["mutation_performed"], false);
    assert_eq!(fs::read_to_string(&fixture.backup).unwrap(), original);
}

#[test]
fn edits_catalog_and_option_changes_invalidate_the_plan_before_writes() {
    for change in ["config", "catalog", "option", "target"] {
        let mut fixture = Fixture::new("model_context_window=0\n");
        let (_, preview) = fixture.run(&[]);
        match change {
            "config" => fs::write(&fixture.config, "# edited\nmodel_context_window=0\n").unwrap(),
            "catalog" => {
                let mut catalog = fs::read(&fixture.catalog).unwrap();
                catalog.push(b'\n');
                fs::write(&fixture.catalog, catalog).unwrap();
            }
            "target" => {
                fixture.config = fixture.root.path().join("different.toml");
                common::write_owned_config(&fixture.config, b"model_context_window=0\n");
            }
            _ => (),
        }
        let before = fs::read(&fixture.config).unwrap();
        let extra: &[&str] = if change == "option" {
            &["--restore-native-context"]
        } else {
            &[]
        };
        let (code, result) = fixture.apply(&preview, extra);
        assert_eq!(code, 1, "{change}");
        assert_eq!(result["error"], "config_repair_plan_changed");
        assert_eq!(fs::read(&fixture.config).unwrap(), before);
        assert!(!fixture.backup.exists());
    }
}

#[test]
fn valid_manual_context_needs_explicit_native_default_restoration() {
    let fixture = Fixture::new("model_context_window=100\nmodel_auto_compact_token_limit=90\n");
    let (_, untouched) = fixture.run(&[]);
    assert_eq!(untouched["status"], "REVIEW_REQUIRED");
    assert_eq!(untouched["removed_keys"], json!([]));
    let (_, preview) = fixture.run(&["--restore-native-context"]);
    assert_eq!(preview["removed_keys"].as_array().unwrap().len(), 2);
    assert_eq!(fixture.apply(&preview, &["--restore-native-context"]).0, 0);
    assert_eq!(fs::read_to_string(fixture.config).unwrap(), "");
}

#[test]
fn invalid_or_unresolved_settings_fail_without_automatic_reinterpretation() {
    for text in [
        "model=[]",
        "model='one'\nmodel='two'",
        "model_context_window='PRIVATE_SENTINEL'",
        "profile='private'\nmodel_context_window=0",
        "model_provider='custom'\nmodel_context_window=0",
        "model_catalog_json='private'\nmodel_context_window=0",
        "model='missing'\nmodel_context_window=0",
        "model='gpt-6-astra'\nmodel_reasoning_effort='minimal'\nmodel_context_window=0",
    ] {
        let fixture = Fixture::new(text);
        let (code, preview) = fixture.run(&[]);
        assert_eq!(code, 1);
        if preview["plan_sha256"].is_string() {
            assert_eq!(fixture.apply(&preview, &[]).0, 1);
        }
        assert_eq!(fs::read_to_string(&fixture.config).unwrap(), text);
        assert_eq!(fs::read_dir(fixture.root.path()).unwrap().count(), 2);
    }
}

#[test]
fn existing_backup_and_busy_repair_preserve_config() {
    let fixture = Fixture::new("model_context_window=0");
    let (_, preview) = fixture.run(&[]);
    fs::write(&fixture.backup, "PRIVATE_SENTINEL").unwrap();
    assert_eq!(fixture.apply(&preview, &[]).0, 1);
    assert_eq!(
        fs::read_to_string(&fixture.backup).unwrap(),
        "PRIVATE_SENTINEL"
    );
    assert_eq!(
        fs::read_to_string(&fixture.config).unwrap(),
        "model_context_window=0"
    );

    let fixture = Fixture::new("model_context_window=0");
    let (_, preview) = fixture.run(&[]);
    let lock = groundline_runtime::local_file::open_or_create_private_lock(
        &fixture
            .root
            .path()
            .join("config.toml.groundline-repair.lock"),
    )
    .unwrap();
    lock.try_lock().unwrap();
    let (code, result) = fixture.apply(&preview, &[]);
    assert_eq!(code, 1);
    assert_eq!(result["error"], "config_repair_config_busy");
    assert!(!fixture.backup.exists());
}

#[cfg(windows)]
#[test]
fn native_default_owner_is_checked_without_weakening_user_ownership() {
    let mut fixture = Fixture::new("model_context_window=0");
    fixture.config = fixture.root.path().join("native-default-owner.toml");
    fs::write(&fixture.config, "model_context_window=0").unwrap();
    let file =
        groundline_runtime::local_file::open_bounded_regular_file(&fixture.config, 0, 512).unwrap();
    let user_owned = groundline_runtime::local_file::owned_by_current_user(&file);
    drop(file);
    let (code, report) = fixture.run(&[]);
    println!("native_default_owner_matches_process_user={user_owned}");
    if user_owned {
        assert_eq!(code, 0, "{report}");
        assert_eq!(report["status"], "READY");
    } else {
        assert_eq!(code, 1, "{report}");
        assert_eq!(report["error"], "config_repair_config_owner_mismatch");
    }
    assert_eq!(report["mutation_performed"], false);
    assert_eq!(
        fs::read_to_string(&fixture.config).unwrap(),
        "model_context_window=0"
    );
    assert!(!fixture.backup.exists());
}

#[cfg(unix)]
#[test]
fn linked_files_are_rejected_and_linked_backup_is_preserved() {
    let mut fixture = Fixture::new("model_context_window=0");
    let real = fixture.config.clone();
    fixture.config = fixture.root.path().join("link.toml");
    std::os::unix::fs::symlink(&real, &fixture.config).unwrap();
    assert_eq!(fixture.run(&[]).0, 1);
    fs::remove_file(&fixture.config).unwrap();
    fs::hard_link(&real, &fixture.config).unwrap();
    assert_eq!(fixture.run(&[]).0, 1);
    fs::remove_file(&fixture.config).unwrap();
    fixture.config = real;
    let (_, preview) = fixture.run(&[]);
    std::os::unix::fs::symlink(&fixture.catalog, &fixture.backup).unwrap();
    let catalog = fs::read(&fixture.catalog).unwrap();
    assert_eq!(fixture.apply(&preview, &[]).0, 1);
    assert_eq!(fs::read(&fixture.catalog).unwrap(), catalog);
}

#[test]
fn apply_arguments_are_required_and_oversized_config_is_rejected() {
    let fixture = Fixture::new("model_context_window=0");
    for args in [vec!["--apply"], vec!["--expect-plan", "not-a-plan"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
            .arg("config-repair")
            .arg("--config")
            .arg(&fixture.config)
            .arg("--catalog")
            .arg(&fixture.catalog)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
    assert_eq!(fs::read_dir(fixture.root.path()).unwrap().count(), 2);
    fs::write(&fixture.config, vec![b' '; 512 * 1024 + 1]).unwrap();
    assert_eq!(fixture.run(&[]).0, 1);
    assert!(!fixture.backup.exists());
}

#[cfg(unix)]
#[test]
fn non_unicode_target_reports_an_error_without_panicking_or_writing() {
    use std::os::unix::ffi::OsStringExt;
    let mut fixture = Fixture::new("model_context_window=0");
    fixture.config = fixture
        .root
        .path()
        .join(std::ffi::OsString::from_vec(vec![0xff]));
    let (code, result) = fixture.run(&[]);
    assert_eq!(code, 1);
    assert_eq!(result["error"], "config_repair_invalid_path");
    assert!(!fixture.backup.exists());
}
