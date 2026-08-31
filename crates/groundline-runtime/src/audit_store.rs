use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use groundline_contracts::ContractError;
use groundline_contracts::audit::{AuditWindow, audit_rollouts};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use thiserror::Error;

use crate::local_file::{open_bounded_regular_file, owned_by_current_user};

const MAX_ROLLOUT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_AUDIT_BYTES: u64 = 512 * 1024 * 1024;
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
    archived: bool,
    visible: bool,
    recency_ms: i64,
}

fn state_database(codex_home: &Path) -> Result<PathBuf, AuditStoreError> {
    let home_metadata =
        std::fs::symlink_metadata(codex_home).map_err(|_| AuditStoreError::DatabaseNotFound)?;
    if !home_metadata.is_dir() || home_metadata.file_type().is_symlink() {
        return Err(AuditStoreError::DatabaseUnavailable);
    }
    let preferred = codex_home.join("state_5.sqlite");
    if std::fs::symlink_metadata(&preferred).is_ok() {
        let file = open_bounded_regular_file(&preferred, 1, MAX_STATE_DATABASE_BYTES)
            .map_err(|_| AuditStoreError::DatabaseUnavailable)?;
        if !owned_by_current_user(&file) {
            return Err(AuditStoreError::DatabaseUnavailable);
        }
        return preferred
            .canonicalize()
            .map_err(|_| AuditStoreError::DatabaseUnavailable);
    }
    let mut candidates = std::fs::read_dir(codex_home)
        .map_err(|_| AuditStoreError::DatabaseNotFound)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("state_") && value.ends_with(".sqlite"))
        })
        .filter(|path| {
            open_bounded_regular_file(path, 1, MAX_STATE_DATABASE_BYTES)
                .is_ok_and(|file| owned_by_current_user(&file))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .pop()
        .ok_or(AuditStoreError::DatabaseNotFound)?
        .canonicalize()
        .map_err(|_| AuditStoreError::DatabaseUnavailable)
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
    if !["rollout_path", "source", "archived", "has_user_event"]
        .iter()
        .all(|column| columns.contains(*column))
    {
        return Err(AuditStoreError::UnsupportedDatabase);
    }
    let recency = if ["recency_at_ms", "updated_at_ms", "updated_at"]
        .iter()
        .all(|column| columns.contains(*column))
    {
        "COALESCE(NULLIF(recency_at_ms, 0), updated_at_ms, updated_at * 1000)"
    } else if columns.contains("updated_at_ms") {
        "updated_at_ms"
    } else if columns.contains("updated_at") {
        "updated_at * 1000"
    } else {
        "0"
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
        "SELECT rollout_path, source, archived, {visible}, {recency} FROM threads LIMIT {}",
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
                archived: row.get::<_, i64>(2)? != 0,
                visible: row.get::<_, i64>(3)? != 0,
                recency_ms: row.get::<_, i64>(4).unwrap_or(0),
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

fn read_rollout(
    path: &Path,
    allowed_roots: &[PathBuf],
    total: &mut u64,
) -> Result<String, AuditStoreError> {
    if !path.is_absolute() {
        return Err(AuditStoreError::InputUnavailable);
    }
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| AuditStoreError::InputUnavailable)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(AuditStoreError::InputUnavailable);
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| AuditStoreError::InputUnavailable)?;
    if !allowed_roots.iter().any(|root| canonical.starts_with(root)) {
        return Err(AuditStoreError::InputUnavailable);
    }
    let mut file = open_bounded_regular_file(&canonical, 1, MAX_ROLLOUT_BYTES)
        .map_err(|_| AuditStoreError::InputUnavailable)?;
    if !owned_by_current_user(&file) {
        return Err(AuditStoreError::InputUnavailable);
    }
    let size = file
        .metadata()
        .map_err(|_| AuditStoreError::InputUnavailable)?
        .len();
    *total = total
        .checked_add(size)
        .filter(|value| *value <= MAX_AUDIT_BYTES)
        .ok_or(AuditStoreError::InputUnavailable)?;
    let mut result = String::with_capacity(size as usize);
    file.read_to_string(&mut result)
        .map_err(|_| AuditStoreError::InputUnavailable)?;
    Ok(result)
}

fn has_task_complete(contents: &str) -> bool {
    contents.lines().any(|line| {
        serde_json::from_str::<Value>(line)
            .ok()
            .is_some_and(|record| {
                record.get("type").and_then(Value::as_str) == Some("event_msg")
                    && record.pointer("/payload/type").and_then(Value::as_str)
                        == Some("task_complete")
            })
    })
}

fn originator(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let record = serde_json::from_str::<Value>(line).ok()?;
        if record.get("type").and_then(Value::as_str) != Some("session_meta") {
            return None;
        }
        record
            .pointer("/payload/originator")
            .and_then(Value::as_str)
            .map(str::to_owned)
    })
}

