//! Exercise the public installer with real setup/artifact validation and a fake
//! native provider. Actual marketplace/network delivery is a separate release gate.
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::{TempDir, tempdir};

mod common;
#[path = "common/package.rs"]
mod package;
use package::receipt;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
    codex: PathBuf,
    installed: PathBuf,
    target: String,
    catalog: PathBuf,
    calls: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempdir().unwrap();
        let root = temp.path().join("reviewed stable distribution");
        let home = temp.path().join("different pc home");
        fs::create_dir_all(&home).unwrap();
        let binary = Path::new(env!("CARGO_BIN_EXE_groundline"));
        let platform: Value = serde_json::from_slice(
            &Command::new(binary)
                .args(["platform", "--json"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let target = platform["target"].as_str().unwrap();
        let version = env!("CARGO_PKG_VERSION");
        let executable = "groundline";
        let source = root.join("plugins/groundline");
        let installed = home.join(format!("plugins/cache/groundline/groundline/{version}"));
        for package in [&source, &installed] {
            package::stage(package, binary, "groundline", target);
        }
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        fs::copy(repo.join("install.sh"), root.join("install.sh")).unwrap();
        let empty = temp.path().join("empty-market.json");
        fs::write(&empty, r#"{"marketplaces":[]}"#).unwrap();
        fs::write(temp.path().join("market.json"), fs::read(&empty).unwrap()).unwrap();
        fs::write(temp.path().join("market-after.json"), serde_json::to_vec(&serde_json::json!({"marketplaces":[{
            "name":"groundline","root":root,"marketplaceSource":{"sourceType":"git","source":"https://github.com/jukqaz/groundline.git"}
        }]})).unwrap()).unwrap();
        fs::write(
            temp.path().join("empty-plugins.json"),
            r#"{"installed":[],"available":[]}"#,
        )
        .unwrap();
        fs::write(
            temp.path().join("plugins.json"),
            r#"{"installed":[],"available":[]}"#,
        )
        .unwrap();
        for name in ["groundline", "groundline-insights"] {
            fs::write(temp.path().join(format!("{name}-after.json")), serde_json::to_vec(&serde_json::json!({"installed":[{
                "name":name,"pluginId":format!("{name}@groundline"),"marketplaceName":"groundline","version":version,"installed":true,"enabled":true,
                "source":{"source":"local","path":root.join("plugins").join(name)},
                "marketplaceSource":{"sourceType":"git","source":"https://github.com/jukqaz/groundline.git"}
            }],"available":[]})).unwrap()).unwrap();
        }
        let catalog = temp.path().join("models.json");
        fs::write(&catalog, r#"{"models":[{"slug":"gpt-6-astra","default_reasoning_level":"xhigh","supported_reasoning_levels":[{"effort":"xhigh"}]}]}"#).unwrap();
        let calls = temp.path().join("calls.txt");
        let codex = temp.path().join("fake codex.sh");
        fs::write(&codex, "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$GROUNDLINE_TEST_CALLS\"\n[ \"${3:-}\" != --help ] && [ \"${4:-}\" != --help ] || exit 0\nfixture=$(dirname \"$GROUNDLINE_TEST_CALLS\")\ncase \"$1 $2 ${3:-}\" in\n 'plugin marketplace list') cat \"$fixture/market.json\" ;;\n 'plugin list --json') cat \"$fixture/plugins.json\" ;;\n 'plugin marketplace add') cp \"$fixture/market-after.json\" \"$fixture/market.json\" ;;\n 'plugin marketplace remove') cp \"$fixture/empty-market.json\" \"$fixture/market.json\" ;;\n 'plugin add groundline@groundline') cp \"$fixture/groundline-after.json\" \"$fixture/plugins.json\" ;;\n 'plugin add groundline-insights@groundline') cp \"$fixture/groundline-insights-after.json\" \"$fixture/plugins.json\" ;;\n plugin\\ remove\\ *) cp \"$fixture/empty-plugins.json\" \"$fixture/plugins.json\" ;;\nesac\nif [ \"$1 $2\" = 'debug models' ]; then\n cat \"$GROUNDLINE_TEST_CATALOG\"\n [ \"${GROUNDLINE_TEST_FAIL_CATALOG:-0}\" != 1 ] || exit 1\nfi\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&codex, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let result = Self {
            _temp: temp,
            root,
            home,
            codex,
            installed: installed.join("bin").join(target).join(executable),
            target: target.to_owned(),
            catalog,
            calls,
        };
        result.commit_distribution();
        result
    }
    fn commit_distribution(&self) {
        for args in [
            vec!["init"],
            vec!["config", "core.autocrlf", "false"],
            vec!["add", "--all"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-m",
                "reviewed fixture",
            ],
        ] {
            let output = Command::new("git")
                .current_dir(&self.root)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    fn run(&self, fail_catalog: bool) -> Output {
        self.run_options(fail_catalog, &[])
    }

    fn run_options(&self, fail_catalog: bool, options: &[&str]) -> Output {
        // Exercise the macOS system shell even when PATH contains newer Bash.
        let mut command = Command::new(if cfg!(target_os = "macos") {
            "/bin/bash"
        } else {
            "bash"
        });
        command
            .arg(self.root.join("install.sh"))
            .arg("--codex")
            .arg(&self.codex)
            .args(options);
        command
            .env("CODEX_HOME", &self.home)
            .env("GROUNDLINE_TEST_CATALOG", &self.catalog)
            .env("GROUNDLINE_TEST_CALLS", &self.calls)
            .env(
                "GROUNDLINE_TEST_FAIL_CATALOG",
                if fail_catalog { "1" } else { "0" },
            )
            .output()
            .unwrap()
    }
    fn failure_diagnostics(&self) -> String {
        fs::read_to_string(&self.calls).unwrap_or_default()
    }

    #[cfg(target_os = "macos")]
    fn fake_codex(&self, path: &Path, marker: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let source = fs::read_to_string(&self.codex).unwrap();
        let script = source.replacen(
            "#!/bin/sh\n",
            &format!("#!/bin/sh\nprintf '{marker}\\n' >> \"$GROUNDLINE_TEST_CALLS\"\n"),
            1,
        );
        fs::write(path, script).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }

    #[cfg(target_os = "macos")]
    fn fake_app(&self, applications: &Path, name: &str, bundle_id: &str, layouts: &[&str]) {
        let app = applications.join(format!("{name}.app"));
        let contents = app.join("Contents");
        fs::create_dir_all(&contents).unwrap();
        fs::write(
            contents.join("Info.plist"),
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>{bundle_id}</string></dict></plist>\n"),
        )
        .unwrap();
        for (index, layout) in layouts.iter().enumerate() {
            self.fake_codex(
                &contents.join("Resources").join(layout),
                &format!("APP_{name}_{index}"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    fn fake_insights_package(&self) {
        use std::os::unix::fs::PermissionsExt;
        let relative = Path::new("bin")
            .join(&self.target)
            .join("groundline-insights");
        let script = format!(
            "#!/bin/sh\ncase \"$1\" in\n --version) echo 'groundline-insights {}' ;;\n setup) printf 'FAMILY=%s\\n' \"${{GROUNDLINE_RUNTIME_FAMILY-unset}}\" >> \"$GROUNDLINE_TEST_CALLS\" ;;\nesac\n",
            env!("CARGO_PKG_VERSION")
        );
        let source = self
            .root
            .join("plugins/groundline-insights")
            .join(&relative);
        let installed = self
            .home
            .join(format!(
                "plugins/cache/groundline/groundline-insights/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .join(&relative);
        for path in [source, installed] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, &script).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(
                path.with_file_name("groundline-insights.sha256"),
                b"synthetic artifact fixture",
            )
            .unwrap();
            fs::write(path.with_file_name("manifest.json"), b"{}").unwrap();
            let manifest = path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(".codex-plugin");
            fs::create_dir_all(&manifest).unwrap();
            fs::write(manifest.join("plugin.json"), serde_json::to_vec(&serde_json::json!({"name":"groundline-insights","version":env!("CARGO_PKG_VERSION")})).unwrap()).unwrap();
        }
    }

    #[cfg(target_os = "macos")]
    fn run_with_app_search(&self, applications: &Path, explicit_codex: Option<&Path>) -> Output {
        let script_path = self.root.join("install.sh");
        let original = fs::read_to_string(&script_path).unwrap();
        let old = "for applications in /Applications \"$HOME/Applications\"; do";
        let replacement = format!(
            "for applications in \"{}\" \"$HOME/Applications\"; do",
            applications.display()
        );
        if original.contains(old) {
            fs::write(&script_path, original.replacen(old, &replacement, 1)).unwrap();
        } else {
            assert!(original.contains(&replacement));
        }
        self.commit_distribution();
        let path_dir = self.home.join("path-bin");
        self.fake_codex(&path_dir.join("codex"), "PATH_CODEX");
        let mut paths = vec![path_dir];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        let mut command = Command::new("/bin/bash");
        command.arg(script_path);
        if let Some(path) = explicit_codex {
            command.arg("--codex").arg(path);
        }
        command
            .args(["--profile", "insights"])
            .env("HOME", &self.home)
            .env("PATH", std::env::join_paths(paths).unwrap())
            .env("CODEX_HOME", &self.home)
            .env("GROUNDLINE_TEST_CATALOG", &self.catalog)
            .env("GROUNDLINE_TEST_CALLS", &self.calls)
            .env_remove("GROUNDLINE_RUNTIME_FAMILY")
            .env_remove("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")
            .output()
            .unwrap()
    }
}

#[test]
fn installer_applies_and_checks_without_another_manual_setup_request() {
    let f = Fixture::new();
    common::write_owned_config(
        &f.home.join("config.toml"),
        b"model_context_window=0\nservice_tier='fast'\n",
    );
    let output = f.run_options(
        false,
        &[
            "--model",
            "gpt-6-astra",
            "--effort",
            "xhigh",
            "--service-tier",
            "default",
            "--restore-native-context",
        ],
    );
    assert!(
        output.status.success(),
        "{}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        f.failure_diagnostics()
    );
    let config: toml::Table =
        toml::from_str(&fs::read_to_string(f.home.join("config.toml")).unwrap()).unwrap();
    assert_eq!(config["model"].as_str(), Some("gpt-6-astra"));
    assert_eq!(config["model_reasoning_effort"].as_str(), Some("xhigh"));
    assert_eq!(config["service_tier"].as_str(), Some("default"));
    assert!(!config.contains_key("model_context_window"));
    let calls = fs::read_to_string(&f.calls).unwrap();
    assert!(calls.contains("plugin add groundline@groundline --json"));
    assert!(calls.contains("--strict-config doctor --summary --no-color --ascii"));
    assert!(!calls.contains("groundline-insights"));
    let before = fs::read(f.home.join("config.toml")).unwrap();
    assert!(
        f.run_options(
            false,
            &[
                "--model",
                "gpt-6-astra",
                "--effort",
                "xhigh",
                "--service-tier",
                "default",
                "--restore-native-context"
            ]
        )
        .status
        .success()
    );
    assert_eq!(before, fs::read(f.home.join("config.toml")).unwrap());
    assert_eq!(
        fs::read_dir(&f.home)
            .unwrap()
            .filter(|f| f
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("config.toml.groundline-backup-"))
            .count(),
        1
    );
}

#[test]
fn installer_accepts_crlf_checksum_records() {
    let f = Fixture::new();
    let relative = f
        .installed
        .strip_prefix(f.home.join(format!(
            "plugins/cache/groundline/groundline/{}",
            env!("CARGO_PKG_VERSION")
        )))
        .unwrap();
    for binary in [
        f.root.join("plugins/groundline").join(relative),
        f.installed.clone(),
    ] {
        let checksum = binary.with_file_name(format!(
            "{}.sha256",
            binary.file_name().unwrap().to_str().unwrap()
        ));
        let text = fs::read_to_string(&checksum).unwrap();
        fs::write(checksum, text.replace('\n', "\r\n")).unwrap();
    }
    f.commit_distribution();
    let result = f.run(false);
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        fs::read_to_string(&f.calls)
            .unwrap()
            .contains("--strict-config doctor")
    );
}

#[test]
fn failed_catalog_with_valid_partial_output_cannot_change_configuration() {
    let f = Fixture::new();
    assert!(!f.run(true).status.success());
    assert!(!f.home.join("config.toml").exists());
}

#[test]
fn mismatched_installed_artifact_stops_before_setup() {
    let f = Fixture::new();
    fs::write(&f.installed, "mismatched artifact").unwrap();
    assert!(!f.run(false).status.success());
    assert!(!f.home.join("config.toml").exists());
    assert!(
        !fs::read_to_string(&f.calls)
            .unwrap()
            .contains("debug models\n")
    );
}

#[test]
fn invalid_distribution_checksum_stops_before_native_marketplace_changes() {
    let f = Fixture::new();
    let original = b"model='gpt-6-astra'\nmodel_reasoning_effort='xhigh'\nservice_tier='fast'\n";
    common::write_owned_config(&f.home.join("config.toml"), original);
    let executable = f.installed.file_name().unwrap().to_str().unwrap();
    let checksum = f
        .root
        .join("plugins/groundline/bin")
        .join(&f.target)
        .join(format!("{executable}.sha256"));
    fs::write(checksum, format!("{}  {executable}\n", "0".repeat(64))).unwrap();
    let output = f.run(false);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        receipt(&output)["stages"]["distribution_groundline"]["status"],
        "FAIL"
    );
    let calls = fs::read_to_string(&f.calls).unwrap();
    assert!(!calls.contains("plugin marketplace add "), "{calls}");
    assert!(
        !calls.contains("plugin marketplace upgrade groundline"),
        "{calls}"
    );
    assert!(
        !calls.contains("plugin add groundline@groundline"),
        "{calls}"
    );
    assert_eq!(fs::read(f.home.join("config.toml")).unwrap(), original);
}

#[test]
fn default_install_preserves_existing_choices_and_emits_stage_results() {
    let f = Fixture::new();
    let original = b"model='gpt-6-astra'\nmodel_reasoning_effort='xhigh'\nservice_tier='fast'\n";
    common::write_owned_config(&f.home.join("config.toml"), original);
    let result = f.run(false);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(fs::read(f.home.join("config.toml")).unwrap(), original);
    let report = receipt(&result);
    assert_eq!(report["status"], "PASS");
    assert_eq!(report["stages"]["settings"]["status"], "PASS");
    assert_eq!(report["stages"]["insights_setup"]["status"], "NOT_SELECTED");
}

#[test]
fn doctor_failure_preserves_completed_settings_and_can_resume() {
    let f = Fixture::new();
    let original = fs::read_to_string(&f.codex).unwrap();
    let failure = "if [ \"$1 $2\" = '--strict-config doctor' ]; then exit 42; fi\n";
    let changed = original.replacen("#!/bin/sh\n", &format!("#!/bin/sh\n{failure}"), 1);
    fs::write(&f.codex, changed).unwrap();
    let result = f.run_options(false, &["--model", "gpt-6-astra"]);
    assert_eq!(result.status.code(), Some(2), "{result:?}");
    let report = receipt(&result);
    assert_eq!(report["status"], "ACTION_REQUIRED");
    assert_eq!(report["stages"]["settings"]["status"], "PASS");
    assert_eq!(report["stages"]["native_doctor"]["exit_code"], 42);
    let before = fs::read(f.home.join("config.toml")).unwrap();
    fs::write(&f.codex, original).unwrap();
    let result = f.run_options(false, &["--model", "gpt-6-astra"]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(fs::read(f.home.join("config.toml")).unwrap(), before);
    assert_eq!(receipt(&result)["status"], "PASS");
}

#[cfg(target_os = "macos")]
#[test]
fn implicit_installer_prefers_verified_new_app_cli_and_marks_app_family() {
    let f = Fixture::new();
    f.fake_insights_package();
    let applications = f.home.join("test-system-applications");
    f.fake_app(
        &applications,
        "A-Other",
        "example.other",
        &["codex-cli/bin/codex"],
    );
    f.fake_app(
        &applications,
        "Z-RenamedCodex",
        "com.openai.codex",
        &["codex", "codex-cli/bin/codex"],
    );
    let result = f.run_with_app_search(&applications, None);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        receipt(&result)["stages"]["insights_setup"]["status"],
        "PASS"
    );
    let calls = fs::read_to_string(&f.calls).unwrap();
    assert!(calls.contains("APP_Z-RenamedCodex_1"), "{calls}");
    assert!(calls.contains("FAMILY=codex_app"), "{calls}");
    assert!(!calls.contains("APP_Z-RenamedCodex_0"), "{calls}");
    assert!(!calls.contains("APP_A-Other"), "{calls}");
    assert!(!calls.contains("PATH_CODEX"), "{calls}");
}

#[cfg(target_os = "macos")]
#[test]
fn explicit_codex_stays_selected_and_legacy_app_layout_remains_recognized() {
    let f = Fixture::new();
    f.fake_insights_package();
    let applications = f.home.join("test-system-applications");
    let user_applications = f.home.join("Applications");
    f.fake_app(
        &user_applications,
        "CodexOldLayout",
        "com.openai.codex",
        &["codex"],
    );
    let explicit = f.codex.clone();
    let result = f.run_with_app_search(&applications, Some(&explicit));
    assert!(result.status.success(), "{result:?}");
    let calls = fs::read_to_string(&f.calls).unwrap();
    assert!(calls.contains("FAMILY=unset"), "{calls}");
    assert!(!calls.contains("APP_CodexOldLayout"), "{calls}");
    fs::write(&f.calls, "").unwrap();
    let result = f.run_with_app_search(&applications, None);
    assert!(result.status.success(), "{result:?}");
    let calls = fs::read_to_string(&f.calls).unwrap();
    assert!(calls.contains("APP_CodexOldLayout_0"), "{calls}");
    assert!(calls.contains("FAMILY=codex_app"), "{calls}");
    assert!(!calls.contains("PATH_CODEX"), "{calls}");
}

#[test]
fn unsupported_native_source_stops_before_writes() {
    let f = Fixture::new();
    let path = f.calls.parent().unwrap().join("market-after.json");
    let mut source: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    source["marketplaces"][0]["marketplaceSource"]["source"] =
        serde_json::json!("https://example.invalid/unreviewed.git");
    fs::write(
        f.calls.parent().unwrap().join("market.json"),
        serde_json::to_vec(&source).unwrap(),
    )
    .unwrap();
    let result = f.run(false);
    let report = receipt(&result);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(report["stages"]["native_source"]["status"], "FAIL");
    assert!(report["stages"].get("marketplace_add").is_none());
    assert!(!f.home.join("config.toml").exists());
}

#[test]
fn uncommitted_distribution_stops_before_native_writes() {
    let f = Fixture::new();
    fs::write(f.root.join("unreviewed.txt"), b"uncommitted distribution").unwrap();
    let result = f.run(false);
    let report = receipt(&result);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(report["stages"]["distribution_revision"]["status"], "FAIL");
    assert!(report["stages"].get("marketplace_add").is_none());
    assert!(!f.home.join("config.toml").exists());
}

#[test]
fn failed_new_plugin_add_does_not_block_source_rollback_with_remove_not_installed() {
    let f = Fixture::new();
    let original = fs::read_to_string(&f.codex).unwrap();
    let script = original.replacen("#!/bin/sh\n", "#!/bin/sh\nif [ \"$1 $2 ${3:-}\" = 'plugin add groundline@groundline' ]; then exit 19; fi\n", 1);
    fs::write(&f.codex, script).unwrap();
    let result = f.run(false);
    let report = receipt(&result);
    assert_eq!(result.status.code(), Some(1), "{report}");
    assert_eq!(report["stages"]["install_groundline"]["status"], "FAIL");
    assert_eq!(report["stages"]["source_rollback"]["status"], "PASS");
    assert_eq!(report["rollback"], "fresh_registration_removed");
    let calls = fs::read_to_string(&f.calls).unwrap();
    assert!(!calls.contains("plugin remove groundline@groundline"));
    assert!(calls.contains("plugin marketplace remove groundline --json"));
    assert!(!f.home.join("config.toml").exists());
}

#[cfg(unix)]
#[test]
fn failed_refresh_before_any_plugin_add_restores_source_and_emits_receipt() {
    for existing in [false, true] {
        let f = Fixture::new();
        if existing {
            let installed = f.run(false);
            assert!(
                installed.status.success(),
                "{}",
                String::from_utf8_lossy(&installed.stderr)
            );
        }
        let before_config = fs::read(f.home.join("config.toml")).ok();
        let before_plugins = fs::read(f.calls.parent().unwrap().join("plugins.json")).unwrap();
        let original = fs::read_to_string(&f.codex).unwrap();
        let script = original.replacen(
            "#!/bin/sh\n",
            r#"#!/bin/sh
marker="$GROUNDLINE_TEST_CALLS.refresh-failed"
if [ "$1 $2 ${3:-} ${4:-}" = 'plugin marketplace upgrade groundline' ] && [ ! -e "$marker" ]; then
 : > "$marker"
 exit 19
fi
"#,
            1,
        );
        fs::write(&f.codex, script).unwrap();
        let result = f.run(false);
        let report = receipt(&result);
        assert_eq!(result.status.code(), Some(1), "{report}");
        assert_eq!(report["stages"]["marketplace_refresh"]["exit_code"], 19);
        assert_eq!(report["stages"]["source_rollback"]["status"], "PASS");
        assert_eq!(
            report["rollback"],
            if existing {
                "previous_commit_pinned"
            } else {
                "fresh_registration_removed"
            }
        );
        assert!(report["stages"].get("install_groundline").is_none());
        assert_eq!(fs::read(f.home.join("config.toml")).ok(), before_config);
        assert_eq!(
            fs::read(f.calls.parent().unwrap().join("plugins.json")).unwrap(),
            before_plugins
        );
        assert!(!String::from_utf8_lossy(&result.stderr).contains("unbound variable"));
    }
}
