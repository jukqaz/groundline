fn main() {
    assert!(
        std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
            && std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64"),
        "GroundLine Desktop preview supports macOS Apple Silicon only; see apps/desktop/SECURITY.md before adding another platform"
    );
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "snapshot",
            "usage_summary",
            "server_health",
            "export_diagnostics",
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
            "get_app_preferences",
            "save_app_preferences",
        ]),
    ))
    .expect("desktop build");
}
