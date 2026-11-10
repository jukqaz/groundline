//! Public installer contracts; real native transport is covered in native_install.
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::{TempDir, tempdir};

mod common;
#[path = "common/package.rs"]
mod package;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
    codex: PathBuf,
    calls: PathBuf,
    state: PathBuf,
    target: String,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempdir().unwrap();
        let root = temp.path().join("reviewed Core distribution 한글");
        let home = temp.path().join("isolated home");
        let state = temp.path().join("fake native");
        for dir in [&root, &home, &state] {
            fs::create_dir_all(dir).unwrap();
        }
        let binary = Path::new(env!("CARGO_BIN_EXE_groundline"));
        let platform: Value = serde_json::from_slice(
            &Command::new(binary)
                .args(["platform", "--json"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let target = platform["target"].as_str().unwrap().to_owned();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        fs::copy(repo.join("install.sh"), root.join("install.sh")).unwrap();
        fs::create_dir_all(root.join(".agents/plugins")).unwrap();
        let mut catalog: Value = serde_json::from_slice(
            &fs::read(repo.join(".agents/plugins/marketplace.json")).unwrap(),
        )
        .unwrap();
        catalog["name"] = json!("groundline-core");
        catalog["plugins"]
            .as_array_mut()
            .unwrap()
            .retain(|p| p["name"] == "groundline");
        fs::write(
            root.join(".agents/plugins/marketplace.json"),
            serde_json::to_vec(&catalog).unwrap(),
        )
        .unwrap();
        fs::write(state.join("market.json"), b"{\"marketplaces\":[]}").unwrap();
        fs::write(state.join("plugins.json"), b"{\"installed\":[]}").unwrap();
        fs::write(state.join("market-after.json"), serde_json::to_vec(&json!({"marketplaces":[{"name":"groundline-core","root":root,"marketplaceSource":{"sourceType":"git","source":"git@github.com:jukqaz/groundline.git"}}]})).unwrap()).unwrap();
        fs::write(state.join("plugin-after.json"), serde_json::to_vec(&json!({"installed":[{"pluginId":"groundline@groundline-core","name":"groundline","marketplaceName":"groundline-core","version":env!("CARGO_PKG_VERSION"),"enabled":true}]})).unwrap()).unwrap();
        fs::write(state.join("models.json"), br#"{"models":[{"slug":"gpt-6-astra","default_reasoning_level":"xhigh","supported_reasoning_levels":[{"effort":"xhigh"}]}]}"#).unwrap();
        let calls = state.join("calls.txt");
        let codex = state.join("fake codex.sh");
        fs::write(&codex, r#"#!/bin/sh
printf '%s\n' "$*" >> "$GROUNDLINE_TEST_CALLS"
[ "${3:-}" != --help ] && [ "${4:-}" != --help ] || exit 0
fixture=$(dirname "$GROUNDLINE_TEST_CALLS")
case "$1 $2 ${3:-}" in
 'plugin marketplace list') cat "$fixture/market.json" ;;
 'plugin list --json') cat "$fixture/plugins.json" ;;
 'plugin marketplace add') cp "$fixture/market-after.json" "$fixture/market.json" ;;
 'plugin marketplace remove') printf '{"marketplaces":[]}' > "$fixture/market.json" ;;
 'plugin marketplace upgrade') [ "${GROUNDLINE_TEST_FAIL_REFRESH:-0}" != 1 ] || exit 19 ;;
 'plugin add groundline@groundline-core') cp "$fixture/plugin-after.json" "$fixture/plugins.json" ;;
 plugin\ remove\ *) printf '{"installed":[]}' > "$fixture/plugins.json" ;;
 'debug models '*) cat "$fixture/models.json"; [ "${GROUNDLINE_TEST_FAIL_CATALOG:-0}" != 1 ] || exit 18 ;;
 '--strict-config doctor --summary') [ "${GROUNDLINE_TEST_FAIL_DOCTOR:-0}" != 1 ] || exit 42 ;;
