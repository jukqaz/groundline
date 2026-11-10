//! Real Codex, real packages, isolated home; deliberately no account or service credentials.
use groundline_contracts::insights::BASIC_CONTRACT_REVISION;
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

fn installed_without_credentials(output: &Output) -> Value {
    let receipt = package::receipt(output);
    if !output.status.success() {
        // New Codex doctor versions include account readiness. This isolated
        // fixture deliberately has no credentials; assert that exact limitation
        // without fabricating auth or accepting a different installation failure.
        assert_eq!(
            output.status.code(),
            Some(2),
            "{receipt}; {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(receipt["status"], "ACTION_REQUIRED");
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("no Codex credentials were found")
        );
        assert_eq!(
            receipt["stages"]["native_doctor"]["status"],
            "ACTION_REQUIRED"
        );
        for (name, stage) in receipt["stages"].as_object().unwrap() {
            if name != "native_doctor" {
                assert!(
                    ["PASS", "PRESERVED"].iter().any(|s| stage["status"] == *s),
                    "{receipt}"
                );
            }
        }
    } else {
        assert_eq!(receipt["status"], "PASS");
    }
    receipt
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
                plugin["pluginId"].as_str().unwrap().to_owned(),
                (
                    plugin["version"].as_str().unwrap().to_owned(),
                    plugin["enabled"].as_bool().unwrap(),
                ),
            )
        })
        .collect()
}

