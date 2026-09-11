#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod commands;
mod desktop_settings;
mod lifecycle;
mod monitor;
mod preferences;
mod setup;
mod usage;
mod worker;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--worker") {
        worker::main();
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            lifecycle::show(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(commands::PendingConnection::default())
        .setup(|app| {
            use tauri::Manager;
            let home = groundline_runtime::insights::default_codex_home()
                .map_err(|_| "local_state_failed")?;
            let state = lifecycle::Lifecycle::new(home);
            app.manage(state);
            lifecycle::install_tray(app)?;
            monitor::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                lifecycle::close(window, api);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::usage_summary,
            commands::server_health,
            commands::export_diagnostics,
            commands::check_connection,
            commands::connect,
            commands::set_collection,
            commands::export_compose,
            commands::open_dashboard,
            commands::cancel_connection,
            commands::resume_collection,
            commands::save_dashboard,
            commands::reveal_export,
            commands::get_app_preferences,
            commands::save_app_preferences
        ])
        .build(tauri::generate_context!())
        .expect("desktop runtime failed")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => lifecycle::exit_requested(app, &api),
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => lifecycle::show(app),
            _ => {}
        });
}

#[cfg(test)]
#[test]
fn bundled_capability_does_not_enable_remote_url_patterns() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    assert_eq!(
        config["app"]["security"]["capabilities"],
        serde_json::json!(["main"])
    );
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
    assert!(capability.get("remote").is_none());
    assert_ne!(
        capability.get("local"),
        Some(&serde_json::Value::Bool(false))
    );
    assert_eq!(capability["windows"], serde_json::json!(["main"]));
}