esac
"#).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&codex, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let f = Self {
            _temp: temp,
            root,
            home,
            codex,
            calls,
            state,
            target,
        };
        f.commit();
        let source = f.git(&["rev-parse", "HEAD"]);
        package::stage(
            &f.root.join("plugins/groundline"),
            binary,
            "groundline",
            &f.target,
        );
        package::stage(&f.cache(), binary, "groundline", &f.target);
        fs::write(f.root.join("distribution.json"), serde_json::to_vec(&json!({"kind":"groundline-plugin-distribution","schema":1,"product":"core","plugin":"groundline","marketplace":"groundline-core","channel":"core-stable","version":env!("CARGO_PKG_VERSION"),"release_tag":format!("core-v{}",env!("CARGO_PKG_VERSION")),"source_sha":source})).unwrap()).unwrap();
        f.commit();
        f
    }
    fn cache(&self) -> PathBuf {
        self.home
            .join("plugins/cache/groundline-core/groundline")
            .join(env!("CARGO_PKG_VERSION"))
    }
    fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    }
    fn commit(&self) {
        if !self.root.join(".git").exists() {
            self.git(&["init"]);
        }
        self.git(&["add", "--all"]);
        self.git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "test: installer fixture",
        ]);
    }
    fn run(&self, options: &[&str], fail: Option<&str>) -> Output {
        let mut c = Command::new(if cfg!(target_os = "macos") {
            "/bin/bash"
        } else {
            "bash"
        });
        c.arg(self.root.join("install.sh"))
            .arg("--codex")
            .arg(&self.codex)
            .args(options)
            .env("CODEX_HOME", &self.home)
            .env("GROUNDLINE_TEST_CALLS", &self.calls);
        if let Some(key) = fail {
            c.env(key, "1");
        }
        c.output().unwrap()
    }
    fn assert_blocked(&self, stage: &str, output: Output) {
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let receipt = package::receipt(&output);
        assert_eq!(receipt["stages"][stage]["status"], "FAIL", "{receipt}");
        let calls = fs::read_to_string(&self.calls).unwrap_or_default();
        assert!(
            !calls.contains("plugin marketplace add ")
                && !calls.contains("plugin marketplace remove ")
                && !calls.contains("plugin add "),
            "{calls}"
        );
    }
}

#[test]
fn default_install_preserves_choices_and_never_loads_models_or_insights() {
    let f = Fixture::new();
    let original = b"model='gpt-6-astra'\nmodel_reasoning_effort='xhigh'\nservice_tier='fast'\n";
    common::write_owned_config(&f.home.join("config.toml"), original);
    let out = f.run(&[], None);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(fs::read(f.home.join("config.toml")).unwrap(), original);
    let receipt = package::receipt(&out);
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["stages"]["settings"]["status"], "PRESERVED");
    assert_eq!(receipt["stages"]["insights_setup"]["status"], "PRESERVED");
    let calls = fs::read_to_string(&f.calls).unwrap();
    assert!(
        !calls.contains("debug models") && !calls.contains("groundline-insights"),
        "{calls}"
    );
    let backups = fs::read_dir(f.home.join("groundline/install-backups"))
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1);
    let backup = backups[0].as_ref().unwrap().path().join("config.toml");
    assert_eq!(fs::read(backup).unwrap(), original);
}

#[test]
fn explicit_settings_are_applied_and_doctor_failure_remains_resumable() {
    let f = Fixture::new();
    common::write_owned_config(&f.home.join("config.toml"), b"model_context_window=0\n");
    let options = [
        "--model",
        "gpt-6-astra",
        "--effort",
        "xhigh",
        "--service-tier",
        "default",
        "--restore-native-context",
    ];
    let out = f.run(&options, Some("GROUNDLINE_TEST_FAIL_DOCTOR"));
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let report = package::receipt(&out);
    assert_eq!(report["stages"]["native_doctor"]["exit_code"], 42);
    assert_eq!(report["stages"]["settings"]["status"], "PASS");
    let values: toml::Table =
        toml::from_str(&fs::read_to_string(f.home.join("config.toml")).unwrap()).unwrap();
    assert_eq!(values["model"].as_str(), Some("gpt-6-astra"));
    assert_eq!(values["model_reasoning_effort"].as_str(), Some("xhigh"));
    assert_eq!(values["service_tier"].as_str(), Some("default"));
    assert!(!values.contains_key("model_context_window"));
    let before = fs::read(f.home.join("config.toml")).unwrap();
    assert!(f.run(&options, None).status.success());
    assert_eq!(fs::read(f.home.join("config.toml")).unwrap(), before);
}

