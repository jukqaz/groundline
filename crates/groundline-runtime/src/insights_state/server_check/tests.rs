use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use groundline_contracts::insights::BASIC_CONTRACT_REVISION;
use serde_json::{Value, json};
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{PROFILE_PATH, check_server};
use crate::local_file::atomic_write_private;

fn profile(home: &Path, endpoint: &str) {
    let mut value: Value = serde_json::from_slice(&super::super::tests::profile("")).unwrap();
    value.as_object_mut().unwrap().remove("enrollment_token");
    value["endpoint"] = endpoint.into();
    atomic_write_private(
        &home.join(PROFILE_PATH),
        &serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}

fn snapshot(path: &Path) -> Vec<(PathBuf, Vec<u8>, SystemTime)> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            files.extend(snapshot(&entry.path()));
        } else {
            files.push((
                entry.path(),
                fs::read(entry.path()).unwrap(),
                entry.metadata().unwrap().modified().unwrap(),
            ));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

#[tokio::test]
async fn missing_profile_is_not_configured_without_creating_state() {
    let home = tempdir().unwrap();
    for path in [home.path().to_path_buf(), home.path().join("missing-home")] {
        let result = check_server(Some(&path)).await;
        assert_eq!(result["status"], "NOT_CONFIGURED");
        assert_eq!(result["result_code"], "owner_profile_not_configured");
        assert_eq!(result["network_attempted"], false);
        assert_eq!(result["mutation_performed"], false);
    }
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn invalid_profiles_are_rejected_without_network_or_writes() {
    let home = tempdir().unwrap();
    let path = home.path().join(PROFILE_PATH);
    for bytes in [b"PRIVATE_INVALID_PROFILE".as_slice(), b"{}", b""] {
        atomic_write_private(&path, bytes).unwrap();
        let before = snapshot(home.path());
        let result = check_server(Some(home.path())).await;
        assert_eq!(result["status"], "FAIL");
        assert_eq!(result["result_code"], "invalid_owner_profile");
        assert_eq!(result["network_attempted"], false);
        assert_eq!(result["mutation_performed"], false);
        assert!(!result.to_string().contains("PRIVATE_INVALID_PROFILE"));
        assert_eq!(snapshot(home.path()), before);
    }
    #[cfg(unix)]
    {
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(home.path().join("missing-profile"), &path).unwrap();
        let result = check_server(Some(home.path())).await;
        assert_eq!(result["result_code"], "invalid_owner_profile");
        assert_eq!(result["network_attempted"], false);
        assert!(fs::symlink_metadata(path).unwrap().is_symlink());
    }
}

#[tokio::test]
async fn health_checks_preserve_state_and_never_send_credentials_or_collect() {
    for (status, body, expected) in [
        (200, json!({"storage_ready":true,"ingest_capabilities":groundline_contracts::insights::ingest_capabilities()}).to_string(), "api_compatible"),
        (200, json!({"storage_ready":true,"ingest_capabilities":{"basic_schema_versions":[5],"basic_contract_revision":7}}).to_string(), "api_upgrade_required"),
        (200, json!({"storage_ready":true,"ingest_capabilities":{"basic_schema_versions":[5],"basic_contract_revision":8}}).to_string(), "api_upgrade_required"),
        (404, "old api".to_owned(), "api_upgrade_required"),
        (503, json!({"storage_ready":false}).to_string(), "event_upload_failed"),
        (200, "PRIVATE_SERVER_BODY".to_owned(), "event_upload_failed"),
    ] {
        let home = tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        profile(home.path(), &endpoint);
        for name in ["enrollment-token", "codex-cli/identity.json", "codex-cli/consent.json", "codex-cli/owner-auto-status.json"] {
            atomic_write_private(&home.path().join("groundline/insights").join(name), b"PRIVATE_STATE_UNCHANGED").unwrap();
        }
        let before = snapshot(home.path());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let size = stream.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..size]).to_ascii_lowercase();
            assert!(request.starts_with("get /healthz http/1.1\r\n"));
            assert!(!request.contains("authorization:"));
            assert!(!request.contains("private_state"));
            let response = format!("HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let result = check_server(Some(home.path())).await;
        server.await.unwrap();
        assert_eq!(result["status"], if expected == "api_compatible" { "PASS" } else { "FAIL" });
        assert_eq!(result["result_code"], expected);
        assert_eq!(result["required_basic_contract_revision"], BASIC_CONTRACT_REVISION);
        assert_eq!(result["network_attempted"], true);
        assert_eq!(result["mutation_performed"], false);
        assert!(!result.to_string().contains(&endpoint));
        assert!(!result.to_string().contains("PRIVATE_"));
        assert_eq!(snapshot(home.path()), before);
    }
}

#[tokio::test]
async fn transport_failure_is_blocking_and_does_not_initialize_retry_state() {
    let home = tempdir().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    profile(
        home.path(),
        &format!("http://{}", listener.local_addr().unwrap()),
    );
    drop(listener);
    let before = snapshot(home.path());
    let result = check_server(Some(home.path())).await;
    assert_eq!(result["status"], "FAIL");
    assert_eq!(result["result_code"], "event_upload_failed");
    assert_eq!(result["network_attempted"], true);
    assert_eq!(result["mutation_performed"], false);
    assert_eq!(snapshot(home.path()), before);
}
