//! Real Codex, real packages, isolated home; deliberately no account or service credentials.
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

#[path = "common/package.rs"]
mod package;

fn checked(command: &mut Command, step: &str) -> Output {
    let output = command.output().expect("start native command");
    assert!(
        output.status.success(),
        "{step}: exit {:?}; stderr bytes {}",
        output.status.code(),
        output.stderr.len()
    );
    output
}

#[test]
#[ignore = "requires GROUNDLINE_NATIVE_CODEX and GROUNDLINE_NATIVE_INSIGHTS_BINARY; CI runs this on six native hosts"]
fn real_codex_installs_both_packages_and_preserves_model_choices() {
    let codex =
        PathBuf::from(std::env::var_os("GROUNDLINE_NATIVE_CODEX").expect("real Codex path"));
    let insights = PathBuf::from(
        std::env::var_os("GROUNDLINE_NATIVE_INSIGHTS_BINARY").expect("built Insights path"),
    );
    assert!(codex.is_file() && insights.is_file());
    let root = tempdir().unwrap();
    let home = root.path().join("fresh home 한글");
    fs::create_dir(&home).unwrap();
    let market = root.path().join("reviewed marketplace 한글");
    fs::create_dir_all(market.join(".agents/plugins")).unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::copy(
        repo.join(".agents/plugins/marketplace.json"),
        market.join(".agents/plugins/marketplace.json"),
    )
    .unwrap();
    let core = Path::new(env!("CARGO_BIN_EXE_groundline"));
    let platform: Value = serde_json::from_slice(
        &checked(Command::new(core).args(["platform", "--json"]), "platform").stdout,
    )
    .unwrap();
    let target = platform["target"].as_str().unwrap();
    for (name, binary) in [
        ("groundline", core),
        ("groundline-insights", insights.as_path()),
    ] {
        package::stage(&market.join("plugins").join(name), binary, name, target);
    }
    for file in ["install.sh", "install.ps1", ".gitattributes"] {
        fs::copy(repo.join(file), market.join(file)).unwrap();
    }
    // Route only this fixture's Git transport to the reviewed local stable
    // distribution. The real installer and real Codex commands are unmodified.
    let git_config = root.path().join("fixture.gitconfig");
    fs::write(&git_config, format!(
        "[url \"{}\"]\n insteadOf = https://github.com/jukqaz/groundline.git\n[user]\n name = GroundLine Test\n email = groundline-test@example.invalid\n[commit]\n gpgsign = false\n",
        market.to_str().unwrap().replace('\\', "/")
    )).unwrap();
    for args in [
        vec!["init", "--initial-branch=stable"],
        vec!["add", "."],
        vec!["commit", "-m", "test: stage native installation fixture"],
    ] {
        checked(
            Command::new("git")
                .current_dir(&market)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", &git_config)
                .args(args),
            "stage local stable distribution",
        );
    }
    let native = || {
        let mut command = Command::new(&codex);
        command
            .env("CODEX_HOME", &home)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .current_dir(root.path());
        command
    };
    for _ in 0..2 {
        let mut installer = if cfg!(windows) {
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-File"])
                .arg(market.join("install.ps1"))
                .args(["-Profile", "both", "-Codex"]);
            c
        } else {
            let mut c = Command::new("bash");
            c.arg(market.join("install.sh"))
                .args(["--profile", "both", "--codex"]);
            c
        };
        let output = installer
            .arg(&codex)
            .env("CODEX_HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &git_config)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .env_remove("GROUNDLINE_RUNTIME_FAMILY")
            .env_remove("GROUNDLINE_EXECUTION_MODE")
            .env_remove("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")
            .current_dir(root.path())
            .output()
            .unwrap();
        let receipt = package::receipt(&output);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{receipt}; {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(receipt["status"], "ACTION_REQUIRED", "{receipt}");
        for stage in [
            "marketplace_add",
            "marketplace_refresh",
            "install_groundline",
            "install_groundline-insights",
            "verify_groundline",
            "verify_groundline-insights",
            "catalog",
            "settings",
        ] {
            assert_eq!(receipt["stages"][stage]["status"], "PASS", "{receipt}");
        }
        assert_eq!(
            receipt["stages"]["insights_setup"]["status"],
            "ACTION_REQUIRED"
        );
    }
    let cache = home.join("plugins/cache/groundline");
    for name in ["groundline", "groundline-insights"] {
        let root = cache.join(name).join(env!("CARGO_PKG_VERSION"));
        let executable = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        };
        let installed = root.join("bin").join(target).join(executable);
        checked(
            Command::new(&installed)
                .args(["provider-smoke", "--plugin-root"])
                .arg(&root)
                .args(["--require-installed", "--json"]),
            "installed package integrity",
        );
    }
    let catalog = checked(native().args(["debug", "models"]), "native catalog").stdout;
    let catalog_path = root.path().join("models.json");
    groundline_runtime::local_file::atomic_write_private(&catalog_path, &catalog).unwrap();
    let config = home.join("config.toml");
    // Native package setup may add its own plugin entries. Keep those entries.
    let original = fs::read_to_string(&config).unwrap_or_default();
    let selected = format!("model='gpt-5.6-sol'\nmodel_reasoning_effort='low'\n{original}");
    groundline_runtime::local_file::atomic_write_private(&config, selected.as_bytes()).unwrap();
    let setup = || {
        let mut command = Command::new(core);
        command
            .args(["setup", "--codex-home"])
            .arg(&home)
            .arg("--catalog")
            .arg(&catalog_path)
            .arg("--apply");
        command
    };
    checked(&mut setup(), "preserve real 5.6 choice");
    assert_eq!(fs::read(&config).unwrap(), selected.as_bytes());
    checked(
        setup().args(["--preset", "astra"]),
        "explicit Astra selection",
    );
    let after = fs::read(&config).unwrap();
    let parsed: toml::Table = toml::from_str(std::str::from_utf8(&after).unwrap()).unwrap();
    assert_eq!(parsed["model"].as_str(), Some("gpt-6-astra"));
    checked(setup().args(["--preset", "astra"]), "repeat Astra setup");
    assert_eq!(fs::read(&config).unwrap(), after);
    let pending = Command::new(insights)
        .args(["setup", "--codex-home"])
        .arg(&home)
        .output()
        .unwrap();
    assert_eq!(pending.status.code(), Some(2));
    let pending: Value = serde_json::from_slice(&pending.stdout).unwrap();
    assert_eq!(pending["stages"]["collection_consented"], false);
    assert!(!home.join("groundline/insights").exists());
    println!(
        "public installer with real Codex and local stable Git transport: install/repeat, native catalog, 5.6 preservation, Astra opt-in, and inert Insights PASS; authenticated task/private delivery UNVERIFIED"
    );
}
