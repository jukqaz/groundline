use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
#[cfg(unix)]
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use command_group::CommandGroup;
use semver::Version;
use serde::Serialize;
use serde_json::{Value, json};
use tempfile::NamedTempFile;
use uuid::Uuid;

use super::XtaskError;

const ACTIONLINT_VERSION: &str = "1.7.12";
const MAX_CAPTURE_BYTES: usize = 16 * 1024;
const OVERALL_TIMEOUT: Duration = Duration::from_secs(40 * 60);
const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
struct CheckSpec {
    name: &'static str,
    program: OsString,
    args: Vec<OsString>,
    environment: Vec<(OsString, OsString)>,
    timeout: Duration,
    isolated_cargo_target: bool,
}

#[derive(Debug)]
struct ProcessResult {
    success: bool,
    timed_out: bool,
    duration: Duration,
    stdout: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct CheckReceipt {
    name: &'static str,
    status: &'static str,
    duration_ms: u64,
}

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn cargo_check(name: &'static str, values: &[&str], timeout_secs: u64) -> CheckSpec {
    CheckSpec {
        name,
        program: "cargo".into(),
        args: args(values),
        environment: Vec::new(),
        timeout: Duration::from_secs(timeout_secs),
        isolated_cargo_target: true,
    }
}

fn check_plan(empty_codex_home: &Path) -> Vec<CheckSpec> {
    let mut doctor = args(&[
        "run",
        "--locked",
        "--bin",
        "groundline",
        "--",
        "doctor",
        "--plugin-root",
        ".",
        "--codex-home",
    ]);
    doctor.push(empty_codex_home.as_os_str().to_owned());
    doctor.push("--json".into());

    let mut checks = vec![
        cargo_check("format", &["fmt", "--all", "--", "--check"], 180),
        CheckSpec {
            name: "workflow-lint",
            program: "actionlint".into(),
            args: Vec::new(),
            environment: Vec::new(),
            timeout: Duration::from_secs(120),
            isolated_cargo_target: false,
        },
        cargo_check("dependency-policy", &["deny", "check"], 300),
        cargo_check(
            "contracts-base",
            &[
                "check",
                "--locked",
                "-p",
                "groundline-contracts",
                "--no-default-features",
            ],
            300,
        ),
    ];
    for (name, feature) in [
        ("contracts-batch", "batch"),
        ("contracts-efficiency", "efficiency"),
        ("contracts-insights", "insights"),
        ("contracts-integrity", "integrity"),
        ("contracts-version", "version"),
    ] {
        checks.push(cargo_check(
            name,
            &[
                "check",
                "--locked",
                "-p",
                "groundline-contracts",
                "--no-default-features",
                "--features",
                feature,
            ],
            300,
        ));
    }
    checks.extend([
        cargo_check(
            "runtime-base",
            &[
                "check",
                "--locked",
                "-p",
                "groundline-runtime",
                "--no-default-features",
            ],
            300,
        ),
        cargo_check(
            "runtime-insights-client",
            &[
                "check",
                "--locked",
                "-p",
                "groundline-runtime",
                "--no-default-features",
                "--features",
                "insights-client",
            ],
            300,
        ),
        cargo_check(
            "runtime-tailnet-probe",
            &[
                "check",
                "--locked",
                "-p",
                "groundline-runtime",
                "--no-default-features",
                "--features",
                "tailnet-probe",
            ],
            300,
        ),
        cargo_check(
            "portable-tests",
            &["test", "--workspace", "--all-features", "--locked"],
            900,
        ),
        cargo_check(
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            900,
        ),
        cargo_check(
            "source-contract",
            &[
                "run",
                "--locked",
                "-p",
                "xtask",
                "--",
                "verify-source",
                "--root",
                ".",
                "--json",
            ],
            300,
        ),
        CheckSpec {
            name: "doctor",
            program: "cargo".into(),
            args: doctor,
            environment: Vec::new(),
            timeout: Duration::from_secs(300),
            isolated_cargo_target: true,
        },
        cargo_check(
            "project-audit",
            &[
                "run",
                "--locked",
                "--bin",
                "groundline",
                "--",
                "project-audit",
                "--repo",
                ".",
                "--json",
            ],
            300,
        ),
    ]);
    checks
}

fn read_tail(mut reader: impl Read, maximum: usize) -> Vec<u8> {
    let mut tail = VecDeque::with_capacity(maximum);
    let mut chunk = [0_u8; 8192];
    loop {
        let count = match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        for byte in &chunk[..count] {
            if tail.len() == maximum {
                tail.pop_front();
            }
            tail.push_back(*byte);
        }
    }
    tail.into_iter().collect()
}

fn run_process(
    root: &Path,
    cargo_target: &Path,
    spec: &CheckSpec,
    overall_deadline: Instant,
) -> Result<ProcessResult, XtaskError> {
    let remaining = overall_deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Ok(ProcessResult {
            success: false,
            timed_out: true,
            duration: Duration::ZERO,
            stdout: Vec::new(),
        });
    }
    let timeout = spec.timeout.min(remaining);
    let started = Instant::now();
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .envs(spec.environment.iter().cloned());
    for key in [
        "CARGO_BUILD_RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_TARGET",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTDOC",
        "RUSTFLAGS",
    ] {
        command.env_remove(key);
    }
    if spec.isolated_cargo_target {
        command.env("CARGO_TARGET_DIR", cargo_target);
    }
    let mut child = command
        .group_spawn()
        .map_err(|_| XtaskError::LocalVerificationFailed)?;
    let stdout = child.inner().stdout.take();
    let stderr = child.inner().stderr.take();
    let stdout_reader =
        stdout.map(|value| thread::spawn(move || read_tail(value, MAX_CAPTURE_BYTES)));
    let stderr_reader =
        stderr.map(|value| thread::spawn(move || read_tail(value, MAX_CAPTURE_BYTES)));
    let deadline = started + timeout;
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (Some(status), false),
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => break (None, true),
            Err(_) => break (None, false),
        }
    };
    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let stdout = stdout_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    let _stderr = stderr_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    Ok(ProcessResult {
        success: status.is_some_and(|value| value.success()),
        timed_out,
        duration: started.elapsed(),
        stdout,
    })
}

