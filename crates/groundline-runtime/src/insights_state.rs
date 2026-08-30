use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use groundline_contracts::event::{
    CollectorIdentity as EventIdentity, ConsentReceipt as EventConsent, build_basic_event,
};
use groundline_contracts::insights::validate_basic_event_bytes;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::audit_store::{collect_audit, earliest_recency};
use crate::insights::{default_codex_home, discover_plugin_root, report_url, state_directory};
use crate::local_file::{
    atomic_write_private, create_private_new, open_bounded_regular_file, private_for_current_user,
};
use crate::tailnet;

const PROFILE_PATH: &str = "groundline/insights/owner-profile.json";
const ENROLLMENT_TOKEN_PATH: &str = "groundline/insights/enrollment-token";
const IDENTITY_FILE: &str = "identity.json";
const CONSENT_FILE: &str = "consent.json";
const POLICY_FILE: &str = "owner-auto-policy.json";
const STATUS_FILE: &str = "owner-auto-status.json";
const TOKEN_FILE: &str = "collector-token";
const TOKEN_METADATA_FILE: &str = "collector-token.json";
const OUTBOX_DIR: &str = "outbox";
const LOCK_FILE: &str = "owner-auto.lock";
const MAX_STATE_BYTES: u64 = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
static CRYPTO_PROVIDER: OnceLock<Result<(), ()>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum StateError {
    #[error("invalid_owner_profile")]
    InvalidProfile,
    #[error("local_state_failed")]
    LocalState,
    #[error("already_running")]
    AlreadyRunning,
    #[error("disabled")]
    Disabled,
    #[error("tailnet_not_connected")]
    TailnetDisconnected,
    #[error("audit_failed")]
    AuditFailed,
    #[error("collector_enrollment_failed")]
    EnrollmentFailed,
    #[error("event_upload_failed")]
    UploadFailed,
}

