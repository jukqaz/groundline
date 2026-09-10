use groundline_runtime::{insights, insights_state};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeroize::Zeroizing;

struct Verified {
    ticket: String,
    endpoint: String,
    grafana_url: String,
    key: SecretString,
    runtime: String,
    created: Instant,
}
#[derive(Default)]
pub struct PendingConnection {
    verified: Arc<Mutex<Option<Verified>>>,
    operation: tokio::sync::Mutex<()>,
    exported: Mutex<Option<std::path::PathBuf>>,
}

#[tauri::command]
pub async fn cancel_connection(state: tauri::State<'_, PendingConnection>) -> Result<(), String> {
    let _operation = state.operation.lock().await;
    *state.verified.lock().map_err(|_| "worker_failed")? = None;
    Ok(())
}

#[tauri::command]
pub async fn resume_collection(
    runtime: String,
    consent: bool,
    state: tauri::State<'_, PendingConnection>,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<Value, String> {
    let _operation = state.operation.lock().await;
    if !consent {
        return Err("consent_required".into());
    }
    worker("resume", &runtime, json!({}), &lifecycle).await
}

#[tauri::command]
pub async fn save_dashboard(
    url: String,
    state: tauri::State<'_, PendingConnection>,
) -> Result<(), String> {
    let _operation = state.operation.lock().await;
    let home = insights::default_codex_home().map_err(|_| "local_state_failed")?;
    crate::desktop_settings::save(&home, url.trim())
}

#[tauri::command]
pub fn reveal_export(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingConnection>,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = state
        .exported
        .lock()
        .map_err(|_| "worker_failed")?
        .clone()
        .ok_or("export_not_available")?;
    if !path.is_dir() {
        return Err("export_not_available".into());
    }
    app.opener()
        .reveal_item_in_dir(path.join("README.ko.md"))
        .map_err(|_| "export_not_available".into())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionInput {
    endpoint: String,
    grafana_url: String,
    key: String,
    runtime: String,
}

fn runtime_mode(runtime: &str) -> Result<&'static str, String> {
    match runtime {
        "codex_app" => Ok("desktop"),
        "codex_cli" => Ok("local_headless"),
        _ => Err("invalid_runtime".into()),
    }
}

pub(crate) async fn worker(
    action: &str,
    runtime: &str,
    data: Value,
    lifecycle: &crate::lifecycle::Lifecycle,
) -> Result<Value, String> {
    let _tracked = lifecycle.tasks.token();
    if lifecycle.shutdown.is_cancelled() {
        return Err("app_exiting".into());
    }
    let mode = runtime_mode(runtime)?;
    let bytes = Zeroizing::new(serde_json::to_vec(&data).map_err(|_| "invalid_input")?);
    let mut command =
        tokio::process::Command::new(std::env::current_exe().map_err(|_| "worker_failed")?);
    command
        .args(["--worker", action])
        .env("GROUNDLINE_RUNTIME_FAMILY", runtime)
        .env("GROUNDLINE_EXECUTION_MODE", mode)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let child = command.spawn().map_err(|_| "worker_failed")?;
    let (output, status) = wait_worker(child, &bytes, lifecycle).await?;
    if output.len() > 65536 || !status.success() {
        return Err("worker_failed".into());
    }
    let response: Value = serde_json::from_slice(&output).map_err(|_| "worker_failed")?;
    if response["ok"] == true {
        Ok(response["value"].clone())
    } else {
        Err(response["code"]
            .as_str()
            .unwrap_or("worker_failed")
            .to_owned())
    }
}

