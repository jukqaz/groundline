use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use command_group::CommandGroup;
use semver::Version;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const RUNNER_IMAGE: &str = "groundline-release-runner:2.337.0";
const CACHE_LABEL: &str = "io.groundline.arm64-controller-cache=true";
const RUN_LABEL: &str = "io.groundline.arm64-controller-run=true";
const CACHE_PREFIX: &str = "groundline-arm64-controller";
const MAX_COMMAND_OUTPUT_BYTES: usize = 512 * 1024;
const RUSTUP_LIMIT_KIB: u64 = 2 * 1024 * 1024;
const CARGO_LIMIT_KIB: u64 = 6 * 1024 * 1024;
const TARGET_LIMIT_KIB: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureKind {
    Compilation,
    DockerDaemonUnavailable,
    DockerStreamEof,
    DockerVolumePermission,
    DockerCommandTimedOut,
    DockerCommand,
    InvalidSource,
}

impl FailureKind {
    const fn code(self) -> &'static str {
        match self {
            Self::Compilation => "compilation_failed",
            Self::DockerDaemonUnavailable => "docker_daemon_unavailable",
            Self::DockerStreamEof => "docker_stream_unexpected_eof",
            Self::DockerVolumePermission => "docker_volume_permission_failed",
            Self::DockerCommandTimedOut => "docker_command_timed_out",
            Self::DockerCommand => "docker_command_failed",
            Self::InvalidSource => "invalid_source",
        }
    }
}

#[derive(Debug)]
struct Failure {
    kind: FailureKind,
    stage: &'static str,
}

#[derive(Debug)]
struct CommandOutput {
    success: bool,
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

#[derive(Debug)]
struct CacheIdentity {
    rust_version: Version,
    lock_sha256: String,
    rustup: String,
    cargo: String,
    target: String,
}

impl CacheIdentity {
    fn from_version_and_lock(rust_version: Version, lock_sha256: String) -> Self {
        let version_token = rust_version.to_string().replace('.', "-");
        let lock_token = &lock_sha256[..16];
        Self {
            rustup: format!("{CACHE_PREFIX}-rustup-{version_token}"),
            cargo: format!("{CACHE_PREFIX}-cargo-{version_token}-{lock_token}"),
            target: format!("{CACHE_PREFIX}-target-{version_token}-{lock_token}"),
            rust_version,
            lock_sha256,
        }
    }

    fn current(root: &Path) -> Result<Self, Failure> {
        let rust =
            run("rustc", &["--version"], None, Duration::from_secs(30)).map_err(|_| Failure {
                kind: FailureKind::InvalidSource,
                stage: "rust_stable_version",
            })?;
        if !rust.success {
            return Err(Failure {
                kind: FailureKind::InvalidSource,
                stage: "rust_stable_version",
            });
        }
        let rust_version = parse_rust_version(&rust.stdout).ok_or(Failure {
            kind: FailureKind::InvalidSource,
            stage: "rust_stable_version",
        })?;
        if !rust_version.pre.is_empty() || !rust_version.build.is_empty() {
            return Err(Failure {
                kind: FailureKind::InvalidSource,
                stage: "rust_stable_version",
            });
        }
        let lock = root.join("Cargo.lock");
        let metadata = fs::symlink_metadata(&lock).map_err(|_| Failure {
            kind: FailureKind::InvalidSource,
            stage: "cargo_lock",
        })?;
        if metadata.file_type().is_symlink()
            || !metadata.file_type().is_file()
            || metadata.len() == 0
            || metadata.len() > 4 * 1024 * 1024
        {
            return Err(Failure {
                kind: FailureKind::InvalidSource,
                stage: "cargo_lock",
            });
        }
        let lock_bytes = fs::read(lock).map_err(|_| Failure {
            kind: FailureKind::InvalidSource,
            stage: "cargo_lock",
        })?;
        let lock_sha256 = format!("{:x}", Sha256::digest(lock_bytes));
        Ok(Self::from_version_and_lock(rust_version, lock_sha256))
    }