impl StateError {
    pub fn network_performed(&self) -> bool {
        matches!(self, Self::EnrollmentFailed | Self::UploadFailed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Consent {
    schema_version: u8,
    kind: String,
    scope: String,
    status: String,
    receipt_id: Uuid,
    accepted_at_utc: String,
    diagnostic_enabled: bool,
    network_upload_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Status {
    schema_version: u8,
    kind: String,
    enabled: bool,
    last_result_code: String,
    last_check_utc: String,
    last_success_utc: Option<String>,
    last_collected_through_utc: Option<String>,
    uploaded_count: u64,
    pending_event_count: u64,
    tailnet_status: String,
    last_trigger: String,
}

struct CycleLock {
    path: PathBuf,
    _file: File,
}

impl CycleLock {
    fn acquire(directory: &Path) -> Result<Self, StateError> {
        let path = directory.join(LOCK_FILE);
        if let Ok(metadata) = std::fs::symlink_metadata(&path) {
            let stale = metadata.file_type().is_file()
                && metadata
                    .modified()
                    .ok()
                    .and_then(|value| SystemTime::now().duration_since(value).ok())
                    .is_some_and(|age| age > Duration::from_secs(30 * 60));
            if stale {
                std::fs::remove_file(&path).map_err(|_| StateError::AlreadyRunning)?;
            } else {
                return Err(StateError::AlreadyRunning);
            }
        }
        let mut file = create_private_new(&path).map_err(|_| StateError::AlreadyRunning)?;
        writeln!(file, "{}", std::process::id()).map_err(|_| StateError::LocalState)?;
        file.sync_all().map_err(|_| StateError::LocalState)?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for CycleLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn read_bytes(path: &Path, maximum: u64, private: bool) -> Result<Vec<u8>, StateError> {
    let mut file =
        open_bounded_regular_file(path, 1, maximum).map_err(|_| StateError::LocalState)?;
    if private && !private_for_current_user(&file) {
        return Err(StateError::LocalState);
    }
    let mut bytes =
        Vec::with_capacity(file.metadata().map_err(|_| StateError::LocalState)?.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(|_| StateError::LocalState)?;
    Ok(bytes)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, private: bool) -> Result<T, StateError> {
    serde_json::from_slice(&read_bytes(path, MAX_STATE_BYTES, private)?)
        .map_err(|_| StateError::LocalState)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), StateError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|_| StateError::LocalState)?;
    bytes.push(b'\n');
    atomic_write_private(path, &bytes).map_err(|_| StateError::LocalState)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, StateError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StateError::LocalState)
}

fn runtime_family() -> String {
    let explicit = std::env::var("GROUNDLINE_RUNTIME_FAMILY")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(explicit.as_str(), "codex_app" | "codex_cli") {
        return explicit;
    }
    let originator = std::env::var("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if ["app", "chatgpt", "desktop"]
        .iter()
        .any(|marker| originator.contains(marker))
    {
        "codex_app".to_owned()
    } else {
        "codex_cli".to_owned()
    }
}

fn execution_mode(runtime: &str) -> String {
    match std::env::var("GROUNDLINE_EXECUTION_MODE")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "desktop" => "desktop".to_owned(),
        "remote_headless" => "remote_headless".to_owned(),
        "local_headless" => "local_headless".to_owned(),
        _ if runtime == "codex_app" => "desktop".to_owned(),
        _ => "local_headless".to_owned(),
    }
}

fn valid_profile(profile: &Profile) -> bool {
    profile.schema_version == 7
        && profile.kind == "groundline-insights-owner-profile"
        && profile.mode == "private_owner"
        && profile.automatic_activity_checkpoints
        && profile.automatic_initial_history_sync
        && profile.collection_scope == "all_activity"
        && profile.checkpoint_min_interval_seconds == 900
        && !profile.diagnostic_enabled
        && profile.trigger_mode == "native_hook_checkpoints"
        && report_url(&profile.endpoint, 7).is_ok()
}

fn load_profile(codex_home: &Path) -> Result<Profile, StateError> {
    let profile: Profile = serde_json::from_slice(
        &read_bytes(&codex_home.join(PROFILE_PATH), 16 * 1024, true)
            .map_err(|_| StateError::InvalidProfile)?,
    )
    .map_err(|_| StateError::InvalidProfile)?;
    if !valid_profile(&profile) {
        return Err(StateError::InvalidProfile);
    }
    Ok(profile)
}

pub fn configure_profile(codex_home: &Path, bytes: &[u8]) -> Result<Value, StateError> {
    if bytes.is_empty() || bytes.len() > 16 * 1024 {
        return Err(StateError::InvalidProfile);
    }
    let mut input: serde_json::Map<String, Value> =
        serde_json::from_slice(bytes).map_err(|_| StateError::InvalidProfile)?;
    let enrollment_token = input
        .remove("enrollment_token")
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .map(Zeroizing::new)
        .ok_or(StateError::InvalidProfile)?;
    if !(32..=4096).contains(&enrollment_token.len()) {
        return Err(StateError::InvalidProfile);
    }
    let profile: Profile =
        serde_json::from_value(Value::Object(input)).map_err(|_| StateError::InvalidProfile)?;
    if !valid_profile(&profile) {
        return Err(StateError::InvalidProfile);
    }
    let mut canonical =
        serde_json::to_vec_pretty(&profile).map_err(|_| StateError::InvalidProfile)?;
    canonical.push(b'\n');
    let mut credential = Zeroizing::new(enrollment_token.as_bytes().to_vec());
    credential.push(b'\n');
    atomic_write_private(&codex_home.join(ENROLLMENT_TOKEN_PATH), &credential)
        .map_err(|_| StateError::LocalState)?;
    atomic_write_private(&codex_home.join(PROFILE_PATH), &canonical)
        .map_err(|_| StateError::LocalState)?;
    Ok(json!({
        "status":"PASS",
        "profile_schema":7,
        "owner_profile_configured":true,
        "enrollment_credential_configured":true,
        "endpoint_emitted":false,
        "private_paths_emitted":false,
        "secret_value_printed":false,
    }))
}

fn environment() -> (&'static str, String, String) {
    let os = match std::env::consts::OS {
        "macos" => "macos",
        "windows" => "windows",
        "linux" => "linux",
        _ => "unknown",
    };
    let runtime = runtime_family();
    let mode = execution_mode(&runtime);
    (os, runtime, mode)
}

fn initialize(directory: &Path, now: DateTime<Utc>) -> Result<(Identity, Consent), StateError> {
    let (os, runtime, mode) = environment();
    if os == "unknown" {
        return Err(StateError::LocalState);
    }
    let identity_path = directory.join(IDENTITY_FILE);
    let identity = if identity_path.exists() {
        let value: Identity = read_json(&identity_path, true)?;
        if value.schema_version != 1
            || value.kind != "groundline-insights-identity"
            || (
                value.os_family.as_str(),
                value.runtime_family.as_str(),
                value.execution_mode.as_str(),
            ) != (os, runtime.as_str(), mode.as_str())
            || !value.resettable
            || parse_timestamp(&value.created_at_utc).is_err()
        {
            return Err(StateError::LocalState);
        }
        value
    } else {
        let value = Identity {
            schema_version: 1,
            kind: "groundline-insights-identity".to_owned(),
            collector_instance_id: Uuid::new_v4(),
            os_family: os.to_owned(),
            runtime_family: runtime,
            execution_mode: mode,
            created_at_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            resettable: true,
        };
        write_json(&identity_path, &value)?;
        value
    };
    let consent_path = directory.join(CONSENT_FILE);
    let consent = if consent_path.exists() {
        let value: Consent = read_json(&consent_path, true)?;
        if value.schema_version != 1
            || value.kind != "groundline-insights-consent"
            || value.scope != "basic_weekly"
            || value.status != "active"
            || value.diagnostic_enabled
            || value.network_upload_enabled
            || parse_timestamp(&value.accepted_at_utc).is_err()
        {
            return Err(StateError::Disabled);
        }
        value
    } else {
        let value = Consent {
            schema_version: 1,
            kind: "groundline-insights-consent".to_owned(),
            scope: "basic_weekly".to_owned(),
            status: "active".to_owned(),
            receipt_id: Uuid::new_v4(),
            accepted_at_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            diagnostic_enabled: false,
            network_upload_enabled: false,
        };
        write_json(&consent_path, &value)?;
        value
    };
    Ok((identity, consent))
}

fn policy_enabled(directory: &Path) -> Result<bool, StateError> {
    let path = directory.join(POLICY_FILE);
    if !path.exists() {
        return Ok(true);
    }
    let value: Value = read_json(&path, true)?;
    Ok(value.get("status").and_then(Value::as_str) == Some("active"))
}

fn set_policy(directory: &Path, enabled: bool, now: DateTime<Utc>) -> Result<(), StateError> {
    write_json(
        &directory.join(POLICY_FILE),
        &json!({
            "schema_version":1,"kind":"groundline-insights-owner-auto-policy",
            "status":if enabled {"active"} else {"revoked"},
            "automatic_activity_checkpoints":enabled,"collection_scope":"all_activity",
            "diagnostic_enabled":false,"trigger_mode":"native_hook_checkpoints",
            "updated_at_utc":now.to_rfc3339_opts(SecondsFormat::Millis, true),
        }),
    )
}

fn endpoint(profile: &Profile, path: &str) -> Result<Url, StateError> {
    if !matches!(path, "/v1/enroll" | "/v1/events") {
        return Err(StateError::InvalidProfile);
    }
    let mut url = report_url(&profile.endpoint, 7).map_err(|_| StateError::InvalidProfile)?;
    url.set_path(path);
    url.set_query(None);
    Ok(url)
}

fn ensure_crypto_provider() -> Result<(), StateError> {
    CRYPTO_PROVIDER
        .get_or_init(|| {
            if rustls::crypto::CryptoProvider::get_default().is_some()
                || rustls::crypto::ring::default_provider()
                    .install_default()
                    .is_ok()
            {
                Ok(())
            } else {
                Err(())
            }
        })
        .map_err(|_| StateError::LocalState)
}

fn client() -> Result<reqwest::Client, StateError> {
    ensure_crypto_provider()?;
    reqwest::Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(REQUEST_TIMEOUT)
        .user_agent("groundline-rust-runtime/1")
        .build()
        .map_err(|_| StateError::LocalState)
}

async fn bounded_response(
    mut response: reqwest::Response,
) -> Result<(reqwest::StatusCode, Value), StateError> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(StateError::UploadFailed);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| StateError::UploadFailed)?
    {
        if body
            .len()
            .checked_add(chunk.len())
            .is_none_or(|length| length > MAX_RESPONSE_BYTES)
        {
            return Err(StateError::UploadFailed);
        }
        body.extend_from_slice(&chunk);
    }
    let value = serde_json::from_slice(&body).map_err(|_| StateError::UploadFailed)?;
    Ok((status, value))
}

fn token_value(directory: &Path) -> Result<Option<SecretString>, StateError> {
    let path = directory.join(TOKEN_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = Zeroizing::new(read_bytes(&path, 4096, true)?);
    let text =
        Zeroizing::new(String::from_utf8(bytes.to_vec()).map_err(|_| StateError::LocalState)?);
    let token = text.trim().to_owned();
    if !(32..=4096).contains(&token.len()) {
        return Err(StateError::LocalState);
    }
    Ok(Some(SecretString::from(token)))
}

fn enrollment_token(codex_home: &Path) -> Result<SecretString, StateError> {
    let bytes = Zeroizing::new(read_bytes(
        &codex_home.join(ENROLLMENT_TOKEN_PATH),
        4096,
        true,
    )?);
    let text =
        Zeroizing::new(String::from_utf8(bytes.to_vec()).map_err(|_| StateError::LocalState)?);
    let token = text.trim().to_owned();
    if !(32..=4096).contains(&token.len()) {
        return Err(StateError::LocalState);
    }
    Ok(SecretString::from(token))
}

async fn enroll(
    profile: &Profile,
    codex_home: &Path,
    directory: &Path,
    identity: &Identity,
) -> Result<SecretString, StateError> {
    if directory.join(TOKEN_METADATA_FILE).is_file()
        && let Some(token) = token_value(directory)?
    {
        return Ok(token);
    }
    let token = token_value(directory)?.unwrap_or_else(|| {
        SecretString::from(format!(
            "{}{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        ))
    });
    if !directory.join(TOKEN_FILE).exists() {
        atomic_write_private(
            &directory.join(TOKEN_FILE),
            format!("{}\n", token.expose_secret()).as_bytes(),
        )
        .map_err(|_| StateError::LocalState)?;
    }
    let body = serde_json::to_vec(&json!({
        "schema_version":2,"kind":"groundline-insights-owner-enrollment",
        "collector_instance_id":identity.collector_instance_id,"collector_token":token.expose_secret(),
        "os_family":identity.os_family,"runtime_family":identity.runtime_family,"execution_mode":identity.execution_mode,
        "groundline_version":env!("CARGO_PKG_VERSION"),
    }))
    .map_err(|_| StateError::LocalState)?;
    let response = client()?
        .post(endpoint(profile, "/v1/enroll")?)
        .header(
            AUTHORIZATION,
            format!("Bearer {}", enrollment_token(codex_home)?.expose_secret()),
        )
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .map_err(|_| StateError::EnrollmentFailed)?;
    let (status, value) = bounded_response(response)
        .await
        .map_err(|_| StateError::EnrollmentFailed)?;
    if !matches!(status.as_u16(), 200 | 201)
        || value.get("status").and_then(Value::as_str) != Some("PASS")
        || value.get("collector_instance_id").and_then(Value::as_str)
            != Some(identity.collector_instance_id.to_string().as_str())
    {
        return Err(StateError::EnrollmentFailed);
    }
    write_json(
        &directory.join(TOKEN_METADATA_FILE),
        &json!({
            "schema_version":2,"kind":"groundline-insights-token-metadata","collector_instance_id":identity.collector_instance_id,
            "enrolled_at_utc":Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),"os_family":identity.os_family,
            "runtime_family":identity.runtime_family,"execution_mode":identity.execution_mode,"groundline_version":env!("CARGO_PKG_VERSION"),
        }),
    )?;
    Ok(token)
}

fn pending_events(directory: &Path) -> Result<Vec<(PathBuf, Value)>, StateError> {
    let outbox = directory.join(OUTBOX_DIR);
    if !outbox.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for entry in std::fs::read_dir(&outbox).map_err(|_| StateError::LocalState)? {
        let path = entry.map_err(|_| StateError::LocalState)?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            return Err(StateError::LocalState);
        }
        let bytes = read_bytes(&path, 64 * 1024, true)?;
        let event = validate_basic_event_bytes(&bytes).map_err(|_| StateError::LocalState)?;
        if path.file_stem().and_then(|value| value.to_str())
            != event.get("event_id").and_then(Value::as_str)
        {
            return Err(StateError::LocalState);
        }
        result.push((path, event));
    }
    result.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(result)
}

fn enqueue(directory: &Path, event: &Value) -> Result<(), StateError> {
    let event_id = event
        .get("event_id")
        .and_then(Value::as_str)
        .ok_or(StateError::LocalState)?;
    let path = directory.join(OUTBOX_DIR).join(format!("{event_id}.json"));
    if path.exists() {
        let existing: Value = read_json(&path, true)?;
        return if &existing == event {
            Ok(())
        } else {
            Err(StateError::LocalState)
        };
    }
    write_json(&path, event)
}

async fn upload(
    profile: &Profile,
    identity: &Identity,
    token: &SecretString,
    events: Vec<(PathBuf, Value)>,
) -> Result<u64, StateError> {
    let mut uploaded = 0_u64;
    for (path, event) in events {
        let body = serde_json::to_vec(&event).map_err(|_| StateError::LocalState)?;
        let mut headers = HeaderMap::new();
        let mut auth =
            HeaderValue::from_str(&Zeroizing::new(format!("Bearer {}", token.expose_secret())))
                .map_err(|_| StateError::LocalState)?;
        auth.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-groundline-collector-id",
            HeaderValue::from_str(&identity.collector_instance_id.to_string())
                .map_err(|_| StateError::LocalState)?,
        );
        headers.insert(
            "x-groundline-version",
            HeaderValue::from_str(env!("CARGO_PKG_VERSION")).map_err(|_| StateError::LocalState)?,
        );
        headers.insert(
            "idempotency-key",
            HeaderValue::from_str(
                event
                    .get("idempotency_key")
                    .and_then(Value::as_str)
                    .ok_or(StateError::LocalState)?,
            )
            .map_err(|_| StateError::LocalState)?,
        );
        let response = client()?
            .post(endpoint(profile, "/v1/events")?)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|_| StateError::UploadFailed)?;
        let (status, value) = bounded_response(response).await?;
        if !matches!(status.as_u16(), 200 | 202)
            || value.get("status").and_then(Value::as_str) != Some("PASS")
            || !matches!(
                value.get("outcome").and_then(Value::as_str),
                Some("accepted" | "duplicate")
            )
        {
            return Err(StateError::UploadFailed);
        }
        std::fs::remove_file(path).map_err(|_| StateError::LocalState)?;
        uploaded = uploaded.checked_add(1).ok_or(StateError::LocalState)?;
    }
    Ok(uploaded)
}

fn previous_status(directory: &Path) -> Option<Status> {
    read_json(&directory.join(STATUS_FILE), true).ok()
}

fn write_status(directory: &Path, status: &Status) -> Result<(), StateError> {
    write_json(&directory.join(STATUS_FILE), status)
}

pub fn enable(codex_home: &Path) -> Result<Value, StateError> {
    let directory = state_directory(codex_home);
    let now = Utc::now();
    initialize(&directory, now)?;
    set_policy(&directory, true, now)?;
    Ok(json!({"status":"PASS","enabled":true,"mutation_performed":true}))
}

pub fn disable(codex_home: &Path) -> Result<Value, StateError> {
    let directory = state_directory(codex_home);
    set_policy(&directory, false, Utc::now())?;
    Ok(json!({"status":"PASS","disabled":true,"mutation_performed":true}))
}

pub fn status(codex_home: &Path) -> Result<Value, StateError> {
    let directory = state_directory(codex_home);
    let enabled = policy_enabled(&directory)?;
    let pending = pending_events(&directory)
        .map(|items| items.len() as u64)
        .unwrap_or(0);
    let status = previous_status(&directory);
    let tailnet = tailnet::probe();
    Ok(json!({
        "status":"PASS","collection_enabled":enabled,"owner_profile_configured":codex_home.join(PROFILE_PATH).is_file(),"enrollment_credential_present":codex_home.join(ENROLLMENT_TOKEN_PATH).is_file(),"identity_present":directory.join(IDENTITY_FILE).is_file(),
        "consent_status":if directory.join(CONSENT_FILE).is_file() {"active"} else {"missing"},"collector_token_present":directory.join(TOKEN_FILE).is_file(),
        "pending_event_count":pending,"last_check_result_code":status.as_ref().map(|value| value.last_result_code.as_str()),
        "last_check_utc":status.as_ref().map(|value| value.last_check_utc.as_str()),"last_success_utc":status.as_ref().and_then(|value| value.last_success_utc.as_deref()),
        "last_collected_through_utc":status.as_ref().and_then(|value| value.last_collected_through_utc.as_deref()),
        "tailnet":tailnet,"raw_content_emitted":false,"private_paths_emitted":false,"secret_value_printed":false,
    }))
}

pub async fn run_once(
    _plugin_root: &Path,
    codex_home: &Path,
    trigger: &str,
) -> Result<Value, StateError> {
    if !matches!(
        trigger,
        "manual"
            | "history_sync"
            | "session_start_hook"
            | "stop_hook"
            | "post_compact_hook"
            | "session_end_hook"
    ) {
        return Err(StateError::LocalState);
    }
    let profile = load_profile(codex_home)?;
    let directory = state_directory(codex_home);
    let _lock = CycleLock::acquire(&directory)?;
    if !policy_enabled(&directory)? {
        return Err(StateError::Disabled);
    }
    let now = Utc::now();
    let (identity, consent) = initialize(&directory, now)?;
    let tailnet_state = tailnet::probe();
    let tailnet_status = tailnet_state
        .get("tailnet_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    if tailnet_state
        .get("tailnet_connected")
        .and_then(Value::as_bool)
        != Some(true)
    {
        write_status(
            &directory,
            &Status {
                schema_version: 4,
                kind: "groundline-insights-owner-auto-status".to_owned(),
                enabled: true,
                last_result_code: "tailnet_not_connected".to_owned(),
                last_check_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
                last_success_utc: previous_status(&directory)
                    .and_then(|value| value.last_success_utc),
                last_collected_through_utc: previous_status(&directory)
                    .and_then(|value| value.last_collected_through_utc),
                uploaded_count: 0,
                pending_event_count: pending_events(&directory)
                    .map(|items| items.len() as u64)
                    .unwrap_or(0),
                tailnet_status,
                last_trigger: trigger.to_owned(),
            },
        )?;
        return Err(StateError::TailnetDisconnected);
    }
    let token = enroll(&profile, codex_home, &directory, &identity).await?;
    let previous = previous_status(&directory);
    let start = previous
        .as_ref()
        .and_then(|value| value.last_collected_through_utc.as_deref())
        .and_then(|value| parse_timestamp(value).ok())
        .or_else(|| earliest_recency(codex_home).ok().flatten())
        .unwrap_or_else(|| now - chrono::Duration::days(7));
    let start = start.min(now - chrono::Duration::seconds(1));
    let audit = collect_audit(
        codex_home,
        start,
        now,
        Some(identity.runtime_family.as_str()),
        false,
    )
    .map_err(|_| StateError::AuditFailed)?;
    if audit
        .pointer("/scope/observed_root_sample_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        let event = build_basic_event(
            &audit,
            EventIdentity {
                instance_id: identity.collector_instance_id,
                os_family: &identity.os_family,
                runtime_family: &identity.runtime_family,
                execution_mode: &identity.execution_mode,
            },
            EventConsent {
                receipt_id: consent.receipt_id,
                accepted_at_utc: &consent.accepted_at_utc,
            },
            env!("CARGO_PKG_VERSION"),
            0,
            trigger,
        )
        .map_err(|_| StateError::AuditFailed)?;
        enqueue(&directory, &event)?;
    }
    let uploaded = upload(&profile, &identity, &token, pending_events(&directory)?).await?;
    let pending = pending_events(&directory)?.len() as u64;
    write_status(
        &directory,
        &Status {
            schema_version: 4,
            kind: "groundline-insights-owner-auto-status".to_owned(),
            enabled: true,
            last_result_code: "pass".to_owned(),
            last_check_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            last_success_utc: Some(now.to_rfc3339_opts(SecondsFormat::Millis, true)),
            last_collected_through_utc: Some(now.to_rfc3339_opts(SecondsFormat::Millis, true)),
            uploaded_count: uploaded,
            pending_event_count: pending,
            tailnet_status,
            last_trigger: trigger.to_owned(),
        },
    )?;
    Ok(json!({
        "status":"PASS","result_code":"pass","uploaded_count":uploaded,"pending_event_count":pending,
        "last_collected_through_utc":now.to_rfc3339_opts(SecondsFormat::Millis, true),
        "raw_content_emitted":false,"private_paths_emitted":false,"secret_value_printed":false,
    }))
}

pub fn resolve_roots(
    plugin_root: Option<PathBuf>,
    codex_home: Option<PathBuf>,
) -> Result<(PathBuf, PathBuf), StateError> {
    Ok((
        plugin_root
            .map(Ok)
            .unwrap_or_else(discover_plugin_root)
            .map_err(|_| StateError::LocalState)?,
        codex_home
            .map(Ok)
            .unwrap_or_else(default_codex_home)
            .map_err(|_| StateError::LocalState)?,
    ))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::local_file::{open_bounded_regular_file, private_for_current_user};

    use super::{ENROLLMENT_TOKEN_PATH, PROFILE_PATH, configure_profile};

    fn profile(extra: &str) -> Vec<u8> {
        format!(
            r#"{{
  "schema_version": 7,
  "kind": "groundline-insights-owner-profile",
  "mode": "private_owner",
  "endpoint": "http://100.64.0.1:18080",
  "automatic_activity_checkpoints": true,
  "automatic_initial_history_sync": true,
  "collection_scope": "all_activity",
  "checkpoint_min_interval_seconds": 900,
  "diagnostic_enabled": false,
  "trigger_mode": "native_hook_checkpoints",
  "enrollment_token": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"{extra}
}}"#
        )
        .into_bytes()
    }

    #[test]
    fn configures_owner_profile_outside_plugin_with_private_permissions() {
        let home = tempdir().expect("temporary Codex home");
        let receipt = configure_profile(home.path(), &profile("")).expect("configure profile");
        assert_eq!(receipt["owner_profile_configured"], true);
        assert_eq!(receipt["enrollment_credential_configured"], true);
        assert_eq!(receipt["endpoint_emitted"], false);
        let path = home.path().join(PROFILE_PATH);
        let file = open_bounded_regular_file(&path, 1, 16 * 1024).expect("profile file");
        assert!(private_for_current_user(&file));
        let profile_bytes = std::fs::read(&path).expect("sanitized profile");
        assert!(
            !profile_bytes
                .windows(16)
                .any(|value| value == b"enrollment_token")
        );
        assert!(
            !profile_bytes
                .windows(32)
                .any(|value| value == b"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")
        );
        let token = open_bounded_regular_file(&home.path().join(ENROLLMENT_TOKEN_PATH), 32, 4096)
            .expect("enrollment token file");
        assert!(private_for_current_user(&token));
    }

    #[test]
    fn rejects_embedded_release_version_and_unknown_profile_fields() {
        let home = tempdir().expect("temporary Codex home");
        assert!(
            configure_profile(
                home.path(),
                &profile(",\n  \"groundline_version\": \"0.20.0\"")
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_missing_or_short_enrollment_credentials() {
        let home = tempdir().expect("temporary Codex home");
        let without = String::from_utf8(profile("")).unwrap().replace(
            ",\n  \"enrollment_token\": \"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\"",
            "",
        );
        assert!(configure_profile(home.path(), without.as_bytes()).is_err());
        let short = String::from_utf8(profile(""))
            .unwrap()
            .replace("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee", "short");
        assert!(configure_profile(home.path(), short.as_bytes()).is_err());
    }
}
