use groundline_runtime::local_file::{
    atomic_write_private, open_bounded_regular_file, private_for_current_user,
};
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    schema: u8,
    grafana_url: String,
}
pub fn validate_grafana(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    let url = url::Url::parse(value).map_err(|_| "invalid_grafana_url")?;
    if value.len() > 2048
        || url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid_grafana_url".into());
    }
    Ok(())
}
pub fn save(home: &Path, value: &str) -> Result<(), String> {
    validate_grafana(value)?;
    let data = serde_json::to_vec(&Settings {
        schema: 1,
        grafana_url: value.into(),
    })
    .map_err(|_| "local_state_failed")?;
    atomic_write_private(&home.join("groundline/desktop/settings.json"), &data)
        .map_err(|_| "local_state_failed".into())
}
pub fn load(home: &Path) -> Result<String, String> {
    let path = home.join("groundline/desktop/settings.json");
    if !path.try_exists().map_err(|_| "local_state_failed")? {
        return Ok(String::new());
    }
    let mut file = open_bounded_regular_file(&path, 1, 4096).map_err(|_| "local_state_failed")?;
    if !private_for_current_user(&file) {
        return Err("local_state_failed".into());
    }
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|_| "local_state_failed")?;
    let settings: Settings = serde_json::from_slice(&data).map_err(|_| "local_state_failed")?;
    if settings.schema != 1 {
        return Err("unsupported_local_state".into());
    }
    validate_grafana(&settings.grafana_url)?;
    Ok(settings.grafana_url)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dashboard_setting_is_private_and_rejects_executable_or_credential_urls() {
        let home = tempfile::tempdir().unwrap();
        save(home.path(), "https://grafana.example.com").unwrap();
        assert_eq!(load(home.path()).unwrap(), "https://grafana.example.com");
        for url in [
            "file:///tmp/example",
            "javascript:alert(1)",
            "https://user:password@example.com",
            "https://example.com?token=value",
        ] {
            assert!(save(home.path(), url).is_err());
        }
        assert_eq!(load(home.path()).unwrap(), "https://grafana.example.com");
    }
}
