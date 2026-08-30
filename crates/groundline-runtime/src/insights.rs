use std::fs::File;
use std::io::Read;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use groundline_contracts::insights::{MAX_WEEKLY_REPORT_BYTES, WeeklyReport};
use ipnet::Ipv4Net;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use thiserror::Error;
use url::{Host, Url};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::local_file::{open_bounded_regular_file, private_for_current_user};

const OWNER_PROFILE_PATH: &str = "groundline/insights/owner-profile.json";
const MAX_PROFILE_BYTES: u64 = 16 * 1024;
const MAX_IDENTITY_BYTES: u64 = 64 * 1024;
const MAX_TOKEN_BYTES: u64 = 4096;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
static CRYPTO_PROVIDER: OnceLock<Result<(), ()>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum InsightsRuntimeError {
    #[error("invalid_local_state")]
    InvalidLocalState,
    #[error("invalid_report_response")]
    InvalidReportResponse,
    #[error("report_request_failed")]
    ReportRequestFailed,
}

impl InsightsRuntimeError {
    pub fn network_performed(&self) -> bool {
        !matches!(self, Self::InvalidLocalState)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerProfile {
    schema_version: u8,
    kind: String,
    mode: String,
    endpoint: String,
    automatic_activity_checkpoints: bool,
    automatic_initial_history_sync: bool,
    collection_scope: String,
    checkpoint_min_interval_seconds: u64,
    diagnostic_enabled: bool,
    trigger_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    schema_version: u8,
    kind: String,
    collector_instance_id: Uuid,
    os_family: String,
    runtime_family: String,
    execution_mode: String,
    created_at_utc: String,
    resettable: bool,
}

fn read_opened_bounded(mut file: File, maximum: u64) -> Result<Vec<u8>, InsightsRuntimeError> {
    let capacity = file
        .metadata()
        .map_err(|_| InsightsRuntimeError::InvalidLocalState)?
        .len() as usize;
    let mut bytes = Vec::with_capacity(capacity);
    file.by_ref()
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    if bytes.is_empty() || bytes.len() as u64 > maximum {
        return Err(InsightsRuntimeError::InvalidLocalState);
    }
    Ok(bytes)
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, InsightsRuntimeError> {
    let file = open_bounded_regular_file(path, 1, maximum)
        .map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    read_opened_bounded(file, maximum)
}

fn read_profile(codex_home: &Path) -> Result<OwnerProfile, InsightsRuntimeError> {
    let bytes = read_bounded(&codex_home.join(OWNER_PROFILE_PATH), MAX_PROFILE_BYTES)?;
    let profile: OwnerProfile =
        serde_json::from_slice(&bytes).map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    if profile.schema_version != 7
        || profile.kind != "groundline-insights-owner-profile"
        || profile.mode != "private_owner"
        || !profile.automatic_activity_checkpoints
        || !profile.automatic_initial_history_sync
        || profile.collection_scope != "all_activity"
        || profile.checkpoint_min_interval_seconds != 900
        || profile.diagnostic_enabled
        || profile.trigger_mode != "native_hook_checkpoints"
        || report_url(&profile.endpoint, 7).is_err()
    {
        return Err(InsightsRuntimeError::InvalidLocalState);
    }
    Ok(profile)
}

fn read_identity(directory: &Path) -> Result<Identity, InsightsRuntimeError> {
    let bytes = read_bounded(&directory.join("identity.json"), MAX_IDENTITY_BYTES)?;
    let identity: Identity =
        serde_json::from_slice(&bytes).map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    if identity.schema_version != 1
        || identity.kind != "groundline-insights-identity"
        || !matches!(
            identity.os_family.as_str(),
            "macos" | "windows" | "linux" | "unknown"
        )
        || !matches!(
            identity.runtime_family.as_str(),
            "codex_app" | "codex_cli" | "unknown"
        )
        || !matches!(
            identity.execution_mode.as_str(),
            "desktop" | "local_headless" | "remote_headless" | "unknown"
        )
        || chrono::DateTime::parse_from_rfc3339(&identity.created_at_utc).is_err()
        || !identity.resettable
    {
        return Err(InsightsRuntimeError::InvalidLocalState);
    }
    Ok(identity)
}

fn read_token(directory: &Path) -> Result<SecretString, InsightsRuntimeError> {
    let path = directory.join("collector-token");
    let file = open_bounded_regular_file(&path, 32, MAX_TOKEN_BYTES)
        .map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    if !private_for_current_user(&file) {
        return Err(InsightsRuntimeError::InvalidLocalState);
    }
    let bytes = read_opened_bounded(file, MAX_TOKEN_BYTES)?;
    let raw = Zeroizing::new(
        String::from_utf8(bytes).map_err(|_| InsightsRuntimeError::InvalidLocalState)?,
    );
    let token = raw.trim().to_owned();
    if !(32..=MAX_TOKEN_BYTES as usize).contains(&token.len()) {
        return Err(InsightsRuntimeError::InvalidLocalState);
    }
    Ok(SecretString::from(token))
}

fn ensure_crypto_provider() -> Result<(), InsightsRuntimeError> {
    CRYPTO_PROVIDER
        .get_or_init(|| {
            if rustls::crypto::CryptoProvider::get_default().is_some() {
                return Ok(());
            }
            if rustls::crypto::ring::default_provider()
                .install_default()
                .is_ok()
                || rustls::crypto::CryptoProvider::get_default().is_some()
            {
                Ok(())
            } else {
                Err(())
            }
        })
        .map_err(|_| InsightsRuntimeError::InvalidLocalState)
}

fn is_tailnet_host(host: Host<&str>) -> bool {
    match host {
        Host::Ipv4(address) => Ipv4Net::new(Ipv4Addr::new(100, 64, 0, 0), 10)
            .expect("valid fixed network")
            .contains(&address),
        Host::Domain(domain) => domain.to_ascii_lowercase().ends_with(".ts.net"),
        Host::Ipv6(_) => false,
    }
}

pub fn report_url(endpoint: &str, days: u16) -> Result<Url, InsightsRuntimeError> {
    if !matches!(days, 7 | 30 | 90) {
        return Err(InsightsRuntimeError::InvalidLocalState);
    }
    let mut url = Url::parse(endpoint).map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || !url.host().is_some_and(is_tailnet_host)
    {
        return Err(InsightsRuntimeError::InvalidLocalState);
    }
    url.set_path("/v3/reports/weekly");
    url.query_pairs_mut().append_pair("days", &days.to_string());
    Ok(url)
}

fn runtime_family() -> &'static str {
    match std::env::var("GROUNDLINE_RUNTIME_FAMILY")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "codex_app" => "codex_app",
        "codex_cli" => "codex_cli",
        _ => {
            let originator = std::env::var("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")
                .unwrap_or_default()
                .to_ascii_lowercase();
            if ["app", "chatgpt", "desktop"]
                .iter()
                .any(|marker| originator.contains(marker))
            {
                "codex_app"
            } else {
                "codex_cli"
            }
        }
    }
}

fn execution_mode(runtime: &str) -> &'static str {
    match std::env::var("GROUNDLINE_EXECUTION_MODE")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "desktop" => "desktop",
        "remote_headless" => "remote_headless",
        "local_headless" => "local_headless",
        _ if runtime == "codex_app" => "desktop",
        _ => "local_headless",
    }
}

