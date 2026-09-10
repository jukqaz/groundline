#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod commands;
mod desktop_settings;
mod setup;
mod worker;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--worker") {
        worker::main();
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(commands::PendingConnection::default())
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::check_connection,
            commands::connect,
            commands::set_collection,
            commands::export_compose,
            commands::core_diagnostic,
            commands::open_dashboard,
            commands::cancel_connection,
            commands::resume_collection,
            commands::save_dashboard,
            commands::reveal_export
        ])
        .run(tauri::generate_context!())
        .expect("desktop runtime failed");
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