    fn names(&self) -> [&str; 3] {
        [&self.rustup, &self.cargo, &self.target]
    }
}

pub struct Outcome {
    pub receipt: Value,
    pub success: bool,
}

fn read_tail(mut reader: impl Read, maximum: usize) -> Vec<u8> {
    let mut tail = Vec::with_capacity(maximum);
    let mut chunk = [0_u8; 8192];
    loop {
        let count = match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        if count >= maximum {
            tail.clear();
            tail.extend_from_slice(&chunk[count - maximum..count]);
            continue;
        }
        let overflow = tail.len().saturating_add(count).saturating_sub(maximum);
        if overflow > 0 {
            tail.drain(..overflow);
        }
        tail.extend_from_slice(&chunk[..count]);
    }
    tail
}

fn run(
    program: &str,
    args: &[&str],
    current_dir: Option<&Path>,
    timeout: Duration,
) -> Result<CommandOutput, ()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(root) = current_dir {
        command.current_dir(root);
    }
    let mut child = command.group_spawn().map_err(|_| ())?;
    let stdout = child.inner().stdout.take();
    let stderr = child.inner().stderr.take();
    let stdout_reader =
        stdout.map(|value| thread::spawn(move || read_tail(value, MAX_COMMAND_OUTPUT_BYTES)));
    let stderr_reader =
        stderr.map(|value| thread::spawn(move || read_tail(value, MAX_COMMAND_OUTPUT_BYTES)));
    let deadline = Instant::now() + timeout;
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (Some(status), false),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
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
    let stderr = stderr_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    Ok(CommandOutput {
        success: status.is_some_and(|value| value.success()),
        code: status.and_then(|value| value.code()),
        stdout,
        stderr,
        timed_out,
    })
}

fn docker(args: &[&str]) -> Result<CommandOutput, Failure> {
    docker_with_timeout(args, Duration::from_secs(60))
}

fn docker_with_timeout(args: &[&str], timeout: Duration) -> Result<CommandOutput, Failure> {
    run("docker", args, None, timeout).map_err(|_| Failure {
        kind: FailureKind::DockerDaemonUnavailable,
        stage: "docker_command",
    })
}

fn parse_rust_version(bytes: &[u8]) -> Option<Version> {
    let value = std::str::from_utf8(bytes).ok()?.trim();
    let mut fields = value.split_ascii_whitespace();
    if fields.next()? != "rustc" {
        return None;
    }
    Version::parse(fields.next()?).ok()
}

fn combined_text(output: &CommandOutput) -> String {
    String::from_utf8_lossy(&output.stderr).to_lowercase()
        + &String::from_utf8_lossy(&output.stdout).to_lowercase()
}

fn classify(output: &CommandOutput, stage: &'static str) -> Failure {
    let text = combined_text(output);
    let kind = if output.timed_out {
        FailureKind::DockerCommandTimedOut
    } else if text.contains("failed to connect to the docker api")
        || text.contains("cannot connect to the docker daemon")
        || text.contains("is the docker daemon running")
        || text.contains("no such file or directory") && text.contains("docker.sock")
    {
        FailureKind::DockerDaemonUnavailable
    } else if text.contains("unexpected eof") || text.contains("error waiting for container") {
        FailureKind::DockerStreamEof
    } else if text.contains("permission denied")
        && (text.contains("/target")
            || text.contains("/cargo-cache")
            || text.contains("/rustup-cache"))
    {
        FailureKind::DockerVolumePermission
    } else if stage == "compile"
        && (text.contains("could not compile")
            || text.contains("error[e")
            || text.contains("aborting due to"))
    {
        FailureKind::Compilation
    } else {
        FailureKind::DockerCommand
    };
    Failure { kind, stage }
}

fn volume_exists(name: &str) -> bool {
    docker(&["volume", "inspect", name]).is_ok_and(|output| output.success)
}

fn create_volume(name: &str, kind: &str, identity: &CacheIdentity) -> Result<(), Failure> {
    let rust_label = format!(
        "io.groundline.arm64-controller-cache.rust={}",
        identity.rust_version
    );
    let lock_label = format!(
        "io.groundline.arm64-controller-cache.lock={}",
        identity.lock_sha256
    );
    let kind_label = format!("io.groundline.arm64-controller-cache.kind={kind}");
    let output = docker(&[
        "volume",
        "create",
        "--label",
        CACHE_LABEL,
        "--label",
        &rust_label,
        "--label",
        &lock_label,
        "--label",
        &kind_label,
        name,
    ])?;
    if output.success {
        Ok(())
    } else {
        Err(classify(&output, "cache_create"))
    }
}

