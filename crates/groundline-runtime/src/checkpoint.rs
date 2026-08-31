use std::path::Path;
use std::process::{Command, Stdio};

use chrono::{SecondsFormat, Utc};
use serde_json::json;
use thiserror::Error;

use crate::insights::state_directory;
use crate::local_file::{
    atomic_write_private, open_bounded_regular_file, open_or_create_private_lock,
    private_for_current_user,
};

const CAPTURE_DIRECTORY: &str = "hook-captures";
const CLAIM_DIRECTORY: &str = "hook-capture-claims";
const CAPTURE_LOCK_FILE: &str = "hook-capture.lock";
const MAX_CAPTURE_BYTES: u64 = 4 * 1024;

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("invalid_trigger")]
    InvalidTrigger,
    #[error("worker_spawn_failed")]
    SpawnFailed,
    #[error("checkpoint_capture_failed")]
    CaptureFailed,
}

fn capture_path(codex_home: &Path, trigger: &str) -> Result<std::path::PathBuf, CheckpointError> {
    if !valid_trigger(trigger) {
        return Err(CheckpointError::InvalidTrigger);
    }
    Ok(state_directory(codex_home)
        .join(CAPTURE_DIRECTORY)
        .join(format!("{trigger}.json")))
}

fn claim_path(codex_home: &Path, trigger: &str) -> Result<std::path::PathBuf, CheckpointError> {
    if !valid_trigger(trigger) {
        return Err(CheckpointError::InvalidTrigger);
    }
    Ok(state_directory(codex_home)
        .join(CLAIM_DIRECTORY)
        .join(format!("{trigger}.json")))
}

fn capture_lock(codex_home: &Path) -> Result<std::fs::File, CheckpointError> {
    let file = open_or_create_private_lock(&state_directory(codex_home).join(CAPTURE_LOCK_FILE))
        .map_err(|_| CheckpointError::CaptureFailed)?;
    file.lock().map_err(|_| CheckpointError::CaptureFailed)?;
    Ok(file)
}

pub fn capture_trigger(codex_home: &Path, trigger: &str) -> Result<(), CheckpointError> {
    let path = capture_path(codex_home, trigger)?;
    let _lock = capture_lock(codex_home)?;
    let mut value = serde_json::to_vec_pretty(&json!({
        "schema_version":1,
        "kind":"groundline-insights-hook-capture",
        "trigger":trigger,
        "captured_at_utc":Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    }))
    .map_err(|_| CheckpointError::CaptureFailed)?;
    value.push(b'\n');
    atomic_write_private(&path, &value).map_err(|_| CheckpointError::CaptureFailed)
}

pub fn claim_triggers(codex_home: &Path) -> Result<(), CheckpointError> {
    let _lock = capture_lock(codex_home)?;
    for trigger in [
        "session_start_hook",
        "stop_hook",
        "post_compact_hook",
        "session_end_hook",
    ] {
        let source = capture_path(codex_home, trigger)?;
        let claim = claim_path(codex_home, trigger)?;
        if claim.exists() {
            let file = open_bounded_regular_file(&claim, 1, MAX_CAPTURE_BYTES)
                .map_err(|_| CheckpointError::CaptureFailed)?;
            if !private_for_current_user(&file) {
                return Err(CheckpointError::CaptureFailed);
            }
            continue;
        }
        if !source.exists() {
            continue;
        }
        let file = open_bounded_regular_file(&source, 1, MAX_CAPTURE_BYTES)
            .map_err(|_| CheckpointError::CaptureFailed)?;
        if !private_for_current_user(&file) {
            return Err(CheckpointError::CaptureFailed);
        }
        drop(file);
        let parent = claim.parent().ok_or(CheckpointError::CaptureFailed)?;
        std::fs::create_dir_all(parent).map_err(|_| CheckpointError::CaptureFailed)?;
        if std::fs::symlink_metadata(parent)
            .map_err(|_| CheckpointError::CaptureFailed)?
            .file_type()
            .is_symlink()
        {
            return Err(CheckpointError::CaptureFailed);
        }
        std::fs::rename(source, claim).map_err(|_| CheckpointError::CaptureFailed)?;
    }
    Ok(())
}

