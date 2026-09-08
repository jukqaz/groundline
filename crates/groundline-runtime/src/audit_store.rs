use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use groundline_contracts::ContractError;
use groundline_contracts::audit::{AuditWindow, audit_rollouts};
use groundline_contracts::rollout::Record;
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use thiserror::Error;

use crate::local_file::{open_bounded_regular_file, owned_by_current_user};

const MAX_STATE_DATABASE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_THREAD_ROWS: usize = 100_000;
const MAX_ROLLOUT_PATH_BYTES: usize = 4096;
const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MINIMUM_ROOTS: u64 = 5;

#[derive(Debug, Error)]
pub enum AuditStoreError {
    #[error("state_database_not_found")]
    DatabaseNotFound,
    #[error("unsupported_state_database")]
    UnsupportedDatabase,
    #[error("state_database_unavailable")]
    DatabaseUnavailable,
    #[error("audit_input_unavailable")]
    InputUnavailable,
    #[error("audit_failed")]
    AuditFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadKind {
    Root,
    Delegated,
    Guardian,
}

#[derive(Debug)]
struct ThreadRow {
    rollout: PathBuf,
    source: String,
    kind: ThreadKind,
    visible: bool,
    recency_ms: i64,
}

fn state_database(codex_home: &Path) -> Result<PathBuf, AuditStoreError> {
    let home_metadata =
        std::fs::symlink_metadata(codex_home).map_err(|_| AuditStoreError::DatabaseNotFound)?;
    if !home_metadata.is_dir() || home_metadata.file_type().is_symlink() {
        return Err(AuditStoreError::DatabaseUnavailable);
    }
    let selected = std::fs::read_dir(codex_home)
        .map_err(|_| AuditStoreError::DatabaseNotFound)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let version = name
                .to_str()?
                .strip_prefix("state_")?
                .strip_suffix(".sqlite")?;
            if version.is_empty() || !version.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let number = version.parse::<u64>().ok()?;
            (number.to_string() == version).then(|| (number, entry.path()))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, path)| path)
        .ok_or(AuditStoreError::DatabaseNotFound)?;
    // A newer unreadable store must not silently send us to an obsolete copy.
    let file = open_bounded_regular_file(&selected, 1, MAX_STATE_DATABASE_BYTES)
        .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
    if !owned_by_current_user(&file) {
        return Err(AuditStoreError::DatabaseUnavailable);
    }
    selected
        .canonicalize()
        .map_err(|_| AuditStoreError::DatabaseUnavailable)
}

pub fn state_store_present(codex_home: &Path) -> bool {
    state_database(codex_home).is_ok()
}

fn source_kind(source: &str) -> ThreadKind {
    let parsed: Value = serde_json::from_str(source).unwrap_or(Value::Null);
    let Some(subagent) = parsed.get("subagent").and_then(Value::as_object) else {
        return ThreadKind::Root;
    };
    if subagent.get("other").and_then(Value::as_str) == Some("guardian") {
        ThreadKind::Guardian
    } else {
        ThreadKind::Delegated
    }
}