#[test]
#[ignore = "requires GROUNDLINE_NATIVE_CODEX and GROUNDLINE_NATIVE_INSIGHTS_BINARY; CI runs this on the three supported native hosts"]
fn real_codex_migrates_and_updates_products_independently_without_resetting_state() {
    let codex =
        PathBuf::from(std::env::var_os("GROUNDLINE_NATIVE_CODEX").expect("real Codex path"));
    let insights = PathBuf::from(
        std::env::var_os("GROUNDLINE_NATIVE_INSIGHTS_BINARY").expect("Insights binary"),
    );
    let core = Path::new(env!("CARGO_BIN_EXE_groundline"));
    let temporary = tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let home = root.join("isolated home 한글");
    fs::create_dir(&home).unwrap();
    let market = root.join("local Git distribution");
    fs::create_dir_all(market.join(".agents/plugins")).unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original_catalog: Value =
        serde_json::from_slice(&fs::read(repo.join(".agents/plugins/marketplace.json")).unwrap())
            .unwrap();
    fs::write(
        market.join(".agents/plugins/marketplace.json"),
        serde_json::to_vec(&original_catalog).unwrap(),
    )
    .unwrap();
    for file in ["install.sh", ".gitattributes"] {
        fs::copy(repo.join(file), market.join(file)).unwrap();
    }
    let platform: Value = serde_json::from_slice(
        &checked(Command::new(core).args(["platform", "--json"]), "platform").stdout,
    )
    .unwrap();
    let target = platform["target"].as_str().unwrap();
    // Only metadata represents the older release; both fixture binaries are current.
    for (name, binary) in [
        ("groundline", core),
        ("groundline-insights", insights.as_path()),
    ] {
        package::stage(&market.join("plugins").join(name), binary, name, target);
        let manifest = market
            .join("plugins")
            .join(name)
            .join(".codex-plugin/plugin.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
        value["version"] = serde_json::json!("0.29.0");
        fs::write(manifest, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    let git_config = root.join("fixture.gitconfig");
    fs::write(&git_config,format!("[url \"{}\"]\n insteadOf = https://github.com/jukqaz/groundline.git\n insteadOf = git@github.com:jukqaz/groundline.git\n[user]\n name = Fixture\n email = fixture@example.invalid\n[commit]\n gpgsign = false\n",market.display())).unwrap();
    let git = |args: &[&str]| {
        String::from_utf8(
            checked(
                Command::new("git")
                    .arg("-c")
                    .arg("core.fsmonitor=false")
                    .arg("-C")
                    .arg(&market)
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .env("GIT_CONFIG_GLOBAL", &git_config)
                    .args(args),
                "local fixture Git",
            )
            .stdout,
        )
        .unwrap()
        .trim()
        .to_owned()
    };
    git(&["init", "--initial-branch=stable"]);
    git(&["add", "."]);
    git(&["commit", "-m", "old metadata fixture"]);
    let old_sha = git(&["rev-parse", "HEAD"]);
    let native = || {
        let mut c = Command::new(&codex);
        c.env("CODEX_HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &git_config)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .current_dir(&root);
        c
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
        "old source",
    );
    for name in ["groundline", "groundline-insights"] {
        checked(
            native().args(["plugin", "add", &format!("{name}@groundline"), "--json"]),
            "old package",
        );
    }
    let config = home.join("config.toml");
    let set_enabled = |id: &str, enabled: bool| {
        let mut value: toml::Table = toml::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        value["plugins"].as_table_mut().unwrap()[id]
            .as_table_mut()
            .unwrap()
            .insert("enabled".to_owned(), toml::Value::Boolean(enabled));
        groundline_runtime::local_file::atomic_write_private(
            &config,
            toml::to_string(&value).unwrap().as_bytes(),
        )
        .unwrap();
    };
    set_enabled("groundline-insights@groundline", false);
    let old_config = fs::read_to_string(&config).unwrap();
    groundline_runtime::local_file::atomic_write_private(
        &config,
        format!(
            "model='gpt-6.1-sol'\nmodel_reasoning_effort='max'\nservice_tier='fast'\n{old_config}"
        )
        .as_bytes(),
    )
    .unwrap();
    let health = HealthServer::start();
    let owner = root.join("owner-profile.json");
    let profile = serde_json::json!({"schema_version":7,"kind":"groundline-insights-owner-profile","mode":"private_owner","endpoint":health.endpoint,
        "enrollment_token":"fixture-token-never-uploaded-0123456789","automatic_activity_checkpoints":true,"automatic_initial_history_sync":true,
        "collection_scope":"all_activity","checkpoint_min_interval_seconds":900,"diagnostic_enabled":false,"trigger_mode":"native_hook_checkpoints"});
    groundline_runtime::local_file::atomic_write_private(
        &owner,
        &serde_json::to_vec(&profile).unwrap(),
    )
    .unwrap();
    let worker = |verb: &str| {
        let mut c = Command::new(&insights);
        c.args(["worker", verb, "--codex-home"])
            .arg(&home)
            .env("GROUNDLINE_RUNTIME_FAMILY", "codex_cli")
            .env("GROUNDLINE_EXECUTION_MODE", "local_headless")
            .env_remove("CODEX_INTERNAL_ORIGINATOR_OVERRIDE");
        c
    };
    checked(
        worker("configure").arg("--input").arg(&owner),
        "private fixture profile",
    );
    checked(&mut worker("enable"), "fixture consent identity");
    checked(&mut worker("disable"), "disabled collection");
    let insights_state = home.join("groundline/insights");
    let saved_state = saved_files(&insights_state);
    assert!(
        saved_state.keys().any(|p| p.ends_with("consent.json"))
            && saved_state.keys().any(|p| p.ends_with("identity.json"))
    );
    let records = home.join("groundline/learning/records");
    fs::create_dir_all(&records).unwrap();
    fs::write(
        records.join("historical-record.fixture"),
        b"preserve historical fixture bytes",
    )
    .unwrap();
    let saved_records = saved_files(&records);
    let old_insights = home.join("plugins/cache/groundline/groundline-insights/0.29.0");
    let old_insights_bytes = saved_files(&old_insights);
    git(&["checkout", "-b", "core-stable"]);
    package::stage(
        &market.join("plugins/groundline"),
        core,
        "groundline",
        target,
    );
    fs::remove_dir_all(market.join("plugins/groundline/bin")).unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "Core source"]);
    let source = git(&["rev-parse", "HEAD"]);
    let stage_product = |product: &str,
                         name: &str,
                         channel: &str,
                         namespace: &str,
                         source: &str,
                         binary: &Path| {
        package::stage(&market.join("plugins").join(name), binary, name, target);
        let mut catalog = original_catalog.clone();
        catalog["name"] = serde_json::json!(namespace);
        catalog["plugins"]
            .as_array_mut()
            .unwrap()
            .retain(|p| p["name"] == name);
        fs::write(
            market.join(".agents/plugins/marketplace.json"),
            serde_json::to_vec(&catalog).unwrap(),
        )
        .unwrap();
        fs::write(market.join("distribution.json"),serde_json::to_vec(&serde_json::json!({"kind":"groundline-plugin-distribution","schema":1,"product":product,"plugin":name,
            "marketplace":namespace,"channel":channel,"version":env!("CARGO_PKG_VERSION"),"release_tag":format!("{product}-v{}",env!("CARGO_PKG_VERSION")),"source_sha":source})).unwrap()).unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "generated product fixture"]);
    };
    stage_product(
        "core",
        "groundline",
        "core-stable",
        "groundline-core",
        &source,
        core,
    );
    let reviewed = root.join("reviewed Core");
    checked(
        Command::new("git")
            .args(["clone", "--no-hardlinks", "--branch", "core-stable"])
            .arg(&market)
            .arg(&reviewed),
        "reviewed clone",
    );
    let candidate = git(&["rev-parse", "HEAD"]);
    // A moving channel cannot replace the reviewed commit during native refresh.
    fs::write(market.join("future.fixture"), b"future channel").unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "move channel after review"]);
    let wrapper = root.join("fail-once.sh");
    fs::write(&wrapper,r#"#!/bin/sh
if [ "${4:-}" != --help ] && [ ! -e "$GROUNDLINE_FAIL_MARKER" ] && [ "$1 $2 ${3:-}" = "$GROUNDLINE_FAIL_STAGE" ]; then
 : > "$GROUNDLINE_FAIL_MARKER"
 if [ "${3:-}" = upgrade ]; then "$GROUNDLINE_REAL_CODEX" "$@" || exit $?; fi
 exit 19
fi
exec "$GROUNDLINE_REAL_CODEX" "$@"
"#).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let run = |distribution: &Path, fail: Option<&str>| {
        let mut c = Command::new(if cfg!(target_os = "macos") {
            "/bin/bash"
        } else {
            "bash"
        });
        c.arg(distribution.join("install.sh"))
            .arg("--codex")
            .arg(if fail.is_some() { &wrapper } else { &codex })
            .env("CODEX_HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &git_config)
            .env("GROUNDLINE_REAL_CODEX", &codex)
            .env("GROUNDLINE_FAIL_STAGE", fail.unwrap_or("none"))
            .env(
                "GROUNDLINE_FAIL_MARKER",
                root.join(format!(
                    "failed-{}",
                    fail.unwrap_or("none").replace(' ', "-")
                )),
            )
            .env("GROUNDLINE_RUNTIME_FAMILY", "codex_cli")
            .env("GROUNDLINE_EXECUTION_MODE", "local_headless")
            .env_remove("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .current_dir(&root);
        c.output().unwrap()
    };
    let assert_preserved = || {
        assert_eq!(saved_files(&insights_state), saved_state);
        assert_eq!(saved_files(&records), saved_records);
        let settings: toml::Table = toml::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        assert_eq!(settings["model"].as_str(), Some("gpt-6.1-sol"));
        assert_eq!(settings["model_reasoning_effort"].as_str(), Some("max"));
        assert_eq!(settings["service_tier"].as_str(), Some("fast"));
    };
    let assert_blocked = |stage: &str, out: Output| {
        let receipt = package::receipt(&out);
        assert_eq!(out.status.code(), Some(1), "{receipt}");
        assert_eq!(receipt["stages"][stage]["status"], "FAIL", "{receipt}");
        assert!(receipt["stages"].get("marketplace_add").is_none());
        assert_preserved();
    };
    set_enabled("groundline@groundline", false);
    let disabled = fs::read(&config).unwrap();
    assert_blocked("legacy_core_requires_review", run(&reviewed, None));
    assert_eq!(fs::read(&config).unwrap(), disabled);
    set_enabled("groundline@groundline", true);
    let learning_profile = home.join("groundline/learning/profile.json");
    fs::write(&learning_profile, b"{\"enabled\":true}").unwrap();
    assert_blocked("learning_profile_requires_review", run(&reviewed, None));
    fs::remove_file(learning_profile).unwrap();
    for failure in [
        "plugin marketplace add",
        "plugin marketplace upgrade",
        "plugin add groundline@groundline-core",
    ] {
        let before = installed_versions(&mut native());
        let out = run(&reviewed, Some(failure));
        let receipt = package::receipt(&out);
        assert_eq!(out.status.code(), Some(1), "{receipt}");
        assert_eq!(
            receipt["stages"]["source_rollback"]["status"], "PASS",
            "{receipt}"
        );
        assert_eq!(installed_versions(&mut native()), before);
        assert_preserved();
        assert_eq!(saved_files(&old_insights), old_insights_bytes);
    }
    let out = run(&reviewed, None);
    let receipt = installed_without_credentials(&out);
    assert_eq!(receipt["source_commit"], candidate);
    let versions = installed_versions(&mut native());
    assert_eq!(
        versions["groundline@groundline-core"],
        (env!("CARGO_PKG_VERSION").to_owned(), true)
    );
    assert_eq!(
        versions["groundline-insights@groundline"],
        ("0.29.0".to_owned(), false)
    );
    assert!(!versions.contains_key("groundline@groundline"));
    let markets: Value = serde_json::from_slice(
        &checked(
            native().args(["plugin", "marketplace", "list", "--json"]),
            "market list",
        )
        .stdout,
    )
    .unwrap();
    let old_root = markets["marketplaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["name"] == "groundline")
        .unwrap()["root"]
        .as_str()
        .unwrap();
    assert_eq!(
        String::from_utf8(
            checked(
                Command::new("git")
                    .arg("-C")
                    .arg(old_root)
                    .args(["rev-parse", "HEAD"]),
                "old source pin"
            )
            .stdout
        )
        .unwrap()
        .trim(),
        old_sha
    );
    assert_eq!(saved_files(&old_insights), old_insights_bytes);
    assert_preserved();
    set_enabled("groundline@groundline-core", false);
    let out = run(&reviewed, None);
    installed_without_credentials(&out);
    assert!(!installed_versions(&mut native())["groundline@groundline-core"].1);
    assert_preserved();
    let core_cache = home
        .join("plugins/cache/groundline-core/groundline")
        .join(env!("CARGO_PKG_VERSION"));
    let core_bytes = saved_files(&core_cache);

    // An Insights update advances only its original namespace, with compatibility
    // preflight and disabled flags intact. It does not republish Core.
    git(&["checkout", "stable"]);
    package::stage(
        &market.join("plugins/groundline-insights"),
        &insights,
        "groundline-insights",
        target,
    );
    fs::remove_dir_all(market.join("plugins/groundline-insights/bin")).unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "Insights source"]);
    let source = git(&["rev-parse", "HEAD"]);
    stage_product(
        "insights",
        "groundline-insights",
        "stable",
        "groundline",
        &source,
        &insights,
    );
    let reviewed_insights = root.join("reviewed Insights");
    checked(
        Command::new("git")
            .args(["clone", "--no-hardlinks", "--branch", "stable"])
            .arg(&market)
            .arg(&reviewed_insights),
        "Insights clone",
    );
    assert_blocked(
        "insights_server_compatibility",
        run(&reviewed_insights, None),
    );
    health
        .revision
        .store(BASIC_CONTRACT_REVISION, Ordering::Relaxed);
    fs::remove_file(root.join("failed-plugin-marketplace-upgrade")).unwrap();
    let out = run(&reviewed_insights, Some("plugin marketplace upgrade"));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        package::receipt(&out)["stages"]["source_rollback"]["status"],
        "PASS"
    );
    assert_eq!(
        installed_versions(&mut native())["groundline-insights@groundline"].0,
        "0.29.0"
    );
    assert_eq!(saved_files(&core_cache), core_bytes);
    assert_preserved();
    let out = run(&reviewed_insights, None);
    installed_without_credentials(&out);
    assert_eq!(
        installed_versions(&mut native())["groundline-insights@groundline"],
        (env!("CARGO_PKG_VERSION").to_owned(), false)
    );
    assert!(!installed_versions(&mut native())["groundline@groundline-core"].1);
    assert_eq!(saved_files(&core_cache), core_bytes);
    assert_preserved();
}

