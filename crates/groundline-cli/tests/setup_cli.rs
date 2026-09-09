use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::{TempDir, tempdir};

mod common;

struct Fixture {
    root: TempDir,
    home: PathBuf,
    catalog: PathBuf,
}
impl Fixture {
    fn new(config: Option<&str>) -> Self {
        let root = tempdir().unwrap();
        let home = root.path().join("another pc 한글 home");
        fs::create_dir(&home).unwrap();
        if let Some(text) = config {
            common::write_owned_config(&home.join("config.toml"), text.as_bytes());
        }
        let catalog = root.path().join("catalog.json");
        fs::write(
            &catalog,
            serde_json::to_vec(&json!({"models":[{
                "slug":"gpt-6-astra", "default_reasoning_level":"low",
                "supported_reasoning_levels":[{"effort":"low"},{"effort":"xhigh"}],
                "support_verbosity":true, "base_instructions":"PRIVATE_SENTINEL"
            }]}))
            .unwrap(),
        )
        .unwrap();
        Self {
            root,
            home,
            catalog,
        }
    }
    fn run(&self, apply: bool) -> (i32, Value) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_groundline"));
        // Test the same automatic home resolution used by the installer.
        command
            .env("CODEX_HOME", &self.home)
            .args(["setup", "--catalog"])
            .arg(&self.catalog);
        if apply {
            command.arg("--apply");
        }
        let output = command.output().unwrap();
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
    fn bytes(&self) -> Vec<u8> {
        fs::read(self.home.join("config.toml")).unwrap()
    }
}

#[test]
fn fresh_home_installs_requested_defaults_and_repeat_writes_nothing() {
    let f = Fixture::new(None);
    let (_, preview) = f.run(false);
    assert_eq!(preview["status"], "READY");
    assert_eq!(fs::read_dir(&f.home).unwrap().count(), 0);
    let (code, report) = f.run(true);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["status"], "PASS");
    assert_eq!(report["configuration_existed"], false);
    assert_eq!(report["backup_written"], false);
    let config: toml::Table = toml::from_str(std::str::from_utf8(&f.bytes()).unwrap()).unwrap();
    assert_eq!(config["model"].as_str(), Some("gpt-6-astra"));
    assert_eq!(config["model_reasoning_effort"].as_str(), Some("xhigh"));
    assert_eq!(config["service_tier"].as_str(), Some("default"));
    let before = f.bytes();
    let count = fs::read_dir(&f.home).unwrap().count();
    assert_eq!(f.run(true).1["mutation_performed"], false);
    assert_eq!(before, f.bytes());
    assert_eq!(fs::read_dir(&f.home).unwrap().count(), count);
}

#[test]
fn existing_pc_repairs_baseline_context_and_only_known_retired_core_trust() {
    let original = format!(
        "# user comment\r\nmodel = 'older-model' # keep inline comment\r\nmodel_reasoning_effort='ultra'\r\nservice_tier='fast'\r\nmodel_context_window=999\r\nmodel_auto_compact_token_limit=0\r\napproval_policy='on-request'\r\n[features]\r\nchronicle=true\r\n[hooks.state.\"groundline@groundline:hooks/hooks.json:stop:0:0\"]\r\nenabled=true\r\ntrusted_hash='{}'\r\n[hooks.state.\"groundline-insights@groundline:hooks/hooks.json:stop:0:0\"]\r\nenabled=true\r\n[private]\r\nsecret='PRIVATE_SENTINEL'\r\n",
        format_args!("sha256:{}", "a".repeat(64))
    );
    let f = Fixture::new(Some(&original));
    let (code, report) = f.run(true);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["file_verified"], true);
    let backup = f.home.join(report["backup_file"].as_str().unwrap());
    assert_eq!(fs::read(&backup).unwrap(), original.as_bytes());
    let text = String::from_utf8(f.bytes()).unwrap();
    assert!(text.contains("# keep inline comment"));
    assert!(!text.replace("\r\n", "").contains('\n'));
    let actual: toml::Table = toml::from_str(&text).unwrap();
    let mut expected: toml::Table = toml::from_str(&original).unwrap();
    for (k, v) in [
        ("model", "gpt-6-astra"),
        ("model_reasoning_effort", "xhigh"),
        ("service_tier", "default"),
    ] {
        expected.insert(k.to_owned(), toml::Value::String(v.to_owned()));
    }
    expected.remove("model_context_window");
    expected.remove("model_auto_compact_token_limit");
    expected
        .get_mut("hooks")
        .unwrap()
        .get_mut("state")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .remove("groundline@groundline:hooks/hooks.json:stop:0:0");
    assert_eq!(actual, expected);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [f.home.join("config.toml"), backup] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    assert_eq!(f.run(true).1["mutation_performed"], false);
}

