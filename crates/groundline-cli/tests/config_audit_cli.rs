use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

fn catalog() -> Vec<u8> {
    serde_json::to_vec(&json!({"models":[{
        "slug":"gpt-6-astra", "default_reasoning_level":"low",
        "supported_reasoning_levels":[{"effort":"low"},{"effort":"medium"}],
        "support_verbosity":true
    }]}))
    .unwrap()
}

#[test]
fn native_catalog_stdin_is_private_bounded_and_read_only() {
    let root = tempdir().unwrap();
    let config = root.path().join("config.toml");
    for (text, expected) in [
        ("", 0),
        ("model='gpt-6-astra'\nmodel_reasoning_effort='medium'", 0),
        ("model='gpt-6-astra'\nmodel_reasoning_effort='none'", 1),
        ("model_context_window=272000", 0),
    ] {
        fs::write(&config, text).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_groundline"))
            .args(["config-audit", "--config"])
            .arg(&config)
            .args(["--catalog", "-", "--json"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&catalog()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(expected));
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["mutation_performed"], false);
        assert_eq!(result["network_performed"], false);
        assert!(!String::from_utf8_lossy(&output.stdout).contains(config.to_str().unwrap()));
        assert_eq!(fs::read_to_string(&config).unwrap(), text);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}

#[test]
fn invalid_catalog_files_do_not_fall_back_to_native_or_bundled_models() {
    let root = tempdir().unwrap();
    let config = root.path().join("config.toml");
    let models = root.path().join("models.json");
    fs::write(&config, "model='gpt-6-astra'").unwrap();
    for bytes in [
        b"PRIVATE_SENTINEL".to_vec(),
        vec![b' '; 8 * 1024 * 1024 + 1],
    ] {
        fs::write(&models, bytes).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
            .args(["config-audit", "--config"])
            .arg(&config)
            .arg("--catalog")
            .arg(&models)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["network_performed"], false);
        assert_eq!(result["mutation_performed"], false);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_SENTINEL"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_SENTINEL"));
    }
}

#[test]
fn catalog_type_errors_report_only_public_schema_fields() {
    let root = tempdir().unwrap();
    let config = root.path().join("config.toml");
    let models = root.path().join("models.json");
    fs::write(&config, "").unwrap();
    let mut native: Value = serde_json::from_slice(&catalog()).unwrap();
    native["models"][0]["slug"] = json!({"PRIVATE_SENTINEL":"PRIVATE_VALUE"});
    fs::write(&models, serde_json::to_vec(&native).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .args(["config-audit", "--config"])
        .arg(&config)
        .arg("--catalog")
        .arg(&models)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["error"], "config_audit_invalid_catalog_model_slug");
    assert_eq!(report["mutation_performed"], false);
    for private in [
        "PRIVATE_SENTINEL",
        "PRIVATE_VALUE",
        root.path().to_str().unwrap(),
    ] {
        assert!(!String::from_utf8_lossy(&output.stdout).contains(private));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(private));
    }
    assert!(fs::read(&config).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn config_symlinks_are_not_followed() {
    let root = tempdir().unwrap();
    let config = root.path().join("config.toml");
    let link = root.path().join("link.toml");
    let models = root.path().join("models.json");
    fs::write(&config, "model='gpt-6-astra'").unwrap();
    fs::write(&models, catalog()).unwrap();
    std::os::unix::fs::symlink(&config, &link).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .args(["config-audit", "--config"])
        .arg(link)
        .arg("--catalog")
        .arg(models)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_to_string(config).unwrap(), "model='gpt-6-astra'");
}
