use std::path::Path;
use std::process::{Command, Stdio};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("invalid_trigger")]
    InvalidTrigger,
    #[error("worker_spawn_failed")]
    SpawnFailed,
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