#[test]
fn unsupported_models_efforts_layers_and_formats_fail_without_any_writes() {
    for text in [
        "profile='custom'",
        "model_provider='custom'",
        "model_catalog_json='custom'",
        "model=42",
        "model_context_window='bad'",
        "broken=[",
        "[hooks.state]\n'groundline@groundline:hooks/hooks.json:stop:0:0'='unknown'",
    ] {
        let f = Fixture::new(Some(text));
        let (code, report) = f.run(true);
        assert_eq!(code, 1, "{text}: {report}");
        assert_eq!(report["mutation_performed"], false);
        assert_eq!(f.bytes(), text.as_bytes());
        assert_eq!(fs::read_dir(&f.home).unwrap().count(), 1);
    }
    for (from, to) in [("gpt-6-astra", "unavailable-model"), ("xhigh", "medium")] {
        let f = Fixture::new(Some("service_tier='fast'"));
        let catalog = fs::read_to_string(&f.catalog).unwrap().replace(from, to);
        fs::write(&f.catalog, catalog).unwrap();
        assert_eq!(f.run(true).0, 1);
        assert_eq!(f.bytes(), b"service_tier='fast'");
        assert_eq!(fs::read_dir(&f.home).unwrap().count(), 1);
    }
}

#[test]
fn inline_hook_state_preserves_other_plugins_and_unknown_core_events() {
    let f = Fixture::new(Some(
        "[hooks]\nstate = { 'groundline@groundline:hooks/hooks.json:stop:0:0' = { enabled = false }, 'groundline@groundline:hooks/hooks.json:custom:0:0' = { enabled = true }, other = { enabled = true } }\n",
    ));
    let (code, report) = f.run(true);
    assert_eq!(code, 0, "{report}");
    let text = String::from_utf8(f.bytes()).unwrap();
    assert!(!text.contains(":stop:0:0"));
    assert!(text.contains(":custom:0:0"));
    assert!(text.contains("other"));
}

#[test]
fn unrecognized_trust_hash_formats_are_preserved_without_writes() {
    for hash in [
        "a".repeat(64),
        format!("sha512:{}", "a".repeat(64)),
        "sha256:invalid".to_owned(),
    ] {
        let original = format!(
            "[hooks.state.\"groundline@groundline:hooks/hooks.json:stop:0:0\"]\ntrusted_hash='{hash}'\n"
        );
        let f = Fixture::new(Some(&original));
        let (code, report) = f.run(true);
        assert_eq!(code, 1);
        assert_eq!(report["error"], "setup_unsupported_hook_state");
        assert_eq!(f.bytes(), original.as_bytes());
        assert_eq!(fs::read_dir(&f.home).unwrap().count(), 1);
    }
}

#[cfg(unix)]
#[test]
fn symlink_and_hardlink_configs_are_rejected() {
    use std::os::unix::fs::symlink;
    for hard in [false, true] {
        let f = Fixture::new(None);
        let target = f.root.path().join("target.toml");
        fs::write(&target, "model_context_window=0").unwrap();
        let link = f.home.join("config.toml");
        if hard {
            fs::hard_link(&target, &link).unwrap();
        } else {
            symlink(&target, &link).unwrap();
        }
        assert_eq!(f.run(true).0, 1);
        assert_eq!(
            fs::read_to_string(target).unwrap(),
            "model_context_window=0"
        );
        assert_eq!(fs::read_dir(&f.home).unwrap().count(), 1);
    }
}
