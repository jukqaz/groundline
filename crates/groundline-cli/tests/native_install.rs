//! Real Codex, real packages, isolated home; deliberately no account or service credentials.
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[path = "common/package.rs"]
mod package;

struct HealthServer {
    endpoint: String,
    revision: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl HealthServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let revision = Arc::new(AtomicU64::new(7));
        let stop = Arc::new(AtomicBool::new(false));
        let server_revision = revision.clone();
        let server_stop = stop.clone();
        let thread = thread::spawn(move || {
            'connections: while !server_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut buffer = [0; 1024];
                        while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                            let n = match stream.read(&mut buffer) {
                                Ok(0) if request.is_empty() => continue 'connections,
                                Ok(n) => n,
                                Err(error)
                                    if request.is_empty()
                                        && matches!(
                                            error.kind(),
                                            std::io::ErrorKind::WouldBlock
                                                | std::io::ErrorKind::TimedOut
                                        ) =>
                                {
                                    continue 'connections;
                                }
                                Err(error) => panic!("fixture health request: {error}"),
                            };
                            assert!(n > 0 && request.len() < 8192);
                            request.extend_from_slice(&buffer[..n]);
                        }
                        let request = String::from_utf8(request).unwrap();
                        assert!(request.starts_with("GET /healthz HTTP/1.1\r\n"));
                        assert!(!request.to_ascii_lowercase().contains("authorization:"));
                        let body = serde_json::json!({"storage_ready":true,"ingest_capabilities":{
                            "basic_schema_versions":[5],"basic_contract_revision":server_revision.load(Ordering::Relaxed)
                        }}).to_string();
                        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(error) => panic!("fixture health listener: {error}"),
                }
            }
        });
        Self {
            endpoint,
            revision,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for HealthServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let result = self.thread.take().unwrap().join();
        if !thread::panicking() {
            result.unwrap();
        }
    }
}

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

