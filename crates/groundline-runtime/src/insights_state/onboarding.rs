//! Explicit onboarding over the existing collector. No second identity or cursor.
use super::*;

fn private_input(path: &Path, limit: u64) -> Result<Zeroizing<Vec<u8>>, StateError> {
    let mut file =
        open_bounded_regular_file(path, 1, limit).map_err(|_| StateError::InvalidProfile)?;
    if !private_for_current_user(&file) {
        return Err(StateError::InvalidProfile);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.by_ref()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StateError::InvalidProfile)?;
    if bytes.len() as u64 > limit {
        return Err(StateError::InvalidProfile);
    }
    Ok(bytes)
}

fn input_bytes(
    input: Option<&Path>,
    endpoint: Option<&str>,
    token: Option<&Path>,
) -> Result<Option<Zeroizing<Vec<u8>>>, StateError> {
    match (input, endpoint, token) {
        (Some(path), None, None) => private_input(path, 16 * 1024).map(Some),
        (None, Some(endpoint), Some(path)) => {
            let bytes = private_input(path, 4096)?;
            let token = std::str::from_utf8(&bytes)
                .map_err(|_| StateError::InvalidProfile)?
                .trim();
            let mut value = json!({
                "schema_version":7,"kind":"groundline-insights-owner-profile","mode":"private_owner",
                "endpoint":endpoint,"automatic_activity_checkpoints":true,"automatic_initial_history_sync":true,
                "collection_scope":"all_activity","checkpoint_min_interval_seconds":900,
                "diagnostic_enabled":false,"trigger_mode":"native_hook_checkpoints"
            });
            value["enrollment_token"] = json!(token);
            let result = serde_json::to_vec(&value).map_err(|_| StateError::InvalidProfile);
            if let Some(Value::String(secret)) = value.get_mut("enrollment_token") {
                use zeroize::Zeroize;
                secret.zeroize();
            }
            result.map(Zeroizing::new).map(Some)
        }
        (None, None, None) => Ok(None),
        _ => Err(StateError::InvalidProfile),
    }
}

fn configure_once(home: &Path, bytes: &[u8]) -> Result<bool, StateError> {
    let (profile, token) = parse_owner_profile(bytes)?;
    let value = serde_json::to_value(&profile).map_err(|_| StateError::InvalidProfile)?;
    let _lock = profile_lock(home)?;
    let mut missing = Vec::new();
    for (path, is_profile) in [
        (home.join(PROFILE_PATH), true),
        (home.join(ENROLLMENT_TOKEN_PATH), false),
    ] {
        if path.try_exists().map_err(|_| StateError::LocalState)? {
            let bytes = private_input(&path, 16 * 1024)?;
            let same = if is_profile {
                serde_json::from_slice::<Value>(&bytes).map_err(|_| StateError::InvalidProfile)?
                    == value
            } else {
                std::str::from_utf8(&bytes)
                    .map_err(|_| StateError::InvalidProfile)?
                    .trim()
                    == token.as_str()
            };
            if !same {
                return Err(StateError::SetupConnectionConflict);
            }
        } else {
            missing.push((path, is_profile));
        }
    }
    let changed = !missing.is_empty();
    // The shared profile lock serializes setup and explicit profile configuration.
    // Exclusive creation also rejects an unrelated writer after this check.
    // A previous partial write resumes only when its surviving input agrees.
    for (path, is_profile) in missing {
        use std::io::Write;
        let mut content = Zeroizing::new(if is_profile {
            serde_json::to_vec_pretty(&profile).map_err(|_| StateError::InvalidProfile)?
        } else {
            token.as_bytes().to_vec()
        });
        content.push(b'\n');
        let mut file =
            crate::local_file::create_private_new(&path).map_err(|_| StateError::LocalState)?;
        file.write_all(&content)
            .and_then(|()| file.sync_all())
            .map_err(|_| StateError::LocalState)?;
    }
    Ok(changed)
}

async fn verify_connection(home: &Path) -> Result<(), StateError> {
    let directory = state_directory(home)?;
    let _lock = CycleLock::acquire(&directory)?;
    let _permit = collection_permit(&directory)?;
    let profile = load_profile(home)?;
    if crate::insights::endpoint_requires_tailnet(&profile.endpoint)
        && connection_probe(true)["tailnet_connected"] != true
    {
        return Err(StateError::TailnetDisconnected);
    }
    let (identity, _) = initialize(&directory, Utc::now())?;
    enroll(&profile, home, &directory, &identity).await?;
    Ok(())
}