fn thread_rows(database: &Path) -> Result<Vec<ThreadRow>, AuditStoreError> {
    let database_file = open_bounded_regular_file(database, 1, MAX_STATE_DATABASE_BYTES)
        .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
    if !owned_by_current_user(&database_file) {
        return Err(AuditStoreError::DatabaseUnavailable);
    }
    let connection = Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
    connection
        .execute_batch("BEGIN DEFERRED")
        .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
    let columns = connection
        .prepare("PRAGMA table_info(threads)")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<BTreeSet<_>, _>>()
        })
        .map_err(|_| AuditStoreError::UnsupportedDatabase)?;
    if !["rollout_path", "source", "has_user_event"]
        .iter()
        .all(|column| columns.contains(*column))
    {
        return Err(AuditStoreError::UnsupportedDatabase);
    }
    // Sidebar recency can remain at the last user prompt while a long-running
    // turn keeps updating its rollout. Use the newest available activity clock.
    let mut clocks = ["recency_at_ms", "updated_at_ms"]
        .into_iter()
        .filter(|column| columns.contains(*column))
        .map(|column| format!("COALESCE({column}, 0)"))
        .collect::<Vec<_>>();
    if columns.contains("updated_at") {
        // Seconds-resolution values are conservative upper bounds.
        clocks.push("CASE WHEN updated_at > 0 THEN updated_at * 1000 + 999 ELSE 0 END".to_owned());
    }
    let recency = match clocks.len() {
        0 => "0".to_owned(),
        1 => clocks.remove(0),
        _ => format!("MAX({})", clocks.join(", ")),
    };
    let visible = if columns.contains("preview") {
        "(has_user_event != 0 OR preview != '')"
    } else {
        "(has_user_event != 0)"
    };
    let (row_count, oversized_count) = connection
        .query_row(
            "SELECT count(), coalesce(sum(CASE WHEN length(CAST(rollout_path AS BLOB)) > ?1 OR length(CAST(source AS BLOB)) > ?2 THEN 1 ELSE 0 END), 0) FROM threads",
            [MAX_ROLLOUT_PATH_BYTES as i64, MAX_SOURCE_BYTES as i64],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
    if row_count < 0 || row_count as usize > MAX_THREAD_ROWS || oversized_count != 0 {
        return Err(AuditStoreError::UnsupportedDatabase);
    }
    let query = format!(
        "SELECT rollout_path, source, {visible}, {recency} FROM threads ORDER BY {recency} DESC, rollout_path ASC LIMIT {}",
        MAX_THREAD_ROWS + 1
    );
    let mut statement = connection
        .prepare(&query)
        .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
    let rows = statement
        .query_map([], |row| {
            let source = row.get::<_, String>(1)?;
            Ok(ThreadRow {
                rollout: PathBuf::from(row.get::<_, String>(0)?),
                kind: source_kind(&source),
                source,
                visible: row.get::<_, i64>(2)? != 0,
                recency_ms: row.get::<_, i64>(3).unwrap_or(0),
            })
        })
        .map_err(|_| AuditStoreError::DatabaseUnavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
    if rows.len() > MAX_THREAD_ROWS {
        return Err(AuditStoreError::UnsupportedDatabase);
    }
    Ok(rows)
}

fn rollout_roots(codex_home: &Path) -> Result<Vec<PathBuf>, AuditStoreError> {
    let mut roots = Vec::new();
    for directory in ["sessions", "archived_sessions"] {
        let path = codex_home.join(directory);
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(AuditStoreError::InputUnavailable);
        }
        roots.push(
            path.canonicalize()
                .map_err(|_| AuditStoreError::InputUnavailable)?,
        );
    }
    if roots.is_empty() {
        return Err(AuditStoreError::InputUnavailable);
    }
    Ok(roots)
}

fn latest_turn_completed(contents: &str) -> bool {
    // A completion belongs to a turn, not the lifetime of a reusable thread.
    contents
        .lines()
        .rev()
        .find_map(|line| {
            let record = Record::parse(line).ok()??;
            if record.string("type").as_deref() != Some("event_msg") {
                return None;
            }
            let payload = record.value("payload").ok()??;
            match payload.get("type").and_then(Value::as_str)? {
                "task_complete" => Some(true),
                "task_started" | "user_message" | "turn_aborted" => Some(false),
                _ => None,
            }
        })
        .unwrap_or(false)
}

fn runtime_family(originator: Option<&str>, source: &str) -> Option<&'static str> {
    if let Some(originator) = originator {
        let normalized = originator.trim().to_ascii_lowercase().replace('-', "_");
        if matches!(
            normalized.as_str(),
            "codex desktop" | "codex_app" | "codex vscode" | "codex_vscode"
        ) {
            return Some("codex_app");
        }
        if matches!(
            normalized.as_str(),
            "codex_tui" | "codex_exec" | "codex_cli"
        ) {
            return Some("codex_cli");
        }
        return None;
    }
    match source {
        "vscode" => Some("codex_app"),
        "cli" | "exec" => Some("codex_cli"),
        _ => None,
    }
}

