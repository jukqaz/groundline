use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

fn setup(endpoint: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let input = json!({
        "schema_version":7,"kind":"groundline-insights-owner-profile","mode":"private_owner",
        "endpoint":endpoint,"automatic_activity_checkpoints":true,"automatic_initial_history_sync":true,
        "collection_scope":"all_activity","checkpoint_min_interval_seconds":900,
        "diagnostic_enabled":false,"trigger_mode":"native_hook_checkpoints",
        "enrollment_token":"e".repeat(32),
    });
    configure_profile(home.path(), &serde_json::to_vec(&input).unwrap()).unwrap();
    enable(home.path()).unwrap();
    home
}

async fn request(listener: &TcpListener, route: &str) -> (TcpStream, Value) {
    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut bytes = Vec::new();
    loop {
        let mut chunk = [0; 4096];
        let size = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut chunk))
            .await
            .unwrap()
            .unwrap();
        assert!(size > 0 && bytes.len() + size <= 64 * 1024);
        bytes.extend_from_slice(&chunk[..size]);
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
            assert!(headers.starts_with(route));
            let length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .map(|s| s.parse::<usize>().unwrap())
                .unwrap_or(0);
            if bytes.len() >= end + 4 + length {
                let body = if length == 0 {
                    Value::Null
                } else {
                    serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap()
                };
                return (stream, body);
            }
        }
    }
}

async fn respond(mut stream: TcpStream, body: Value) {
    let body = body.to_string();
    stream
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
}

async fn wait_for_revocation(home: &Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while policy_enabled(&state_directory(home)).unwrap() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn stop_during_health_or_enrollment_prevents_following_requests_and_collection() {
    for delayed_step in [0, 1] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let home = setup(&format!("http://{}", listener.local_addr().unwrap()));
        let worker_home = home.path().to_owned();
        let worker =
            tokio::spawn(async move { run_once(Path::new("."), &worker_home, "manual").await });
        let (reached_tx, reached_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        let server = tokio::spawn(async move {
            let (health, _) = request(&listener, "get /healthz ").await;
            let response = json!({"storage_ready":true,"ingest_capabilities":groundline_contracts::insights::ingest_capabilities()});
            if delayed_step == 0 {
                reached_tx.send(()).unwrap();
                release_rx.await.unwrap();
                respond(health, response).await;
            } else {
                respond(health, response).await;
                let (enrollment, body) = request(&listener, "post /v1/enroll ").await;
                reached_tx.send(()).unwrap();
                release_rx.await.unwrap();
                respond(enrollment, json!({"status":"PASS","collector_instance_id":body["collector_instance_id"],"current_generation":0})).await;
            }
            listener
        });
        reached_rx.await.unwrap();
        let stop_home = home.path().to_owned();
        let stop = tokio::task::spawn_blocking(move || disable(&stop_home));
        wait_for_revocation(home.path()).await;
        assert!(
            !stop.is_finished(),
            "stop must wait for the in-flight request"
        );
        release_tx.send(()).unwrap();
        assert_eq!(stop.await.unwrap().unwrap()["disabled"], true);
        assert!(matches!(worker.await.unwrap(), Err(StateError::Disabled)));
        let listener = server.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
        let directory = state_directory(home.path());
        assert_eq!(pending_events(&directory, 0).unwrap().observed_count, 0);
        assert!(collection::read(&directory).unwrap().is_none());
    }
}

#[tokio::test]
async fn stop_after_first_upload_preserves_unsent_events_and_received_ack() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let home = setup(&format!("http://{}", listener.local_addr().unwrap()));
    let directory = state_directory(home.path());
    let profile = load_profile(home.path()).unwrap();
    let (identity, _) = initialize(&directory, Utc::now()).unwrap();
    let paths: Vec<_> = (0..2)
        .map(|i| directory.join(format!("event-{i}.json")))
        .collect();
    let events: Vec<_> = paths.iter().map(|path| {
        let event = json!({"idempotency_key":Uuid::new_v4(),"period":{"end_utc":Utc::now().to_rfc3339()}});
        write_json(path, &event).unwrap();
        (path.clone(), event)
    }).collect();
    let original = std::fs::read(&paths[1]).unwrap();
    let upload_dir = directory.clone();
    let worker = tokio::spawn(async move {
        upload(
            &upload_dir,
            &profile,
            &identity,
            &SecretString::from("t".repeat(32)),
            events,
        )
        .await
    });
    let (first, _) = request(&listener, "post /v1/events ").await;
    let stop_home = home.path().to_owned();
    let stop = tokio::task::spawn_blocking(move || disable(&stop_home));
    wait_for_revocation(home.path()).await;
    assert!(!stop.is_finished());
    respond(first, json!({"status":"PASS","outcome":"accepted"})).await;
    let receipt = worker.await.unwrap().unwrap();
    assert_eq!(receipt.uploaded_count, 1);
    assert_eq!(receipt.acknowledged_paths, vec![paths[0].clone()]);
    assert_eq!(stop.await.unwrap().unwrap()["disabled"], true);
    assert_eq!(std::fs::read(&paths[1]).unwrap(), original);
    assert!(paths[0].exists(), "ACK removal waits for durable status");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
}

// Execute the real stop path in another process: GUI-only mutexes are insufficient.
#[test]
fn subprocess_stop_uses_the_same_control_lock() {
    const CHILD_HOME: &str = "GROUNDLINE_TEST_STOP_HOME";
    if let Some(home) = std::env::var_os(CHILD_HOME) {
        assert_eq!(disable(Path::new(&home)).unwrap()["disabled"], true);
        return;
    }
    let home = setup("http://127.0.0.1:18080");
    let directory = state_directory(home.path());
    let permit = collection_permit(&directory).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "insights_state::collection_stop_tests::subprocess_stop_uses_the_same_control_lock",
            "--nocapture",
        ])
        .env(CHILD_HOME, home.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    while policy_enabled(&directory).unwrap() {
        if started.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("child did not revoke collection");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(child.try_wait().unwrap().is_none());
    drop(permit);
    assert!(child.wait().unwrap().success());
    assert!(matches!(
        collection_permit(&directory),
        Err(StateError::Disabled)
    ));
}