#[test]
#[ignore = "requires GROUNDLINE_NATIVE_CODEX; isolated native metadata only, no model turns"]
fn real_codex_discovers_the_single_core_skill_without_changing_choices() {
    let codex =
        PathBuf::from(std::env::var_os("GROUNDLINE_NATIVE_CODEX").expect("real Codex path"));
    let temporary = tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let home = root.join("isolated-home");
    fs::create_dir(&home).unwrap();
    let market = root.join("core candidate");
    fs::create_dir_all(market.join(".agents/plugins")).unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut catalog: Value =
        serde_json::from_slice(&fs::read(repo.join(".agents/plugins/marketplace.json")).unwrap())
            .unwrap();
    catalog["name"] = serde_json::json!("groundline-core-fixture");
    catalog["plugins"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["name"] == "groundline");
    fs::write(
        market.join(".agents/plugins/marketplace.json"),
        serde_json::to_vec(&catalog).unwrap(),
    )
    .unwrap();
    let core = Path::new(env!("CARGO_BIN_EXE_groundline"));
    let platform: Value = serde_json::from_slice(
        &checked(Command::new(core).args(["platform", "--json"]), "platform").stdout,
    )
    .unwrap();
    let target = platform["target"].as_str().unwrap();
    package::stage(
        &market.join("plugins/groundline"),
        core,
        "groundline",
        target,
    );
    let git_config = root.join("fixture.gitconfig");
    fs::write(&git_config,format!("[url \"{}\"]\n insteadOf = git@github.com:jukqaz/groundline.git\n[user]\n name = GroundLine Test\n email = groundline-test@example.invalid\n[commit]\n gpgsign = false\n",market.display())).unwrap();
    for args in [
        vec!["init", "--initial-branch=stable"],
        vec!["add", "."],
        vec!["commit", "-m", "test: stage single-skill Core candidate"],
    ] {
        checked(
            Command::new("git")
                .current_dir(&market)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", &git_config)
                .args(args),
            "stage isolated candidate",
        );
    }
    let config = home.join("config.toml");
    groundline_runtime::local_file::atomic_write_private(
        &config,
        b"model='gpt-6.1-sol'\nmodel_reasoning_effort='max'\nservice_tier='default'\n",
    )
    .unwrap();
    let native = || {
        let mut c = Command::new(&codex);
        c.env("CODEX_HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &git_config)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY");
        c
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
        "register isolated candidate",
    );
    checked(
        native().args([
            "plugin",
            "add",
            "groundline@groundline-core-fixture",
            "--json",
        ]),
        "install isolated candidate",
    );
    let installed = home
        .join("plugins/cache/groundline-core-fixture/groundline")
        .join(env!("CARGO_PKG_VERSION"));
    let names: Vec<_> = fs::read_dir(installed.join("skills"))
        .unwrap()
        .map(|p| p.unwrap().file_name())
        .collect();
    assert_eq!(
        names,
        vec![std::ffi::OsString::from("optimize-codex-workflow")]
    );
    let before = fs::read(&config).unwrap();
    checked(
        Command::new(installed.join("bin").join(target).join("groundline"))
            .args(["provider-smoke", "--plugin-root"])
            .arg(&installed)
            .args(["--require-installed", "--json"]),
        "installed reduced Core integrity",
    );
    let values: toml::Table = toml::from_str(std::str::from_utf8(&before).unwrap()).unwrap();
    assert_eq!(values["model"].as_str(), Some("gpt-6.1-sol"));
    assert_eq!(values["model_reasoning_effort"].as_str(), Some("max"));
    assert_eq!(values["service_tier"].as_str(), Some("default"));
    let mut observation = Command::new(core);
    observation
        .env("CODEX_HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", &git_config)
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .current_dir(&root)
        .arg("environment-observe")
        .arg("--app-cli")
        .arg(&codex)
        .arg("--path-cli")
        .arg(&codex)
        .arg("--codex-home")
        .arg(&home)
        .arg("--target")
        .arg(installed.join("skills/optimize-codex-workflow/SKILL.md"));
    let output = checked(&mut observation, "fresh native skill discovery");
    let snapshot: Value = serde_json::from_slice(&output.stdout).unwrap();
    for lane in ["app_cli", "path_cli"] {
        assert_eq!(snapshot[lane]["skills"]["supported"], true, "{snapshot}");
        assert_eq!(
            snapshot[lane]["skills"]["observed"]["targets"][0]["enabled"], true,
            "{snapshot}"
        );
    }
    assert_eq!(snapshot["model_calls_requested"], 0);
    assert_eq!(snapshot["native_mutations_requested"], 0);
    assert_eq!(fs::read(&config).unwrap(), before);
    assert!(!home.join("groundline").exists());
}
