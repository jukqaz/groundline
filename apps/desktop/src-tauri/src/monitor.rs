use crate::{commands, lifecycle::Lifecycle};
use serde_json::Value;
use tauri::{Emitter, Manager, menu::MenuItem};
use tauri_plugin_notification::{NotificationExt, PermissionState};

pub struct TrayStatus {
    pub summary: MenuItem<tauri::Wry>,
    pub pending: MenuItem<tauri::Wry>,
    pub receipt: MenuItem<tauri::Wry>,
    pub dashboard: MenuItem<tauri::Wry>,
}

pub fn show_page(app: &tauri::AppHandle, page: &str) {
    crate::lifecycle::show(app);
    let _ = app.emit("groundline:navigate", page);
}

pub fn summary(value: &Value) -> &'static str {
    if value["collection_enabled"] == false {
        return "수집 꺼짐";
    }
    if value["delivery_operator_required"] == true {
        return "연결 확인 필요";
    }
    if value["pending_event_count"].as_u64().unwrap_or(0) > 0 {
        return "전송 대기";
    }
    if value["collection_state"] == "active" && value["delivery_confirmation"].is_object() {
        return "서버 수신 확인";
    }
    match value["collection_state"].as_str() {
        Some("awaiting_first_collection") => "첫 수집 대기",
        Some("active") => "수집 확인 · 수신 기록 없음",
        _ => "상태 확인 필요",
    }
}

#[derive(Default)]
pub struct IssueLatch {
    last: Option<String>,
}
impl IssueLatch {
    pub fn observe(&mut self, runtime: &str, value: &Value, enabled: bool) -> bool {
        let actionable = value["collection_enabled"] == true
            && (value["delivery_operator_required"] == true
                || value["collection_operator_required"] == true
                || value["delivery_attempt_count"].as_u64().unwrap_or(0) >= 3);
        if !enabled || !actionable {
            self.last = None;
            return false;
        }
        let issue = format!(
            "{runtime}:{}:{}",
            value["collection_state"], value["last_delivery_error_code"]
        );
        if self.last.as_ref() == Some(&issue) {
            return false;
        }
        self.last = Some(issue);
        true
    }
}

pub fn start(app: tauri::AppHandle) {
    let state = app.state::<Lifecycle>().inner().clone();
    tauri::async_runtime::spawn(async move {
        let mut latch = IssueLatch::default();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            tokio::select! {
                _ = state.shutdown.cancelled() => break,
                _ = interval.tick() => {}
            }
            let Ok(preferences) = state.preferences() else {
                continue;
            };
            let runtime = preferences.runtime.as_str();
            let result = commands::worker("status", runtime, serde_json::json!({}), &state).await;
            // A preference change while the subprocess runs must not label another runtime's data.
            if state
                .preferences()
                .ok()
                .is_none_or(|p| p.runtime != preferences.runtime)
            {
                continue;
            }
            let tray = app.state::<TrayStatus>();
            match result {
                Ok(value) => {
                    let title = summary(&value);
                    let _ = tray.summary.set_text(format!(
                        "{} · {title}",
                        if runtime == "codex_app" {
                            "Codex App"
                        } else {
                            "Codex CLI"
                        }
                    ));
                    let pending = value["pending_event_count"].as_u64();
                    let _ = tray.pending.set_text(
                        pending
                            .map(|n| format!("전송 대기 {n}건"))
                            .unwrap_or("전송 대기 확인 전".into()),
                    );
                    let received = value["delivery_confirmation"]["confirmed_at_utc"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|v| {
                            v.with_timezone(&chrono::Local)
                                .format("%m/%d %H:%M")
                                .to_string()
                        });
                    let _ = tray.receipt.set_text(
                        received
                            .map(|t| format!("최근 수신 {t}"))
                            .unwrap_or("수신 기록 없음".into()),
                    );
                    let _ = tray
                        .dashboard
                        .set_enabled(value["grafana_url"].as_str().is_some_and(|s| !s.is_empty()));
                    if let Some(icon) = app.tray_by_id("groundline") {
                        let _ = icon.set_tooltip(Some(format!("GroundLine · {title}")));
                        // State glyphs work on macOS without replacing the app's identity icon.
                        let glyph = if value["collection_enabled"] == false {
                            "Ⅱ"
                        } else if value["delivery_operator_required"] == true {
                            "!"
                        } else if pending.is_some_and(|n| n > 0) {
                            "↑"
                        } else {
                            ""
                        };
                        let _ = icon.set_title(Some(glyph));
                    }
                    if latch.observe(runtime, &value, preferences.alerts_enabled)
                        && app.notification().permission_state().ok()
                            == Some(PermissionState::Granted)
                    {
                        let _ = app.notification().builder().title("GroundLine 연결 확인")
                            .body("통계 전송에 확인이 필요한 문제가 있습니다. 앱의 서버 메뉴에서 원인과 해결 방법을 확인하세요.")
                            .show();
                    }
                }
                Err(_) => {
                    let _ = tray.summary.set_text("상태를 읽을 수 없음 · 앱에서 확인");
                    let _ = tray.pending.set_text("전송 대기 확인 불가");
                    let _ = tray.receipt.set_text("최근 수신 확인 불가");
                    if let Some(icon) = app.tray_by_id("groundline") {
                        let _ = icon.set_title(Some("?"));
                        let _ = icon.set_tooltip(Some("GroundLine · 상태 확인 필요"));
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn alerts_once_per_issue_and_never_for_paused_collection() {
        let mut latch = IssueLatch::default();
        let mut failed = json!({"collection_enabled":true,"delivery_operator_required":true,"collection_state":"error"});
        assert!(!latch.observe("codex_app", &failed, false));
        assert!(latch.observe("codex_app", &failed, true));
        assert!(!latch.observe("codex_app", &failed, true));
        failed["collection_enabled"] = json!(false);
        assert!(!latch.observe("codex_app", &failed, true));
        assert_eq!(summary(&failed), "수집 꺼짐");
        failed["collection_enabled"] = json!(true);
        assert!(latch.observe("codex_app", &failed, true));
    }
    #[test]
    fn collection_success_alone_never_claims_delivery() {
        assert_ne!(
            summary(&json!({"collection_state":"active"})),
            "서버 수신 확인"
        );
    }
}
