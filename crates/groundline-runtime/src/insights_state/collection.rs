//! One immutable collection window at a time. Never publish a partial aggregate.
use super::*;

const FILE: &str = "collection-window.json";
const MAX_WINDOW_BYTES: u64 = 128 * 1024;
const MAX_ATTEMPTS: u32 = 3;
const INITIAL_LOOKBACK: chrono::Duration = chrono::Duration::days(7);

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Window {
    schema_version: u8,
    start_utc: String,
    end_utc: String,
    attempts: u32,
    prepared: bool,
    event: Option<Value>,
}

impl Window {
    pub(super) fn blocked(&self, trigger: &str) -> bool {
        !self.prepared && self.attempts >= MAX_ATTEMPTS && !explicit_operator_retry(trigger)
    }
}

pub(super) fn read(directory: &Path) -> Result<Option<Window>, StateError> {
    let path = directory.join(FILE);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(StateError::LocalState),
        Ok(_) => (),
    }
    let window: Window = serde_json::from_slice(&read_bytes(&path, MAX_WINDOW_BYTES, true)?)
        .map_err(|_| StateError::LocalState)?;
    if window.schema_version != 1
        || window.attempts > MAX_ATTEMPTS
        || parse_timestamp(&window.start_utc)? >= parse_timestamp(&window.end_utc)?
        || (!window.prepared && window.event.is_some())
    {
        return Err(StateError::LocalState);
    }
    if let Some(event) = &window.event {
        let encoded = serde_json::to_vec(event).map_err(|_| StateError::LocalState)?;
        validate_basic_event_bytes(&encoded).map_err(|_| StateError::LocalState)?;
        for (key, expected) in [
            ("start_utc", &window.start_utc),
            ("end_utc", &window.end_utc),
        ] {
            if event
                .pointer(&format!("/period/{key}"))
                .and_then(Value::as_str)
                .and_then(|s| parse_timestamp(s).ok())
                != Some(parse_timestamp(expected)?)
            {
                return Err(StateError::LocalState);
            }
        }
    }
    Ok(Some(window))
}

pub(super) fn finish_committed(directory: &Path, cursor: Option<&str>) -> Result<(), StateError> {
    if let Some(window) = read(directory)?
        && let Some(cursor) = cursor
        && parse_timestamp(cursor)? >= parse_timestamp(&window.end_utc)?
    {
        if !window.prepared {
            return Err(StateError::LocalState);
        }
        std::fs::remove_file(directory.join(FILE)).map_err(|_| StateError::LocalState)?;
    }
    Ok(())
}

pub(super) struct Source<'a> {
    pub generation: u32,
    pub trigger: &'a str,
}

