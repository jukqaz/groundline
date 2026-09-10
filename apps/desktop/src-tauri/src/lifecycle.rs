use crate::preferences::{self, CloseAction, Preferences};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

#[derive(Clone)]
pub struct Lifecycle {
    home: PathBuf,
    preferences: Arc<Mutex<Result<Preferences, String>>>,
    pub shutdown: CancellationToken,
    pub tasks: TaskTracker,
    exiting: Arc<AtomicBool>,
    exited: Arc<AtomicBool>,
}

impl Lifecycle {
    pub fn new(home: PathBuf) -> Self {
        Self {
            preferences: Arc::new(Mutex::new(preferences::load(&home))),
            home,
            shutdown: CancellationToken::new(),
            tasks: TaskTracker::new(),
            exiting: Arc::new(AtomicBool::new(false)),
            exited: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn preferences(&self) -> Result<Preferences, String> {
        self.preferences
            .lock()
            .map_err(|_| "local_state_failed")?
            .clone()
    }
    pub fn save(&self, value: Preferences) -> Result<Preferences, String> {
        let mut current = self.preferences.lock().map_err(|_| "local_state_failed")?;
        preferences::save(&self.home, &value)?;
        *current = Ok(value.clone());
        Ok(value)
    }
}

pub fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn quit(app: &tauri::AppHandle) {
    let state = app.state::<Lifecycle>().inner().clone();
    if state.exiting.swap(true, Ordering::AcqRel) {
        return;
    }
    state.shutdown.cancel();
    state.tasks.close();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        state.tasks.wait().await;
        state.exited.store(true, Ordering::Release);
        app.exit(0);
    });
}

pub fn exit_requested(app: &tauri::AppHandle, api: &tauri::ExitRequestApi) {
    if !app.state::<Lifecycle>().exited.load(Ordering::Acquire) {
        api.prevent_exit();
        quit(app);
    }
}

pub fn close(window: &tauri::Window, api: &tauri::CloseRequestApi) {
    api.prevent_close();
    let action = window
        .state::<Lifecycle>()
        .preferences()
        .map(|p| p.close_action)
        .unwrap_or(CloseAction::Quit);
    match action {
        CloseAction::Tray => {
            if window.hide().is_err() {
                show(window.app_handle());
            }
        }
        CloseAction::Quit => quit(window.app_handle()),
    }
}

pub fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "show-main", "GroundLine 열기", true, None::<&str>)?;
    let summary = MenuItem::with_id(app, "status", "상태 확인 중…", false, None::<&str>)?;
    let pending = MenuItem::with_id(app, "pending", "전송 대기 확인 전", false, None::<&str>)?;
    let receipt = MenuItem::with_id(app, "receipt", "최근 수신 확인 전", false, None::<&str>)?;
    let dashboard = MenuItem::with_id(app, "dashboard", "Grafana 열기", false, None::<&str>)?;
    let collection = MenuItem::with_id(app, "collection", "수집 설정…", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "quit-app", "완전히 종료", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &summary,
            &pending,
            &receipt,
            &open,
            &dashboard,
            &collection,
            &exit,
        ],
    )?;
    app.manage(crate::monitor::TrayStatus {
        summary,
        pending,
        receipt,
        dashboard,
    });
    let mut tray = TrayIconBuilder::with_id("groundline")
        .tooltip("GroundLine · 활동 통계")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show-main" => show(app),
            "dashboard" => {
                if crate::commands::open_dashboard(app.clone()).is_err() {
                    crate::monitor::show_page(app, "dashboard");
                }
            }
            "collection" => crate::monitor::show_page(app, "collection"),
            "quit-app" => quit(app),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[tokio::test]
    async fn cancellation_waits_for_worker_cleanup() {
        let home = tempfile::tempdir().unwrap();
        let state = Lifecycle::new(home.path().into());
        let shutdown = state.shutdown.clone();
        let cleaned = Arc::new(AtomicBool::new(false));
        let finished = cleaned.clone();
        let token = state.tasks.token();
        tokio::spawn(async move {
            let _token = token;
            shutdown.cancelled().await;
            finished.store(true, Ordering::Release);
        });
        state.shutdown.cancel();
        state.tasks.close();
        tokio::time::timeout(Duration::from_secs(1), state.tasks.wait())
            .await
            .unwrap();
        assert!(cleaned.load(Ordering::Acquire));
    }
}
