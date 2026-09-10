use groundline_runtime::local_file::{
    atomic_write_private, open_bounded_regular_file, private_for_current_user,
};
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloseAction {
    #[default]
    Tray,
    Quit,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Runtime {
    #[default]
    CodexApp,
    CodexCli,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub schema: u8,
    pub close_action: CloseAction,
    pub runtime: Runtime,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema: 1,
            close_action: CloseAction::Tray,
            runtime: Runtime::CodexApp,
        }
    }
}

pub fn load(home: &Path) -> Result<Preferences, String> {
    let path = home.join("groundline/desktop/behavior.json");
    if !path.try_exists().map_err(|_| "local_state_failed")? {
        return Ok(Preferences::default());
    }
    let mut file = open_bounded_regular_file(&path, 1, 4096).map_err(|_| "local_state_failed")?;
    if !private_for_current_user(&file) {
        return Err("local_state_failed".into());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| "local_state_failed")?;
    let value: Preferences =
        serde_json::from_slice(&bytes).map_err(|_| "unsupported_local_state")?;
    if value.schema != 1 {
        return Err("unsupported_local_state".into());
    }
    Ok(value)
}

pub fn save(home: &Path, value: &Preferences) -> Result<(), String> {
    load(home)?; // Reject unsupported existing state without rewriting it.
    if value.schema != 1 {
        return Err("unsupported_local_state".into());
    }
    let bytes = serde_json::to_vec(value).map_err(|_| "local_state_failed")?;
    atomic_write_private(&home.join("groundline/desktop/behavior.json"), &bytes)
        .map_err(|_| "local_state_failed".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_private_saved_close_behavior_preserve_collection_settings() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(load(home.path()).unwrap(), Preferences::default());
        assert!(!home.path().join("groundline").exists());
        let value = Preferences {
            close_action: CloseAction::Quit,
            runtime: Runtime::CodexCli,
            ..Preferences::default()
        };
        save(home.path(), &value).unwrap();
        assert_eq!(load(home.path()).unwrap(), value);
        assert!(!home.path().join("groundline/insights").exists());
        let path = home.path().join("groundline/desktop/behavior.json");
        atomic_write_private(
            &path,
            br#"{"schema":99,"close_action":"tray","runtime":"codex_app"}"#,
        )
        .unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(save(home.path(), &value).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