fn runtime_family(contents: &str, source: &str) -> Option<&'static str> {
    if let Some(originator) = originator(contents) {
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

fn audit_component(rollouts: &[String], window: AuditWindow) -> Result<Value, AuditStoreError> {
    let references = rollouts.iter().map(String::as_str).collect::<Vec<_>>();
    let storage_bytes = rollouts
        .iter()
        .try_fold(0_u64, |total, value| total.checked_add(value.len() as u64))
        .ok_or(AuditStoreError::AuditFailed)?;
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
    let end_ms = end.timestamp_millis();
    let mut root = Vec::new();
    let mut delegated = Vec::new();
    let mut guardian = Vec::new();
    let mut seen = BTreeSet::new();
    let mut unreadable = 0_u64;
    let mut unclassified = 0_u64;
    let mut source_fallback = 0_u64;
    let mut duplicates = 0_u64;
    let mut total_bytes = 0_u64;
    for row in rows {
        if row.recency_ms <= start_ms
            || row.recency_ms > end_ms
            || (row.kind == ThreadKind::Root && !row.visible)
        {
            continue;
        }
        let normalized = row.rollout.clone();
        if !seen.insert(normalized) {
            duplicates = duplicates.saturating_add(1);
            continue;
        }
        let contents = match read_rollout(&row.rollout, &allowed_roots, &mut total_bytes) {
            Ok(contents) => contents,
            Err(_) => {
                if row.kind == ThreadKind::Root {
                    unreadable = unreadable.saturating_add(1);
                }
                continue;
            }
        };
        if completed_only
            && row.kind == ThreadKind::Root
            && !row.archived
            && !has_task_complete(&contents)
        {
            continue;
        }
        match runtime_family(&contents, &row.source) {
            Some(value) if runtime_filter.is_some_and(|expected| expected != value) => continue,
            Some(_) if originator(&contents).is_none() => {
                source_fallback = source_fallback.saturating_add(1)
            }
            None => {
                unclassified = unclassified.saturating_add(1);
                continue;
            }
            _ => {}
        }
        match row.kind {
            ThreadKind::Root => root.push(contents),
            ThreadKind::Delegated => delegated.push(contents),
            ThreadKind::Guardian => guardian.push(contents),
        }
    }
    let window = AuditWindow {
        start: Some(start),
        end: Some(end),
    };
    let root_audit = audit_component(&root, window)?;
    let delegated_audit = audit_component(&delegated, window)?;
    let guardian_session = audit_component(&guardian, window)?;
    let guardian_audit = guardian_from_session(guardian_session, guardian.len());
    let sample = root.len() as u64;
    let status = if sample >= MINIMUM_ROOTS
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
        }) {
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
        "schema":1,"kind":kind,"status":status,"errors":[],
        "scope":{
            "generated_at":end.to_rfc3339(),"requested_days":((end-start).num_seconds().max(1) as u64).div_ceil(86_400),
            "requested_window_start":start.to_rfc3339(),"requested_window_end":end.to_rfc3339(),"selection_mode":selection_mode,
            "runtime_family":runtime_filter.unwrap_or("all"),"observed_root_sample_count":if completed_only {0} else {sample},
            "completed_root_sample_count":if completed_only {sample} else {0},"eligible_root_count":sample,"selected_root_count":sample,
            "root_truncated_count":0,"selection_coverage":selection_coverage,"selected_recency_start_utc":start.to_rfc3339(),"selected_recency_end_utc":end.to_rfc3339(),
            "minimum_root_sample_count":MINIMUM_ROOTS,"sample_sufficient":sample>=MINIMUM_ROOTS,"delegated_rollout_count":delegated.len(),
            "guardian_rollout_count":guardian.len(),"guardian_incomplete_excluded_count":0,"duplicate_rollout_reference_excluded_count":duplicates,
            "unreadable_completed_root_count":unreadable,"originator_unclassified_excluded_root_count":unclassified,
            "originator_source_fallback_root_count":source_fallback,"delegated_truncated_count":0,"guardian_truncated_count":0,
        },
        "root":root_audit,"delegated":delegated_audit,"guardian":guardian_audit,
        "usage_source_contract":{"cumulative_total_preferred_per_rollout":true,"last_usage_sum_is_fallback_only":true,"window_delta_prevents_double_counting":true,"billing_inference_performed":false},
        "mutation_performed":false,"raw_content_emitted":false,"private_paths_emitted":false,"thread_ids_emitted":false,"rollout_paths_emitted":false,"secret_value_printed":false,
    }))
}

pub fn earliest_recency(codex_home: &Path) -> Result<Option<DateTime<Utc>>, AuditStoreError> {
    let minimum = thread_rows(&state_database(codex_home)?)?
        .into_iter()
        .filter(|row| row.visible && row.recency_ms > 0)
        .map(|row| row.recency_ms)
        .min();
    Ok(minimum.and_then(DateTime::<Utc>::from_timestamp_millis))
}

pub fn contract_error(error: AuditStoreError) -> ContractError {
    ContractError(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::{Connection, params};
    use tempfile::{TempDir, tempdir, tempdir_in};

    use super::{read_rollout, rollout_roots, state_database, thread_rows};

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
        assert!(read_rollout(&rollout, &roots, &mut total).is_ok());
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
        assert!(read_rollout(&outside, &roots, &mut total).is_err());
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
