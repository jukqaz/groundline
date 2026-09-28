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
        let executable = if cfg!(windows) {
            "groundline.exe"
        } else {
            "groundline"
        };
        let source = root.join("plugins/groundline");
        let installed = home.join(format!("plugins/cache/groundline/groundline/{version}"));
        for package in [&source, &installed] {
            package::stage(package, binary, "groundline", target);
        }
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for file in ["install.sh", "install.ps1"] {
            fs::copy(repo.join(file), root.join(file)).unwrap();
        }
        let catalog = temp.path().join("models.json");
        fs::write(&catalog, r#"{"models":[{"slug":"gpt-6-astra","default_reasoning_level":"xhigh","supported_reasoning_levels":[{"effort":"xhigh"}]}]}"#).unwrap();
        let calls = temp.path().join("calls.txt");
        let codex = temp.path().join(if cfg!(windows) {
            "fake codex.cmd"
        } else {
            "fake codex.sh"
        });
        if cfg!(windows) {
            fs::write(&codex, "@echo off\r\necho %*>>\"%GROUNDLINE_TEST_CALLS%\"\r\nif \"%~3\"==\"--help\" exit /b 0\r\nif \"%~1 %~2\"==\"debug models\" (\r\n type \"%GROUNDLINE_TEST_CATALOG%\"\r\n if \"%GROUNDLINE_TEST_FAIL_CATALOG%\"==\"1\" exit /b 1\r\n)\r\nexit /b 0\r\n").unwrap();
        } else {
            fs::write(&codex, "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$GROUNDLINE_TEST_CALLS\"\n[ \"${3:-}\" != --help ] || exit 0\nif [ \"$1 $2\" = 'debug models' ]; then\n cat \"$GROUNDLINE_TEST_CATALOG\"\n [ \"${GROUNDLINE_TEST_FAIL_CATALOG:-0}\" != 1 ] || exit 1\nfi\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&codex, fs::Permissions::from_mode(0o700)).unwrap();
            }
        }
        Self {
            _temp: temp,
            root,
            home,
            codex,
            installed: installed.join("bin").join(target).join(executable),
            target: target.to_owned(),
            catalog,
            calls,
        }
    }
    fn run(&self, fail_catalog: bool) -> Output {
        self.run_options(fail_catalog, &[])
    }

    fn run_options(&self, fail_catalog: bool, options: &[&str]) -> Output {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("powershell.exe");
            command
                .args(["-NoProfile", "-File"])
                .arg(self.root.join("install.ps1"))
                .arg("-Codex");
            command
        } else {
            let mut command = Command::new("bash");
            command.arg(self.root.join("install.sh")).arg("--codex");
            command
        };
        command.arg(&self.codex);
        for option in options {
            let argument = if cfg!(windows) {
                match *option {
                    "--profile" => "-Profile",
                    "--model" => "-Model",
                    "--effort" => "-Effort",
                    "--restore-native-context" => "-RestoreNativeContext",
                    other => other,
                }
            } else {
                option
            };
            command.arg(argument);
        }
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
        let calls = fs::read_to_string(&self.calls).unwrap_or_default();
        #[cfg(windows)]
        {
            let probe = Command::new("powershell.exe")
                .args([
                    "-NoProfile",
                    "-Command",
                    "& $env:GROUNDLINE_TEST_CODEX debug models",
                ])
                .env("GROUNDLINE_TEST_CODEX", &self.codex)
                .env("GROUNDLINE_TEST_CATALOG", &self.catalog)
                .env("GROUNDLINE_TEST_CALLS", &self.calls)
                .output()
                .unwrap();
            return format!(
                "calls={calls}; synthetic_catalog_exit={}; valid_json={}; byte_count={}; prefix={:x?}",
                probe.status,
                serde_json::from_slice::<Value>(&probe.stdout).is_ok(),
                probe.stdout.len(),
                &probe.stdout[..probe.stdout.len().min(16)]
            );
        }
        #[cfg(not(windows))]
        calls
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
        let path_dir = self.home.join("path-bin");
        self.fake_codex(&path_dir.join("codex"), "PATH_CODEX");
        let mut paths = vec![path_dir];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        let mut command = Command::new("bash");
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
    let failure = if cfg!(windows) {
        "if \"%~1 %~2\"==\"--strict-config doctor\" exit /b 42\r\n"
    } else {
        "if [ \"$1 $2\" = '--strict-config doctor' ]; then exit 42; fi\n"
    };
    let changed = if cfg!(windows) {
        original
            .replacen("@echo off\n", &format!("@echo off\n{failure}"), 1)
            .replacen("@echo off\r\n", &format!("@echo off\r\n{failure}"), 1)
    } else {
        original.replacen("#!/bin/sh\n", &format!("#!/bin/sh\n{failure}"), 1)
    };
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