fn preflight(
    root: &Path,
    cargo_target: &Path,
    name: &'static str,
    program: &'static str,
    values: &[&str],
    deadline: Instant,
) -> Result<String, XtaskError> {
    let result = run_process(
        root,
        cargo_target,
        &CheckSpec {
            name,
            program: program.into(),
            args: args(values),
            environment: Vec::new(),
            timeout: PREFLIGHT_TIMEOUT,
            isolated_cargo_target: false,
        },
        deadline,
    )?;
    if !result.success || result.timed_out {
        eprintln!("local-verify: {name}: preflight_failed");
        return Err(XtaskError::LocalVerificationFailed);
    }
    String::from_utf8(result.stdout).map_err(|_| XtaskError::LocalVerificationFailed)
}

fn simple_version(value: &str) -> Result<String, XtaskError> {
    let version = value
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .find(|part| {
            part.bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_digit())
        })
        .unwrap_or_default();
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return Err(XtaskError::LocalVerificationFailed);
    }
    Version::parse(version).map_err(|_| XtaskError::LocalVerificationFailed)?;
    Ok(version.to_owned())
}

fn stable_rust_version(value: &str) -> Result<String, XtaskError> {
    let version = rustc_field(value, "release:")?;
    let parsed = Version::parse(version).map_err(|_| XtaskError::LocalVerificationFailed)?;
    if !parsed.pre.is_empty() || !parsed.build.is_empty() {
        return Err(XtaskError::LocalVerificationFailed);
    }
    Ok(version.to_owned())
}

fn rustc_field<'a>(value: &'a str, field: &str) -> Result<&'a str, XtaskError> {
    let value = value
        .lines()
        .find_map(|line| line.strip_prefix(field))
        .map(str::trim)
        .unwrap_or_default();
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(XtaskError::LocalVerificationFailed);
    }
    Ok(value)
}

fn clean_commit(root: &Path, cargo_target: &Path, deadline: Instant) -> Result<String, XtaskError> {
    let top = preflight(
        root,
        cargo_target,
        "repository-root",
        "git",
        &["-c", "core.fsmonitor=false", "rev-parse", "--show-toplevel"],
        deadline,
    )?;
    let top = PathBuf::from(top.trim())
        .canonicalize()
        .map_err(|_| XtaskError::LocalVerificationFailed)?;
    if top != root {
        return Err(XtaskError::LocalVerificationFailed);
    }
    let status = preflight(
        root,
        cargo_target,
        "source-state",
        "git",
        &[
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
        ],
        deadline,
    )?;
    if !status.trim().is_empty() {
        eprintln!("local-verify: source-state: clean_commit_required");
        return Err(XtaskError::LocalVerificationFailed);
    }
    let revision = preflight(
        root,
        cargo_target,
        "source-revision",
        "git",
        &["-c", "core.fsmonitor=false", "rev-parse", "HEAD"],
        deadline,
    )?;
    let revision = revision.trim();
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(XtaskError::LocalVerificationFailed);
    }
    Ok(revision.to_owned())
}

