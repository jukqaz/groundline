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
        while policy_enabled(&state_directory(home).unwrap()).unwrap() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn onboarding_requires_consent_and_keeps_fresh_history_uncommitted() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let home = setup(&format!("http://{}", listener.local_addr().unwrap()));
    disable(home.path()).unwrap();
    let report = super::onboarding::setup(home.path(), None, None, None, false, true)
        .await
        .unwrap();
    assert_eq!(report["stages"]["connection_verified_now"], false);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
    let server = tokio::spawn(async move {
        let (health, _) = request(&listener, "get /healthz ").await;
        respond(health, json!({"storage_ready":true,"ingest_capabilities":groundline_contracts::insights::ingest_capabilities()})).await;
        let (enrollment, body) = request(&listener, "post /v1/enroll ").await;
        respond(enrollment, json!({"status":"PASS","collector_instance_id":body["collector_instance_id"],"current_generation":0})).await;
        assert!(
            tokio::time::timeout(Duration::from_millis(150), listener.accept())
                .await
                .is_err()
        );
    });
    let report = super::onboarding::setup(home.path(), None, None, None, true, true)
        .await
        .unwrap();
    server.await.unwrap();
    assert_eq!(report["status"], "ACTION_REQUIRED");
    assert_eq!(report["stages"]["connection_verified_now"], true);
    assert_eq!(report["stages"]["recent_hook_dispatch_observed"], false);
    assert_eq!(report["stages"]["recent_delivery_confirmed"], false);
    let directory = state_directory(home.path()).unwrap();
    assert!(collection::read(&directory).unwrap().is_none());
    assert_eq!(pending_events(&directory, 0).unwrap().observed_count, 0);
}

#[tokio::test]
async fn revision_eight_api_preserves_prepared_window_cursor_identity_and_ack() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let home = setup(&format!("http://{}", listener.local_addr().unwrap()));
    let directory = state_directory(home.path()).unwrap();
    let now = Utc::now();
    let start = (now - chrono::Duration::hours(2)).to_rfc3339();
    let end = (now - chrono::Duration::hours(1)).to_rfc3339();
    // A prepared empty window is valid and must survive an API incompatibility.
    write_json(
        &directory.join("collection-window.json"),
        &json!({
            "schema_version":1,"start_utc":start,"end_utc":end,
            "attempts":1,"prepared":true,"event":null
        }),
    )
    .unwrap();
    persist_cycle_status(
        &directory,
        None,
        now - chrono::Duration::hours(2),
        StatusUpdate {
            result_code: "pass",
            collected_through_utc: Some(start.clone()),
            uploaded_count: 1,
            pending_event_count: 0,
            tailnet_status: "not_required".into(),
            trigger: "manual",
            successful: true,
        },
    )
    .unwrap();
    delivery_confirmation::record(&directory, 1, now - chrono::Duration::hours(2)).unwrap();
    let files = [
        IDENTITY_FILE,
        CONSENT_FILE,
        POLICY_FILE,
        "collection-window.json",
        "delivery-confirmation.json",
    ];
    let before = files.map(|file| std::fs::read(directory.join(file)).unwrap());
    let worker_home = home.path().to_owned();
    let worker =
        tokio::spawn(async move { run_once(Path::new("."), &worker_home, "manual").await });
    let (health, _) = request(&listener, "get /healthz ").await;
    respond(
        health,
        json!({"storage_ready":true,"ingest_capabilities":{
            "basic_schema_versions":[5],"basic_contract_revision":8
        }}),
    )
    .await;
    assert!(matches!(
        worker.await.unwrap(),
        Err(StateError::ApiUpgradeRequired)
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
    assert_eq!(
        files.map(|file| std::fs::read(directory.join(file)).unwrap()),
        before
    );
    let status = current_status(&directory).unwrap().unwrap();
    assert_eq!(
        status.last_collected_through_utc.as_deref(),
        Some(start.as_str())
    );
    assert_eq!(status.last_result_code, "api_upgrade_required");
    assert!(
        read_delivery_retry(&directory)
            .unwrap()
            .unwrap()
            .operator_required
    );
    assert_eq!(pending_events(&directory, 0).unwrap().observed_count, 0);
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
        let directory = state_directory(home.path()).unwrap();
        assert_eq!(pending_events(&directory, 0).unwrap().observed_count, 0);
        assert!(collection::read(&directory).unwrap().is_none());
    }
}

