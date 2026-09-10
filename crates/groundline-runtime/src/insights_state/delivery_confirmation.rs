use super::*;

const FILE: &str = "delivery-confirmation.json";

/// Latest acknowledged batch only; no identifiers, payloads, or credentials.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Confirmation {
    schema: u8,
    kind: String,
    pub(super) confirmed_at_utc: String,
    pub(super) event_count: u64,
}

pub(super) fn read(directory: &Path) -> Result<Option<Confirmation>, StateError> {
    let path = directory.join(FILE);
    if !path.try_exists().map_err(|_| StateError::LocalState)? {
        return Ok(None);
    }
    let value: Confirmation = read_json(&path, true)?;
    if value.schema != 1 || value.kind != "groundline-insights-delivery-confirmation" {
        return Err(StateError::UnsupportedState);
    }
    if value.event_count == 0
        || value.event_count > UPLOAD_BATCH_EVENTS as u64
        || parse_timestamp(&value.confirmed_at_utc).is_err()
    {
        return Err(StateError::LocalState);
    }
    Ok(Some(value))
}

pub(super) fn record(directory: &Path, count: u64, now: DateTime<Utc>) -> Result<(), StateError> {
    if count == 0 || count > UPLOAD_BATCH_EVENTS as u64 {
        return Err(StateError::LocalState);
    }
    write_json(
        &directory.join(FILE),
        &Confirmation {
            schema: 1,
            kind: "groundline-insights-delivery-confirmation".into(),
            confirmed_at_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            event_count: count,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confirmation_is_private_bounded_and_does_not_claim_a_zero_upload() {
        let directory = tempfile::tempdir().unwrap();
        assert!(read(directory.path()).unwrap().is_none());
        record(directory.path(), 2, Utc::now()).unwrap();
        let path = directory.path().join(FILE);
        let bytes = std::fs::read(&path).unwrap();
        assert!(record(directory.path(), 0, Utc::now()).is_err());
        assert!(record(directory.path(), 17, Utc::now()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(read(directory.path()).unwrap().unwrap().event_count, 2);
        assert!(private_for_current_user(
            &open_bounded_regular_file(&path, 1, 4096).unwrap()
        ));
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value["schema"] = json!(99);
        write_json(&path, &value).unwrap();
        assert!(read(directory.path()).is_err());
    }
}