fn checked_duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn output_directory(root: &Path) -> Result<PathBuf, XtaskError> {
    let dist = root.join("dist");
    if dist.exists() && fs::symlink_metadata(&dist)?.file_type().is_symlink() {
        return Err(XtaskError::LocalVerificationFailed);
    }
    fs::create_dir_all(&dist)?;
    let output = dist.join("local-verification");
    if output.exists() && fs::symlink_metadata(&output)?.file_type().is_symlink() {
        return Err(XtaskError::LocalVerificationFailed);
    }
    fs::create_dir_all(&output)?;
    Ok(output)
}

fn write_receipt(root: &Path, revision: &str, receipt: &Value) -> Result<PathBuf, XtaskError> {
    let directory = output_directory(root)?;
    let name = format!("{revision}-{}.json", Uuid::new_v4());
    let destination = directory.join(name);
    let mut temporary = NamedTempFile::new_in(&directory)?;
    serde_json::to_writer_pretty(&mut temporary, receipt)?;
    writeln!(temporary)?;
    temporary.as_file_mut().sync_all()?;
    temporary
        .persist_noclobber(&destination)
        .map_err(|error| XtaskError::Io(error.error))?;
    #[cfg(unix)]
    File::open(&directory)?.sync_all()?;
    Ok(destination)
}