pub fn acknowledge_claimed_triggers(codex_home: &Path) -> Result<(), CheckpointError> {
    for trigger in [
        "session_start_hook",
        "stop_hook",
        "post_compact_hook",
        "session_end_hook",
    ] {
        let path = claim_path(codex_home, trigger)?;
        if !path.exists() {
            continue;
        }
        let file = open_bounded_regular_file(&path, 1, MAX_CAPTURE_BYTES)
            .map_err(|_| CheckpointError::CaptureFailed)?;
        if !private_for_current_user(&file) {
            return Err(CheckpointError::CaptureFailed);
        }
        drop(file);
        std::fs::remove_file(path).map_err(|_| CheckpointError::CaptureFailed)?;
    }
    Ok(())
}

pub fn valid_trigger(trigger: &str) -> bool {
    matches!(
        trigger,
        "session_start_hook" | "stop_hook" | "post_compact_hook" | "session_end_hook"
    )
}

pub fn spawn_worker(
    trigger: &str,
    plugin_root: Option<&Path>,
    codex_home: Option<&Path>,
) -> Result<(), CheckpointError> {
    if !valid_trigger(trigger) {
        return Err(CheckpointError::InvalidTrigger);
    }
    let executable = std::env::current_exe().map_err(|_| CheckpointError::SpawnFailed)?;
    let mut command = Command::new(executable);
    command
        .arg("worker")
        .arg("run-once")
        .arg("--trigger")
        .arg(trigger)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(root) = plugin_root {
        command.arg("--plugin-root").arg(root);
    }
    if let Some(home) = codex_home {
        command.arg("--codex-home").arg(home);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::{
            CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, DETACHED_PROCESS,
        };

        command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    command.spawn().map_err(|_| CheckpointError::SpawnFailed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{acknowledge_claimed_triggers, capture_path, capture_trigger, claim_triggers};
    use crate::local_file::{open_bounded_regular_file, private_for_current_user};

    #[test]
    fn hook_capture_is_private_durable_and_coalesced_by_trigger() {
        let home = tempdir().expect("temporary Codex home");
        capture_trigger(home.path(), "session_end_hook").expect("first capture");
        capture_trigger(home.path(), "session_end_hook").expect("coalesced capture");
        capture_trigger(home.path(), "session_start_hook").expect("second trigger");
        let session_end = capture_path(home.path(), "session_end_hook").expect("capture path");
        let session_start = capture_path(home.path(), "session_start_hook").expect("capture path");
        let file = open_bounded_regular_file(&session_end, 1, 4096).expect("bounded capture");
        assert!(private_for_current_user(&file));
        assert!(session_start.is_file());
        claim_triggers(home.path()).expect("claim captures");
        acknowledge_claimed_triggers(home.path()).expect("acknowledge captures");
        assert!(!session_end.exists());
        assert!(!session_start.exists());
    }

    #[test]
    fn capture_written_during_a_claimed_cycle_survives_acknowledgement() {
        let home = tempdir().expect("temporary Codex home");
        capture_trigger(home.path(), "session_end_hook").expect("first capture");
        claim_triggers(home.path()).expect("claim first capture");
        capture_trigger(home.path(), "session_end_hook").expect("concurrent capture");
        acknowledge_claimed_triggers(home.path()).expect("acknowledge claimed capture");
        assert!(
            capture_path(home.path(), "session_end_hook")
                .expect("capture path")
                .is_file()
        );
        claim_triggers(home.path()).expect("claim surviving capture");
        acknowledge_claimed_triggers(home.path()).expect("acknowledge surviving capture");
        assert!(
            !capture_path(home.path(), "session_end_hook")
                .expect("capture path")
                .exists()
        );
    }

    #[test]
    fn hook_capture_rejects_unknown_trigger_without_state() {
        let home = tempdir().expect("temporary Codex home");
        assert!(capture_trigger(home.path(), "unknown").is_err());
        assert!(!home.path().join("groundline/insights").exists());
    }
}
