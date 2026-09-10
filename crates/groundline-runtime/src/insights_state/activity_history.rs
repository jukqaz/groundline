//! Bounded, local operational history. Never retain event bodies or identifiers.
use super::*;

const FILE: &str = "activity-history.json";
const LIMIT: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Accepted,
    Duplicate,
    Checked,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    at_utc: String,
    outcome: Outcome,
    event_count: u64,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct History {
    schema: u8,
    pub last_hook_at_utc: Option<String>,
    entries: Vec<Entry>,
}

fn reason(code: &str) -> &'static str {
    match code {
        "accepted" => "accepted",
        "duplicate" => "duplicate",
        "pass" => "no_new_delivery",
        "remote_authentication_rejected"
        | "enrollment_credential_rejected"
        | "proxy_authentication_rejected" => "authentication",
        "api_upgrade_required" => "api_upgrade",
        "audit_failed" | "collection_incomplete" | "collection_operator_action_required" => {
            "collection"
        }
        "tailnet_not_connected" | "tailnet_peer_rejected" => "connection",
        "event_upload_failed" | "remote_request_rejected" => "delivery",
        _ => "check_required",
    }
}

pub(super) fn read(directory: &Path) -> Result<History, StateError> {
    let path = directory.join(FILE);
    if !path.try_exists().map_err(|_| StateError::LocalState)? {
        return Ok(History {
            schema: 1,
            last_hook_at_utc: None,
            entries: Vec::new(),
        });
    }
    let value: History = read_json(&path, true)?;
    if value.schema != 1 {
        return Err(StateError::UnsupportedState);
    }
    if value.entries.len() > LIMIT
        || value
            .last_hook_at_utc
            .as_ref()
            .is_some_and(|v| parse_timestamp(v).is_err())
        || value.entries.iter().any(|entry| {
            parse_timestamp(&entry.at_utc).is_err()
                || entry.event_count > 1
                || !matches!(
                    entry.reason.as_str(),
                    "accepted"
                        | "duplicate"
                        | "no_new_delivery"
                        | "authentication"
                        | "api_upgrade"
                        | "collection"
                        | "connection"
                        | "delivery"
                        | "check_required"
                )
                || (matches!(entry.outcome, Outcome::Accepted | Outcome::Duplicate)
                    != (entry.event_count == 1))
        })
    {
        return Err(StateError::LocalState);
    }
    Ok(value)
}

pub(super) fn record(
    directory: &Path,
    outcome: Outcome,
    code: &str,
    now: DateTime<Utc>,
) -> Result<(), StateError> {
    let mut value = read(directory)?;
    let count = u64::from(matches!(outcome, Outcome::Accepted | Outcome::Duplicate));
    value.entries.insert(
        0,
        Entry {
            at_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            outcome,
            event_count: count,
            reason: reason(code).into(),
        },
    );
    value.entries.truncate(LIMIT);
    write_json(&directory.join(FILE), &value)
}

pub(super) fn cycle(directory: &Path, update: &StatusUpdate<'_>, now: DateTime<Utc>) {
    // This diagnostic log must not change delivery/ACK or retry semantics.
    let result = (|| -> Result<(), StateError> {
        if update.trigger.ends_with("_hook") && valid_status_trigger(update.trigger) {
            let mut value = read(directory)?;
            value.last_hook_at_utc = Some(now.to_rfc3339_opts(SecondsFormat::Millis, true));
            write_json(&directory.join(FILE), &value)?;
        }
        match update.result_code {
            "collection_staged" | "delivery_acknowledged" | "delivery_pending" => Ok(()),
            "pass" if update.uploaded_count > 0 => Ok(()),
            "pass" => record(directory, Outcome::Checked, "pass", now),
            code => record(directory, Outcome::Failed, code, now),
        }
    })();
    let _ = result; // Status exposes unreadable history separately from collection health.
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_twenty_safe_results_and_preserves_unsupported_history() {
        let dir = tempfile::tempdir().unwrap();
        for _ in 0..25 {
            record(dir.path(), Outcome::Failed, "private free text", Utc::now()).unwrap();
        }
        record(dir.path(), Outcome::Accepted, "accepted", Utc::now()).unwrap();
        record(dir.path(), Outcome::Duplicate, "duplicate", Utc::now()).unwrap();
        let value = read(dir.path()).unwrap();
        assert_eq!(value.entries.len(), 20);
        assert_eq!(value.entries[0].event_count, 1);
        let text = serde_json::to_string(&value).unwrap();
        assert!(!text.contains("private free text"));
        assert_eq!(value.entries[2].reason, "check_required");
        let path = dir.path().join(FILE);
        let mut invalid = serde_json::to_value(value).unwrap();
        invalid["schema"] = json!(99);
        write_json(&path, &invalid).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(record(dir.path(), Outcome::Checked, "pass", Utc::now()).is_err());
        assert_eq!(before, std::fs::read(&path).unwrap());
    }
}
