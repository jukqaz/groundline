use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use command_group::{CommandGroup, GroupChild};
use serde_json::{Value, json};

const PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_STATUS_BYTES: u64 = 1024 * 1024;
const MAX_DIAGNOSTIC_BYTES: u64 = 64 * 1024;
const PROBE_ENV_ALLOWLIST: &[&str] = &[
    "APPDATA",
    "HOME",
    "LOCALAPPDATA",
    "PROGRAMFILES",
    "PROGRAMW6432",
    "SYSTEMROOT",
    "TEMP",
    "TMP",
    "TMPDIR",
    "USERPROFILE",
    "WINDIR",
    "XDG_RUNTIME_DIR",
];

pub fn checked_at_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn candidates(os: &str, environment: &BTreeMap<String, String>) -> Vec<PathBuf> {
    match os {
        "macos" => [
            "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
            "/opt/homebrew/bin/tailscale",
            "/usr/local/bin/tailscale",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect(),
        "linux" => [
            "/usr/bin/tailscale",
            "/usr/local/bin/tailscale",
            "/snap/bin/tailscale",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect(),
        "windows" => ["PROGRAMW6432", "PROGRAMFILES"]
            .into_iter()
            .filter_map(|name| environment.get(name))
            .map(|root| PathBuf::from(root).join("Tailscale").join("tailscale.exe"))
            .collect(),
        _ => Vec::new(),
    }
}

fn normalized_environment<I>(environment: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = (String, String)>,
{
    environment
        .into_iter()
        .map(|(name, value)| (name.to_ascii_uppercase(), value))
        .collect()
}

pub fn resolve_tailscale_cli() -> Option<PathBuf> {
    let environment = normalized_environment(std::env::vars());
    candidates(std::env::consts::OS, &environment)
        .into_iter()
        .find(|path| path.is_absolute() && path.is_file())
}

pub fn probe_environment() -> BTreeMap<String, String> {
    probe_environment_from(std::env::vars())
}

fn probe_environment_from<I>(environment: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut child = normalized_environment(environment)
        .into_iter()
        .filter(|(name, _)| PROBE_ENV_ALLOWLIST.contains(&name.as_str()))
        .collect::<BTreeMap<_, _>>();
    child.insert("TAILSCALE_BE_CLI".to_owned(), "1".to_owned());
    child
}

fn result(
    state: &str,
    connected: Option<bool>,
    health: &str,
    cli_available: bool,
    timestamp: &str,
    reason_code: &str,
) -> Value {
    json!({
        "tailnet_status": state,
        "tailnet_connected": connected,
        "tailnet_health": health,
        "tailnet_cli_available": cli_available,
        "tailnet_reason_code": reason_code,
        "last_tailnet_check_utc": timestamp,
        "probe_method": "local_cli_only",
        "network_performed": false,
        "private_values_emitted": false,
    })
}

fn health_status(payload: &serde_json::Map<String, Value>) -> &'static str {
    let health = payload.get("Health");
    let self_state = payload.get("Self");
    let non_empty_health = match health {
        Some(Value::Array(value)) => !value.is_empty(),
        Some(Value::Object(value)) => !value.is_empty(),
        _ => false,
    };
    if non_empty_health
        || self_state
            .and_then(Value::as_object)
            .and_then(|value| value.get("Online"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        "degraded"
    } else if matches!(health, Some(Value::Array(_) | Value::Object(_)))
        || self_state.is_some_and(Value::is_object)
    {
        "ok"
    } else {
        "unknown"
    }
}

pub fn classify_payload(bytes: &[u8], timestamp: &str) -> Value {
    if bytes.len() as u64 > MAX_STATUS_BYTES {
        return result(
            "unknown",
            None,
            "unknown",
            true,
            timestamp,
            "oversized_output",
        );
    }
    let Ok(Value::Object(payload)) = serde_json::from_slice::<Value>(bytes) else {
        let reason = if bytes.starts_with(b"The Tailscale CLI failed to start") {
            "cli_start_failed"
        } else {
            "invalid_json"
        };
        return result("unknown", None, "unknown", true, timestamp, reason);
    };
    let (state, connected, reason) = match payload.get("BackendState").and_then(Value::as_str) {
        Some("Running") => ("connected", Some(true), "probe_succeeded"),
        Some("Stopped") => ("disconnected", Some(false), "backend_stopped"),
        Some("NeedsLogin") => ("login_required", Some(false), "login_required"),
        Some("NeedsMachineAuth") => (
            "machine_approval_required",
            Some(false),
            "machine_approval_required",
        ),
        Some("Starting" | "NoState") => ("starting", Some(false), "backend_starting"),
        _ => ("unknown", None, "unknown_backend_state"),
    };
    result(
        state,
        connected,
        health_status(&payload),
        true,
        timestamp,
        reason,
    )
}

fn read_bounded(stream: impl Read, maximum: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = stream.take(maximum + 1).read_to_end(&mut bytes);
    bytes
}

fn wait_with_timeout(
    child: &mut GroupChild,
    timeout: Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let now = Instant::now();
        if now >= deadline {
            return Ok(None);
        }
        thread::sleep((deadline - now).min(Duration::from_millis(20)));
    }
}

pub fn probe() -> Value {
    let timestamp = checked_at_utc();
    let Some(cli) = resolve_tailscale_cli() else {
        return result(
            "cli_unavailable",
            None,
            "unknown",
            false,
            &timestamp,
            "cli_unavailable",
        );
    };
    let mut command = Command::new(cli);
    command
        .args(["status", "--json", "--peers=false", "--self=true"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(probe_environment());
    let mut child = match command.group_spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            return result(
                "probe_denied",
                None,
                "unknown",
                true,
                &timestamp,
                "permission_denied",
            );
        }
        Err(_) => {
            return result(
                "local_api_unavailable",
                None,
                "unknown",
                true,
                &timestamp,
                "spawn_failed",
            );
        }
    };
    let stdout = child.inner().stdout.take();
    let stderr = child.inner().stderr.take();
    let stdout_reader =
        stdout.map(|stdout| thread::spawn(move || read_bounded(stdout, MAX_STATUS_BYTES)));
    let stderr_reader =
        stderr.map(|stderr| thread::spawn(move || read_bounded(stderr, MAX_DIAGNOSTIC_BYTES)));
    let status = wait_with_timeout(&mut child, PROBE_TIMEOUT);
    if !matches!(status.as_ref(), Ok(Some(_))) {
        let _ = child.kill();
        let _ = child.wait();
    }
    let bytes = stdout_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    let _diagnostic = stderr_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    match status {
        Ok(None) => result(
            "probe_timeout",
            None,
            "unknown",
            true,
            &timestamp,
            "timeout",
        ),
        Ok(Some(status)) if !status.success() => result(
            "local_api_unavailable",
            None,
            "unknown",
            true,
            &timestamp,
            "nonzero_exit",
        ),
        Ok(Some(_)) => classify_payload(&bytes, &timestamp),
        Err(_) => result(
            "local_api_unavailable",
            None,
            "unknown",
            true,
            &timestamp,
            "wait_failed",
        ),
    }
}

pub fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_file())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::{candidates, classify_payload, probe_environment_from};

    #[test]
    fn resolver_candidates_never_use_path_or_the_working_directory() {
        let environment = BTreeMap::from([
            ("PATH".to_owned(), ".:/private/bin".to_owned()),
            ("PROGRAMFILES".to_owned(), "C:\\Program Files".to_owned()),
        ]);
        let windows = candidates("windows", &environment);
        assert_eq!(windows.len(), 1);
        let windows_path = windows[0].to_string_lossy().replace('\\', "/");
        assert!(windows_path.starts_with("C:/Program Files"));
        assert!(windows_path.ends_with("Tailscale/tailscale.exe"));
        assert!(
            candidates("linux", &environment)
                .iter()
                .all(|path| path.to_string_lossy().starts_with('/'))
        );
    }

    #[test]
    fn probe_environment_excludes_paths_and_credentials() {
        let environment = probe_environment_from([
            ("HOME".to_owned(), "/private/home".to_owned()),
            ("PATH".to_owned(), ".:/private/bin".to_owned()),
            ("GITHUB_TOKEN".to_owned(), "private-token".to_owned()),
        ]);
        assert_eq!(
            environment,
            BTreeMap::from([
                ("HOME".to_owned(), "/private/home".to_owned()),
                ("TAILSCALE_BE_CLI".to_owned(), "1".to_owned()),
            ])
        );
    }

    #[test]
    fn running_state_and_health_are_kept_separate() {
        let result = classify_payload(
            &serde_json::to_vec(&json!({
                "BackendState": "Running",
                "Health": ["private health text"],
                "Self": {"Online": false, "HostName": "private"},
                "Peer": {"private": {"DNSName": "private.ts.net"}}
            }))
            .expect("fixture"),
            "2026-08-09T00:00:00Z",
        );
        assert_eq!(result["tailnet_status"], "connected");
        assert_eq!(result["tailnet_connected"], true);
        assert_eq!(result["tailnet_health"], "degraded");
        assert_eq!(result["tailnet_reason_code"], "probe_succeeded");
        let serialized = serde_json::to_string(&result).expect("result");
        for private in ["private health text", "HostName", "Peer", "ts.net"] {
            assert!(!serialized.contains(private));
        }
    }

    #[test]
    fn explicit_non_running_states_are_not_connected() {
        for (backend, state) in [
            ("Stopped", "disconnected"),
            ("NeedsLogin", "login_required"),
            ("NeedsMachineAuth", "machine_approval_required"),
            ("Starting", "starting"),
            ("NoState", "starting"),
        ] {
            let bytes = serde_json::to_vec(&json!({"BackendState": backend})).unwrap();
            let result = classify_payload(&bytes, "2026-08-09T00:00:00Z");
            assert_eq!(result["tailnet_status"], state);
            assert_eq!(result["tailnet_connected"], false);
        }
    }

    #[test]
    fn malformed_or_oversized_payload_is_unknown() {
        let malformed = classify_payload(b"not-json", "2026-08-09T00:00:00Z");
        let oversized = classify_payload(&vec![b'x'; 1024 * 1024 + 1], "2026-08-09T00:00:00Z");
        assert_eq!(malformed["tailnet_status"], "unknown");
        assert_eq!(oversized["tailnet_status"], "unknown");
        assert_eq!(malformed["tailnet_reason_code"], "invalid_json");
        assert_eq!(oversized["tailnet_reason_code"], "oversized_output");
        assert_eq!(malformed["tailnet_connected"], Value::Null);
    }

    #[test]
    fn tailscale_cli_start_failure_is_distinguished_without_emitting_output() {
        let result = classify_payload(
            b"The Tailscale CLI failed to start because the local backend is unavailable",
            "2026-08-09T00:00:00Z",
        );
        assert_eq!(result["tailnet_status"], "unknown");
        assert_eq!(result["tailnet_reason_code"], "cli_start_failed");
        assert!(
            !serde_json::to_string(&result)
                .expect("result")
                .contains("local backend")
        );
    }

    use serde_json::Value;
}
