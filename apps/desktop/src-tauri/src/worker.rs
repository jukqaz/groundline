use groundline_runtime::{insights, insights_state, local_file};
use serde_json::{Value, json};
use std::{io::Read, path::Path};
use zeroize::Zeroizing;

fn read_profile(home: &Path) -> Result<Option<Value>, String> {
    let path = home.join("groundline/insights/owner-profile.json");
    if !path.try_exists().map_err(|_| "local_state_failed")? {
        return Ok(None);
    }
    let mut file = local_file::open_bounded_regular_file(&path, 1, 16384)
        .map_err(|_| "invalid_owner_profile")?;
    if !local_file::private_for_current_user(&file) {
        return Err("invalid_owner_profile".into());
    }
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| "local_state_failed")?;
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|_| "invalid_owner_profile".into())
}

pub fn profile_input(endpoint: &str, key: &str) -> Value {
    json!({
        "schema_version":7, "kind":"groundline-insights-owner-profile",
        "mode":"private_owner", "endpoint":endpoint, "enrollment_token":key,
        "automatic_activity_checkpoints":true, "automatic_initial_history_sync":true,
        "collection_scope":"all_activity", "checkpoint_min_interval_seconds":900,
        "diagnostic_enabled":false, "trigger_mode":"native_hook_checkpoints"
    })
}

pub async fn execute(action: &str, input: Value, home: &Path) -> Result<Value, String> {
    match action {
        "status" => {
            let mut result = insights_state::status(home).map_err(|e| e.to_string())?;
            result["endpoint"] = read_profile(home)?
                .and_then(|p| p.get("endpoint").cloned())
                .unwrap_or(Value::Null);
            result["grafana_url"] = json!(crate::desktop_settings::load(home)?);
            Ok(result)
        }
        "configure" => {
            let endpoint = input["endpoint"].as_str().ok_or("invalid_owner_profile")?;
            insights::report_url(endpoint, 7).map_err(|_| "invalid_owner_profile")?;
            let grafana_url = input["grafana_url"].as_str().unwrap_or_default();
            crate::desktop_settings::validate_grafana(grafana_url)?;
            if let Some(profile) = read_profile(home)?
                && profile["endpoint"].as_str() != Some(endpoint)
            {
                return Err("endpoint_change_requires_review".into());
            }
            // Validate existing policy, consent and outbox before any profile write.
            insights_state::status(home).map_err(|e| e.to_string())?;
            let key = input["key"].as_str().ok_or("invalid_owner_profile")?;
            let encoded = Zeroizing::new(
                serde_json::to_vec(&profile_input(endpoint, key))
                    .map_err(|_| "invalid_owner_profile")?,
            );
            insights_state::configure_profile(home, &encoded).map_err(|e| e.to_string())?;
            let result = insights_state::enable(home).map_err(|e| e.to_string())?;
            crate::desktop_settings::save(home, grafana_url)?;
            Ok(result)
        }
        "disable" => insights_state::disable(home).map_err(|e| e.to_string()),
        "resume" => {
            let current = insights_state::status(home).map_err(|e| e.to_string())?;
            if current["owner_profile_configured"] != true
                || current["enrollment_credential_valid"] != true
            {
                return Err("collection_configuration_required".into());
            }
            insights_state::enable(home).map_err(|e| e.to_string())
        }
        "enable" => insights_state::enable(home).map_err(|e| e.to_string()),
        "run" => insights_state::run_once(Path::new("."), home, "manual")
            .await
            .map_err(|e| e.to_string()),
        _ => Err("unsupported_operation".into()),
    }
}

pub fn main() {
    let action = std::env::args().nth(2).unwrap_or_default();
    let result = (|| -> Result<Value, String> {
        let home = insights::default_codex_home().map_err(|_| "local_state_failed")?;
        let mut bytes = Zeroizing::new(Vec::new());
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| "invalid_input")?;
        if bytes.len() > 16384 {
            return Err("invalid_input".into());
        }
        let input = if bytes.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&bytes).map_err(|_| "invalid_input")?
        };
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "worker_failed")?
            .block_on(execute(&action, input, &home))
    })();
    println!(
        "{}",
        match result {
            Ok(value) => json!({"ok":true,"value":value}),
            Err(code) => json!({"ok":false,"code":code}),
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn resume_requires_configuration_and_preserves_existing_endpoint() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(
            execute("resume", json!({}), home.path()).await.unwrap_err(),
            "collection_configuration_required"
        );
        execute(
            "configure",
            json!({"endpoint":"https://insights.example.com", "key":"x".repeat(32)}),
            home.path(),
        )
        .await
        .unwrap();
        execute("disable", json!({}), home.path()).await.unwrap();
        let stopped = execute("status", json!({}), home.path()).await.unwrap();
        assert_eq!(stopped["collection_enabled"], false);
        execute("resume", json!({}), home.path()).await.unwrap();
        let resumed = execute("status", json!({}), home.path()).await.unwrap();
        assert_eq!(resumed["collection_enabled"], true);
        assert_eq!(resumed["consent_status"], "active");
        assert_eq!(resumed["endpoint"], stopped["endpoint"]);
        assert_eq!(resumed["last_success_utc"], serde_json::Value::Null);
    }
    #[test]
    fn generated_profile_matches_the_native_contract() {
        let root = tempfile::tempdir().unwrap();
        let data = serde_json::to_vec(&profile_input(
            "https://insights.example.com",
            &"x".repeat(32),
        ))
        .unwrap();
        insights_state::configure_profile(root.path(), &data).unwrap();
        assert!(
            !read_profile(root.path())
                .unwrap()
                .unwrap()
                .to_string()
                .contains(&"x".repeat(32))
        );
    }

    #[tokio::test]
    async fn changed_endpoint_does_not_overwrite_existing_profile() {
        let root = tempfile::tempdir().unwrap();
        let original =
            serde_json::to_vec(&profile_input("https://one.example.com", &"x".repeat(32))).unwrap();
        insights_state::configure_profile(root.path(), &original).unwrap();
        let result = execute(
            "configure",
            json!({"endpoint":"https://two.example.com","key":"y".repeat(32)}),
            root.path(),
        )
        .await;
        assert_eq!(result.unwrap_err(), "endpoint_change_requires_review");
        assert_eq!(
            read_profile(root.path()).unwrap().unwrap()["endpoint"],
            "https://one.example.com"
        );
    }
}