fn guardian_from_session(session: Value, rollout_count: usize) -> Value {
    let usage = session
        .get("provider_reported_usage")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "status":session.get("status").cloned().unwrap_or(Value::from("UNKNOWN")),
        "collection_complete":session.get("collection_complete").and_then(Value::as_bool).unwrap_or(false),
        "rollout_count":rollout_count,
        "review_count":session.pointer("/activity/task_completed").and_then(Value::as_u64).unwrap_or(0),
        "provider_reported_usage":usage,
        "outcomes":{},
        "risk_levels":{},
        "signals":{
            "outside_workspace_action_rate":null,
            "temporary_workspace_action_rate":null,
            "reviewer_already_low_effort":false,
            "workspace_attributed_review_count":0,
            "workspace_attribution_coverage":null,
        },
        "raw_content_emitted":false,
        "private_paths_emitted":false,
        "secret_value_printed":false,
    })
}

fn audit_component(
    rollouts: &[String],
    storage_bytes: u64,
    window: AuditWindow,
) -> Result<Value, AuditStoreError> {
    let references = rollouts.iter().map(String::as_str).collect::<Vec<_>>();
    let mut result = audit_rollouts(&references, storage_bytes, 20, window)
        .map_err(|_| AuditStoreError::AuditFailed)?;
    if rollouts.is_empty() {
        result["status"] = Value::from("INSUFFICIENT_EVIDENCE");
    }
    Ok(result)
}