#[test]
fn failed_catalog_partial_output_cannot_apply_settings() {
    let f = Fixture::new();
    let config = f.home.join("config.toml");
    common::write_owned_config(&config, b"service_tier='fast'\n");
    let before = fs::read(&config).unwrap();
    let out = f.run(
        &["--model", "gpt-6-astra"],
        Some("GROUNDLINE_TEST_FAIL_CATALOG"),
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        package::receipt(&out)["stages"]["catalog"]["status"],
        "FAIL"
    );
    assert_eq!(fs::read(config).unwrap(), before);
}

#[test]
fn corrupt_or_uncommitted_distribution_stops_before_native_writes() {
    for mode in ["dirty", "extra-commit", "amended"] {
        let f = Fixture::new();
        let path = f
            .root
            .join("plugins/groundline/bin")
            .join(&f.target)
            .join("groundline.sha256");
        fs::write(path, format!("{}  groundline\n", "0".repeat(64))).unwrap();
        if mode == "extra-commit" {
            f.commit();
        } else if mode == "amended" {
            f.git(&["add", "--all"]);
            f.git(&[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--amend",
                "--no-edit",
            ]);
        }
        f.assert_blocked(
            if mode == "amended" {
                "distribution_package"
            } else {
                "distribution_revision"
            },
            f.run(&[], None),
        );
    }
}

#[test]
fn installed_bytes_are_checked_and_failure_rolls_back_only_new_registration() {
    let f = Fixture::new();
    fs::write(
        f.cache()
            .join("bin")
            .join(&f.target)
            .join("groundline.sha256"),
        b"corrupt",
    )
    .unwrap();
    let out = f.run(&[], None);
    let report = package::receipt(&out);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(report["stages"]["installed_bytes"]["status"], "FAIL");
    assert_eq!(report["stages"]["source_rollback"]["status"], "PASS");
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(f.state.join("plugins.json")).unwrap()).unwrap()
            ["installed"],
        json!([])
    );
}

#[test]
fn refresh_failure_before_plugin_add_recovers_fresh_source() {
    let f = Fixture::new();
    let out = f.run(&[], Some("GROUNDLINE_TEST_FAIL_REFRESH"));
    let report = package::receipt(&out);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(report["stages"]["marketplace_refresh"]["exit_code"], 19);
    assert_eq!(report["stages"]["source_rollback"]["status"], "PASS");
    assert!(
        !fs::read_to_string(&f.calls)
            .unwrap()
            .contains("plugin remove")
    );
}

#[test]
fn wrong_product_and_unsupported_marketplace_stop_before_writes() {
    let f = Fixture::new();
    f.assert_blocked("product_selection", f.run(&["--profile", "insights"], None));
    let mut rows: Value =
        serde_json::from_slice(&fs::read(f.state.join("market-after.json")).unwrap()).unwrap();
    rows["marketplaces"][0]["marketplaceSource"]["source"] =
        json!("https://example.invalid/unreviewed.git");
    fs::write(
        f.state.join("market.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
    f.assert_blocked("native_source", f.run(&[], None));
}

#[test]
fn disabled_existing_core_keeps_its_choice_without_readding() {
    let f = Fixture::new();
    let mut rows: Value =
        serde_json::from_slice(&fs::read(f.state.join("plugin-after.json")).unwrap()).unwrap();
    rows["installed"][0]["enabled"] = json!(false);
    fs::write(
        f.state.join("plugins.json"),
        serde_json::to_vec(&rows).unwrap(),
    )
    .unwrap();
    fs::copy(
        f.state.join("market-after.json"),
        f.state.join("market.json"),
    )
    .unwrap();
    let out = f.run(&[], None);
    assert!(out.status.success(), "{out:?}");
    assert!(
        !fs::read_to_string(&f.calls)
            .unwrap()
            .contains("plugin add ")
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(f.state.join("plugins.json")).unwrap()).unwrap(),
        rows
    );
}
