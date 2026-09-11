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

    #[test]
    fn diagnostics_rebuilds_allowlisted_fields_without_private_text() {
        let value = safe_diagnostics(
            &json!({"endpoint":"private endpoint","key":"private credential",
            "last_delivery_error_code":"private error","pending_event_count":2,"collection_enabled":true,
            "codex_state_store_present":"private text"}),
        );
        assert_eq!(value["pending_event_count"], 2);
        assert!(value["codex_state_store_present"].is_null());
        assert!(!value.to_string().contains("private"));
    }

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
pub async fn usage_summary(
    runtime: String,
    period: String,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<Value, String> {
    worker("usage", &runtime, json!({"period":period}), &lifecycle).await
}

#[tauri::command]
pub async fn server_health(
    runtime: String,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<Value, String> {
    worker("health", &runtime, json!({}), &lifecycle).await
}

pub(crate) fn safe_diagnostics(status: &Value) -> Value {
    let mut result = json!({"schema":1,"kind":"groundline-desktop-diagnostics",
        "desktop_version":env!("CARGO_PKG_VERSION"),"endpoints_included":false,
        "credentials_included":false,"raw_content_included":false});
    for name in [
        "collection_enabled",
        "codex_state_store_present",
        "owner_profile_configured",
        "enrollment_credential_valid",
        "tailnet_required",
        "delivery_operator_required",
        "history_unavailable",
    ] {
        result[name] = status[name]
            .as_bool()
            .map(Value::from)
            .unwrap_or(Value::Null);
    }
    for name in [
        "pending_event_count",
        "quarantined_event_count",
        "delivery_attempt_count",
    ] {
        result[name] = status[name]
            .as_u64()
            .map(Value::from)
            .unwrap_or(Value::Null);
    }
    result["delivery_confirmed"] = json!(
        status["delivery_confirmation"]["event_count"]
            .as_u64()
            .is_some_and(|v| v > 0)
    );
    result
}

#[tauri::command]
pub async fn export_diagnostics(
    runtime: String,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
    state: tauri::State<'_, PendingConnection>,
) -> Result<Value, String> {
    let _operation = state.operation.lock().await;
    let status = worker("status", &runtime, json!({}), &lifecycle).await?;
    let directory = dirs::download_dir()
        .ok_or("downloads_unavailable")?
        .join(format!("GroundLine-diagnostics-{}", uuid::Uuid::new_v4()));
    let mut report = safe_diagnostics(&status);
    report["runtime"] = json!(runtime);
    let bytes = serde_json::to_vec_pretty(&report).map_err(|_| "local_state_failed")?;
    groundline_runtime::local_file::atomic_write_private(
        &directory.join("diagnostics.json"),
        &bytes,
    )
    .map_err(|_| "local_state_failed")?;
    groundline_runtime::local_file::atomic_write_private(&directory.join("README.ko.md"),
        "# GroundLine 진단\n\n진단 파일: diagnostics.json\n서버 주소, 등록키, 대화 원문, 파일 경로는 포함하지 않습니다.\n".as_bytes())
        .map_err(|_| "local_state_failed")?;
    *state.exported.lock().map_err(|_| "worker_failed")? = Some(directory);
    Ok(json!({"exported":true}))
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
    app: tauri::AppHandle,
    lifecycle: tauri::State<'_, crate::lifecycle::Lifecycle>,
) -> Result<crate::preferences::Preferences, String> {
    if preferences.alerts_enabled && !lifecycle.preferences()?.alerts_enabled {
        use tauri_plugin_notification::{NotificationExt, PermissionState};
        let permission = app
            .notification()
            .request_permission()
            .map_err(|_| "notification_unavailable")?;
        if permission != PermissionState::Granted {
            return Err("notification_permission_required".into());
        }
    }
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
    let url = crate::desktop_settings::dashboard_url(&crate::desktop_settings::load(&home)?)?;
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|_| "dashboard_open_failed".into())
}