fn saved_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            (
                entry.path().strip_prefix(root).unwrap().to_owned(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

fn installed_versions(command: &mut Command) -> BTreeMap<String, (String, bool)> {
    let output = checked(
        command.args(["plugin", "list", "--json"]),
        "installed versions",
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    value["installed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|plugin| {
            assert_eq!(plugin["installed"], true);
            (
                plugin["name"].as_str().unwrap().to_owned(),
                (
                    plugin["version"].as_str().unwrap().to_owned(),
                    plugin["enabled"].as_bool().unwrap(),
                ),
            )
        })
        .collect()
}

#[test]
#[ignore = "requires GROUNDLINE_NATIVE_CODEX and GROUNDLINE_NATIVE_INSIGHTS_BINARY; CI runs this on six native hosts"]
fn real_codex_upgrades_metadata_repeats_and_preserves_settings_and_consent() {
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
        // The old package is metadata-only test evidence: both releases use the
        // current test binary. Do not claim execution of an old runtime here.
        let manifest = market
            .join("plugins")
            .join(name)
            .join(".codex-plugin/plugin.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
        value["version"] = serde_json::json!("0.29.0");
        fs::write(manifest, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
    for file in ["install.sh", "install.ps1", ".gitattributes"] {
        fs::copy(repo.join(file), market.join(file)).unwrap();
    }
    // Route only this fixture's Git transport to the reviewed local stable
    // distribution. The real installer and real Codex commands are unmodified.
    let git_config = root.path().join("fixture.gitconfig");
    fs::write(&git_config, format!(
        "[url \"{}\"]\n insteadOf = https://github.com/jukqaz/groundline.git\n insteadOf = git@github.com:jukqaz/groundline.git\n[user]\n name = GroundLine Test\n email = groundline-test@example.invalid\n[commit]\n gpgsign = false\n",
        market.to_str().unwrap().replace('\\', "/")
    )).unwrap();
    for args in [
        vec!["init", "--initial-branch=stable"],
        vec!["add", "."],
        vec!["commit", "-m", "test: stage old native metadata fixture"],
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
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &git_config)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .current_dir(root.path());
        command
    };
    checked(
        native().args([
            "plugin",
            "marketplace",
            "add",
            "git@github.com:jukqaz/groundline.git",
            "--ref",
            "stable",
            "--json",
        ]),
        "register old metadata fixture",
    );
    for name in ["groundline", "groundline-insights"] {
        checked(
            native().args(["plugin", "add", &format!("{name}@groundline"), "--json"]),
            "install old metadata fixture",
        );
        let manifest = home.join(format!(
            "plugins/cache/groundline/{name}/0.29.0/.codex-plugin/plugin.json"
        ));
        let old: Value = serde_json::from_slice(&fs::read(manifest).unwrap()).unwrap();
        assert_eq!(old["version"], "0.29.0");
    }
    let config = home.join("config.toml");
    let mut native_settings: toml::Table =
        toml::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    native_settings["plugins"].as_table_mut().unwrap()["groundline-insights@groundline"]
        .as_table_mut()
        .unwrap()
        .insert("enabled".to_owned(), toml::Value::Boolean(false));
    fs::write(&config, toml::to_string(&native_settings).unwrap()).unwrap();
    let previous_versions = installed_versions(&mut native());
    for name in ["groundline", "groundline-insights"] {
        assert_eq!(
            previous_versions[name],
            ("0.29.0".to_owned(), name == "groundline")
        );
    }
    let config = home.join("config.toml");
    let original = fs::read_to_string(&config).unwrap_or_default();
    let selected = format!(
        "model='gpt-5.6-sol'\nmodel_reasoning_effort='low'\nservice_tier='fast'\n{original}"
    );
    groundline_runtime::local_file::atomic_write_private(&config, selected.as_bytes()).unwrap();

    // Consent and identity are real current-format local state. Collection is
    // disabled; only the read-only compatibility gate reaches local health.
    let health = HealthServer::start();
    let profile_path = root.path().join("owner-profile.json");
    let profile = serde_json::json!({
        "schema_version":7,"kind":"groundline-insights-owner-profile","mode":"private_owner",
        "endpoint":health.endpoint,"enrollment_token":"fixture-token-never-used-for-network-0123456789",
        "automatic_activity_checkpoints":true,"automatic_initial_history_sync":true,
        "collection_scope":"all_activity","checkpoint_min_interval_seconds":900,
        "diagnostic_enabled":false,"trigger_mode":"native_hook_checkpoints"
    });
    groundline_runtime::local_file::atomic_write_private(
        &profile_path,
        &serde_json::to_vec(&profile).unwrap(),
    )
    .unwrap();
    let worker = |verb: &str| {
        let mut command = Command::new(&insights);
        command
            .args(["worker", verb, "--codex-home"])
            .arg(&home)
            .env("GROUNDLINE_RUNTIME_FAMILY", "codex_cli")
            .env("GROUNDLINE_EXECUTION_MODE", "local_headless")
            .env_remove("CODEX_INTERNAL_ORIGINATOR_OVERRIDE");
        command
    };
    checked(
        worker("configure").arg("--input").arg(&profile_path),
        "configure synthetic profile",
    );
    checked(&mut worker("enable"), "create current consent and identity");
    checked(
        &mut worker("disable"),
        "keep collection disabled during upgrade",
    );
    let insights_state = home.join("groundline/insights");
    let saved_state = saved_files(&insights_state);
    assert!(
        saved_state
            .keys()
            .any(|path| path.ends_with("consent.json"))
    );
    assert!(
        saved_state
            .keys()
            .any(|path| path.ends_with("identity.json"))
    );

    for (name, binary) in [
        ("groundline", core),
        ("groundline-insights", insights.as_path()),
    ] {
        package::stage(&market.join("plugins").join(name), binary, name, target);
    }
    for args in [
        vec!["add", "."],
        vec!["commit", "-m", "test: advance current native distribution"],
    ] {
        checked(
            Command::new("git")
                .current_dir(&market)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", &git_config)
                .args(args),
            "advance local stable fixture",
        );
    }
    let reviewed = root.path().join("immutable reviewed distribution");
    checked(
        Command::new("git")
            .args(["clone", "--no-hardlinks"])
            .arg(&market)
            .arg(&reviewed),
        "copy reviewed distribution",
    );
    let candidate_commit = String::from_utf8(
        checked(
            Command::new("git")
                .arg("-C")
                .arg(&reviewed)
                .args(["rev-parse", "HEAD"]),
            "candidate commit",
        )
        .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();
    // stable moves after review. Installation must use candidate_commit, never
    // these future manifests, even though native refresh normally follows stable.
    for name in ["groundline", "groundline-insights"] {
        let path = market
            .join("plugins")
            .join(name)
            .join(".codex-plugin/plugin.json");
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["version"] = serde_json::json!("2026.929.2");
        fs::write(path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    }
    for args in [
        vec!["add", "."],
        vec!["commit", "-m", "move stable beyond reviewed candidate"],
    ] {
        checked(
            Command::new("git")
                .current_dir(&market)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", &git_config)
                .args(args),
            "move stable",
        );
    }
    let wrapper = root.path().join(if cfg!(windows) {
        "fail-once.cmd"
    } else {
        "fail-once.sh"
    });
    if cfg!(windows) {
        fs::write(&wrapper, "@echo off\r\nif \"%~4\"==\"--help\" goto delegate\r\nif exist \"%GROUNDLINE_FAIL_MARKER%\" goto delegate\r\nif not \"%~1 %~2 %~3\"==\"plugin marketplace %GROUNDLINE_FAIL_STAGE%\" goto delegate\r\ntype nul > \"%GROUNDLINE_FAIL_MARKER%\"\r\nif \"%GROUNDLINE_FAIL_STAGE%\"==\"upgrade\" call \"%GROUNDLINE_REAL_CODEX%\" %*\r\nexit /b 19\r\n:delegate\r\ncall \"%GROUNDLINE_REAL_CODEX%\" %*\r\nexit /b %ERRORLEVEL%\r\n").unwrap();
    } else {
        fs::write(&wrapper, "#!/bin/sh\nif [ \"${4:-}\" != --help ] && [ ! -e \"$GROUNDLINE_FAIL_MARKER\" ] && [ \"$1 $2 ${3:-}\" = \"plugin marketplace $GROUNDLINE_FAIL_STAGE\" ]; then\n : > \"$GROUNDLINE_FAIL_MARKER\"\n if [ \"$GROUNDLINE_FAIL_STAGE\" = upgrade ]; then \"$GROUNDLINE_REAL_CODEX\" \"$@\" || exit $?; fi\n exit 19\nfi\nexec \"$GROUNDLINE_REAL_CODEX\" \"$@\"\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }
    let run_installer_with_failure = |profile: &str, fail_stage: Option<&str>| {
        let mut installer = if cfg!(windows) {
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-File"])
                .arg(reviewed.join("install.ps1"))
                .args(["-Profile", profile, "-Codex"]);
            c
        } else {
            let mut c = Command::new("bash");
            c.arg(reviewed.join("install.sh"))
                .args(["--profile", profile, "--codex"]);
            c
        };
        installer
            .arg(if fail_stage.is_some() {
                &wrapper
            } else {
                &codex
            })
            .env("GROUNDLINE_REAL_CODEX", &codex)
            .env("GROUNDLINE_FAIL_STAGE", fail_stage.unwrap_or("none"))
            .env(
                "GROUNDLINE_FAIL_MARKER",
                root.path()
                    .join(format!("failed-{}", fail_stage.unwrap_or("none"))),
            )
            .env("CODEX_HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &git_config)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .env("GROUNDLINE_RUNTIME_FAMILY", "codex_cli")
            .env("GROUNDLINE_EXECUTION_MODE", "local_headless")
            .env_remove("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")
            .current_dir(root.path())
            .output()
            .unwrap()
    };
    let run_installer = |profile: &str| run_installer_with_failure(profile, None);
    let assert_blocked = |output: Output, failed_stage: &str| {
        let receipt = package::receipt(&output);
        assert_eq!(output.status.code(), Some(1), "{receipt}");
        assert_eq!(receipt["stages"][failed_stage]["status"], "FAIL");
        assert!(receipt["stages"].get("marketplace_add").is_none());
        assert_eq!(installed_versions(&mut native()), previous_versions);
        assert_eq!(fs::read(&config).unwrap(), selected.as_bytes());
    };
    let executable = if cfg!(windows) {
        "groundline-insights.exe"
    } else {
        "groundline-insights"
    };
    let checksum = reviewed
        .join("plugins/groundline-insights/bin")
        .join(target)
        .join(format!("{executable}.sha256"));
    let valid_checksum = fs::read(&checksum).unwrap();
    fs::write(&checksum, format!("{}  {executable}\n", "0".repeat(64))).unwrap();
    assert_blocked(run_installer("core"), "distribution_groundline-insights");
    assert_eq!(saved_files(&insights_state), saved_state);
    fs::write(checksum, valid_checksum).unwrap();

    let stored_profile = insights_state.join("owner-profile.json");
    let valid_profile = fs::read(&stored_profile).unwrap();
    groundline_runtime::local_file::atomic_write_private(&stored_profile, b"invalid profile")
        .unwrap();
    assert_blocked(run_installer("core"), "insights_server_compatibility");
    assert_eq!(fs::read(&stored_profile).unwrap(), b"invalid profile");
    groundline_runtime::local_file::atomic_write_private(&stored_profile, &valid_profile).unwrap();

    assert_blocked(run_installer("core"), "insights_server_compatibility");
    assert_eq!(saved_files(&insights_state), saved_state);
    health.revision.store(8, Ordering::Relaxed);
    for failure in ["add", "upgrade"] {
        let output = run_installer_with_failure("core", Some(failure));
        let receipt = package::receipt(&output);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{receipt}; {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            receipt["stages"]["source_rollback"]["status"],
            "PASS",
            "{receipt}; {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(receipt["rollback"], "previous_commit_pinned");
        assert_eq!(installed_versions(&mut native()), previous_versions);
        let listing: Value = serde_json::from_slice(
            &checked(
                native().args(["plugin", "marketplace", "list", "--json"]),
                "restored source",
            )
            .stdout,
        )
        .unwrap();
        let source = Path::new(listing["marketplaces"][0]["root"].as_str().unwrap());
        for name in ["groundline", "groundline-insights"] {
            let executable = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.to_owned()
            };
            for file in [
                format!("bin/{target}/{executable}"),
                format!("bin/{target}/{executable}.sha256"),
                format!("bin/{target}/manifest.json"),
                ".codex-plugin/plugin.json".to_owned(),
            ] {
                assert_eq!(
                    fs::read(source.join("plugins").join(name).join(&file)).unwrap(),
                    fs::read(
                        home.join(format!("plugins/cache/groundline/{name}/0.29.0"))
                            .join(file)
                    )
                    .unwrap()
                );
            }
        }
        assert_eq!(saved_files(&insights_state), saved_state);
    }

    let mut selected_values: toml::Table = toml::from_str(&selected).unwrap();
    selected_values["marketplaces"].as_table_mut().unwrap()["groundline"]
        .as_table_mut()
        .unwrap()
        .insert(
            "ref".to_owned(),
            toml::Value::String(candidate_commit.clone()),
        );
    let mut previous_config = None;
    for profile in ["core", "both", "both"] {
        let output = run_installer(profile);
        let receipt = package::receipt(&output);
        assert!(
            matches!(output.status.code(), Some(0 | 2)),
            "{receipt}; {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(receipt["source_commit"], candidate_commit);
        for stage in [
            "distribution_revision",
            "snapshot_revision",
            "installed_state",
            "marketplace_add",
            "marketplace_refresh",
            "install_groundline",
            "verify_groundline",
            "catalog",
            "settings",
            "insights_server_compatibility",
        ] {
            assert_eq!(receipt["stages"][stage]["status"], "PASS", "{receipt}");
        }
        if profile == "core" {
            assert_eq!(
                receipt["stages"]["insights_setup"]["status"],
                "NOT_SELECTED"
            );
            // A profile selects explicit installation/setup, not native update
            // isolation. Refreshing this shared marketplace also advances the
            // already-installed Insights package while preserving its disabled flag.
            let current_versions = installed_versions(&mut native());
            for name in ["groundline", "groundline-insights"] {
                assert_eq!(
                    current_versions[name],
                    (env!("CARGO_PKG_VERSION").to_owned(), name == "groundline")
                );
                assert!(
                    home.join(format!(
                        "plugins/cache/groundline/{name}/{}",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .exists()
                );
            }
        } else {
            assert_eq!(receipt["status"], "ACTION_REQUIRED", "{receipt}");
            for stage in ["install_groundline-insights", "verify_groundline-insights"] {
                assert_eq!(receipt["stages"][stage]["status"], "PASS", "{receipt}");
            }
            assert_eq!(
                receipt["stages"]["insights_setup"]["status"],
                "ACTION_REQUIRED"
            );
        }
        let current_config = fs::read(&config).unwrap();
        let current_values: toml::Table =
            toml::from_str(std::str::from_utf8(&current_config).unwrap()).unwrap();
        // Native installation may normalize TOML whitespace once. It must
        // preserve every setting, and unchanged repeats must remain stable.
        assert_eq!(current_values, selected_values);
        if let Some(previous) = previous_config {
            assert_eq!(current_config, previous);
        }
        previous_config = Some(current_config);
        assert_eq!(saved_files(&insights_state), saved_state);
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
    assert_eq!(fs::read(&config).unwrap(), previous_config.unwrap());
    checked(
        setup().args(["--model", "gpt-6-astra"]),
        "explicit Astra selection",
    );
    let after = fs::read(&config).unwrap();
    let parsed: toml::Table = toml::from_str(std::str::from_utf8(&after).unwrap()).unwrap();
    assert_eq!(parsed["model"].as_str(), Some("gpt-6-astra"));
    checked(
        setup().args(["--model", "gpt-6-astra"]),
        "repeat Astra setup",
    );
    assert_eq!(fs::read(&config).unwrap(), after);
    let pending = Command::new(insights)
        .args(["setup", "--codex-home"])
        .arg(&home)
        .env("GROUNDLINE_RUNTIME_FAMILY", "codex_cli")
        .env("GROUNDLINE_EXECUTION_MODE", "local_headless")
        .env_remove("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")
        .output()
        .unwrap();
    assert_eq!(pending.status.code(), Some(2));
    let pending: Value = serde_json::from_slice(&pending.stdout).unwrap();
    assert_eq!(pending["stages"]["collection_consented"], false);
    assert_eq!(saved_files(&insights_state), saved_state);
    println!(
        "real Codex and local stable transport: old metadata -> exact reviewed cache despite stable movement, add/upgrade failure rollback artifacts, disabled plugin and consent preservation, repeat, native catalog/model/effort/Fast PASS; old runtime execution, authenticated task and private delivery UNVERIFIED"
    );
}