pub fn collect_audit(
    codex_home: &Path,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    runtime_filter: Option<&str>,
    completed_only: bool,
) -> Result<Value, AuditStoreError> {
    if start >= end
        || runtime_filter.is_some_and(|value| !matches!(value, "codex_app" | "codex_cli"))
    {
        return Err(AuditStoreError::AuditFailed);
    }
    let rows = thread_rows(&state_database(codex_home)?)?;
    let allowed_roots = rollout_roots(codex_home)?;
    let start_ms = start.timestamp_millis();
    let mut root = Vec::new();
    let mut delegated = Vec::new();
    let mut guardian = Vec::new();
    let mut seen = BTreeSet::new();
    let mut unreadable = 0_u64;
    let mut unreadable_delegated = 0_u64;
    let mut unreadable_guardian = 0_u64;
    let mut unclassified = 0_u64;
    let mut source_fallback = 0_u64;
    let mut duplicates = 0_u64;
    let mut total_bytes = 0_u64;
    let mut retained_bytes = 0_u64;
    let mut component_bytes = [0_u64; 3];
    for row in rows {
        if (row.recency_ms > 0 && row.recency_ms <= start_ms)
            || (row.kind == ThreadKind::Root && !row.visible)
        {
            continue;
        }
        let normalized =
            crate::rollout::logical_path(&row.rollout).unwrap_or_else(|_| row.rollout.clone());
        if !seen.insert(normalized) {
            duplicates = duplicates.saturating_add(1);
            continue;
        }
        let before_read = total_bytes;
        let mut originator_missing = false;
        let mut classified = None;
        let contents = match crate::rollout::read_audit_rollout(
            &row.rollout,
            &allowed_roots,
            &mut total_bytes,
            &mut retained_bytes,
            |metadata| {
                let originator = metadata.get("originator").and_then(Value::as_str);
                originator_missing = originator.is_none();
                classified = runtime_family(originator, &row.source);
                classified
                    .is_some_and(|family| runtime_filter.is_none_or(|expected| family == expected))
            },
        ) {
            Ok(Some(contents)) => contents,
            Ok(None) => {
                if classified.is_none() {
                    unclassified = unclassified.saturating_add(1);
                }
                continue;
            }
            Err(_) => {
                match row.kind {
                    ThreadKind::Root => unreadable = unreadable.saturating_add(1),
                    ThreadKind::Delegated => {
                        unreadable_delegated = unreadable_delegated.saturating_add(1)
                    }
                    ThreadKind::Guardian => {
                        unreadable_guardian = unreadable_guardian.saturating_add(1)
                    }
                }
                continue;
            }
        };
        if completed_only && row.kind == ThreadKind::Root && !latest_turn_completed(&contents) {
            continue;
        }
        if originator_missing {
            source_fallback = source_fallback.saturating_add(1);
        }
        match row.kind {
            ThreadKind::Root => {
                component_bytes[0] += total_bytes - before_read;
                root.push(contents);
            }
            ThreadKind::Delegated => {
                component_bytes[1] += total_bytes - before_read;
                delegated.push(contents);
            }
            ThreadKind::Guardian => {
                component_bytes[2] += total_bytes - before_read;
                guardian.push(contents);
            }
        }
    }
    let window = AuditWindow {
        start: Some(start),
        end: Some(end),
    };
    let mut root_audit = audit_component(&root, component_bytes[0], window)?;
    let mut delegated_audit = audit_component(&delegated, component_bytes[1], window)?;
    let mut guardian_session = audit_component(&guardian, component_bytes[2], window)?;
    for (audit, missing) in [
        (&mut root_audit, unreadable + unclassified),
        (&mut delegated_audit, unreadable_delegated),
        (&mut guardian_session, unreadable_guardian),
    ] {
        if missing > 0 {
            audit["status"] = Value::from("PARTIAL");
        }
    }
    let guardian_audit = guardian_from_session(guardian_session, guardian.len());
    let sample = root.len() as u64;
    let selection_incomplete =
        unreadable > 0 || unclassified > 0 || unreadable_delegated > 0 || unreadable_guardian > 0;
    // Small samples affect statistical confidence, not collection completeness.
    let collection_complete = !selection_incomplete
        && [&root_audit, &delegated_audit, &guardian_audit]
            .iter()
            .all(|audit| audit.get("collection_complete").and_then(Value::as_bool) == Some(true));
    let status = if selection_incomplete {
        "PARTIAL"
    } else if sample >= MINIMUM_ROOTS
        && [
            root_audit.get("status"),
            delegated_audit.get("status"),
            guardian_audit.get("status"),
        ]
        .iter()
        .all(|value| {
            matches!(
                value.and_then(|value| value.as_str()),
                Some("PASS" | "INSUFFICIENT_EVIDENCE")
            )
        })
    {
        "PASS"
    } else if sample == 0 {
        "INSUFFICIENT_EVIDENCE"
    } else {
        "PARTIAL"
    };
    let kind = if completed_only {
        "groundline-codex-weekly-audit"
    } else {
        "groundline-codex-activity-audit"
    };
    let selection_mode = if completed_only {
        "requested_window"
    } else {
        "activity_window"
    };
    let selection_coverage = if sample == 0 {
        Value::Null
    } else {
        Value::from(1.0)
    };
    Ok(json!({
        "schema":1,"kind":kind,"status":status,"errors":[],"collection_complete":collection_complete,
        "scope":{
            "generated_at":end.to_rfc3339(),"requested_days":((end-start).num_seconds().max(1) as u64).div_ceil(86_400),
            "requested_window_start":start.to_rfc3339(),"requested_window_end":end.to_rfc3339(),"selection_mode":selection_mode,
            "runtime_family":runtime_filter.unwrap_or("all"),"observed_root_sample_count":if completed_only {0} else {sample},
            "completed_root_sample_count":if completed_only {sample} else {0},"eligible_root_count":sample,"selected_root_count":sample,
            "root_truncated_count":0,"selection_coverage":selection_coverage,"selected_recency_start_utc":start.to_rfc3339(),"selected_recency_end_utc":end.to_rfc3339(),
            "minimum_root_sample_count":MINIMUM_ROOTS,"sample_sufficient":sample>=MINIMUM_ROOTS,"delegated_rollout_count":delegated.len(),
            "guardian_rollout_count":guardian.len(),"guardian_incomplete_excluded_count":0,"duplicate_rollout_reference_excluded_count":duplicates,
            "unreadable_completed_root_count":unreadable,"originator_unclassified_excluded_root_count":unclassified,
            "unreadable_delegated_count":unreadable_delegated,"unreadable_guardian_count":unreadable_guardian,
            "read_budget_exhausted":total_bytes>=crate::rollout::MAX_AUDIT_SCAN_BYTES || retained_bytes>=crate::rollout::MAX_AUDIT_BYTES,
            "source_read_bytes":total_bytes,"retained_audit_bytes":retained_bytes,
            "originator_source_fallback_root_count":source_fallback,"delegated_truncated_count":0,"guardian_truncated_count":0,
        },
        "root":root_audit,"delegated":delegated_audit,"guardian":guardian_audit,
        "usage_source_contract":{"cumulative_total_preferred_per_rollout":true,"last_usage_sum_is_fallback_only":true,"window_delta_prevents_double_counting":true,"billing_inference_performed":false},
        "mutation_performed":false,"raw_content_emitted":false,"private_paths_emitted":false,"thread_ids_emitted":false,"rollout_paths_emitted":false,"secret_value_printed":false,
    }))
}

