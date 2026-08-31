use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
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
const QUARANTINE_DIR: &str = "outbox-quarantine";
const RETRY_FILE: &str = "delivery-retry.json";
const LOCK_FILE: &str = "owner-auto.lock";
const MAX_STATE_BYTES: u64 = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_OUTBOX_EVENTS: usize = 256;
const OUTBOX_HIGH_WATERMARK_EVENTS: usize = 224;
const MAX_OUTBOX_BYTES: u64 = 16 * 1024 * 1024;
const UPLOAD_BATCH_EVENTS: usize = 16;
const UPLOAD_CYCLE_TIMEOUT: Duration = Duration::from_secs(45);
const RETRY_BASE_SECONDS: u64 = 60;
const RETRY_MAX_SECONDS: u64 = 60 * 60;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const COLLECTION_STALE_AFTER: chrono::Duration = chrono::Duration::days(7);
const MAX_CLOCK_SKEW: chrono::Duration = chrono::Duration::minutes(5);
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
    #[error("remote_request_rejected")]
    RemoteRejected,
    #[error("outbox_capacity_exceeded")]
    OutboxCapacity,
    #[error("reconsent_required")]
    ReconsentRequired,
}

impl StateError {
    pub fn network_performed(&self) -> bool {
        matches!(
            self,
            Self::EnrollmentFailed | Self::UploadFailed | Self::RemoteRejected
        )
    }

