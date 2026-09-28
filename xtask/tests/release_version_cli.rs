use std::process::Command;

#[test]
fn release_name_cli_derives_the_shared_canonical_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["release-name", "--version", env!("CARGO_PKG_VERSION")])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        groundline_contracts::version::release_display_name(env!("CARGO_PKG_VERSION")).unwrap()
    );
}

#[test]
fn release_name_cli_rejects_historical_versions_invalid_dates_and_display_labels() {
    for value in ["0.29.0", "2026.229.1", "2026.929.0", "2026.09.29-a"] {
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args(["release-name", "--version", value])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{value}");
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("invalid_release_channel")
        );
    }
}