pub(super) fn stage(
    directory: &Path,
    cursor: Option<&str>,
    now: DateTime<Utc>,
    identity: &Identity,
    consent: &Consent,
    source: Source<'_>,
    audit: impl FnOnce(DateTime<Utc>, DateTime<Utc>) -> Result<Value, StateError>,
) -> Result<String, StateError> {
    let Source {
        generation,
        trigger,
    } = source;
    finish_committed(directory, cursor)?;
    let mut window = match read(directory)? {
        Some(window) => window,
        None => {
            let start = cursor
                .map(parse_timestamp)
                .transpose()?
                .unwrap_or(now - INITIAL_LOOKBACK);
            if start >= now {
                return Err(StateError::LocalState);
            }
            let window = Window {
                schema_version: 1,
                start_utc: start.to_rfc3339_opts(SecondsFormat::Millis, true),
                end_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
                attempts: 0,
                prepared: false,
                event: None,
            };
            write_json(&directory.join(FILE), &window)?;
            window
        }
    };
    if cursor.is_some_and(|cursor| {
        parse_timestamp(cursor).ok() != parse_timestamp(&window.start_utc).ok()
    }) {
        return Err(StateError::LocalState);
    }
    if window.blocked(trigger) {
        return Err(StateError::CollectionPaused);
    }
    if !window.prepared {
        // Persist the attempt before reading: repeated process crashes also stop.
        window.attempts = window.attempts.saturating_add(1).min(MAX_ATTEMPTS);
        write_json(&directory.join(FILE), &window)?;
        let result = audit(
            parse_timestamp(&window.start_utc)?,
            parse_timestamp(&window.end_utc)?,
        )?;
        if result.get("collection_complete").and_then(Value::as_bool) != Some(true) {
            return Err(StateError::AuditIncomplete);
        }
        let has_samples = [
            "observed_root_sample_count",
            "delegated_rollout_count",
            "guardian_rollout_count",
        ]
        .iter()
        .any(|key| {
            result
                .pointer(&format!("/scope/{key}"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        });
        if has_samples {
            window.event = Some(
                build_basic_event(
                    &result,
                    EventIdentity {
                        instance_id: identity.collector_instance_id,
                        os_family: &identity.os_family,
                        runtime_family: &identity.runtime_family,
                        execution_mode: &identity.execution_mode,
                    },
                    EventConsent {
                        receipt_id: consent.receipt_id,
                        accepted_at_utc: &consent.accepted_at_utc,
                    },
                    env!("CARGO_PKG_VERSION"),
                    generation,
                    trigger,
                )
                .map_err(|_| StateError::AuditFailed)?,
            );
        }
        // Persist exact bytes before enqueue. A crash must replay the same event,
        // not regenerate an aggregate after more records have arrived.
        window.prepared = true;
        write_json(&directory.join(FILE), &window)?;
    }
    if let Some(event) = &window.event {
        if event
            .pointer("/collector/instance_id")
            .and_then(Value::as_str)
            != Some(identity.collector_instance_id.to_string().as_str())
            || event.pointer("/consent/receipt_id").and_then(Value::as_str)
                != Some(consent.receipt_id.to_string().as_str())
        {
            return Err(StateError::ReconsentRequired);
        }
        enqueue(directory, event)?;
    }
    Ok(window.end_utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, params};

    fn at(n: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000 + n, 0).unwrap()
    }
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Identity, Consent) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("sessions")).unwrap();
        let rollout = root.join("sessions/active.jsonl");
        let db = Connection::open(root.join("state_5.sqlite")).unwrap();
        db.execute_batch("CREATE TABLE threads (rollout_path TEXT, source TEXT, has_user_event INTEGER, updated_at INTEGER)").unwrap();
        db.execute(
            "INSERT INTO threads VALUES (?1,'cli',1,?2)",
            params![rollout.to_str(), at(20).timestamp()],
        )
        .unwrap();
        let directory = state_directory(&root).unwrap();
        let consent = grant_consent(&directory, at(0)).unwrap();
        let identity = Identity {
            schema_version: 1,
            kind: "groundline-insights-identity".into(),
            collector_instance_id: Uuid::new_v4(),
            os_family: "linux".into(),
            runtime_family: "codex_cli".into(),
            execution_mode: "local_headless".into(),
            created_at_utc: at(0).to_rfc3339(),
            resettable: true,
        };
        (temp, root, rollout, identity, consent)
    }
    fn write_usage(path: &Path, seconds: i64, tokens: u64) {
        let data = format!(
            "{}\n{}\n",
            json!({"type":"session_meta","payload":{"id":"owner"}}),
            json!({"timestamp":at(seconds).to_rfc3339(),"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":tokens}}}})
        );
        std::fs::write(path, data).unwrap();
    }
    fn audit(root: &Path, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Value, StateError> {
        collect_audit(root, start, end, Some("codex_cli"), false)
            .map_err(|_| StateError::AuditFailed)
    }

    #[test]
    fn partial_recovery_freezes_window_and_never_enqueues_partial_or_duplicate_events() {
        let (_temp, root, rollout, identity, consent) = fixture();
        let dir = state_directory(&root).unwrap();
        let cursor = at(0).to_rfc3339();
        assert!(matches!(
            stage(
                &dir,
                Some(&cursor),
                at(10),
                &identity,
                &consent,
                Source {
                    generation: 7,
                    trigger: "manual"
                },
                |s, e| audit(&root, s, e)
            ),
            Err(StateError::AuditIncomplete)
        ));
        assert_eq!(pending_events(&dir, 0).unwrap().observed_count, 0);
        assert!(
            !dir.join(STATUS_FILE).exists(),
            "a partial read cannot commit a cursor"
        );
        write_usage(&rollout, 6, 9);
        let end = stage(
            &dir,
            Some(&cursor),
            at(30),
            &identity,
            &consent,
            Source {
                generation: 7,
                trigger: "manual",
            },
            |s, e| audit(&root, s, e),
        )
        .unwrap();
        assert_eq!(parse_timestamp(&end).unwrap(), at(10));
        let events = pending_events(&dir, 16).unwrap();
        assert_eq!(events.observed_count, 1);
        let original = events.batch[0].1.clone();
        assert_eq!(original["source"]["collection_generation"], 7);
        assert_eq!(original["metrics"]["root"]["usage"]["total_tokens"], 9);
        // Crash after preparing/enqueue, before committing the cursor.
        std::fs::remove_file(&events.batch[0].0).unwrap();
        write_usage(&rollout, 6, 999);
        assert_eq!(
            stage(
                &dir,
                Some(&cursor),
                at(40),
                &identity,
                &consent,
                Source {
                    generation: 7,
                    trigger: "manual"
                },
                |_, _| panic!("must reuse prepared event")
            )
            .unwrap(),
            end
        );
        assert_eq!(pending_events(&dir, 16).unwrap().batch[0].1, original);
        stage(
            &dir,
            Some(&cursor),
            at(40),
            &identity,
            &consent,
            Source {
                generation: 7,
                trigger: "manual",
            },
            |_, _| panic!(),
        )
        .unwrap();
        assert_eq!(pending_events(&dir, 0).unwrap().observed_count, 1);
        // Cursor persistence precedes ACK cleanup. Repeating cleanup is safe.
        finish_committed(&dir, Some(&end)).unwrap();
        finish_committed(&dir, Some(&end)).unwrap();
        let events = pending_events(&dir, 16).unwrap();
        remove_acknowledged_events(&UploadReceipt {
            uploaded_count: 1,
            acknowledged_paths: events.batch.into_iter().map(|(p, _)| p).collect(),
            collected_through_utc: Some(end),
        })
        .unwrap();
        assert_eq!(pending_events(&dir, 0).unwrap().observed_count, 0);
    }

    #[test]
    fn initial_window_uses_seven_days_not_latest_thread_update() {
        let (_temp, root, rollout, identity, consent) = fixture();
        let dir = state_directory(&root).unwrap();
        write_usage(&rollout, 5, 12);
        stage(
            &dir,
            None,
            at(30),
            &identity,
            &consent,
            Source {
                generation: 7,
                trigger: "manual",
            },
            |s, e| {
                assert_eq!(s, at(30) - INITIAL_LOOKBACK);
                audit(&root, s, e)
            },
        )
        .unwrap();
        let events = pending_events(&dir, 16).unwrap();
        assert_eq!(
            events.batch[0].1["metrics"]["root"]["usage"]["total_tokens"],
            12
        );
    }

    #[test]
    fn native_codex_activity_reaches_outbox_without_config_catalog_or_proxy() {
        let (_temp, root, rollout, identity, consent) = fixture();
        std::fs::rename(root.join("state_5.sqlite"), root.join("state_42.sqlite")).unwrap();
        let database = root.join("state_42.sqlite");
        write_usage(&rollout, 5, 27);
        let before_db = std::fs::read(&database).unwrap();
        let before_rollout = std::fs::read(&rollout).unwrap();
        assert!(!root.join("config.toml").exists());
        assert!(
            !root.join("plugins").exists(),
            "Core is not a collection dependency"
        );
        let dir = state_directory(&root).unwrap();
        let cursor = at(0).to_rfc3339();
        stage(
            &dir,
            Some(&cursor),
            at(10),
            &identity,
            &consent,
            Source {
                generation: 7,
                trigger: "manual",
            },
            |s, e| audit(&root, s, e),
        )
        .unwrap();
        let events = pending_events(&dir, 16).unwrap();
        assert_eq!(events.observed_count, 1);
        assert_eq!(events.batch[0].1["source"]["collection_generation"], 7);
        assert_eq!(
            events.batch[0].1["metrics"]["root"]["usage"]["total_tokens"],
            27
        );
        validate_basic_event_bytes(&serde_json::to_vec(&events.batch[0].1).unwrap()).unwrap();
        assert_eq!(std::fs::read(database).unwrap(), before_db);
        assert_eq!(std::fs::read(rollout).unwrap(), before_rollout);
        assert!(!root.join("config.toml").exists());
    }

    #[test]
    fn failed_windows_stop_after_three_attempts_and_require_explicit_retry() {
        let (_temp, root, rollout, identity, consent) = fixture();
        let dir = state_directory(&root).unwrap();
        let cursor = at(0).to_rfc3339();
        for _ in 0..MAX_ATTEMPTS {
            assert!(
                stage(
                    &dir,
                    Some(&cursor),
                    at(10),
                    &identity,
                    &consent,
                    Source {
                        generation: 7,
                        trigger: "activity_checkpoint"
                    },
                    |s, e| audit(&root, s, e)
                )
                .is_err()
            );
        }
        assert!(matches!(
            stage(
                &dir,
                Some(&cursor),
                at(20),
                &identity,
                &consent,
                Source {
                    generation: 7,
                    trigger: "activity_checkpoint"
                },
                |_, _| panic!("bounded retry")
            ),
            Err(StateError::CollectionPaused)
        ));
        assert!(read(&dir).unwrap().unwrap().blocked("activity_checkpoint"));
        write_usage(&rollout, 5, 12);
        stage(
            &dir,
            Some(&cursor),
            at(30),
            &identity,
            &consent,
            Source {
                generation: 7,
                trigger: "manual",
            },
            |s, e| audit(&root, s, e),
        )
        .unwrap();
        assert!(!read(&dir).unwrap().unwrap().blocked("activity_checkpoint"));
    }

    #[test]
    fn empty_complete_window_can_commit_but_invalid_state_cannot_restart_history() {
        let (_temp, root, _, identity, consent) = fixture();
        let dir = state_directory(&root).unwrap();
        let end = stage(
            &dir,
            None,
            at(30),
            &identity,
            &consent,
            Source {
                generation: 7,
                trigger: "manual",
            },
            |_, _| Ok(json!({"collection_complete":true,"scope":{}})),
        )
        .unwrap();
        assert_eq!(pending_events(&dir, 0).unwrap().observed_count, 0);
        finish_committed(&dir, Some(&end)).unwrap();
        write_json(&dir.join(FILE), &json!({"schema_version":999})).unwrap();
        assert!(read(&dir).is_err());
        write_json(&dir.join(STATUS_FILE), &json!({"schema_version":999})).unwrap();
        assert!(current_status(&dir).is_err());
        #[cfg(unix)]
        {
            std::fs::remove_file(dir.join(STATUS_FILE)).unwrap();
            std::os::unix::fs::symlink(dir.join("absent-status"), dir.join(STATUS_FILE)).unwrap();
            assert!(
                current_status(&dir).is_err(),
                "a dangling status link is not a new installation"
            );
        }
    }
}