pub fn state_directory(codex_home: &Path) -> PathBuf {
    let runtime = runtime_family();
    codex_home
        .join("groundline")
        .join("insights")
        .join(format!("{runtime}-{}", execution_mode(runtime)))
}

pub fn default_codex_home() -> Result<PathBuf, InsightsRuntimeError> {
    if let Some(configured) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(configured));
    }
    dirs::home_dir()
        .map(|home| home.join(".codex"))
        .ok_or(InsightsRuntimeError::InvalidLocalState)
}

pub fn discover_plugin_root() -> Result<PathBuf, InsightsRuntimeError> {
    let executable =
        std::env::current_exe().map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    executable
        .ancestors()
        .take(6)
        .find(|candidate| candidate.join(".codex-plugin/plugin.json").is_file())
        .map(Path::to_path_buf)
        .ok_or(InsightsRuntimeError::InvalidLocalState)
}

pub async fn fetch_weekly_report(
    _plugin_root: &Path,
    codex_home: &Path,
    days: u16,
) -> Result<WeeklyReport, InsightsRuntimeError> {
    let profile = read_profile(codex_home)?;
    let directory = state_directory(codex_home);
    let identity = read_identity(&directory)?;
    let token = read_token(&directory)?;
    let url = report_url(&profile.endpoint, days)?;
    ensure_crypto_provider()?;

    let mut authorization =
        HeaderValue::from_str(&Zeroizing::new(format!("Bearer {}", token.expose_secret())))
            .map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    authorization.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(AUTHORIZATION, authorization);
    headers.insert(
        "x-groundline-collector-id",
        HeaderValue::from_str(&identity.collector_instance_id.to_string())
            .map_err(|_| InsightsRuntimeError::InvalidLocalState)?,
    );
    headers.insert(
        "x-groundline-version",
        HeaderValue::from_str(env!("CARGO_PKG_VERSION"))
            .map_err(|_| InsightsRuntimeError::InvalidLocalState)?,
    );
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .timeout(REQUEST_TIMEOUT)
        .user_agent("groundline-insights-report/4")
        .default_headers(headers)
        .build()
        .map_err(|_| InsightsRuntimeError::InvalidLocalState)?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| InsightsRuntimeError::ReportRequestFailed)?;
    if response.status() != reqwest::StatusCode::OK
        || response
            .content_length()
            .is_some_and(|length| length > MAX_WEEKLY_REPORT_BYTES as u64)
    {
        return Err(InsightsRuntimeError::ReportRequestFailed);
    }
    let mut body = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or(0)
            .min(MAX_WEEKLY_REPORT_BYTES as u64) as usize,
    );
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| InsightsRuntimeError::ReportRequestFailed)?
    {
        if body
            .len()
            .checked_add(chunk.len())
            .is_none_or(|length| length > MAX_WEEKLY_REPORT_BYTES)
        {
            return Err(InsightsRuntimeError::InvalidReportResponse);
        }
        body.extend_from_slice(&chunk);
    }
    let report =
        WeeklyReport::from_slice(&body).map_err(|_| InsightsRuntimeError::InvalidReportResponse)?;
    if report.requested_days != days {
        return Err(InsightsRuntimeError::InvalidReportResponse);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::local_file::atomic_write_private;

    use super::{InsightsRuntimeError, ensure_crypto_provider, read_token, report_url};

    #[test]
    fn ring_crypto_provider_is_installed_once_without_panicking() {
        ensure_crypto_provider().unwrap();
        ensure_crypto_provider().unwrap();
    }

    #[test]
    fn report_endpoint_is_fixed_to_tailnet_hosts_without_redirect_inputs() {
        assert_eq!(
            report_url("http://100.64.0.1:18080", 7).unwrap().as_str(),
            "http://100.64.0.1:18080/v3/reports/weekly?days=7"
        );
        assert_eq!(
            report_url("https://groundline.example.ts.net", 30)
                .unwrap()
                .as_str(),
            "https://groundline.example.ts.net/v3/reports/weekly?days=30"
        );
        for endpoint in [
            "https://example.com",
            "https://user@groundline.example.ts.net",
            "https://groundline.example.ts.net/path",
            "https://groundline.example.ts.net?redirect=1",
            "http://127.0.0.1:18080",
        ] {
            assert!(matches!(
                report_url(endpoint, 7),
                Err(InsightsRuntimeError::InvalidLocalState)
            ));
        }
    }

    #[test]
    fn token_reader_rejects_empty_oversized_and_symlink_state() {
        let root = tempdir().unwrap();
        let token = root.path().join("collector-token");
        atomic_write_private(&token, "x".repeat(32).as_bytes()).unwrap();
        assert_eq!(
            secrecy::ExposeSecret::expose_secret(&read_token(root.path()).unwrap()),
            "x".repeat(32)
        );
        fs::write(&token, "short").unwrap();
        assert!(read_token(root.path()).is_err());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            fs::write(&token, "x".repeat(32)).unwrap();
            let linked_root = tempdir().unwrap();
            symlink(&token, linked_root.path().join("collector-token")).unwrap();
            assert!(read_token(linked_root.path()).is_err());
        }
    }
}