fn remove_volume(name: &str) -> Result<(), Failure> {
    let output = docker(&["volume", "rm", name])?;
    if output.success {
        Ok(())
    } else {
        Err(classify(&output, "cache_remove"))
    }
}

fn volume_size_kib(name: &str) -> Result<u64, Failure> {
    let mount = format!("{name}:/cache");
    let output = docker_with_timeout(
        &[
            "run",
            "--rm",
            "--platform",
            "linux/arm64",
            "--label",
            RUN_LABEL,
            "--user",
            "root",
            "-v",
            &mount,
            RUNNER_IMAGE,
            "du",
            "-sk",
            "/cache",
        ],
        Duration::from_secs(60),
    )?;
    if !output.success {
        return Err(classify(&output, "cache_size"));
    }
    std::str::from_utf8(&output.stdout)
        .ok()
        .and_then(|value| value.split_ascii_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(Failure {
            kind: FailureKind::DockerCommand,
            stage: "cache_size",
        })
}

fn runner_identity() -> Result<(String, String), Failure> {
    let uid = docker(&[
        "run",
        "--rm",
        "--platform",
        "linux/arm64",
        "--label",
        RUN_LABEL,
        RUNNER_IMAGE,
        "id",
        "-u",
    ])?;
    if !uid.success {
        return Err(classify(&uid, "runner_uid"));
    }
    let gid = docker(&[
        "run",
        "--rm",
        "--platform",
        "linux/arm64",
        "--label",
        RUN_LABEL,
        RUNNER_IMAGE,
        "id",
        "-g",
    ])?;
    if !gid.success {
        return Err(classify(&gid, "runner_gid"));
    }
    let uid = String::from_utf8(uid.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| value.chars().all(|character| character.is_ascii_digit()))
        .ok_or(Failure {
            kind: FailureKind::DockerCommand,
            stage: "runner_uid",
        })?;
    let gid = String::from_utf8(gid.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| value.chars().all(|character| character.is_ascii_digit()))
        .ok_or(Failure {
            kind: FailureKind::DockerCommand,
            stage: "runner_gid",
        })?;
    Ok((uid, gid))
}

fn initialize_volume_owner(name: &str, uid: &str, gid: &str) -> Result<(), Failure> {
    let mount = format!("{name}:/cache");
    let owner = format!("{uid}:{gid}");
    let output = docker(&[
        "run",
        "--rm",
        "--platform",
        "linux/arm64",
        "--label",
        RUN_LABEL,
        "--user",
        "root",
        "-v",
        &mount,
        RUNNER_IMAGE,
        "chown",
        "-R",
        &owner,
        "/cache",
    ])?;
    if output.success {
        Ok(())
    } else {
        Err(classify(&output, "cache_owner"))
    }
}

fn reset_oversized_volume(
    name: &str,
    kind: &str,
    limit_kib: u64,
    identity: &CacheIdentity,
    uid: &str,
    gid: &str,
) -> Result<bool, Failure> {
    if volume_size_kib(name)? <= limit_kib {
        return Ok(false);
    }
    remove_volume(name)?;
    create_volume(name, kind, identity)?;
    initialize_volume_owner(name, uid, gid)?;
    Ok(true)
}

fn docker_mounts(identity: &CacheIdentity, root: &Path) -> Result<Vec<String>, Failure> {
    let root = root.to_str().ok_or(Failure {
        kind: FailureKind::InvalidSource,
        stage: "source_root",
    })?;
    Ok(vec![
        format!("{root}:/workspace:ro"),
        format!("{}:/rustup-cache", identity.rustup),
        format!("{}:/cargo-cache", identity.cargo),
        format!("{}:/target-cache", identity.target),
    ])
}

fn toolchain_install(identity: &CacheIdentity, mounts: &[String]) -> Result<(), Failure> {
    let toolchain = format!("{}-aarch64-unknown-linux-gnu", identity.rust_version);
    let probe = docker(&[
        "run",
        "--rm",
        "--platform",
        "linux/arm64",
        "--label",
        RUN_LABEL,
        "-e",
        "RUSTUP_HOME=/rustup-cache",
        "-v",
        &mounts[1],
        RUNNER_IMAGE,
        "rustup",
        "run",
        &toolchain,
        "rustc",
        "--version",
    ])?;
    if probe.success {
        return Ok(());
    }
    let probe_failure = classify(&probe, "toolchain_probe");
    if probe_failure.kind != FailureKind::DockerCommand
        || !combined_text(&probe).contains("is not installed")
    {
        return Err(probe_failure);
    }
    let output = docker_with_timeout(
        &[
            "run",
            "--rm",
            "--platform",
            "linux/arm64",
            "--label",
            RUN_LABEL,
            "-e",
            "RUSTUP_HOME=/rustup-cache",
            "-v",
            &mounts[1],
            RUNNER_IMAGE,
            "rustup",
            "toolchain",
            "install",
            &toolchain,
            "--profile",
            "minimal",
        ],
        Duration::from_secs(5 * 60),
    )?;
    if output.success {
        Ok(())
    } else {
        Err(classify(&output, "toolchain"))
    }
}

fn compile(identity: &CacheIdentity, mounts: &[String]) -> Result<(), Failure> {
    let toolchain = format!("{}-aarch64-unknown-linux-gnu", identity.rust_version);
    let output = docker_with_timeout(
        &[
            "run",
            "--rm",
            "--platform",
            "linux/arm64",
            "--label",
            RUN_LABEL,
            "-e",
            "CARGO_HOME=/cargo-cache",
            "-e",
            "RUSTUP_HOME=/rustup-cache",
            "-e",
            &format!("RUSTUP_TOOLCHAIN={toolchain}"),
            "-e",
            "CARGO_TARGET_DIR=/target-cache",
            "-v",
            &mounts[0],
            "-v",
            &mounts[1],
            "-v",
            &mounts[2],
            "-v",
            &mounts[3],
            "-w",
            "/workspace",
            RUNNER_IMAGE,
            "cargo",
            "build",
            "--quiet",
            "--release",
            "--locked",
            "-p",
            "xtask",
            "--bin",
            "groundline-deploy",
        ],
        Duration::from_secs(20 * 60),
    )?;
    if output.success {
        Ok(())
    } else {
        Err(classify(&output, "compile"))
    }
}

fn smoke(mounts: &[String]) -> Result<(), Failure> {
    let output = docker_with_timeout(
        &[
            "run",
            "--rm",
            "--platform",
            "linux/arm64",
            "--label",
            RUN_LABEL,
            "-v",
            &mounts[3],
            RUNNER_IMAGE,
            "/target-cache/release/groundline-deploy",
            "apply",
            "--image",
            "invalid",
            "--expected-current-config-sha256",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--json",
        ],
        Duration::from_secs(60),
    )?;
    let receipt: Value = serde_json::from_slice(&output.stdout).map_err(|_| Failure {
        kind: FailureKind::DockerCommand,
        stage: "smoke_receipt",
    })?;
    if output.code == Some(1)
        && receipt.get("status").and_then(Value::as_str) == Some("FAIL")
        && receipt.get("phase").and_then(Value::as_str) == Some("input")
        && receipt.get("mutation_started").and_then(Value::as_bool) == Some(false)
        && receipt.get("secret_value_printed").and_then(Value::as_bool) == Some(false)
    {
        Ok(())
    } else {
        Err(classify(&output, "smoke"))
    }
}

fn leftover_container_count() -> Option<usize> {
    let output = docker(&[
        "ps",
        "-a",
        "--filter",
        &format!("label={RUN_LABEL}"),
        "--format",
        "{{.ID}}",
    ])
    .ok()?;
    if !output.success {
        return None;
    }
    let ids = String::from_utf8(output.stdout).ok()?;
    ids.lines()
        .all(|id| {
            (12..=64).contains(&id.len())
                && id.chars().all(|character| character.is_ascii_hexdigit())
        })
        .then_some(ids.lines().count())
}

fn failure_outcome(identity: Option<&CacheIdentity>, failure: Failure) -> Outcome {
    let daemon_reachable = docker(&["info", "--format", "{{.ServerVersion}}"])
        .ok()
        .is_some_and(|output| output.success);
    let target_cache_present = identity.is_some_and(|value| volume_exists(&value.target));
    let leftover_container_count = daemon_reachable.then(leftover_container_count).flatten();
    let recovery = if daemon_reachable && target_cache_present {
        "inspect_containers_then_resume_same_fingerprint"
    } else if target_cache_present {
        "restore_daemon_then_resume_same_fingerprint"
    } else {
        "restore_infrastructure_then_rebuild_current_fingerprint"
    };
    Outcome {
        success: false,
        receipt: json!({
            "kind":"groundline-arm64-controller-verification",
            "schema":1,
            "status":"FAIL",
            "evidence_lane":"arm64_linux",
            "stage":failure.stage,
            "reason":failure.kind.code(),
            "code_failure":failure.kind == FailureKind::Compilation,
            "infrastructure_failure":failure.kind != FailureKind::Compilation && failure.kind != FailureKind::InvalidSource,
            "docker_daemon_reachable":daemon_reachable,
            "target_cache_present":target_cache_present,
            "leftover_container_count":leftover_container_count,
            "recovery":recovery,
            "production_mutation_performed":false,
            "private_path_printed":false,
        }),
    }
}

pub fn verify(root: &Path) -> Outcome {
    let canonical = match root.canonicalize() {
        Ok(value) => value,
        Err(_) => {
            return failure_outcome(
                None,
                Failure {
                    kind: FailureKind::InvalidSource,
                    stage: "source_root",
                },
            );
        }
    };
    let identity = match CacheIdentity::current(&canonical) {
        Ok(value) => value,
        Err(failure) => return failure_outcome(None, failure),
    };
    let existing: Vec<bool> = identity
        .names()
        .iter()
        .map(|name| volume_exists(name))
        .collect();
    let result = (|| {
        for (name, kind) in [
            (&identity.rustup, "rustup"),
            (&identity.cargo, "cargo"),
            (&identity.target, "target"),
        ] {
            if !volume_exists(name) {
                create_volume(name, kind, &identity)?;
            }
        }
        let (uid, gid) = runner_identity()?;
        for (name, was_present) in identity.names().iter().zip(&existing) {
            if !was_present {
                initialize_volume_owner(name, &uid, &gid)?;
            }
        }
        let mut reset = Vec::new();
        if reset_oversized_volume(
            &identity.rustup,
            "rustup",
            RUSTUP_LIMIT_KIB,
            &identity,
            &uid,
            &gid,
        )? {
            reset.push("rustup");
        }
        if reset_oversized_volume(
            &identity.cargo,
            "cargo",
            CARGO_LIMIT_KIB,
            &identity,
            &uid,
            &gid,
        )? {
            reset.push("cargo");
        }
        if reset_oversized_volume(
            &identity.target,
            "target",
            TARGET_LIMIT_KIB,
            &identity,
            &uid,
            &gid,
        )? {
            reset.push("target");
        }
        let mounts = docker_mounts(&identity, &canonical)?;
        toolchain_install(&identity, &mounts)?;
        compile(&identity, &mounts)?;
        smoke(&mounts)?;
        for (name, kind, limit) in [
            (&identity.rustup, "rustup", RUSTUP_LIMIT_KIB),
            (&identity.cargo, "cargo", CARGO_LIMIT_KIB),
            (&identity.target, "target", TARGET_LIMIT_KIB),
        ] {
            if volume_size_kib(name)? > limit {
                remove_volume(name)?;
                reset.push(kind);
            }
        }
        Ok::<Vec<&'static str>, Failure>(reset)
    })();
    match result {
        Ok(mut reset) => {
            reset.sort_unstable();
            reset.dedup();
            Outcome {
                success: true,
                receipt: json!({
                    "kind":"groundline-arm64-controller-verification",
                    "schema":1,
                    "status":"PASS",
                    "evidence_lane":"arm64_linux",
                    "rust_stable_version":identity.rust_version.to_string(),
                    "cargo_lock_sha256":identity.lock_sha256,
                    "cache_key_schema":1,
                    "cache_reused":existing.iter().all(|value| *value) && reset.is_empty(),
                    "cache_reset_kinds":reset,
                    "cache_limits_gib":{"rustup":2,"cargo":6,"target":16},
                    "failure_classification_verified":true,
                    "production_mutation_performed":false,
                    "private_path_printed":false,
                }),
            }
        }
        Err(failure) => failure_outcome(Some(&identity), failure),
    }
}

fn valid_cache_volume_name(name: &str) -> bool {
    name.starts_with(CACHE_PREFIX)
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

pub fn prune(root: &Path, confirm: bool) -> Outcome {
    let identity = match CacheIdentity::current(root) {
        Ok(value) => value,
        Err(failure) => return failure_outcome(None, failure),
    };
    let output = match docker(&[
        "volume",
        "ls",
        "--filter",
        &format!("label={CACHE_LABEL}"),
        "--format",
        "{{.Name}}",
    ]) {
        Ok(value) if value.success => value,
        Ok(value) => return failure_outcome(Some(&identity), classify(&value, "cache_list")),
        Err(failure) => return failure_outcome(Some(&identity), failure),
    };
    let current: BTreeSet<&str> = identity.names().into_iter().collect();
    let stale: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|name| valid_cache_volume_name(name) && !current.contains(*name))
        .map(str::to_owned)
        .collect();
    if confirm {
        for name in &stale {
            if let Err(failure) = remove_volume(name) {
                return failure_outcome(Some(&identity), failure);
            }
        }
    }
    Outcome {
        success: true,
        receipt: json!({
            "kind":"groundline-arm64-controller-cache-prune",
            "schema":1,
            "status":"PASS",
            "evidence_lane":"local_cache",
            "stale_volume_count":stale.len(),
            "current_cache_key_count":3,
            "mutation_performed":confirm && !stale.is_empty(),
            "confirmed":confirm,
            "volume_names_printed":false,
            "private_path_printed":false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CacheIdentity, CommandOutput, FailureKind, classify, parse_rust_version,
        valid_cache_volume_name,
    };
    use semver::Version;

    fn failed(stderr: &str) -> CommandOutput {
        CommandOutput {
            success: false,
            code: Some(1),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
            timed_out: false,
        }
    }

    #[test]
    fn stable_version_and_lock_fingerprint_produce_bounded_cache_names() {
        let identity = CacheIdentity::from_version_and_lock(Version::new(1, 98, 0), "a".repeat(64));
        assert_eq!(
            parse_rust_version(b"rustc 1.98.0 (88d9e12ae 2026-08-18)\n"),
            Some(Version::new(1, 98, 0))
        );
        assert!(identity.names().iter().all(|name| name.len() < 96));
        assert!(
            identity
                .names()
                .iter()
                .all(|name| valid_cache_volume_name(name))
        );
        assert!(identity.rustup.ends_with("rustup-1-98-0"));
        assert!(identity.cargo.ends_with("1-98-0-aaaaaaaaaaaaaaaa"));
        assert!(identity.target.ends_with("1-98-0-aaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn docker_and_compiler_failures_are_not_flattened_together() {
        assert_eq!(
            classify(&failed("error[E0425]: missing item"), "compile").kind,
            FailureKind::Compilation
        );
        assert_eq!(
            classify(
                &failed("failed to create directory /target/release: Permission denied"),
                "compile"
            )
            .kind,
            FailureKind::DockerVolumePermission
        );
        assert_eq!(
            classify(
                &failed("error waiting for container: unexpected EOF"),
                "compile"
            )
            .kind,
            FailureKind::DockerStreamEof
        );
        assert_eq!(
            classify(
                &failed("failed to connect to the docker API at unix:///tmp/docker.sock"),
                "compile"
            )
            .kind,
            FailureKind::DockerDaemonUnavailable
        );
        let mut timed_out = failed("");
        timed_out.timed_out = true;
        assert_eq!(
            classify(&timed_out, "compile").kind,
            FailureKind::DockerCommandTimedOut
        );
    }
}
