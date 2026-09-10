fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "snapshot",
            "check_connection",
            "connect",
            "set_collection",
            "export_compose",
            "core_diagnostic",
            "open_dashboard",
            "cancel_connection",
            "resume_collection",
            "save_dashboard",
            "reveal_export",
        ]),
    ))
    .expect("desktop build");
}