    pub fn mutation_performed(&self) -> Option<bool> {
        match self {
            Self::InvalidProfile
            | Self::AlreadyRunning
            | Self::Disabled
            | Self::ReconsentRequired => Some(false),
            Self::TailnetDisconnected => Some(true),
            Self::LocalState
            | Self::AuditFailed
            | Self::EnrollmentFailed
            | Self::UploadFailed
            | Self::RemoteRejected
            | Self::OutboxCapacity => None,
        }
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
    owner_service_upload_enabled: bool,
    third_party_upload_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedConsentV1 {
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
struct OwnerPolicy {
    schema_version: u8,
    kind: String,
    status: String,
    automatic_activity_checkpoints: bool,
    collection_scope: String,
    diagnostic_enabled: bool,
    trigger_mode: String,
    updated_at_utc: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedPrivatePolicyV1 {
    schema_version: u8,
    kind: String,
    status: String,
    automatic_activity_checkpoints: bool,
    collection_scope: String,
    diagnostic_enabled: bool,
    trigger_mode: String,
    accepted_at_utc: String,
    basic_receipt_id: Uuid,
    chronicle_access_enabled: bool,
    collection_generation: u64,
    cron_or_timer_created: bool,
    global_hook_created: bool,
    grant_source: String,
    mcp_lifecycle_enabled: bool,
    policy_id: Uuid,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeliveryRetry {
    schema_version: u8,
    kind: String,
    attempt_count: u32,
    next_attempt_utc: Option<String>,
    last_error_code: String,
    operator_required: bool,
}

struct OutboxInventory {
    batch: Vec<(PathBuf, Value)>,
    observed_count: usize,
    observed_bytes: u64,
    capacity_exceeded: bool,
}

struct UploadReceipt {
    uploaded_count: u64,
    acknowledged_paths: Vec<PathBuf>,
    collected_through_utc: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedPrivateStatusV3 {
    schema_version: u8,
    kind: String,
    cron_or_timer_created: bool,
    global_hook_created: bool,
    last_attempt_result_code: String,
    last_attempt_utc: String,
    last_check_result_code: String,
    last_check_utc: String,
    last_collected_through_utc: String,
    last_native_lifecycle_receipt: Value,
    last_network_failure: String,
    last_staged_through_utc: Option<String>,
    last_success_utc: String,
    last_tailnet_check_utc: String,
    last_tailnet_notification_utc: Option<String>,
    mcp_lifecycle_enabled: bool,
    next_attempt_after_utc: Option<String>,
    private_paths_recorded: bool,
    raw_content_recorded: bool,
    retry_reason_code: Option<String>,
    secret_value_recorded: bool,
    tailnet_cli_available: bool,
    tailnet_connected: Option<bool>,
    tailnet_health: String,
    tailnet_notification_result: Option<String>,
    tailnet_status: String,
    trigger: String,
    uploaded_count: u64,
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

fn valid_active_consent(value: &Consent) -> bool {
    value.schema_version == 2
        && value.kind == "groundline-insights-consent"
        && value.scope == "basic_weekly"
        && value.status == "active"
        && !value.receipt_id.is_nil()
        && !value.diagnostic_enabled
        && value.owner_service_upload_enabled
        && !value.third_party_upload_enabled
        && parse_timestamp(&value.accepted_at_utc).is_ok()
}

fn valid_legacy_consent(value: &ImportedConsentV1) -> bool {
    value.schema_version == 1
        && value.kind == "groundline-insights-consent"
        && value.scope == "basic_weekly"
        && value.status == "active"
        && !value.receipt_id.is_nil()
        && !value.diagnostic_enabled
        && !value.network_upload_enabled
        && parse_timestamp(&value.accepted_at_utc).is_ok()
}

fn active_consent(directory: &Path) -> Result<Consent, StateError> {
    let path = directory.join(CONSENT_FILE);
    if !path.exists() {
        return Err(StateError::ReconsentRequired);
    }
    let bytes = read_bytes(&path, MAX_STATE_BYTES, true)?;
    if let Ok(value) = serde_json::from_slice::<Consent>(&bytes) {
        return if valid_active_consent(&value) {
            Ok(value)
        } else {
            Err(StateError::ReconsentRequired)
        };
    }
    let imported: ImportedConsentV1 =
        serde_json::from_slice(&bytes).map_err(|_| StateError::Disabled)?;
    if valid_legacy_consent(&imported) {
        Err(StateError::ReconsentRequired)
    } else {
        Err(StateError::Disabled)
    }
}

fn consent_status(directory: &Path) -> Result<&'static str, StateError> {
    let path = directory.join(CONSENT_FILE);
    if !path.exists() {
        return Ok("missing");
    }
    let bytes = read_bytes(&path, MAX_STATE_BYTES, true)?;
    if let Ok(value) = serde_json::from_slice::<Consent>(&bytes) {
        return Ok(if valid_active_consent(&value) {
            "active"
        } else {
            "reconsent_required"
        });
    }
    let imported: ImportedConsentV1 =
        serde_json::from_slice(&bytes).map_err(|_| StateError::LocalState)?;
    if valid_legacy_consent(&imported) {
        Ok("reconsent_required")
    } else {
        Err(StateError::LocalState)
    }
}

fn grant_consent(directory: &Path, now: DateTime<Utc>) -> Result<Consent, StateError> {
    let path = directory.join(CONSENT_FILE);
    if path.exists() {
        let bytes = read_bytes(&path, MAX_STATE_BYTES, true)?;
        if let Ok(value) = serde_json::from_slice::<Consent>(&bytes)
            && valid_active_consent(&value)
        {
            return Ok(value);
        }
        let legacy: ImportedConsentV1 =
            serde_json::from_slice(&bytes).map_err(|_| StateError::Disabled)?;
        if !valid_legacy_consent(&legacy) {
            return Err(StateError::Disabled);
        }
        let archive = directory.join("consent.legacy-v1.json");
        if archive.exists() {
            return Err(StateError::LocalState);
        }
        quarantine_pending_events(directory)?;
        std::fs::rename(&path, archive).map_err(|_| StateError::LocalState)?;
    } else {
        quarantine_pending_events(directory)?;
    }
    let value = Consent {
        schema_version: 2,
        kind: "groundline-insights-consent".to_owned(),
        scope: "basic_weekly".to_owned(),
        status: "active".to_owned(),
        receipt_id: Uuid::new_v4(),
        accepted_at_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
        diagnostic_enabled: false,
        owner_service_upload_enabled: true,
        third_party_upload_enabled: false,
    };
    write_json(&path, &value)?;
    Ok(value)
}

fn initialize(directory: &Path, now: DateTime<Utc>) -> Result<(Identity, Consent), StateError> {
    let consent = active_consent(directory)?;
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
    Ok((identity, consent))
}

fn read_owner_policy(path: &Path) -> Result<OwnerPolicy, StateError> {
    let bytes = read_bytes(path, MAX_STATE_BYTES, true)?;
    if let Ok(policy) = serde_json::from_slice::<OwnerPolicy>(&bytes) {
        return Ok(policy);
    }
    let imported: ImportedPrivatePolicyV1 =
        serde_json::from_slice(&bytes).map_err(|_| StateError::LocalState)?;
    if imported.schema_version != 1
        || imported.kind != "groundline-insights-owner-auto-policy"
        || parse_timestamp(&imported.accepted_at_utc).is_err()
        || imported.basic_receipt_id.is_nil()
        || imported.policy_id.is_nil()
        || imported.chronicle_access_enabled
        || imported.collection_generation != 1
        || imported.cron_or_timer_created
        || imported.global_hook_created
        || imported.mcp_lifecycle_enabled
        || imported.grant_source != "private_plugin_install"
    {
        return Err(StateError::LocalState);
    }
    Ok(OwnerPolicy {
        schema_version: imported.schema_version,
        kind: imported.kind,
        status: imported.status,
        automatic_activity_checkpoints: imported.automatic_activity_checkpoints,
        collection_scope: imported.collection_scope,
        diagnostic_enabled: imported.diagnostic_enabled,
        trigger_mode: imported.trigger_mode,
        updated_at_utc: imported.accepted_at_utc,
    })
}

fn policy_enabled(directory: &Path) -> Result<bool, StateError> {
    let path = directory.join(POLICY_FILE);
    if !path.exists() {
        return Ok(false);
    }
    let policy = read_owner_policy(&path)?;
    let enabled = match policy.status.as_str() {
        "active" if policy.automatic_activity_checkpoints => true,
        "revoked" if !policy.automatic_activity_checkpoints => false,
        _ => return Err(StateError::LocalState),
    };
    if policy.schema_version != 1
        || policy.kind != "groundline-insights-owner-auto-policy"
        || policy.collection_scope != "all_activity"
        || policy.diagnostic_enabled
        || policy.trigger_mode != "native_hook_checkpoints"
        || parse_timestamp(&policy.updated_at_utc).is_err()
    {
        return Err(StateError::LocalState);
    }
    Ok(enabled)
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

fn permanent_remote_rejection(status: reqwest::StatusCode) -> bool {
    status.is_client_error() && !matches!(status.as_u16(), 408 | 429)
}

fn classify_response_status(status: reqwest::StatusCode) -> Result<(), StateError> {
    if permanent_remote_rejection(status) {
        Err(StateError::RemoteRejected)
    } else {
        Ok(())
    }
}

fn validate_upload_response(status: reqwest::StatusCode, value: &Value) -> Result<(), StateError> {
    classify_response_status(status)?;
    if matches!(status.as_u16(), 200 | 202)
        && value.get("status").and_then(Value::as_str) == Some("PASS")
        && matches!(
            value.get("outcome").and_then(Value::as_str),
            Some("accepted" | "duplicate")
        )
    {
        Ok(())
    } else {
        Err(StateError::UploadFailed)
    }
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
    classify_response_status(response.status())?;
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

fn pending_events(directory: &Path, batch_limit: usize) -> Result<OutboxInventory, StateError> {
    let outbox = directory.join(OUTBOX_DIR);
    if !outbox.exists() {
        return Ok(OutboxInventory {
            batch: Vec::new(),
            observed_count: 0,
            observed_bytes: 0,
            capacity_exceeded: false,
        });
    }
    let mut entries = Vec::new();
    let mut observed_bytes = 0_u64;
    let mut capacity_exceeded = false;
    for entry in std::fs::read_dir(&outbox).map_err(|_| StateError::LocalState)? {
        let entry = entry.map_err(|_| StateError::LocalState)?;
        if !entry
            .file_type()
            .map_err(|_| StateError::LocalState)?
            .is_file()
        {
            return Err(StateError::LocalState);
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            return Err(StateError::LocalState);
        }
        let file = open_bounded_regular_file(&path, 1, MAX_STATE_BYTES)
            .map_err(|_| StateError::LocalState)?;
        if !private_for_current_user(&file) {
            return Err(StateError::LocalState);
        }
        observed_bytes = observed_bytes
            .checked_add(file.metadata().map_err(|_| StateError::LocalState)?.len())
            .ok_or(StateError::OutboxCapacity)?;
        entries.push(path);
        if entries.len() > MAX_OUTBOX_EVENTS || observed_bytes > MAX_OUTBOX_BYTES {
            capacity_exceeded = true;
            break;
        }
    }
    entries.sort();
    let observed_count = entries.len();
    let mut batch = Vec::new();
    for path in entries
        .into_iter()
        .take(batch_limit.min(UPLOAD_BATCH_EVENTS))
    {
        let bytes = read_bytes(&path, 64 * 1024, true)?;
        let event = validate_basic_event_bytes(&bytes).map_err(|_| StateError::LocalState)?;
        if path.file_stem().and_then(|value| value.to_str())
            != event.get("event_id").and_then(Value::as_str)
        {
            return Err(StateError::LocalState);
        }
        batch.push((path, event));
    }
    Ok(OutboxInventory {
        batch,
        observed_count,
        observed_bytes,
        capacity_exceeded,
    })
}

fn quarantine_pending_events(directory: &Path) -> Result<(), StateError> {
    let outbox = directory.join(OUTBOX_DIR);
    if !outbox.exists() {
        return Ok(());
    }
    let quarantine = directory.join(QUARANTINE_DIR);
    std::fs::create_dir_all(&quarantine).map_err(|_| StateError::LocalState)?;
    for (moved, entry) in std::fs::read_dir(&outbox)
        .map_err(|_| StateError::LocalState)?
        .enumerate()
    {
        if moved >= MAX_OUTBOX_EVENTS {
            return Err(StateError::OutboxCapacity);
        }
        let entry = entry.map_err(|_| StateError::LocalState)?;
        if !entry
            .file_type()
            .map_err(|_| StateError::LocalState)?
            .is_file()
        {
            return Err(StateError::LocalState);
        }
        let source = entry.path();
        if source.extension().and_then(|value| value.to_str()) != Some("json") {
            return Err(StateError::LocalState);
        }
        let file = open_bounded_regular_file(&source, 1, MAX_STATE_BYTES)
            .map_err(|_| StateError::LocalState)?;
        if !private_for_current_user(&file) {
            return Err(StateError::LocalState);
        }
        drop(file);
        let destination = quarantine.join(source.file_name().ok_or(StateError::LocalState)?);
        if destination.exists() {
            return Err(StateError::LocalState);
        }
        std::fs::rename(source, destination).map_err(|_| StateError::LocalState)?;
    }
    Ok(())
}

fn quarantined_event_count(directory: &Path) -> Result<(usize, bool), StateError> {
    let quarantine = directory.join(QUARANTINE_DIR);
    if !quarantine.exists() {
        return Ok((0, false));
    }
    let mut count = 0_usize;
    for entry in std::fs::read_dir(quarantine).map_err(|_| StateError::LocalState)? {
        let entry = entry.map_err(|_| StateError::LocalState)?;
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|_| StateError::LocalState)?
            .is_file()
            || path.extension().and_then(|value| value.to_str()) != Some("json")
        {
            return Err(StateError::LocalState);
        }
        let file = open_bounded_regular_file(&path, 1, MAX_STATE_BYTES)
            .map_err(|_| StateError::LocalState)?;
        if !private_for_current_user(&file) {
            return Err(StateError::LocalState);
        }
        count = count.checked_add(1).ok_or(StateError::OutboxCapacity)?;
        if count > MAX_OUTBOX_EVENTS {
            return Ok((count, true));
        }
    }
    Ok((count, false))
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
    let inventory = pending_events(directory, 0)?;
    let event_bytes = serde_json::to_vec_pretty(event)
        .map_err(|_| StateError::LocalState)?
        .len()
        .checked_add(1)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or(StateError::OutboxCapacity)?;
    if inventory.capacity_exceeded
        || inventory.observed_count >= MAX_OUTBOX_EVENTS
        || inventory
            .observed_bytes
            .checked_add(event_bytes)
            .is_none_or(|value| value > MAX_OUTBOX_BYTES)
    {
        return Err(StateError::OutboxCapacity);
    }
    write_json(&path, event)
}

async fn upload(
    profile: &Profile,
    identity: &Identity,
    token: &SecretString,
    events: Vec<(PathBuf, Value)>,
) -> Result<UploadReceipt, StateError> {
    let mut uploaded = 0_u64;
    let mut acknowledged_paths = Vec::new();
    let mut collected_through = None::<DateTime<Utc>>;
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
        classify_response_status(response.status())?;
        let (status, value) = bounded_response(response).await?;
        validate_upload_response(status, &value)?;
        let event_through = event
            .pointer("/period/end_utc")
            .and_then(Value::as_str)
            .and_then(|value| parse_timestamp(value).ok());
        if event_through
            .is_some_and(|value| collected_through.is_none_or(|current| value > current))
        {
            collected_through = event_through;
        }
        acknowledged_paths.push(path);
        uploaded = uploaded.checked_add(1).ok_or(StateError::LocalState)?;
    }
    Ok(UploadReceipt {
        uploaded_count: uploaded,
        acknowledged_paths,
        collected_through_utc: collected_through
            .map(|value| value.to_rfc3339_opts(SecondsFormat::Millis, true)),
    })
}

fn remove_acknowledged_events(receipt: &UploadReceipt) -> Result<(), StateError> {
    for path in &receipt.acknowledged_paths {
        let file = open_bounded_regular_file(path, 1, MAX_STATE_BYTES)
            .map_err(|_| StateError::LocalState)?;
        if !private_for_current_user(&file) {
            return Err(StateError::LocalState);
        }
        drop(file);
        std::fs::remove_file(path).map_err(|_| StateError::LocalState)?;
    }
    Ok(())
}

fn latest_timestamp(current: Option<String>, candidate: Option<String>) -> Option<String> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => {
            let current_at = parse_timestamp(&current).ok();
            let candidate_at = parse_timestamp(&candidate).ok();
            if candidate_at
                .zip(current_at)
                .is_some_and(|(candidate, current)| candidate > current)
            {
                Some(candidate)
            } else {
                Some(current)
            }
        }
        (current, candidate) => current.or(candidate),
    }
}

fn read_delivery_retry(directory: &Path) -> Result<Option<DeliveryRetry>, StateError> {
    let path = directory.join(RETRY_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let value: DeliveryRetry = read_json(&path, true)?;
    if value.schema_version != 1
        || value.kind != "groundline-insights-delivery-retry"
        || value.attempt_count > 32
        || value.last_error_code.is_empty()
        || value
            .next_attempt_utc
            .as_deref()
            .is_some_and(|timestamp| parse_timestamp(timestamp).is_err())
        || (value.operator_required && value.next_attempt_utc.is_some())
    {
        return Err(StateError::LocalState);
    }
    Ok(Some(value))
}

fn retry_delay_seconds(directory: &Path, attempt_count: u32) -> u64 {
    let exponent = attempt_count.saturating_sub(1).min(6);
    let base = RETRY_BASE_SECONDS
        .saturating_mul(1_u64 << exponent)
        .min(RETRY_MAX_SECONDS);
    let mut hasher = DefaultHasher::new();
    directory.hash(&mut hasher);
    attempt_count.hash(&mut hasher);
    let jitter_window = base / 4;
    base.saturating_add(hasher.finish() % jitter_window.saturating_add(1))
        .min(RETRY_MAX_SECONDS)
}

fn record_delivery_retry(
    directory: &Path,
    now: DateTime<Utc>,
    error: &StateError,
) -> Result<DeliveryRetry, StateError> {
    let previous = read_delivery_retry(directory)?;
    let attempt_count = previous
        .as_ref()
        .map(|value| value.attempt_count)
        .unwrap_or(0)
        .saturating_add(1)
        .min(32);
    let operator_required = matches!(error, StateError::RemoteRejected);
    let next_attempt_utc = if operator_required {
        None
    } else {
        Some(
            (now + chrono::Duration::seconds(retry_delay_seconds(directory, attempt_count) as i64))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        )
    };
    let value = DeliveryRetry {
        schema_version: 1,
        kind: "groundline-insights-delivery-retry".to_owned(),
        attempt_count,
        next_attempt_utc,
        last_error_code: error.to_string(),
        operator_required,
    };
    write_json(&directory.join(RETRY_FILE), &value)?;
    Ok(value)
}

fn schedule_remaining_delivery(
    directory: &Path,
    now: DateTime<Utc>,
) -> Result<DeliveryRetry, StateError> {
    let value = DeliveryRetry {
        schema_version: 1,
        kind: "groundline-insights-delivery-retry".to_owned(),
        attempt_count: 0,
        next_attempt_utc: Some(
            (now + chrono::Duration::seconds(RETRY_BASE_SECONDS as i64))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        ),
        last_error_code: "delivery_batch_pending".to_owned(),
        operator_required: false,
    };
    write_json(&directory.join(RETRY_FILE), &value)?;
    Ok(value)
}

fn clear_delivery_retry(directory: &Path) -> Result<(), StateError> {
    let path = directory.join(RETRY_FILE);
    if !path.exists() {
        return Ok(());
    }
    let file =
        open_bounded_regular_file(&path, 1, MAX_STATE_BYTES).map_err(|_| StateError::LocalState)?;
    if !private_for_current_user(&file) {
        return Err(StateError::LocalState);
    }
    drop(file);
    std::fs::remove_file(path).map_err(|_| StateError::LocalState)
}

fn delivery_is_due(trigger: &str, retry: Option<&DeliveryRetry>, now: DateTime<Utc>) -> bool {
    if matches!(trigger, "manual" | "history_sync") {
        return true;
    }
    let Some(retry) = retry else {
        return true;
    };
    retry
        .next_attempt_utc
        .as_deref()
        .and_then(|value| parse_timestamp(value).ok())
        .is_some_and(|value| value <= now)
}

fn explicit_operator_retry(trigger: &str) -> bool {
    matches!(trigger, "manual" | "history_sync")
}

fn operator_retry_blocked(trigger: &str, retry: Option<&DeliveryRetry>) -> bool {
    retry.is_some_and(|value| value.operator_required) && !explicit_operator_retry(trigger)
}

fn valid_status_trigger(trigger: &str) -> bool {
    matches!(
        trigger,
        "manual"
            | "history_sync"
            | "session_start_hook"
            | "stop_hook"
            | "post_compact_hook"
            | "session_end_hook"
    )
}

fn collection_is_due(
    trigger: &str,
    previous: Option<&Status>,
    now: DateTime<Utc>,
    minimum_interval_seconds: u64,
) -> bool {
    if matches!(trigger, "manual" | "history_sync") {
        return true;
    }
    let Some(last_check) = previous.and_then(|value| parse_timestamp(&value.last_check_utc).ok())
    else {
        return true;
    };
    let elapsed = now.signed_duration_since(last_check);
    elapsed < chrono::Duration::zero()
        || elapsed >= chrono::Duration::seconds(minimum_interval_seconds as i64)
}

fn valid_tailnet_status(status: &str) -> bool {
    matches!(
        status,
        "connected"
            | "disconnected"
            | "login_required"
            | "machine_approval_required"
            | "starting"
            | "unknown"
            | "cli_unavailable"
            | "probe_denied"
            | "local_api_unavailable"
            | "probe_timeout"
    )
}

fn valid_current_status(status: &Status) -> bool {
    status.schema_version == 4
        && status.kind == "groundline-insights-owner-auto-status"
        && !status.last_result_code.is_empty()
        && parse_timestamp(&status.last_check_utc).is_ok()
        && status
            .last_success_utc
            .as_deref()
            .is_none_or(|value| parse_timestamp(value).is_ok())
        && status
            .last_collected_through_utc
            .as_deref()
            .is_none_or(|value| parse_timestamp(value).is_ok())
        && valid_tailnet_status(&status.tailnet_status)
        && valid_status_trigger(&status.last_trigger)
}

fn read_stored_status(path: &Path) -> Result<Status, StateError> {
    let bytes = read_bytes(path, MAX_STATE_BYTES, true)?;
    if let Ok(status) = serde_json::from_slice::<Status>(&bytes) {
        return if valid_current_status(&status) {
            Ok(status)
        } else {
            Err(StateError::LocalState)
        };
    }
    let imported: ImportedPrivateStatusV3 =
        serde_json::from_slice(&bytes).map_err(|_| StateError::LocalState)?;
    let ImportedPrivateStatusV3 {
        schema_version,
        kind,
        cron_or_timer_created,
        global_hook_created,
        last_attempt_result_code,
        last_attempt_utc,
        last_check_result_code,
        last_check_utc,
        last_collected_through_utc,
        last_native_lifecycle_receipt,
        last_network_failure,
        last_staged_through_utc,
        last_success_utc,
        last_tailnet_check_utc,
        last_tailnet_notification_utc,
        mcp_lifecycle_enabled,
        next_attempt_after_utc,
        private_paths_recorded,
        raw_content_recorded,
        retry_reason_code,
        secret_value_recorded,
        tailnet_cli_available,
        tailnet_connected,
        tailnet_health,
        tailnet_notification_result,
        tailnet_status,
        trigger,
        uploaded_count,
    } = imported;
    let optional_timestamps_valid = [
        last_staged_through_utc.as_deref(),
        last_tailnet_notification_utc.as_deref(),
        next_attempt_after_utc.as_deref(),
    ]
    .into_iter()
    .flatten()
    .all(|value| parse_timestamp(value).is_ok());
    let optional_codes_valid = [
        retry_reason_code.as_deref(),
        tailnet_notification_result.as_deref(),
    ]
    .into_iter()
    .flatten()
    .all(|value| !value.is_empty());
    if schema_version != 3
        || kind != "groundline-insights-owner-auto-status"
        || cron_or_timer_created
        || global_hook_created
        || mcp_lifecycle_enabled
        || private_paths_recorded
        || raw_content_recorded
        || secret_value_recorded
        || last_attempt_result_code.is_empty()
        || last_check_result_code.is_empty()
        || last_network_failure.is_empty()
        || !last_native_lifecycle_receipt.is_object()
        || parse_timestamp(&last_attempt_utc).is_err()
        || parse_timestamp(&last_check_utc).is_err()
        || parse_timestamp(&last_collected_through_utc).is_err()
        || parse_timestamp(&last_success_utc).is_err()
        || parse_timestamp(&last_tailnet_check_utc).is_err()
        || !optional_timestamps_valid
        || !optional_codes_valid
        || !matches!(tailnet_health.as_str(), "ok" | "degraded" | "unknown")
        || !valid_tailnet_status(&tailnet_status)
        || !valid_status_trigger(&trigger)
    {
        return Err(StateError::LocalState);
    }
    let _bounded_probe_state = (tailnet_cli_available, tailnet_connected);
    Ok(Status {
        schema_version: 4,
        kind,
        enabled: true,
        last_result_code: last_attempt_result_code,
        last_check_utc,
        last_success_utc: Some(last_success_utc),
        last_collected_through_utc: Some(last_collected_through_utc),
        uploaded_count,
        pending_event_count: 0,
        tailnet_status,
        last_trigger: trigger,
    })
}

fn previous_status(directory: &Path) -> Option<Status> {
    read_stored_status(&directory.join(STATUS_FILE)).ok()
}

fn write_status(directory: &Path, status: &Status) -> Result<(), StateError> {
    write_json(&directory.join(STATUS_FILE), status)
}

pub fn enable(codex_home: &Path) -> Result<Value, StateError> {
    let directory = state_directory(codex_home);
    let now = Utc::now();
    load_profile(codex_home)?;
    enrollment_token(codex_home).map_err(|_| StateError::InvalidProfile)?;
    grant_consent(&directory, now)?;
    initialize(&directory, now)?;
    set_policy(&directory, true, now)?;
    Ok(json!({
        "status":"PASS","enabled":true,"mutation_performed":true,
        "consent_status":"active",
    }))
}

pub fn disable(codex_home: &Path) -> Result<Value, StateError> {
    let directory = state_directory(codex_home);
    set_policy(&directory, false, Utc::now())?;
    Ok(json!({"status":"PASS","disabled":true,"mutation_performed":true}))
}

fn current_status(directory: &Path) -> Result<Option<Status>, StateError> {
    let path = directory.join(STATUS_FILE);
    if !path.exists() {
        return Ok(None);
    }
    read_stored_status(&path).map(Some)
}

fn status_with_tailnet_at(
    codex_home: &Path,
    tailnet: Value,
    now: DateTime<Utc>,
) -> Result<Value, StateError> {
    let directory = state_directory(codex_home);
    let policy_configured = tailnet::is_regular_file(&directory.join(POLICY_FILE));
    let enabled = policy_enabled(&directory)?;
    let outbox = pending_events(&directory, 0)?;
    let pending = outbox.observed_count as u64;
    let retry = read_delivery_retry(&directory)?;
    let consent_status = consent_status(&directory)?;
    let (quarantined, quarantine_capacity_exceeded) = quarantined_event_count(&directory)?;
    let previous = current_status(&directory)?;
    let profile_present = tailnet::is_regular_file(&codex_home.join(PROFILE_PATH));
    let profile_configured = load_profile(codex_home).is_ok();
    let credential_present = tailnet::is_regular_file(&codex_home.join(ENROLLMENT_TOKEN_PATH));
    let credential_valid = enrollment_token(codex_home).is_ok();
    let tailnet_connected = tailnet.get("tailnet_connected").and_then(Value::as_bool);
    let last_result_code = previous
        .as_ref()
        .map(|value| value.last_result_code.as_str());
    let last_success_utc = previous
        .as_ref()
        .and_then(|value| value.last_success_utc.as_deref());
    let last_success_at = last_success_utc.and_then(|value| parse_timestamp(value).ok());
    let collection_stale =
        last_success_at.is_some_and(|value| now - value > COLLECTION_STALE_AFTER);
    let collection_clock_skew = last_success_at.is_some_and(|value| value - now > MAX_CLOCK_SKEW);
    let ready_to_collect = enabled
        && profile_configured
        && credential_valid
        && consent_status == "active"
        && tailnet_connected == Some(true)
        && retry.as_ref().is_none_or(|value| !value.operator_required);
    let mut blocking_reason_codes = Vec::new();
    let (overall_status, collection_state) = if !enabled {
        ("PASS", "disabled")
    } else if !profile_configured {
        blocking_reason_codes.push(if profile_present {
            "invalid_owner_profile"
        } else {
            "owner_profile_required"
        });
        ("WARN", "configuration_required")
    } else if !credential_valid {
        blocking_reason_codes.push(if credential_present {
            "invalid_enrollment_credential"
        } else {
            "enrollment_credential_required"
        });
        ("WARN", "configuration_required")
    } else if consent_status != "active" {
        blocking_reason_codes.push("reconsent_required");
        ("WARN", "reconsent_required")
    } else if tailnet_connected == Some(false) {
        blocking_reason_codes.push("tailnet_not_connected");
        ("WARN", "tailnet_disconnected")
    } else if tailnet_connected.is_none() {
        blocking_reason_codes.push("tailnet_connection_unverified");
        ("WARN", "tailnet_unverified")
    } else if outbox.capacity_exceeded {
        blocking_reason_codes.push("outbox_capacity_exceeded");
        ("WARN", "outbox_capacity_exceeded")
    } else if retry.as_ref().is_some_and(|value| value.operator_required) {
        blocking_reason_codes.push("delivery_operator_action_required");
        ("WARN", "delivery_operator_action_required")
    } else if pending > 0 {
        blocking_reason_codes.push("delivery_pending");
        ("WARN", "delivery_pending")
    } else if collection_clock_skew {
        blocking_reason_codes.push("collection_clock_skew");
        ("WARN", "clock_skew")
    } else if collection_stale {
        blocking_reason_codes.push("collection_stale");
        ("WARN", "stale")
    } else if last_success_utc.is_none() {
        blocking_reason_codes.push("first_collection_pending");
        ("WARN", "awaiting_first_collection")
    } else if last_result_code != Some("pass") {
        blocking_reason_codes.push("retry_required");
        ("WARN", "retry_required")
    } else {
        ("PASS", "active")
    };
    Ok(json!({
        "kind":"groundline-insights-worker-status","schema":2,"status":overall_status,
        "collection_state":collection_state,"ready_to_collect":ready_to_collect,"collection_stale":collection_stale,"blocking_reason_codes":blocking_reason_codes,
        "policy_configured":policy_configured,"collection_enabled":enabled,
        "owner_profile_present":profile_present,"owner_profile_configured":profile_configured,
        "enrollment_credential_present":credential_present,"enrollment_credential_valid":credential_valid,
        "identity_present":directory.join(IDENTITY_FILE).is_file(),
        "consent_status":consent_status,"collector_token_present":directory.join(TOKEN_FILE).is_file(),
        "pending_event_count":pending,"pending_event_bytes":outbox.observed_bytes,
        "outbox_capacity_exceeded":outbox.capacity_exceeded,
        "quarantined_event_count":quarantined,
        "quarantine_capacity_exceeded":quarantine_capacity_exceeded,
        "delivery_attempt_count":retry.as_ref().map(|value| value.attempt_count).unwrap_or(0),
        "delivery_next_attempt_utc":retry.as_ref().and_then(|value| value.next_attempt_utc.as_deref()),
        "delivery_operator_required":retry.as_ref().is_some_and(|value| value.operator_required),
        "last_check_result_code":last_result_code,
        "last_check_utc":previous.as_ref().map(|value| value.last_check_utc.as_str()),"last_success_utc":last_success_utc,
        "last_collected_through_utc":previous.as_ref().and_then(|value| value.last_collected_through_utc.as_deref()),
        "tailnet":tailnet,"raw_content_emitted":false,"private_paths_emitted":false,"secret_value_printed":false,
    }))
}

fn status_with_tailnet(codex_home: &Path, tailnet: Value) -> Result<Value, StateError> {
    status_with_tailnet_at(codex_home, tailnet, Utc::now())
}

pub fn status(codex_home: &Path) -> Result<Value, StateError> {
    status_with_tailnet(codex_home, tailnet::probe())
}

pub fn checkpoint_enabled(codex_home: &Path) -> Result<bool, StateError> {
    policy_enabled(&state_directory(codex_home))
}

struct StatusUpdate<'a> {
    result_code: &'a str,
    collected_through_utc: Option<String>,
    uploaded_count: u64,
    pending_event_count: u64,
    tailnet_status: String,
    trigger: &'a str,
    successful: bool,
}

fn persist_cycle_status(
    directory: &Path,
    previous: Option<&Status>,
    now: DateTime<Utc>,
    update: StatusUpdate<'_>,
) -> Result<(), StateError> {
    write_status(
        directory,
        &Status {
            schema_version: 4,
            kind: "groundline-insights-owner-auto-status".to_owned(),
            enabled: true,
            last_result_code: update.result_code.to_owned(),
            last_check_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            last_success_utc: if update.successful {
                Some(now.to_rfc3339_opts(SecondsFormat::Millis, true))
            } else {
                previous.and_then(|value| value.last_success_utc.clone())
            },
            last_collected_through_utc: update.collected_through_utc,
            uploaded_count: update.uploaded_count,
            pending_event_count: update.pending_event_count,
            tailnet_status: update.tailnet_status,
            last_trigger: update.trigger.to_owned(),
        },
    )
}

pub async fn run_once(
    _plugin_root: &Path,
    codex_home: &Path,
    trigger: &str,
) -> Result<Value, StateError> {
    if !valid_status_trigger(trigger) {
        return Err(StateError::LocalState);
    }
    let directory = state_directory(codex_home);
    if !policy_enabled(&directory)? {
        return Err(StateError::Disabled);
    }
    let profile = load_profile(codex_home)?;
    let _lock = CycleLock::acquire(&directory)?;
    crate::checkpoint::claim_triggers(codex_home).map_err(|_| StateError::LocalState)?;
    let now = Utc::now();
    let previous = previous_status(&directory);
    let mut outbox = pending_events(&directory, UPLOAD_BATCH_EVENTS)?;
    let retry = read_delivery_retry(&directory)?;
    let collection_due = collection_is_due(
        trigger,
        previous.as_ref(),
        now,
        profile.checkpoint_min_interval_seconds,
    );
    let delivery_due = outbox.observed_count > 0 && delivery_is_due(trigger, retry.as_ref(), now);
    if operator_retry_blocked(trigger, retry.as_ref()) {
        crate::checkpoint::acknowledge_claimed_triggers(codex_home)
            .map_err(|_| StateError::LocalState)?;
        return Ok(json!({
            "status":"WARN","result_code":"delivery_operator_action_required","uploaded_count":0,
            "pending_event_count":outbox.observed_count,"delivery_next_attempt_utc":Value::Null,
            "last_collected_through_utc":previous.as_ref().and_then(|value| value.last_collected_through_utc.as_deref()),
            "network_performed":false,"mutation_performed":false,
            "raw_content_emitted":false,"private_paths_emitted":false,"secret_value_printed":false,
        }));
    }
    if !collection_due && !delivery_due {
        crate::checkpoint::acknowledge_claimed_triggers(codex_home)
            .map_err(|_| StateError::LocalState)?;
        return Ok(json!({
            "status":"PASS","result_code":"not_due","uploaded_count":0,
            "pending_event_count":outbox.observed_count,
            "delivery_next_attempt_utc":retry.as_ref().and_then(|value| value.next_attempt_utc.as_deref()),
            "last_collected_through_utc":previous.as_ref().and_then(|value| value.last_collected_through_utc.as_deref()),
            "network_performed":false,"mutation_performed":false,
            "raw_content_emitted":false,"private_paths_emitted":false,"secret_value_printed":false,
        }));
    }
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
        let error = StateError::TailnetDisconnected;
        record_delivery_retry(&directory, now, &error)?;
        persist_cycle_status(
            &directory,
            previous.as_ref(),
            now,
            StatusUpdate {
                result_code: "tailnet_not_connected",
                collected_through_utc: previous
                    .as_ref()
                    .and_then(|value| value.last_collected_through_utc.clone()),
                uploaded_count: 0,
                pending_event_count: outbox.observed_count as u64,
                tailnet_status,
                trigger,
                successful: false,
            },
        )?;
        return Err(StateError::TailnetDisconnected);
    }
    let token = match enroll(&profile, codex_home, &directory, &identity).await {
        Ok(token) => token,
        Err(error) => {
            record_delivery_retry(&directory, now, &error)?;
            persist_cycle_status(
                &directory,
                previous.as_ref(),
                now,
                StatusUpdate {
                    result_code: &error.to_string(),
                    collected_through_utc: previous
                        .as_ref()
                        .and_then(|value| value.last_collected_through_utc.clone()),
                    uploaded_count: 0,
                    pending_event_count: outbox.observed_count as u64,
                    tailnet_status,
                    trigger,
                    successful: false,
                },
            )?;
            return Err(error);
        }
    };
    let mut collected_through = previous
        .as_ref()
        .and_then(|value| value.last_collected_through_utc.clone());
    let collection_deferred = outbox.observed_count > 0
        || outbox.capacity_exceeded
        || outbox.observed_count >= OUTBOX_HIGH_WATERMARK_EVENTS
        || retry.as_ref().is_some_and(|value| value.operator_required);
    if collection_due && !collection_deferred {
        let start = collected_through
            .as_deref()
            .and_then(|value| parse_timestamp(value).ok())
            .or_else(|| earliest_recency(codex_home).ok().flatten())
            .unwrap_or_else(|| now - chrono::Duration::days(7));
        let start = start.min(now - chrono::Duration::seconds(1));
        let audit = match collect_audit(
            codex_home,
            start,
            now,
            Some(identity.runtime_family.as_str()),
            false,
        ) {
            Ok(audit) => audit,
            Err(_) => {
                let error = StateError::AuditFailed;
                record_delivery_retry(&directory, now, &error)?;
                persist_cycle_status(
                    &directory,
                    previous.as_ref(),
                    now,
                    StatusUpdate {
                        result_code: "audit_failed",
                        collected_through_utc: collected_through.clone(),
                        uploaded_count: 0,
                        pending_event_count: outbox.observed_count as u64,
                        tailnet_status,
                        trigger,
                        successful: false,
                    },
                )?;
                return Err(error);
            }
        };
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
        collected_through = Some(now.to_rfc3339_opts(SecondsFormat::Millis, true));
        outbox = pending_events(&directory, UPLOAD_BATCH_EVENTS)?;
    }
    let uploaded = if outbox.observed_count > 0 && delivery_is_due(trigger, retry.as_ref(), now) {
        let result = tokio::time::timeout(
            UPLOAD_CYCLE_TIMEOUT,
            upload(&profile, &identity, &token, outbox.batch),
        )
        .await
        .unwrap_or(Err(StateError::UploadFailed));
        match result {
            Ok(receipt) => {
                collected_through =
                    latest_timestamp(collected_through, receipt.collected_through_utc.clone());
                persist_cycle_status(
                    &directory,
                    previous.as_ref(),
                    now,
                    StatusUpdate {
                        result_code: "delivery_acknowledged",
                        collected_through_utc: collected_through.clone(),
                        uploaded_count: receipt.uploaded_count,
                        pending_event_count: outbox.observed_count as u64,
                        tailnet_status: tailnet_status.clone(),
                        trigger,
                        successful: false,
                    },
                )?;
                remove_acknowledged_events(&receipt)?;
                receipt.uploaded_count
            }
            Err(error) => {
                record_delivery_retry(&directory, now, &error)?;
                let remaining = pending_events(&directory, 0)?;
                persist_cycle_status(
                    &directory,
                    previous.as_ref(),
                    now,
                    StatusUpdate {
                        result_code: &error.to_string(),
                        collected_through_utc: collected_through.clone(),
                        uploaded_count: 0,
                        pending_event_count: remaining.observed_count as u64,
                        tailnet_status,
                        trigger,
                        successful: false,
                    },
                )?;
                return Err(error);
            }
        }
    } else {
        0
    };
    let remaining = pending_events(&directory, 0)?;
    let result_code = if remaining.observed_count > 0 {
        schedule_remaining_delivery(&directory, now)?;
        if remaining.capacity_exceeded {
            "outbox_capacity_exceeded"
        } else {
            "delivery_pending"
        }
    } else {
        clear_delivery_retry(&directory)?;
        "pass"
    };
    persist_cycle_status(
        &directory,
        previous.as_ref(),
        now,
        StatusUpdate {
            result_code,
            collected_through_utc: collected_through.clone(),
            uploaded_count: uploaded,
            pending_event_count: remaining.observed_count as u64,
            tailnet_status,
            trigger,
            successful: result_code == "pass",
        },
    )?;
    crate::checkpoint::acknowledge_claimed_triggers(codex_home)
        .map_err(|_| StateError::LocalState)?;
    Ok(json!({
        "status":if result_code == "pass" {"PASS"} else {"WARN"},"result_code":result_code,
        "uploaded_count":uploaded,"pending_event_count":remaining.observed_count,
        "outbox_capacity_exceeded":remaining.capacity_exceeded,
        "last_collected_through_utc":collected_through,
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
    use chrono::{DateTime, Utc};
    use serde_json::json;
    use tempfile::tempdir;

    use crate::local_file::{open_bounded_regular_file, private_for_current_user};

    use super::{
        CONSENT_FILE, DeliveryRetry, ENROLLMENT_TOKEN_PATH, MAX_OUTBOX_EVENTS, OUTBOX_DIR,
        POLICY_FILE, PROFILE_PATH, QUARANTINE_DIR, STATUS_FILE, StateError, Status,
        classify_response_status, collection_is_due, configure_profile, delivery_is_due, enable,
        explicit_operator_retry, initialize, latest_timestamp, operator_retry_blocked,
        pending_events, policy_enabled, record_delivery_retry, set_policy, state_directory,
        status_with_tailnet, status_with_tailnet_at, validate_upload_response, write_json,
        write_status,
    };

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

    #[test]
    fn collection_is_disabled_until_owner_explicitly_enables_it() {
        let home = tempdir().expect("temporary Codex home");
        let result = status_with_tailnet(
            home.path(),
            json!({"tailnet_connected":true,"tailnet_status":"connected"}),
        )
        .expect("status");
        assert_eq!(result["status"], "PASS");
        assert_eq!(result["collection_state"], "disabled");
        assert_eq!(result["collection_enabled"], false);
        assert_eq!(result["policy_configured"], false);
        assert_eq!(result["ready_to_collect"], false);
        assert_eq!(
            enable(home.path()).unwrap_err().to_string(),
            "invalid_owner_profile"
        );
    }

    #[test]
    fn configured_enablement_reports_first_collection_pending() {
        let home = tempdir().expect("temporary Codex home");
        configure_profile(home.path(), &profile("")).expect("configure profile");
        enable(home.path()).expect("enable collection");
        let result = status_with_tailnet(
            home.path(),
            json!({"tailnet_connected":true,"tailnet_status":"connected"}),
        )
        .expect("status");
        assert_eq!(result["status"], "WARN");
        assert_eq!(result["collection_state"], "awaiting_first_collection");
        assert_eq!(result["ready_to_collect"], true);
        assert_eq!(
            result["blocking_reason_codes"],
            json!(["first_collection_pending"])
        );
    }

    #[test]
    fn enabled_but_incomplete_configuration_is_not_reported_as_pass() {
        let home = tempdir().expect("temporary Codex home");
        let directory = state_directory(home.path());
        set_policy(&directory, true, Utc::now()).expect("write policy fixture");
        let result = status_with_tailnet(
            home.path(),
            json!({"tailnet_connected":true,"tailnet_status":"connected"}),
        )
        .expect("status");
        assert_eq!(result["status"], "WARN");
        assert_eq!(result["collection_state"], "configuration_required");
        assert_eq!(
            result["blocking_reason_codes"],
            json!(["owner_profile_required"])
        );
    }

    #[test]
    fn malformed_policy_and_outbox_state_are_not_silently_ignored() {
        let home = tempdir().expect("temporary Codex home");
        let directory = state_directory(home.path());
        write_json(
            &directory.join(POLICY_FILE),
            &json!({
                "schema_version":1,
                "kind":"groundline-insights-owner-auto-policy",
                "status":"active",
                "automatic_activity_checkpoints":false,
                "collection_scope":"all_activity",
                "diagnostic_enabled":false,
                "trigger_mode":"native_hook_checkpoints",
                "updated_at_utc":Utc::now().to_rfc3339(),
            }),
        )
        .expect("write malformed policy");
        assert!(policy_enabled(&directory).is_err());

        set_policy(&directory, false, Utc::now()).expect("replace valid policy");
        write_json(
            &directory.join(OUTBOX_DIR).join("unexpected.txt"),
            &json!({"x":1}),
        )
        .expect("write invalid outbox fixture");
        assert!(
            status_with_tailnet(
                home.path(),
                json!({"tailnet_connected":true,"tailnet_status":"connected"}),
            )
            .is_err()
        );
    }

    #[test]
    fn imports_only_the_exact_previous_private_policy_and_status_contracts() {
        let home = tempdir().expect("temporary Codex home");
        let directory = state_directory(home.path());
        write_json(
            &directory.join(POLICY_FILE),
            &json!({
                "schema_version":1,
                "kind":"groundline-insights-owner-auto-policy",
                "status":"active",
                "automatic_activity_checkpoints":true,
                "collection_scope":"all_activity",
                "diagnostic_enabled":false,
                "trigger_mode":"native_hook_checkpoints",
                "accepted_at_utc":"2026-08-28T00:00:00Z",
                "basic_receipt_id":"10000000-0000-4000-8000-000000000001",
                "chronicle_access_enabled":false,
                "collection_generation":1,
                "cron_or_timer_created":false,
                "global_hook_created":false,
                "grant_source":"private_plugin_install",
                "mcp_lifecycle_enabled":false,
                "policy_id":"20000000-0000-4000-8000-000000000002",
            }),
        )
        .expect("write imported policy");
        write_json(
            &directory.join(STATUS_FILE),
            &json!({
                "schema_version":3,
                "kind":"groundline-insights-owner-auto-status",
                "cron_or_timer_created":false,
                "global_hook_created":false,
                "last_attempt_result_code":"pass",
                "last_attempt_utc":"2026-08-28T00:00:00Z",
                "last_check_result_code":"not_due",
                "last_check_utc":"2026-08-28T00:00:00Z",
                "last_collected_through_utc":"2026-08-28T00:00:00Z",
                "last_native_lifecycle_receipt":{},
                "last_network_failure":"none",
                "last_staged_through_utc":null,
                "last_success_utc":"2026-08-28T00:00:00Z",
                "last_tailnet_check_utc":"2026-08-28T00:00:00Z",
                "last_tailnet_notification_utc":null,
                "mcp_lifecycle_enabled":false,
                "next_attempt_after_utc":null,
                "private_paths_recorded":false,
                "raw_content_recorded":false,
                "retry_reason_code":null,
                "secret_value_recorded":false,
                "tailnet_cli_available":true,
                "tailnet_connected":true,
                "tailnet_health":"ok",
                "tailnet_notification_result":null,
                "tailnet_status":"connected",
                "trigger":"session_end_hook",
                "uploaded_count":1,
            }),
        )
        .expect("write imported status");
        let result = status_with_tailnet(
            home.path(),
            json!({"tailnet_connected":true,"tailnet_status":"connected"}),
        )
        .expect("imported status");
        assert_eq!(result["collection_enabled"], true);
        assert_eq!(result["status"], "WARN");
        assert_eq!(result["collection_state"], "configuration_required");
        assert_eq!(result["last_check_result_code"], "pass");
        assert_eq!(result["last_success_utc"], "2026-08-28T00:00:00Z");
    }

    #[test]
    fn status_surfaces_stale_and_future_collection_timestamps() {
        let home = tempdir().expect("temporary Codex home");
        configure_profile(home.path(), &profile("")).expect("configure profile");
        enable(home.path()).expect("enable collection");
        let directory = state_directory(home.path());
        let now = DateTime::parse_from_rfc3339("2026-08-31T00:00:00Z")
            .expect("fixed now")
            .with_timezone(&Utc);
        let mut stored = Status {
            schema_version: 4,
            kind: "groundline-insights-owner-auto-status".to_owned(),
            enabled: true,
            last_result_code: "pass".to_owned(),
            last_check_utc: "2026-08-20T00:00:00Z".to_owned(),
            last_success_utc: Some("2026-08-20T00:00:00Z".to_owned()),
            last_collected_through_utc: Some("2026-08-20T00:00:00Z".to_owned()),
            uploaded_count: 1,
            pending_event_count: 0,
            tailnet_status: "connected".to_owned(),
            last_trigger: "session_end_hook".to_owned(),
        };
        write_status(&directory, &stored).expect("write stale status");
        let stale = status_with_tailnet_at(
            home.path(),
            json!({"tailnet_connected":true,"tailnet_status":"connected"}),
            now,
        )
        .expect("stale status");
        assert_eq!(stale["status"], "WARN");
        assert_eq!(stale["collection_state"], "stale");
        assert_eq!(stale["collection_stale"], true);

        stored.last_check_utc = "2026-08-31T00:10:00Z".to_owned();
        stored.last_success_utc = Some("2026-08-31T00:10:00Z".to_owned());
        stored.last_collected_through_utc = Some("2026-08-31T00:10:00Z".to_owned());
        write_status(&directory, &stored).expect("write future status");
        let future = status_with_tailnet_at(
            home.path(),
            json!({"tailnet_connected":true,"tailnet_status":"connected"}),
            now,
        )
        .expect("future status");
        assert_eq!(future["status"], "WARN");
        assert_eq!(future["collection_state"], "clock_skew");
        assert_eq!(
            future["blocking_reason_codes"],
            json!(["collection_clock_skew"])
        );
    }

    #[test]
    fn collection_cadence_and_delivery_retry_are_independent() {
        let now = DateTime::parse_from_rfc3339("2026-08-31T00:10:00Z")
            .expect("fixed now")
            .with_timezone(&Utc);
        let status = Status {
            schema_version: 4,
            kind: "groundline-insights-owner-auto-status".to_owned(),
            enabled: true,
            last_result_code: "pass".to_owned(),
            last_check_utc: "2026-08-31T00:00:01Z".to_owned(),
            last_success_utc: Some("2026-08-31T00:00:01Z".to_owned()),
            last_collected_through_utc: Some("2026-08-31T00:00:01Z".to_owned()),
            uploaded_count: 1,
            pending_event_count: 0,
            tailnet_status: "connected".to_owned(),
            last_trigger: "session_end_hook".to_owned(),
        };

        assert!(!collection_is_due(
            "session_end_hook",
            Some(&status),
            now,
            900
        ));
        assert!(collection_is_due("manual", Some(&status), now, 900));
        assert!(collection_is_due("history_sync", Some(&status), now, 900));
        assert!(collection_is_due(
            "session_end_hook",
            Some(&Status {
                last_check_utc: "2026-08-31T00:20:00Z".to_owned(),
                ..status
            }),
            now,
            900
        ));

        let retry = DeliveryRetry {
            schema_version: 1,
            kind: "groundline-insights-delivery-retry".to_owned(),
            attempt_count: 1,
            next_attempt_utc: Some("2026-08-31T00:11:00Z".to_owned()),
            last_error_code: "event_upload_failed".to_owned(),
            operator_required: false,
        };
        assert!(!delivery_is_due("session_end_hook", Some(&retry), now));
        assert!(delivery_is_due(
            "session_end_hook",
            Some(&retry),
            now + chrono::Duration::minutes(2)
        ));
        assert!(delivery_is_due("manual", Some(&retry), now));
    }

    #[test]
    fn legacy_no_network_consent_requires_explicit_reconsent_and_quarantines_pending_work() {
        let home = tempdir().expect("temporary Codex home");
        let directory = state_directory(home.path());
        let receipt = "10000000-0000-4000-8000-000000000001";
        configure_profile(home.path(), &profile("")).expect("configure profile");
        write_json(
            &directory.join(CONSENT_FILE),
            &json!({
                "schema_version":1,
                "kind":"groundline-insights-consent",
                "scope":"basic_weekly",
                "status":"active",
                "receipt_id":receipt,
                "accepted_at_utc":"2026-08-31T00:00:00Z",
                "diagnostic_enabled":false,
                "network_upload_enabled":false
            }),
        )
        .expect("legacy consent");
        write_json(
            &directory.join(OUTBOX_DIR).join("legacy.json"),
            &json!({"legacy":"pending"}),
        )
        .expect("legacy pending event");
        let original = std::fs::read(directory.join(CONSENT_FILE)).expect("legacy consent bytes");

        assert_eq!(
            initialize(
                &directory,
                DateTime::parse_from_rfc3339("2026-08-31T00:10:00Z")
                    .expect("fixed now")
                    .with_timezone(&Utc),
            )
            .unwrap_err()
            .to_string(),
            "reconsent_required"
        );
        assert_eq!(
            std::fs::read(directory.join(CONSENT_FILE)).expect("unchanged legacy consent"),
            original
        );
        assert!(!directory.join("identity.json").exists());

        enable(home.path()).expect("explicit reconsent");
        let (_, consent) = initialize(
            &directory,
            DateTime::parse_from_rfc3339("2026-08-31T00:10:00Z")
                .expect("fixed now")
                .with_timezone(&Utc),
        )
        .expect("active consent");
        assert_eq!(consent.schema_version, 2);
        assert!(consent.owner_service_upload_enabled);
        assert!(!consent.third_party_upload_enabled);
        assert_ne!(consent.receipt_id.to_string(), receipt);
        let stored: serde_json::Value = serde_json::from_slice(
            &std::fs::read(directory.join(CONSENT_FILE)).expect("stored consent"),
        )
        .expect("stored JSON");
        assert_eq!(stored["owner_service_upload_enabled"], true);
        assert_eq!(stored["third_party_upload_enabled"], false);
        assert!(stored.get("network_upload_enabled").is_none());
        assert!(directory.join("consent.legacy-v1.json").is_file());
        assert!(directory.join(QUARANTINE_DIR).join("legacy.json").is_file());
        assert!(!directory.join(OUTBOX_DIR).join("legacy.json").exists());
    }

    #[test]
    fn outbox_inventory_and_retry_state_are_bounded() {
        let home = tempdir().expect("temporary Codex home");
        let directory = state_directory(home.path());
        for index in 0..=MAX_OUTBOX_EVENTS {
            write_json(
                &directory.join(OUTBOX_DIR).join(format!("{index:04}.json")),
                &json!({"index":index}),
            )
            .expect("outbox fixture");
        }
        let inventory = pending_events(&directory, 0).expect("bounded inventory");
        assert!(inventory.capacity_exceeded);
        assert_eq!(inventory.observed_count, MAX_OUTBOX_EVENTS + 1);
        assert!(inventory.batch.is_empty());

        let now = DateTime::parse_from_rfc3339("2026-08-31T00:10:00Z")
            .expect("fixed now")
            .with_timezone(&Utc);
        let retry =
            record_delivery_retry(&directory, now, &StateError::UploadFailed).expect("retry state");
        assert_eq!(retry.attempt_count, 1);
        assert!(retry.next_attempt_utc.is_some());
        assert!(!retry.operator_required);
        let rejected = record_delivery_retry(&directory, now, &StateError::RemoteRejected)
            .expect("operator state");
        assert_eq!(rejected.attempt_count, 2);
        assert!(rejected.next_attempt_utc.is_none());
        assert!(rejected.operator_required);
    }

    #[test]
    fn upload_response_matrix_separates_retryable_and_operator_failures() {
        for status in [200, 202] {
            for outcome in ["accepted", "duplicate"] {
                assert!(
                    validate_upload_response(
                        reqwest::StatusCode::from_u16(status).unwrap(),
                        &json!({"status":"PASS","outcome":outcome}),
                    )
                    .is_ok()
                );
            }
        }
        for status in [408, 429, 500, 503] {
            assert_eq!(
                validate_upload_response(
                    reqwest::StatusCode::from_u16(status).unwrap(),
                    &json!({"status":"FAIL"}),
                )
                .unwrap_err()
                .to_string(),
                "event_upload_failed"
            );
        }
        for status in [400, 401, 403, 422] {
            assert_eq!(
                validate_upload_response(
                    reqwest::StatusCode::from_u16(status).unwrap(),
                    &json!({"status":"FAIL"}),
                )
                .unwrap_err()
                .to_string(),
                "remote_request_rejected"
            );
            assert_eq!(
                classify_response_status(reqwest::StatusCode::from_u16(status).unwrap())
                    .unwrap_err()
                    .to_string(),
                "remote_request_rejected"
            );
        }
        assert!(!explicit_operator_retry("session_end_hook"));
        assert!(explicit_operator_retry("manual"));
        assert!(explicit_operator_retry("history_sync"));
        let operator_retry = DeliveryRetry {
            schema_version: 1,
            kind: "groundline-insights-delivery-retry".to_owned(),
            attempt_count: 1,
            next_attempt_utc: None,
            last_error_code: "remote_request_rejected".to_owned(),
            operator_required: true,
        };
        assert!(operator_retry_blocked(
            "session_end_hook",
            Some(&operator_retry)
        ));
        assert!(!operator_retry_blocked("manual", Some(&operator_retry)));
        assert_eq!(
            latest_timestamp(
                Some("2026-08-31T00:00:00Z".to_owned()),
                Some("2026-08-31T00:01:00Z".to_owned())
            )
            .as_deref(),
            Some("2026-08-31T00:01:00Z")
        );
    }

    #[test]
    fn error_receipts_never_claim_unknown_mutation_state_is_false() {
        assert_eq!(StateError::InvalidProfile.mutation_performed(), Some(false));
        assert_eq!(
            StateError::TailnetDisconnected.mutation_performed(),
            Some(true)
        );
        assert_eq!(StateError::UploadFailed.mutation_performed(), None);
        assert_eq!(StateError::LocalState.mutation_performed(), None);
    }
}