async fn wait_worker(
    mut child: tokio::process::Child,
    bytes: &[u8],
    lifecycle: &crate::lifecycle::Lifecycle,
) -> Result<(Vec<u8>, std::process::ExitStatus), String> {
    let mut stdin = child.stdin.take().ok_or("worker_failed")?;
    let mut stdout = child.stdout.take().ok_or("worker_failed")?.take(65537);
    let mut output = Vec::new();
    let result = {
        let wait = async {
            stdin.write_all(bytes).await?;
            drop(stdin);
            tokio::try_join!(stdout.read_to_end(&mut output), child.wait())
        };
        tokio::select! {
            biased;
            _ = lifecycle.shutdown.cancelled() => Err("app_exiting"),
            value = tokio::time::timeout(Duration::from_secs(120), wait) =>
                value.map_err(|_| "worker_timeout").and_then(|value| value.map_err(|_| "worker_failed")),
        }
    };
    let (_, status) = match result {
        Ok(value) => value,
        Err(code) => {
            let _ = child.kill().await;
            return Err(code.into());
        }
    };
    Ok((output, status))
}

#[cfg(test)]
mod worker_cleanup_tests {
    use super::*;

    #[tokio::test]
    async fn cancelling_a_running_worker_waits_for_its_process_exit() {
        let home = tempfile::tempdir().unwrap();
        let lifecycle = crate::lifecycle::Lifecycle::new(home.path().into());
        let mut command = tokio::process::Command::new("/bin/sh");
        command
            .args(["-c", "cat >/dev/null; exec sleep 60"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        let child = command.spawn().unwrap();
        let pid = child.id().unwrap();
        let state = lifecycle.clone();
        let task = tokio::spawn(async move { wait_worker(child, b"{}", &state).await });
        tokio::task::yield_now().await;
        lifecycle.shutdown.cancel();
        let result = tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.unwrap_err(), "app_exiting");
        assert!(
            !std::process::Command::new("/bin/kill")
                .args(["-0", &pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap()
                .success()
        );
    }
}

#[tauri::command]
pub async fn snapshot(
    runtime: String,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<Value, String> {
    worker("status", &runtime, json!({}), &lifecycle).await
}

#[tauri::command]
pub fn get_app_preferences(
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<crate::preferences::Preferences, String> {
    lifecycle.preferences()
}

#[tauri::command]
pub fn save_app_preferences(
    preferences: crate::preferences::Preferences,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<crate::preferences::Preferences, String> {
    lifecycle.save(preferences)
}

#[tauri::command]
pub async fn check_connection(
    input: ConnectionInput,
    state: tauri::State<'_, PendingConnection>,
) -> Result<Value, String> {
    let _operation = state.operation.lock().await;
    *state.verified.lock().map_err(|_| "worker_failed")? = None;
    runtime_mode(&input.runtime)?;
    let endpoint = input.endpoint.trim().trim_end_matches('/').to_owned();
    insights::report_url(&endpoint, 7).map_err(|_| "invalid_owner_profile")?;
    crate::desktop_settings::validate_grafana(&input.grafana_url)?;
    let key = SecretString::from(input.key);
    let receipt = insights_state::check_connection(&endpoint, &key)
        .await
        .map_err(|e| e.to_string())?;
    let ticket = uuid::Uuid::new_v4().to_string();
    *state.verified.lock().map_err(|_| "worker_failed")? = Some(Verified {
        ticket: ticket.clone(),
        endpoint,
        grafana_url: input.grafana_url,
        key,
        runtime: input.runtime,
        created: Instant::now(),
    });
    let pending = Arc::clone(&state.verified);
    let expires = ticket.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(600)).await;
        if let Ok(mut value) = pending.lock()
            && value.as_ref().is_some_and(|v| v.ticket == expires)
        {
            *value = None;
        }
    });
    Ok(json!({"ticket":ticket,"receipt":receipt}))
}

#[tauri::command]
pub async fn connect(
    ticket: String,
    consent: bool,
    state: tauri::State<'_, PendingConnection>,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<Value, String> {
    let _operation = state.operation.lock().await;
    if !consent {
        return Err("consent_required".into());
    }
    let verified = state
        .verified
        .lock()
        .map_err(|_| "worker_failed")?
        .take()
        .ok_or("connection_check_required")?;
    if verified.ticket != ticket || verified.created.elapsed() > Duration::from_secs(600) {
        return Err("connection_check_required".into());
    }
    worker(
        "configure",
        &verified.runtime,
        json!({"endpoint":verified.endpoint,"key":verified.key.expose_secret(),"grafana_url":verified.grafana_url}),
        &lifecycle,
    )
    .await
}

#[tauri::command]
pub async fn set_collection(
    runtime: String,
    action: String,
    state: tauri::State<'_, PendingConnection>,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<Value, String> {
    let _operation = state.operation.lock().await;
    if !matches!(action.as_str(), "disable" | "run") {
        return Err("unsupported_operation".into());
    }
    worker(&action, &runtime, json!({}), &lifecycle).await
}

#[tauri::command]
pub async fn export_compose(
    input: crate::setup::SetupInput,
    state: tauri::State<'_, PendingConnection>,
) -> Result<Value, String> {
    let _operation = state.operation.lock().await;
    let result = tauri::async_runtime::spawn_blocking(move || crate::setup::export(input))
        .await
        .map_err(|_| "compose_export_failed")??;
    let directory = result["directory"]
        .as_str()
        .ok_or("compose_export_failed")?;
    *state.exported.lock().map_err(|_| "worker_failed")? = Some(directory.into());
    Ok(result)
}

#[tauri::command]
pub fn open_dashboard(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let home = insights::default_codex_home().map_err(|_| "local_state_failed")?;
    let url = crate::desktop_settings::load(&home)?;
    if url.is_empty() {
        return Err("invalid_grafana_url".into());
    }
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|_| "dashboard_open_failed".into())
}

#[tauri::command]
pub async fn core_diagnostic() -> Result<Value, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let home = insights::default_codex_home().map_err(|_| "local_state_failed")?;
    let root = home.join("plugins/cache/groundline/groundline");
    let mut versions = std::fs::read_dir(root)
        .map_err(|_| "core_not_installed")?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            semver::Version::parse(&entry.file_name().to_string_lossy())
                .ok()
                .map(|v| (v, entry.path()))
        })
        .collect::<Vec<_>>();
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    let (version, root) = versions.first().ok_or("core_not_installed")?;
    let platform =
        groundline_runtime::platform::current_target().map_err(|_| "unsupported_platform")?;
    let binary = root.join(
        groundline_runtime::platform::packaged_binary_path(platform)
            .map_err(|_| "unsupported_platform")?,
    );
    let parent = binary.parent().ok_or("invalid_core_package")?;
    let mut file = groundline_runtime::local_file::open_bounded_regular_file(
        &parent.join("manifest.json"),
        1,
        4096,
    )
    .map_err(|_| "invalid_core_package")?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| "invalid_core_package")?;
    let manifest: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid_core_package")?;
    let mut file = groundline_runtime::local_file::open_bounded_regular_file(&binary, 1, 134217728)
        .map_err(|_| "invalid_core_package")?;
    bytes.clear();
    file.read_to_end(&mut bytes)
        .map_err(|_| "invalid_core_package")?;
    if manifest["sha256"].as_str() != Some(&format!("{:x}", Sha256::digest(&bytes))) {
        return Err("invalid_core_package".into());
    }
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new(&binary)
            .args(["provider-smoke", "--json"])
            .arg("--plugin-root")
            .arg(root)
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "worker_timeout")?
    .map_err(|_| "core_diagnostic_failed")?;
    if output.stdout.len() > 65536 || !output.status.success() {
        return Err("core_diagnostic_failed".into());
    }
    let result: Value =
        serde_json::from_slice(&output.stdout).map_err(|_| "core_diagnostic_failed")?;
    Ok(
        json!({"version":version.to_string(),"status":result["status"],"checksum_verified":true,"live_hooks_verified":false}),
    )
}