#[tokio::test]
async fn stop_after_first_upload_preserves_unsent_events_and_received_ack() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let home = setup(&format!("http://{}", listener.local_addr().unwrap()));
    let directory = state_directory(home.path()).unwrap();
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
    assert_eq!(
        delivery_confirmation::read(&state_directory(home.path()).unwrap())
            .unwrap()
            .unwrap()
            .event_count,
        1
    );
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

#[tokio::test]
async fn rejected_upload_never_creates_a_server_confirmation() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let home = setup(&format!("http://{}", listener.local_addr().unwrap()));
    let directory = state_directory(home.path()).unwrap();
    let profile = load_profile(home.path()).unwrap();
    let (identity, _) = initialize(&directory, Utc::now()).unwrap();
    let upload_dir = directory.clone();
    let worker = tokio::spawn(async move {
        upload(
            &upload_dir,
            &profile,
            &identity,
            &SecretString::from("t".repeat(32)),
            vec![(
                upload_dir.join("event.json"),
                json!({"idempotency_key":Uuid::new_v4()}),
            )],
        )
        .await
    });
    let (mut stream, _) = request(&listener, "post /v1/events ").await;
    let body = r#"{"status":"FAIL","outcome":"invalid_auth"}"#;
    stream.write_all(format!("HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
    drop(stream);
    assert!(worker.await.unwrap().is_err());
    assert!(delivery_confirmation::read(&directory).unwrap().is_none());
}

#[tokio::test]
async fn user_prompt_submit_preserves_state_and_stop_collects_the_original_window() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let home = setup(&format!("http://{}", listener.local_addr().unwrap()));
    let directory = state_directory(home.path()).unwrap();
    let now = Utc::now();
    let (identity, _) = initialize(&directory, now).unwrap();
    let start = (now - chrono::Duration::minutes(40)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let end = (now - chrono::Duration::minutes(20)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let source = if identity.runtime_family == "codex_app" {
        "vscode"
    } else {
        "cli"
    };
    let sessions = home.path().join("sessions");
    std::fs::create_dir(&sessions).unwrap();
    let rollout = sessions.join("active.jsonl");
    let records = [
        json!({"timestamp":start,"type":"session_meta","payload":{"id":"collection-owner"}}),
        json!({"timestamp":(now - chrono::Duration::minutes(30)).to_rfc3339(),
            "type":"event_msg","payload":{"type":"token_count", "info":{
                "total_token_usage":{"total_tokens":7}}}}),
    ];
    std::fs::write(
        &rollout,
        records
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();
    let db = rusqlite::Connection::open(home.path().join("state_5.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE threads (rollout_path TEXT, source TEXT, has_user_event INTEGER, updated_at INTEGER)").unwrap();
    db.execute(
        "INSERT INTO threads VALUES (?1,?2,1,?3)",
        rusqlite::params![rollout.to_str(), source, now.timestamp()],
    )
    .unwrap();
    drop(db);
    // Preserve the window of an earlier failed hook. A supported Stop must
    // retry those exact timestamps rather than rewrite its collection history.
    write_json(
        &directory.join("collection-window.json"),
        &json!({"schema_version":1,"start_utc":start,"end_utc":end,
            "attempts":1,"prepared":false,"event":null}),
    )
    .unwrap();
    persist_cycle_status_with_failure(
        &directory,
        None,
        now - chrono::Duration::minutes(20),
        StatusUpdate {
            result_code: "audit_failed",
            collected_through_utc: Some(start.clone()),
            uploaded_count: 0,
            pending_event_count: 0,
            tailnet_status: "not_required".into(),
            trigger: "user_prompt_submit_hook",
            successful: false,
        },
        Some(CollectionFailure {
            failure_stage: CollectionFailureStage::EventBuild,
            error_code: CollectionFailureCode::InvalidCollectionTrigger,
        }),
    )
    .unwrap();
    record_delivery_retry(
        &directory,
        now - chrono::Duration::minutes(20),
        &StateError::UploadFailed,
    )
    .unwrap();
    crate::checkpoint::capture_trigger(home.path(), "session_start_hook").unwrap();
    crate::checkpoint::claim_triggers(home.path()).unwrap();
    crate::checkpoint::capture_trigger(home.path(), "stop_hook").unwrap();
    let lock = CycleLock::acquire(&directory).unwrap();
    let preserved = [
        IDENTITY_FILE,
        CONSENT_FILE,
        POLICY_FILE,
        STATUS_FILE,
        RETRY_FILE,
        LOCK_FILE,
        COLLECTION_CONTROL_LOCK,
        "activity-history.json",
        "collection-window.json",
        "hook-captures/stop_hook.json",
        "hook-capture-claims/session_start_hook.json",
    ];
    let before = preserved.map(|file| std::fs::read(directory.join(file)).unwrap());
    let lock_bytes = std::fs::read(directory.join(LOCK_FILE)).unwrap();
    for _ in 0..2 {
        let result = run_once(Path::new("."), home.path(), "user_prompt_submit_hook")
            .await
            .unwrap();
        assert_eq!(result["result_code"], "not_due");
        assert_eq!(result["network_performed"], false);
        assert_eq!(result["mutation_performed"], false);
        assert_eq!(
            std::fs::read(directory.join(LOCK_FILE)).unwrap(),
            lock_bytes
        );
    }
    drop(lock);
    let result = run_once(Path::new("."), home.path(), "user_prompt_submit_hook")
        .await
        .unwrap();
    assert_eq!(result["result_code"], "not_due");
    for (file, original) in preserved.into_iter().zip(before) {
        assert_eq!(
            std::fs::read(directory.join(file)).unwrap(),
            original,
            "{file}"
        );
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );

    let server = tokio::spawn(async move {
        let (health, _) = request(&listener, "get /healthz ").await;
        respond(
            health,
            json!({"storage_ready":true,
            "ingest_capabilities":groundline_contracts::insights::ingest_capabilities()}),
        )
        .await;
        let (enrollment, body) = request(&listener, "post /v1/enroll ").await;
        respond(
            enrollment,
            json!({"status":"PASS",
            "collector_instance_id":body["collector_instance_id"],"current_generation":7}),
        )
        .await;
        let (upload, event) = request(&listener, "post /v1/events ").await;
        respond(upload, json!({"status":"PASS","outcome":"accepted"})).await;
        (listener, event)
    });
    let result = run_once(Path::new("."), home.path(), "stop_hook")
        .await
        .unwrap();
    let (listener, event) = server.await.unwrap();
    assert_eq!(result["result_code"], "pass");
    assert_eq!(result["uploaded_count"], 1);
    assert_eq!(event["source"]["collection_trigger"], "stop_hook");
    assert_eq!(
        parse_timestamp(event["period"]["start_utc"].as_str().unwrap()).unwrap(),
        parse_timestamp(&start).unwrap()
    );
    assert_eq!(
        parse_timestamp(event["period"]["end_utc"].as_str().unwrap()).unwrap(),
        parse_timestamp(&end).unwrap()
    );
    assert_eq!(event["metrics"]["root"]["usage"]["total_tokens"], 7);
    let status = current_status(&directory).unwrap().unwrap();
    assert_eq!(status.last_trigger, "stop_hook");
    assert_eq!(
        parse_timestamp(status.last_collected_through_utc.as_deref().unwrap()).unwrap(),
        parse_timestamp(&end).unwrap()
    );
    assert_eq!(
        status.last_collection_failure.unwrap().error_code,
        CollectionFailureCode::InvalidCollectionTrigger
    );
    assert!(collection::read(&directory).unwrap().is_none());
    assert_eq!(pending_events(&directory, 0).unwrap().observed_count, 0);
    assert_eq!(
        delivery_confirmation::read(&directory)
            .unwrap()
            .unwrap()
            .event_count,
        1
    );

    // A due delivery retry with a valid outbox must also stay untouched at
    // UserPromptSubmit; otherwise delivery would advance Stop's idle cadence.
    enqueue(&directory, &event).unwrap();
    record_delivery_retry(
        &directory,
        now - chrono::Duration::minutes(2),
        &StateError::UploadFailed,
    )
    .unwrap();
    let pending = pending_events(&directory, 16).unwrap();
    assert_eq!(pending.observed_count, 1);
    let event_path = &pending.batch[0].0;
    let event_bytes = std::fs::read(event_path).unwrap();
    let retry_bytes = std::fs::read(directory.join(RETRY_FILE)).unwrap();
    let status_bytes = std::fs::read(directory.join(STATUS_FILE)).unwrap();
    let lock_bytes = std::fs::read(directory.join(LOCK_FILE)).unwrap();
    assert!(delivery_is_due(
        "user_prompt_submit_hook",
        read_delivery_retry(&directory).unwrap().as_ref(),
        Utc::now()
    ));
    let result = run_once(Path::new("."), home.path(), "user_prompt_submit_hook")
        .await
        .unwrap();
    assert_eq!(result["network_performed"], false);
    assert_eq!(std::fs::read(event_path).unwrap(), event_bytes);
    assert_eq!(
        std::fs::read(directory.join(RETRY_FILE)).unwrap(),
        retry_bytes
    );
    assert_eq!(
        std::fs::read(directory.join(STATUS_FILE)).unwrap(),
        status_bytes
    );
    assert_eq!(
        std::fs::read(directory.join(LOCK_FILE)).unwrap(),
        lock_bytes
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
}

#[test]
fn terminated_worker_releases_cycle_lock_without_a_stale_timeout() {
    const CHILD_HOME: &str = "GROUNDLINE_TEST_CYCLE_LOCK_HOME";
    if let Some(home) = std::env::var_os(CHILD_HOME) {
        let directory = Path::new(&home);
        let _lock = CycleLock::acquire(directory).unwrap();
        atomic_write_private(&directory.join("ready"), b"ready").unwrap();
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "insights_state::collection_stop_tests::terminated_worker_releases_cycle_lock_without_a_stale_timeout"])
        .env(CHILD_HOME, directory.path()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::inherit()).spawn().unwrap();
    let started = std::time::Instant::now();
    while !directory.path().join("ready").exists() {
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("worker lock readiness timed out");
        }
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(
        CycleLock::acquire(directory.path()),
        Err(StateError::AlreadyRunning)
    ));
    child.kill().unwrap();
    child.wait().unwrap();
    let lock = CycleLock::acquire(directory.path()).unwrap();
    drop(lock);
    assert!(directory.path().join(LOCK_FILE).exists());
    let original = b"12345\n";
    atomic_write_private(&directory.path().join(LOCK_FILE), original).unwrap();
    assert!(matches!(
        CycleLock::acquire(directory.path()),
        Err(StateError::AlreadyRunning)
    ));
    assert_eq!(
        std::fs::read(directory.path().join(LOCK_FILE)).unwrap(),
        original
    );
}

// Execute the real stop path in another process: in-process mutexes are insufficient.
#[test]
fn subprocess_stop_uses_the_same_control_lock() {
    const CHILD_HOME: &str = "GROUNDLINE_TEST_STOP_HOME";
    if let Some(home) = std::env::var_os(CHILD_HOME) {
        assert_eq!(disable(Path::new(&home)).unwrap()["disabled"], true);
        return;
    }
    let home = setup("http://127.0.0.1:18080");
    let directory = state_directory(home.path()).unwrap();
    // A parallel fork can briefly retain enable's locked file description until
    // exec closes its inherited descriptor. Synchronize this fixture on the real
    // control lock; production collection_permit must remain nonblocking.
    let permit = collection_control_lock(&directory).unwrap();
    let started = std::time::Instant::now();
    loop {
        match permit.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock)
                if started.elapsed() < Duration::from_secs(5) =>
            {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("fixture control lock acquisition failed: {error}"),
        }
    }
    require_collection(&directory).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "insights_state::collection_stop_tests::subprocess_stop_uses_the_same_control_lock",
            "--nocapture",
        ])
        .env(CHILD_HOME, home.path())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    while policy_enabled(&directory).unwrap() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("stop process exited before policy revocation: {status}");
        }
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