pub fn contract_error(error: AuditStoreError) -> ContractError {
    ContractError(error.to_string())
}

#[cfg(test)]
mod tests {
    use crate::rollout::read_audit_rollout;
    use serde_json::Value;
    use std::fs;

    use rusqlite::{Connection, params};
    use tempfile::{TempDir, tempdir, tempdir_in};

    use super::{collect_audit, latest_turn_completed, rollout_roots, state_database, thread_rows};

    fn fixture_database(home: &Path, rollout: &Path, source: &str) -> PathBuf {
        let database = home.join("state_5.sqlite");
        let connection = Connection::open(&database).expect("fixture database");
        connection
            .execute_batch(
                "CREATE TABLE threads (
                    rollout_path TEXT NOT NULL,
                    source TEXT NOT NULL,
                    archived INTEGER NOT NULL,
                    has_user_event INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );",
            )
            .expect("fixture schema");
        connection
            .execute(
                "INSERT INTO threads (rollout_path, source, archived, has_user_event, updated_at) VALUES (?1, ?2, 0, 1, 1)",
                params![rollout.to_string_lossy(), source],
            )
            .expect("fixture row");
        drop(connection);
        database
    }

    use std::path::{Path, PathBuf};

    fn codex_home() -> TempDir {
        tempdir_in(env!("CARGO_MANIFEST_DIR")).expect("Codex home")
    }

    #[test]
    fn later_thread_updates_do_not_remove_earlier_window_usage() {
        let home = codex_home();
        let sessions = home.path().join("sessions");
        fs::create_dir(&sessions).unwrap();
        let rollout = sessions.join("active.jsonl");
        let db = fixture_database(home.path(), &rollout, "cli");
        fs::write(&rollout, concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"owner\"}}\n",
            "{\"timestamp\":\"1970-01-01T00:00:05Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":5}}}}\n",
            "{\"timestamp\":\"1970-01-01T00:00:15Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"total_tokens\":12}}}}\n"
        )).unwrap();
        let conn = Connection::open(db).unwrap();
        conn.execute("UPDATE threads SET updated_at=20", [])
            .unwrap();
        let time = |n| chrono::DateTime::from_timestamp(n, 0).unwrap();
        let a = collect_audit(home.path(), time(0), time(10), None, false).unwrap();
        let b = collect_audit(home.path(), time(10), time(30), None, false).unwrap();
        assert_eq!(a["root"]["provider_reported_usage"]["total_tokens"], 5);
        assert_eq!(b["root"]["provider_reported_usage"]["total_tokens"], 7);
        assert_eq!(
            a["collection_complete"], true,
            "one root is complete but statistically small"
        );
        conn.execute("UPDATE threads SET updated_at=0", []).unwrap();
        assert_eq!(
            collect_audit(home.path(), time(0), time(10), None, false).unwrap()["root"]["provider_reported_usage"]
                ["total_tokens"],
            5
        );
        conn.execute_batch("ALTER TABLE threads ADD recency_at_ms INTEGER; ALTER TABLE threads ADD updated_at_ms INTEGER; UPDATE threads SET recency_at_ms=5000, updated_at_ms=15000;").unwrap();
        let active = collect_audit(home.path(), time(10), time(20), None, false).unwrap();
        assert_eq!(active["collection_complete"], true);
        assert_eq!(
            active["root"]["provider_reported_usage"]["total_tokens"], 7,
            "stale sidebar recency must not hide a still-running turn"
        );
    }

    #[test]
    fn runtime_classification_preserves_explicit_unknown_originators() {
        assert_eq!(
            super::runtime_family(Some("Codex-App"), "cli"),
            Some("codex_app")
        );
        assert_eq!(
            super::runtime_family(Some("codex_exec"), "vscode"),
            Some("codex_cli")
        );
        assert_eq!(
            super::runtime_family(Some("future-originator"), "cli"),
            None
        );
        assert_eq!(super::runtime_family(None, "vscode"), Some("codex_app"));
        assert_eq!(super::runtime_family(None, "cli"), Some("codex_cli"));
        let content = concat!(
            "{\"type\":\"world_state\",\"payload\":{\"originator\":\"ignored\"}}\n",
            "{\"type\":\"session_meta\",\"payload\":{\"originator\":\"codex_app\"}}\n"
        );
        let home = codex_home();
        let path = home.path().join("metadata.jsonl");
        fs::write(&path, content).unwrap();
        let mut observed = None;
        crate::rollout::read_audit_rollout(
            &path,
            &[home.path().to_path_buf()],
            &mut 0,
            &mut 0,
            |metadata| {
                observed = metadata
                    .get("originator")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                true
            },
        )
        .unwrap();
        assert_eq!(observed.as_deref(), Some("codex_app"));
    }

    #[test]
    fn accepts_owned_database_and_rollout_inside_codex_roots() {
        let home = codex_home();
        let sessions = home.path().join("sessions/2026/08/31");
        fs::create_dir_all(&sessions).expect("sessions");
        let rollout = sessions.join("rollout.jsonl");
        fs::write(&rollout, b"{\"type\":\"event_msg\"}\n").expect("rollout");
        let database = fixture_database(home.path(), &rollout, "cli");

        assert_eq!(
            state_database(home.path()).unwrap(),
            database.canonicalize().unwrap()
        );
        assert_eq!(thread_rows(&database).unwrap().len(), 1);
        let roots = rollout_roots(home.path()).unwrap();
        let mut total = 0;
        assert!(read_audit_rollout(&rollout, &roots, &mut total, &mut 0, |_| true).is_ok());
        assert!(total > 0);
    }

    #[test]
    fn rejects_database_metadata_and_rollouts_outside_codex_roots() {
        let home = codex_home();
        let sessions = home.path().join("sessions");
        fs::create_dir(&sessions).expect("sessions");
        let rollout = sessions.join("rollout.jsonl");
        fs::write(&rollout, b"{}\n").expect("rollout");
        let oversized_source = "x".repeat(super::MAX_SOURCE_BYTES + 1);
        let database = fixture_database(home.path(), &rollout, &oversized_source);
        assert!(thread_rows(&database).is_err());

        let outside = home.path().join("outside.jsonl");
        fs::write(&outside, b"{}\n").expect("outside rollout");
        let roots = rollout_roots(home.path()).unwrap();
        let mut total = 0;
        assert!(read_audit_rollout(&outside, &roots, &mut total, &mut 0, |_| true).is_err());
    }

    #[test]
    fn chooses_numeric_latest_store_and_never_silently_uses_an_older_copy() {
        let home = codex_home();
        for name in [
            "state_5.sqlite",
            "state_9.sqlite",
            "state_10.sqlite",
            "state_999backup.sqlite",
            "state_011.sqlite",
        ] {
            fs::write(home.path().join(name), b"fixture").unwrap();
        }
        assert_eq!(
            state_database(home.path()).unwrap().file_name().unwrap(),
            "state_10.sqlite"
        );
        fs::write(home.path().join("state_11.sqlite"), b"").unwrap();
        assert!(state_database(home.path()).is_err());
    }

    #[test]
    fn previous_completion_does_not_make_a_resumed_or_interrupted_task_complete() {
        let completed = "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n";
        assert!(latest_turn_completed(completed));
        for event in ["task_started", "user_message", "turn_aborted"] {
            let resumed = format!(
                "{completed}{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"{event}\"}}}}\n"
            );
            assert!(!latest_turn_completed(&resumed));
            assert!(latest_turn_completed(&format!("{resumed}{completed}")));
        }
    }

    #[test]
    fn compressed_completed_sample_excludes_active_roots_and_reports_missing_input() {
        let home = codex_home();
        let sessions = home.path().join("sessions");
        fs::create_dir(&sessions).unwrap();
        let plain = sessions.join("completed.jsonl");
        let database = fixture_database(home.path(), &plain, "cli");
        let completed = "{\"timestamp\":\"1970-01-01T00:00:01Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n";
        fs::write(
            plain.with_extension("jsonl.zst"),
            zstd::encode_all(completed.as_bytes(), 1).unwrap(),
        )
        .unwrap();
        let resumed = sessions.join("resumed.jsonl");
        fs::write(
            &resumed,
            format!(
                "{completed}{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"task_started\"}}}}\n"
            ),
        )
        .unwrap();
        let connection = Connection::open(database).unwrap();
        for path in [
            &resumed,
            &plain.with_extension("jsonl.zst"),
            &sessions.join("missing.jsonl"),
        ] {
            connection
                .execute(
                    "INSERT INTO threads VALUES (?1, 'cli', 0, 1, 1)",
                    params![path.to_string_lossy()],
                )
                .unwrap();
        }
        let result = collect_audit(
            home.path(),
            chrono::DateTime::from_timestamp(0, 0).unwrap(),
            chrono::DateTime::from_timestamp(2, 0).unwrap(),
            None,
            true,
        )
        .unwrap();
        assert_eq!(result["scope"]["selected_root_count"], 1);
        assert_eq!(result["scope"]["unreadable_completed_root_count"], 1);
        assert_eq!(
            result["scope"]["duplicate_rollout_reference_excluded_count"],
            1
        );
        assert_eq!(result["status"], "PARTIAL");
        assert_eq!(result["root"]["status"], "PARTIAL");
        assert!(!plain.exists());
    }

    #[cfg(feature = "insights-client")]
    #[test]
    fn activity_audit_builds_a_valid_astra_event_without_faking_completed_roots() {
        use groundline_contracts::event::{CollectorIdentity, ConsentReceipt, build_basic_event};
        let home = codex_home();
        let sessions = home.path().join("sessions");
        fs::create_dir(&sessions).unwrap();
        let rollout = sessions.join("active.jsonl");
        fixture_database(home.path(), &rollout, "cli");
        fs::write(&rollout, concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"fixture-owner\"}}\n",
            "{\"timestamp\":\"1970-01-01T00:00:01Z\",\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-6-astra\",\"effort\":\"high\"}}\n",
            "{\"timestamp\":\"1970-01-01T00:00:01Z\",\"type\":\"token_usage_record\",\"payload\":{\"thread_id\":\"fixture-owner\",\"response_id\":\"fixture-response\",\"usage\":{\"total_tokens\":7}}}\n"
        )).unwrap();
        let audit = collect_audit(
            home.path(),
            chrono::DateTime::from_timestamp(0, 0).unwrap(),
            chrono::DateTime::from_timestamp(2, 0).unwrap(),
            None,
            false,
        )
        .unwrap();
        assert_eq!(audit["scope"]["completed_root_sample_count"], 0);
        assert_eq!(audit["scope"]["observed_root_sample_count"], 1);
        let event = build_basic_event(
            &audit,
            CollectorIdentity {
                instance_id: uuid::Uuid::new_v4(),
                os_family: "linux",
                runtime_family: "codex_cli",
                execution_mode: "local_headless",
            },
            ConsentReceipt {
                receipt_id: uuid::Uuid::new_v4(),
                accepted_at_utc: "1970-01-01T00:00:00Z",
            },
            env!("CARGO_PKG_VERSION"),
            0,
            "manual",
        )
        .unwrap();
        assert_eq!(event["capabilities"]["completed_root_coverage"], false);
        assert_eq!(event["sample"]["root_count"], 1);
        assert_eq!(
            event["metrics"]["root"]["usage"]["source"],
            "codex-response-usage-records"
        );
        assert_eq!(
            event["metrics"]["root"]["model_effort"][0]["model_family"],
            "astra"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_database_and_session_root() {
        use std::os::unix::fs::symlink;

        let home = codex_home();
        let target = home.path().join("actual.sqlite");
        fs::write(&target, b"not sqlite").expect("target");
        symlink(&target, home.path().join("state_5.sqlite")).expect("database symlink");
        assert!(state_database(home.path()).is_err());

        fs::remove_file(home.path().join("state_5.sqlite")).unwrap();
        let outside = tempdir().expect("outside sessions");
        symlink(outside.path(), home.path().join("sessions")).expect("sessions symlink");
        assert!(rollout_roots(home.path()).is_err());
    }
}