pub fn verify(root: &Path) -> Result<Value, XtaskError> {
    let started = Instant::now();
    let deadline = started + OVERALL_TIMEOUT;
    let root = root
        .canonicalize()
        .map_err(|_| XtaskError::LocalVerificationFailed)?;
    let cargo_target = root.join("target/local-ci-fallback");
    let revision = clean_commit(&root, &cargo_target, deadline)?;

    let rustc = preflight(
        &root,
        &cargo_target,
        "rustc-version",
        "rustc",
        &["-vV"],
        deadline,
    )?;
    let rustc_release = stable_rust_version(&rustc)?;
    let rustc_host = rustc_field(&rustc, "host:")?.to_owned();
    let cargo_version = simple_version(&preflight(
        &root,
        &cargo_target,
        "cargo-version",
        "cargo",
        &["-V"],
        deadline,
    )?)?;
    let rustc_semver =
        Version::parse(&rustc_release).map_err(|_| XtaskError::LocalVerificationFailed)?;
    let cargo_semver =
        Version::parse(&cargo_version).map_err(|_| XtaskError::LocalVerificationFailed)?;
    if (rustc_semver.major, rustc_semver.minor) != (cargo_semver.major, cargo_semver.minor) {
        eprintln!("local-verify: cargo-version: toolchain_mismatch");
        return Err(XtaskError::LocalVerificationFailed);
    }
    let actionlint_version = simple_version(&preflight(
        &root,
        &cargo_target,
        "actionlint-version",
        "actionlint",
        &["-version"],
        deadline,
    )?)?;
    if actionlint_version != ACTIONLINT_VERSION {
        eprintln!("local-verify: actionlint-version: version_mismatch");
        return Err(XtaskError::LocalVerificationFailed);
    }
    let cargo_deny_version = simple_version(&preflight(
        &root,
        &cargo_target,
        "cargo-deny-version",
        "cargo",
        &["deny", "--version"],
        deadline,
    )?)?;

    let empty_codex_home = tempfile::tempdir()?;
    let plan = check_plan(empty_codex_home.path());
    let mut checks = Vec::with_capacity(plan.len());
    for (index, spec) in plan.iter().enumerate() {
        eprintln!("local-verify: [{}/{}] {}", index + 1, plan.len(), spec.name);
        let result = run_process(&root, &cargo_target, spec, deadline)?;
        if !result.success {
            let reason = if result.timed_out {
                "timeout"
            } else {
                "nonzero_exit"
            };
            eprintln!("local-verify: {}: {reason}", spec.name);
            return Err(XtaskError::LocalVerificationFailed);
        }
        checks.push(CheckReceipt {
            name: spec.name,
            status: "PASS",
            duration_ms: checked_duration_ms(result.duration),
        });
    }

    let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let receipt = json!({
        "kind":"groundline-local-verification-receipt",
        "schema":1,
        "status":"PASS",
        "generated_at":generated_at,
        "source_revision":revision,
        "source_state":"clean",
        "host_target":rustc_host,
        "toolchain":{
            "rustc":rustc_release,
            "cargo":cargo_version,
            "actionlint":actionlint_version,
            "cargo_deny":cargo_deny_version,
        },
        "check_count":checks.len(),
        "checks":checks,
        "duration_ms":checked_duration_ms(started.elapsed()),
        "github_actions_used":false,
        "ambient_compile_overrides_removed":true,
        "source_mutation_performed":false,
        "receipt_written":true,
        "evidence_scope":["source", "current_host"],
        "cross_platform_artifacts_verified":false,
        "installed_runtime_verified":false,
        "live_services_verified":false,
        "release_ready":false,
        "raw_logs_included":false,
        "private_paths_included":false,
    });
    let receipt_path = write_receipt(&root, &revision, &receipt)?;
    let relative = receipt_path
        .strip_prefix(&root)
        .map_err(|_| XtaskError::LocalVerificationFailed)?;
    Ok(json!({
        "kind":"groundline-local-verification-result",
        "schema":1,
        "status":"PASS",
        "source_revision":revision,
        "receipt":relative.to_string_lossy(),
        "check_count":receipt["check_count"],
        "duration_ms":receipt["duration_ms"],
        "github_actions_used":false,
        "release_ready":false,
    }))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;
    use std::time::{Duration, Instant};

    use tempfile::tempdir;

    use super::{
        ACTIONLINT_VERSION, CheckSpec, OVERALL_TIMEOUT, check_plan, run_process, simple_version,
        stable_rust_version,
    };

    #[test]
    fn fallback_plan_is_bounded_and_contains_no_shell_or_network_publisher() {
        let plan = check_plan(Path::new("empty-codex-home"));
        assert_eq!(plan.len(), 17);
        assert!(OVERALL_TIMEOUT.as_secs() <= 40 * 60);
        assert!(plan.iter().all(|check| check.timeout <= OVERALL_TIMEOUT));
        assert!(plan.iter().all(|check| !matches!(
            check.program.to_str(),
            Some("sh" | "bash" | "pwsh" | "gh" | "curl")
        )));
        assert!(plan.iter().any(|check| check.name == "portable-tests"));
        assert!(plan.iter().any(|check| check.name == "workflow-lint"));
        assert!(plan.iter().any(|check| check.name == "dependency-policy"));
        assert!(plan.iter().any(|check| check.name == "source-contract"));
        assert!(plan.iter().any(|check| check.name == "doctor"));
        assert!(plan.iter().any(|check| check.name == "project-audit"));
    }

    #[test]
    fn tool_versions_are_reduced_to_public_semver_tokens() {
        assert_eq!(simple_version("cargo 1.98.0 (commit)\n").unwrap(), "1.98.0");
        assert_eq!(
            simple_version("1.7.12\ninstalled by building from source\n").unwrap(),
            ACTIONLINT_VERSION
        );
        assert!(simple_version("installed at /private/path").is_err());
        assert_eq!(
            stable_rust_version("release: 1.98.0\nhost: aarch64-apple-darwin\n").unwrap(),
            "1.98.0"
        );
        assert!(stable_rust_version("release: 1.99.0-nightly\n").is_err());
    }

    #[test]
    fn process_tree_is_stopped_at_the_deadline() {
        let root = tempdir().unwrap();
        let started = Instant::now();
        let result = run_process(
            root.path(),
            &root.path().join("target"),
            &CheckSpec {
                name: "deadline-regression",
                program: std::env::current_exe().unwrap().into_os_string(),
                args: [
                    "--ignored",
                    "--exact",
                    "local_verify::tests::deadline_helper",
                ]
                .into_iter()
                .map(Into::into)
                .collect(),
                environment: vec![("GROUNDLINE_DEADLINE_PARENT".into(), "1".into())],
                timeout: Duration::from_millis(150),
                isolated_cargo_target: false,
            },
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert!(result.timed_out);
        assert!(!result.success);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    #[ignore = "subprocess fixture"]
    fn deadline_helper() {
        if std::env::var_os("GROUNDLINE_DEADLINE_CHILD").is_some() {
            std::thread::sleep(Duration::from_secs(10));
            return;
        }
        if std::env::var_os("GROUNDLINE_DEADLINE_PARENT").is_some() {
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "local_verify::tests::deadline_helper",
                ])
                .env("GROUNDLINE_DEADLINE_CHILD", "1")
                .spawn()
                .unwrap();
            let _ = child.wait();
        }
    }
}
