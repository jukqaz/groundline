//! One random group per local Codex home, shared by App and CLI.
use super::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Device {
    schema_version: u8,
    device_id: Uuid,
}

fn directory(codex_home: &Path) -> PathBuf {
    codex_home.join("groundline/insights")
}

pub(super) fn device(codex_home: &Path) -> Result<Uuid, StateError> {
    let directory = directory(codex_home);
    let lock = open_or_create_private_lock(&directory.join("analysis-profile.lock"))
        .map_err(|_| StateError::LocalState)?;
    lock.try_lock().map_err(|error| match error {
        std::fs::TryLockError::WouldBlock => StateError::AlreadyRunning,
        std::fs::TryLockError::Error(_) => StateError::LocalState,
    })?;
    let path = directory.join("device.json");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            let value: Device = read_json(&path, true)?;
            if value.schema_version != 1 || value.device_id.is_nil() {
                return Err(StateError::UnsupportedState);
            }
            Ok(value.device_id)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let value = Device {
                schema_version: 1,
                device_id: Uuid::new_v4(),
            };
            write_json(&path, &value)?;
            Ok(value.device_id)
        }
        Err(_) => Err(StateError::LocalState),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Purpose {
    schema_version: u8,
    value: String,
    effective_at_utc: String,
}

pub fn set_purpose(codex_home: &Path, value: &str) -> Result<Value, StateError> {
    if !groundline_contracts::insights::analysis::PURPOSES.contains(&value) {
        return Err(StateError::UnsupportedState);
    }
    // The profile must already be configured; this command does not enroll,
    // grant consent, start collection, or rewrite previously collected data.
    load_profile(codex_home)?;
    let effective_at_utc = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    write_json(
        &directory(codex_home).join("purpose.json"),
        &Purpose {
            schema_version: 1,
            value: value.to_owned(),
            effective_at_utc: effective_at_utc.clone(),
        },
    )?;
    Ok(
        json!({"status":"PASS","purpose":value,"effective_at_utc":effective_at_utc,
        "network_performed":false,"historical_data_modified":false}),
    )
}

pub(super) fn purpose(directory: &Path, start: DateTime<Utc>) -> Result<String, StateError> {
    let path = directory
        .parent()
        .ok_or(StateError::LocalState)?
        .join("purpose.json");
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok("unclassified".to_owned()),
        Err(_) => Err(StateError::LocalState),
        Ok(_) => {
            let p: Purpose = read_json(&path, true)?;
            if p.schema_version != 1
                || !groundline_contracts::insights::analysis::PURPOSES.contains(&p.value.as_str())
            {
                return Err(StateError::UnsupportedState);
            }
            Ok(if start >= parse_timestamp(&p.effective_at_utc)? {
                p.value
            } else {
                "unclassified".to_owned()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_and_cli_share_one_random_group_and_reject_corruption() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(directory(home.path())).unwrap();
        let first = device(home.path()).unwrap();
        assert_eq!(first, device(home.path()).unwrap());
        let other = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(directory(other.path())).unwrap();
        assert_ne!(first, device(other.path()).unwrap());
        write_json(
            &directory(home.path()).join("device.json"),
            &json!({"schema_version":99,"device_id":first}),
        )
        .unwrap();
        assert!(device(home.path()).is_err());
    }
    #[test]
    fn purpose_never_relabels_a_window_started_before_declaration() {
        let home = tempfile::tempdir().unwrap();
        let state = directory(home.path()).join("codex_app-desktop");
        std::fs::create_dir_all(&state).unwrap();
        let at = Utc::now();
        write_json(
            &state.parent().unwrap().join("purpose.json"),
            &Purpose {
                schema_version: 1,
                value: "verification".to_owned(),
                effective_at_utc: at.to_rfc3339(),
            },
        )
        .unwrap();
        assert_eq!(
            purpose(&state, at - chrono::Duration::seconds(1)).unwrap(),
            "unclassified"
        );
        assert_eq!(
            purpose(&state, at + chrono::Duration::seconds(1)).unwrap(),
            "verification"
        );
    }
}