pub async fn setup(
    home: &Path,
    input: Option<&Path>,
    endpoint: Option<&str>,
    token: Option<&Path>,
    enable_collection: bool,
    verify: bool,
) -> Result<Value, StateError> {
    let before = status(home)?;
    let configured = if let Some(bytes) = input_bytes(input, endpoint, token)? {
        configure_once(home, &bytes)?
    } else {
        false
    };
    let enabled = enable_collection
        && (before["collection_enabled"] != true || before["consent_status"] != "active");
    if enabled {
        enable(home)?;
    }
    let current = status(home)?;
    let mut connection_verified = false;
    let mut check_error = None;
    if verify
        && current["collection_enabled"] == true
        && current["owner_profile_configured"] == true
        && current["consent_status"] == "active"
    {
        match verify_connection(home).await {
            Ok(()) => {
                connection_verified = true;
                // Do not freeze an incomplete history window on a genuinely fresh Codex home.
                if (current["ready_to_collect"] == true
                    || current["pending_event_count"]
                        .as_u64()
                        .is_some_and(|n| n > 0))
                    && let Err(error) = run_once(Path::new("."), home, "manual").await
                {
                    check_error = Some(error.to_string());
                }
            }
            Err(error) => check_error = Some(error.to_string()),
        }
    }
    let current = status(home)?;
    let profile_ready = current["owner_profile_configured"] == true
        && current["enrollment_credential_valid"] == true;
    let consent = current["collection_enabled"] == true && current["consent_status"] == "active";
    let hook_seen = current["activity_history"]["last_hook_at_utc"]
        .as_str()
        .and_then(|s| parse_timestamp(s).ok())
        .is_some_and(|t| t <= Utc::now() && Utc::now() - t <= COLLECTION_STALE_AFTER);
    let delivery_seen = current["delivery_confirmation"]["confirmed_at_utc"]
        .as_str()
        .and_then(|s| parse_timestamp(s).ok())
        .is_some_and(|t| t <= Utc::now() && Utc::now() - t <= COLLECTION_STALE_AFTER);
    let active = current["collection_state"] == "active";
    let scope = crate::environment::Environment::current()?;
    let mut actions = Vec::new();
    if !profile_ready {
        actions.push("configure_owner_connection");
    }
    if !consent {
        actions.push("enable_collection_with_explicit_consent");
    }
    if !connection_verified {
        actions.push("verify_owner_connection");
    }
    if !hook_seen {
        actions.push("review_codex_hooks_then_complete_a_native_task");
    }
    if !delivery_seen || !active {
        actions.push("await_or_resolve_first_collection");
    }
    Ok(json!({
        "kind":"groundline-insights-setup","schema":1,
        "status":if check_error.is_some() { "FAIL" } else if actions.is_empty() { "PASS" } else { "ACTION_REQUIRED" },
        "configuration_changed":configured,"collection_enabled_now":enabled,
        "runtime_family":scope.runtime,"execution_mode":scope.mode,
        "stages":{"connection_configured":profile_ready,"collection_consented":consent,
            "connection_verified_now":connection_verified,"recent_hook_dispatch_observed":hook_seen,
            "recent_delivery_confirmed":delivery_seen,"collector_active":active},
        "verification_error":check_error,"next_actions":actions,"worker":current,
        "private_paths_emitted":false,"secret_value_printed":false,
        "evidence_scope":"current_runtime_source; recent hook and delivery observations are separate"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> Vec<u8> {
        super::super::tests::profile("")
    }

    #[test]
    fn interrupted_setup_resumes_only_matching_profile_and_credential() {
        for missing in [PROFILE_PATH, ENROLLMENT_TOKEN_PATH] {
            let home = tempfile::tempdir().unwrap();
            configure_once(home.path(), &input()).unwrap();
            let survivor = if missing == PROFILE_PATH {
                ENROLLMENT_TOKEN_PATH
            } else {
                PROFILE_PATH
            };
            let before = std::fs::read(home.path().join(survivor)).unwrap();
            std::fs::remove_file(home.path().join(missing)).unwrap();
            let mut conflict: Value = serde_json::from_slice(&input()).unwrap();
            conflict["endpoint"] = json!("https://different.example.com");
            conflict["enrollment_token"] = json!("different-token-1234567890123456789012345");
            assert!(matches!(
                configure_once(home.path(), &serde_json::to_vec(&conflict).unwrap()),
                Err(StateError::SetupConnectionConflict)
            ));
            assert!(!home.path().join(missing).exists());
            assert_eq!(std::fs::read(home.path().join(survivor)).unwrap(), before);
            assert!(configure_once(home.path(), &input()).unwrap());
            assert!(!configure_once(home.path(), &input()).unwrap());
            assert_eq!(std::fs::read(home.path().join(survivor)).unwrap(), before);
        }
    }

    #[test]
    fn profile_writers_share_a_lock_without_overwriting_connections() {
        let home = tempfile::tempdir().unwrap();
        let lock = profile_lock(home.path()).unwrap();
        assert!(matches!(
            configure_once(home.path(), &input()),
            Err(StateError::AlreadyRunning)
        ));
        assert!(matches!(
            configure_profile(home.path(), &input()),
            Err(StateError::AlreadyRunning)
        ));
        assert!(!home.path().join(PROFILE_PATH).exists());
        assert!(!home.path().join(ENROLLMENT_TOKEN_PATH).exists());
        drop(lock);
        assert!(configure_once(home.path(), &input()).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn linked_parent_is_rejected_without_writing_the_link_target() {
        let home = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(other.path(), home.path().join("groundline")).unwrap();
        assert!(matches!(
            configure_once(home.path(), &input()),
            Err(StateError::InvalidProfile)
        ));
        assert_eq!(std::fs::read_dir(other.path()).unwrap().count(), 0);
    }
}
