//! Exercise the public installer with real setup/artifact validation and a fake
//! native provider. Actual marketplace/network delivery is a separate release gate.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::{TempDir, tempdir};

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
    codex: PathBuf,
    installed: PathBuf,
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
        let bytes = fs::read(binary).unwrap();
        let hash = format!("{:x}", Sha256::digest(&bytes));
        for package in [&source, &installed] {
            fs::create_dir_all(package.join(".codex-plugin")).unwrap();
            fs::write(
                package.join(".codex-plugin/plugin.json"),
                format!("{{\"name\":\"groundline\",\"version\":\"{version}\"}}"),
            )
            .unwrap();
            let bin = package.join("bin").join(target);
            fs::create_dir_all(&bin).unwrap();
            fs::copy(binary, bin.join(executable)).unwrap();
            fs::write(
                bin.join(format!("{executable}.sha256")),
                format!("{hash}  {executable}\n"),
            )
            .unwrap();
            fs::write(bin.join("manifest.json"), serde_json::to_vec(&json!({
                "schema_version":1,"kind":"groundline-binary-artifact","groundline_version":version,
                "target":target,"executable":executable,"size_bytes":bytes.len(),"sha256":hash
            })).unwrap()).unwrap();
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
            fs::write(&codex, "@echo off\r\necho %*>>\"%GROUNDLINE_TEST_CALLS%\"\r\nif \"%~1 %~2\"==\"debug models\" (\r\n type \"%GROUNDLINE_TEST_CATALOG%\"\r\n if \"%GROUNDLINE_TEST_FAIL_CATALOG%\"==\"1\" exit /b 1\r\n)\r\nexit /b 0\r\n").unwrap();
        } else {
            fs::write(&codex, "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$GROUNDLINE_TEST_CALLS\"\nif [ \"$1 $2\" = 'debug models' ]; then\n cat \"$GROUNDLINE_TEST_CATALOG\"\n [ \"${GROUNDLINE_TEST_FAIL_CATALOG:-0}\" != 1 ] || exit 1\nfi\n").unwrap();
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
            catalog,
            calls,
        }
    }
    fn run(&self, fail_catalog: bool) -> Output {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("powershell.exe");
            command
                .args(["-NoProfile", "-File"])
                .arg(self.root.join("install.ps1"))
                .arg("-Codex");
            command
        } else {
            let mut command = Command::new("bash");
            command.arg(self.root.join("install.sh"));
            command
        };
        command
            .arg(&self.codex)
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
}

#[test]
fn installer_applies_and_checks_without_another_manual_setup_request() {
    let f = Fixture::new();
    fs::write(
        f.home.join("config.toml"),
        "model_context_window=0\nservice_tier='fast'\n",
    )
    .unwrap();
    let output = f.run(false);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
    assert!(f.run(false).status.success());
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
fn failed_catalog_with_valid_partial_output_cannot_change_configuration() {
    let f = Fixture::new();
    assert!(!f.run(true).status.success());
    assert!(!f.home.join("config.toml").exists());
    assert!(!fs::read_to_string(&f.calls).unwrap().contains("doctor"));
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
            .contains("debug models")
    );
}
