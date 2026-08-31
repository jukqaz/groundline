#![forbid(unsafe_code)]
#![recursion_limit = "512"]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::Json;
use axum::body::Body;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Router, serve};
use base64::Engine;
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use groundline_contracts::insights::{WeeklyReport, validate_basic_event_bytes};
use ipnet::Ipv4Net;
use reqwest::redirect::Policy;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;
use tracing::Level;
use url::Url;
use uuid::Uuid;

const API_VERSION: &str = "3";
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_CLICKHOUSE_BYTES: usize = 1024 * 1024;
const MAX_REQUESTS_PER_MINUTE: usize = 600;
const MAX_PRE_AUTH_REQUESTS_PER_MINUTE: usize = 120;
const MAX_PRE_AUTH_GLOBAL_REQUESTS_PER_MINUTE: usize = 2400;
const MAX_ENROLLMENTS_PER_MINUTE: usize = 60;
const MAX_HEALTH_REQUESTS_PER_MINUTE: usize = 120;
const HEALTH_CACHE_SECONDS: u64 = 5;
const MAX_CONCURRENT_COLLECTOR_STORAGE_REQUESTS: usize = 28;
const MAX_CONCURRENT_OPERATOR_STORAGE_REQUESTS: usize = 4;
const MAX_CONCURRENT_COLLECTOR_REQUESTS: usize = 64;
const REQUEST_BODY_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ACTIVE_RATE_SCOPES: usize = 4096;
const DEFAULT_RETENTION_DAYS: u64 = 365;
const DEFAULT_COLLECTOR_MAX_EVENTS: u64 = 4096;
const DEFAULT_COLLECTOR_MAX_PAYLOAD_BYTES: u64 = 256 * 1024 * 1024;
const DEFAULT_DATASET_MAX_ROWS: u64 = 2_000_000;
const DEFAULT_DATASET_MAX_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const DATASET_INGESTION_WATERMARK_PERCENT: u64 = 90;
const TOKEN_MIN_BYTES: usize = 32;
const REPORT_FRESHNESS_HOURS: u64 = 48;
const INITIAL_REPORT_GRACE_HOURS: u64 = 24;
const REPORT_DELIVERY_DELAY_HOURS: u64 = 6;
const REPORT_DELIVERY_OVERDUE_HOURS: u64 = 24;
const REPORT_CLOCK_SKEW_TOLERANCE_MINUTES: u64 = 5;
const REPORT_MINIMUM_EVENTS: u64 = 2;
const REPORT_MINIMUM_ROOTS: u64 = 5;
const STORAGE_MIGRATIONS: &[&str] = &[
    "CREATE DATABASE IF NOT EXISTS groundline",
    r#"CREATE TABLE IF NOT EXISTS groundline.basic_weekly
    (
        schema_version UInt16,
        event_id UUID,
        idempotency_key String,
        collector_id UUID,
        collection_generation UInt32 DEFAULT 0,
        collection_trigger LowCardinality(String) DEFAULT 'pre_checkpoint',
        received_at DateTime64(3, 'UTC'),
        groundline_version LowCardinality(String),
        os_family LowCardinality(String),
        runtime_family LowCardinality(String),
        execution_mode LowCardinality(String),
        period_start Nullable(DateTime64(3, 'UTC')),
        period_end Nullable(DateTime64(3, 'UTC')),
        generated_at DateTime64(3, 'UTC'),
        selection_mode LowCardinality(String),
        requested_days UInt32,
        root_count UInt32,
        observed_root_count UInt32 DEFAULT root_count,
        eligible_root_count UInt32 DEFAULT root_count,
        selected_root_count UInt32 DEFAULT root_count,
        root_truncated_count UInt32 DEFAULT 0,
        selection_coverage Nullable(Float64),
        selected_recency_start Nullable(DateTime64(3, 'UTC')),
        selected_recency_end Nullable(DateTime64(3, 'UTC')),
        minimum_root_count UInt32,
        sample_sufficient UInt8,
        delegated_count UInt32,
        guardian_count UInt32,
        guardian_incomplete_excluded_count UInt32 DEFAULT 0,
        unreadable_root_count UInt32,
        originator_unclassified_excluded_root_count UInt32,
        originator_source_fallback_root_count UInt32,
        truncated_count UInt32,
        capability_completed_root_coverage UInt8 DEFAULT 0,
        capability_latency_completed_count UInt8 DEFAULT 0,
        capability_root_boundary_counts UInt8 DEFAULT 0,
        capability_guardian_workspace_attribution UInt8 DEFAULT 0,
        root_status LowCardinality(String) DEFAULT 'UNKNOWN',
        delegated_status LowCardinality(String) DEFAULT 'UNKNOWN',
        guardian_status LowCardinality(String) DEFAULT 'UNKNOWN',
        model_families Array(LowCardinality(String)),
        efforts Array(LowCardinality(String)),
        model_effort_counts Array(UInt32),
        usage_source LowCardinality(String),
        delegated_usage_source LowCardinality(String) DEFAULT 'unknown',
        guardian_usage_source LowCardinality(String) DEFAULT 'unknown',
        input_tokens UInt64,
        cached_input_tokens UInt64,
        output_tokens UInt64,
        reasoning_output_tokens UInt64,
        total_tokens UInt64,
        non_cached_input_tokens UInt64,
        cached_input_ratio Nullable(Float64),
        cumulative_rollout_count UInt32,
        fallback_rollout_count UInt32,
        delegated_fallback_rollout_count UInt32 DEFAULT 0,
        guardian_fallback_rollout_count UInt32 DEFAULT 0,
        task_started UInt32,
        task_completed UInt32,
        turn_contexts UInt32,
        compactions UInt32,
        user_messages_with_text UInt32,
        latency_median_ms Nullable(Float64),
        latency_p90_ms Nullable(Float64),
        latency_max_ms Nullable(Float64),
        latency_completed_count UInt32 DEFAULT 0,
        long_turn_count UInt32,
        verification_tool_calls UInt32,
        verification_success_count UInt32 DEFAULT 0,
        verification_failure_count UInt32 DEFAULT 0,
        verification_unresolved_count UInt32 DEFAULT verification_tool_calls,
        tool_call_count UInt32,
        short_message_count UInt32,
        broad_scope_message_count UInt32,
        nonzero_exit_count UInt32,
        timeout_count UInt32,
        rejected_count UInt32,
        exact_repeated_call_groups UInt32,
        calls_in_exact_repeated_groups UInt32,
        boundary_review_recommended UInt8,
        long_lived_root_session UInt8,
        boundary_review_root_count UInt32 DEFAULT 0,
        long_lived_root_count UInt32 DEFAULT 0,
        delegated_total_tokens UInt64,
        guardian_total_tokens UInt64,
        guardian_review_count UInt32,
        guardian_workspace_attributed_review_count UInt32 DEFAULT 0,
        guardian_workspace_attribution_coverage Nullable(Float64),
        consent_receipt_id UUID,
        payload_json String CODEC(ZSTD(3))
    ) ENGINE = ReplacingMergeTree(received_at)
    PARTITION BY toYYYYMM(ifNull(period_end, generated_at))
    ORDER BY event_id
    SETTINGS index_granularity = 8192"#,
    r#"CREATE TABLE IF NOT EXISTS groundline.collectors
    (
        collector_id UUID,
        token_hash FixedString(64),
        current_generation UInt32 DEFAULT 0,
        enrollment_schema_version UInt8 DEFAULT 1,
        created_at DateTime64(3, 'UTC'),
        updated_at DateTime64(3, 'UTC'),
        revoked UInt8,
        os_family LowCardinality(String) DEFAULT 'unknown',
        runtime_family LowCardinality(String) DEFAULT 'unknown',
        execution_mode LowCardinality(String) DEFAULT 'unknown',
        groundline_version LowCardinality(String) DEFAULT 'unknown'
    ) ENGINE = ReplacingMergeTree(updated_at)
    ORDER BY collector_id
    SETTINGS index_granularity = 8192"#,
    r#"ALTER TABLE groundline.basic_weekly
    ADD COLUMN IF NOT EXISTS collection_generation UInt32 DEFAULT 0 AFTER collector_id,
    ADD COLUMN IF NOT EXISTS collection_trigger LowCardinality(String) DEFAULT 'pre_checkpoint',
    ADD COLUMN IF NOT EXISTS originator_unclassified_excluded_root_count UInt32,
    ADD COLUMN IF NOT EXISTS originator_source_fallback_root_count UInt32,
    ADD COLUMN IF NOT EXISTS observed_root_count UInt32 DEFAULT root_count,
    ADD COLUMN IF NOT EXISTS eligible_root_count UInt32 DEFAULT root_count,
    ADD COLUMN IF NOT EXISTS selected_root_count UInt32 DEFAULT root_count,
    ADD COLUMN IF NOT EXISTS root_truncated_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS selection_coverage Nullable(Float64),
    ADD COLUMN IF NOT EXISTS selected_recency_start Nullable(DateTime64(3, 'UTC')),
    ADD COLUMN IF NOT EXISTS selected_recency_end Nullable(DateTime64(3, 'UTC')),
    ADD COLUMN IF NOT EXISTS capability_completed_root_coverage UInt8 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS capability_latency_completed_count UInt8 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS capability_root_boundary_counts UInt8 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS capability_guardian_workspace_attribution UInt8 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS root_status LowCardinality(String) DEFAULT 'UNKNOWN',
    ADD COLUMN IF NOT EXISTS delegated_status LowCardinality(String) DEFAULT 'UNKNOWN',
    ADD COLUMN IF NOT EXISTS guardian_status LowCardinality(String) DEFAULT 'UNKNOWN',
    ADD COLUMN IF NOT EXISTS delegated_usage_source LowCardinality(String) DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS guardian_usage_source LowCardinality(String) DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS delegated_fallback_rollout_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS guardian_fallback_rollout_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS guardian_incomplete_excluded_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS user_messages_with_text UInt32,
    ADD COLUMN IF NOT EXISTS tool_call_count UInt32,
    ADD COLUMN IF NOT EXISTS short_message_count UInt32,
    ADD COLUMN IF NOT EXISTS broad_scope_message_count UInt32,
    ADD COLUMN IF NOT EXISTS verification_success_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS verification_failure_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS verification_unresolved_count UInt32 DEFAULT verification_tool_calls,
    ADD COLUMN IF NOT EXISTS latency_completed_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS boundary_review_root_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS long_lived_root_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS guardian_workspace_attributed_review_count UInt32 DEFAULT 0,
    ADD COLUMN IF NOT EXISTS guardian_workspace_attribution_coverage Nullable(Float64)"#,
    r#"ALTER TABLE groundline.collectors
    ADD COLUMN IF NOT EXISTS current_generation UInt32 DEFAULT 0 AFTER token_hash,
    ADD COLUMN IF NOT EXISTS enrollment_schema_version UInt8 DEFAULT 1,
    ADD COLUMN IF NOT EXISTS os_family LowCardinality(String) DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS runtime_family LowCardinality(String) DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS execution_mode LowCardinality(String) DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS groundline_version LowCardinality(String) DEFAULT 'unknown'"#,
    r#"ALTER TABLE groundline.basic_weekly
    MODIFY COLUMN requested_days UInt32,
    MODIFY COLUMN root_count UInt32,
    MODIFY COLUMN minimum_root_count UInt32,
    MODIFY COLUMN delegated_count UInt32,
    MODIFY COLUMN guardian_count UInt32,
    MODIFY COLUMN unreadable_root_count UInt32,
    MODIFY COLUMN truncated_count UInt32,
    MODIFY COLUMN cumulative_rollout_count UInt32,
    MODIFY COLUMN fallback_rollout_count UInt32"#,
];
static CRYPTO_PROVIDER: OnceLock<Result<(), ()>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{reason}")]
    Rejected {
        status: StatusCode,
        reason: &'static str,
    },
}

impl ApiError {
    fn new(status: StatusCode, reason: &'static str) -> Self {
        Self::Rejected { status, reason }
    }

    fn storage() -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let Self::Rejected { status, reason } = self;
        safe_response(status, json!({"status":"FAIL"}), reason)
    }
}

fn safe_response(status: StatusCode, mut payload: Value, reason: &'static str) -> Response {
    if let Some(object) = payload.as_object_mut() {
        object.insert("reason_code".to_owned(), Value::from(reason));
    }
    let encoded = serde_json::to_vec(&payload)
        .unwrap_or_else(|_| br#"{"status":"FAIL","reason_code":"invalid_request"}"#.to_vec());
    let mut response = Response::new(Body::from(encoded));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}

fn required_secret(name: &str) -> Result<SecretString, ApiError> {
    let value = std::env::var(name)
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid_request"))?;
    if value.len() < TOKEN_MIN_BYTES || value.len() > 4096 {
        return Err(ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid_request",
        ));
    }
    Ok(SecretString::from(value))
}

fn ensure_crypto_provider() -> Result<(), ApiError> {
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
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid_request"))
}

#[derive(Clone)]
struct Config {
    listen: SocketAddr,
    clickhouse_url: Url,
    clickhouse_database: String,
    clickhouse_user: String,
    clickhouse_password: SecretString,
    admin_token: SecretString,
    enrollment_token: SecretString,
    proxy_token: SecretString,
    owner_enrollment_enabled: bool,
    latest_version: String,
    minimum_supported_version: String,
    retention_days: u64,
    collector_max_events: u64,
    collector_max_payload_bytes: u64,
    dataset_max_rows: u64,
    dataset_max_bytes: u64,
}

fn bounded_env_u64(name: &str, default: u64, minimum: u64, maximum: u64) -> Result<u64, ApiError> {
    let value = std::env::var(name)
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid_request"))?
        .unwrap_or(default);
    if !(minimum..=maximum).contains(&value) {
        return Err(ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid_request",
        ));
    }
    Ok(value)
}

impl Config {
    fn from_env() -> Result<Self, ApiError> {
        let host = std::env::var("GROUNDLINE_LISTEN_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
        let port = std::env::var("GROUNDLINE_LISTEN_PORT")
            .unwrap_or_else(|_| "8080".to_owned())
            .parse::<u16>()
            .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid_request"))?;
        let listen = SocketAddr::new(
            host.parse::<IpAddr>()
                .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid_request"))?,
            port,
        );
        let mut clickhouse_url = Url::parse(
            &std::env::var("GROUNDLINE_CLICKHOUSE_URL")
                .unwrap_or_else(|_| "http://clickhouse:8123".to_owned()),
        )
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid_request"))?;
        if clickhouse_url.scheme() != "http"
            || clickhouse_url.host_str().is_none()
            || !clickhouse_url.username().is_empty()
            || clickhouse_url.password().is_some()
            || clickhouse_url.query().is_some()
            || clickhouse_url.fragment().is_some()
        {
            return Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid_request",
            ));
        }
        clickhouse_url.set_path("/");
        let latest_version = std::env::var("GROUNDLINE_VERSION")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned());
        let minimum_supported_version = std::env::var("GROUNDLINE_MINIMUM_SUPPORTED_VERSION")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned());
        let retention_days = bounded_env_u64(
            "GROUNDLINE_RETENTION_DAYS",
            DEFAULT_RETENTION_DAYS,
            90,
            3650,
        )?;
        let collector_max_events = bounded_env_u64(
            "GROUNDLINE_COLLECTOR_MAX_EVENTS",
            DEFAULT_COLLECTOR_MAX_EVENTS,
            128,
            1_000_000,
        )?;
        let collector_max_payload_bytes = bounded_env_u64(
            "GROUNDLINE_COLLECTOR_MAX_PAYLOAD_BYTES",
            DEFAULT_COLLECTOR_MAX_PAYLOAD_BYTES,
            8 * 1024 * 1024,
            64 * 1024 * 1024 * 1024,
        )?;
        let dataset_max_rows = bounded_env_u64(
            "GROUNDLINE_DATASET_MAX_ROWS",
            DEFAULT_DATASET_MAX_ROWS,
            4096,
            1_000_000_000,
        )?;
        let dataset_max_bytes = bounded_env_u64(
            "GROUNDLINE_DATASET_MAX_BYTES",
            DEFAULT_DATASET_MAX_BYTES,
            1024 * 1024 * 1024,
            16 * 1024 * 1024 * 1024 * 1024,
        )?;
        let clickhouse_database = std::env::var("GROUNDLINE_CLICKHOUSE_DATABASE")
            .unwrap_or_else(|_| "groundline".to_owned());
        if clickhouse_database != "groundline" {
            return Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid_request",
            ));
        }
        let latest = groundline_contracts::version::strict_version(&latest_version)
            .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid_request"))?;
        let minimum = groundline_contracts::version::strict_version(&minimum_supported_version)
            .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid_request"))?;
        if minimum > latest {
            return Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid_request",
            ));
        }
        Ok(Self {
            listen,
            clickhouse_url,
            clickhouse_database,
            clickhouse_user: std::env::var("GROUNDLINE_CLICKHOUSE_USER")
                .unwrap_or_else(|_| "groundline_ingest".to_owned()),
            clickhouse_password: required_secret("GROUNDLINE_CLICKHOUSE_PASSWORD")?,
            admin_token: required_secret("GROUNDLINE_ADMIN_TOKEN")?,
            enrollment_token: required_secret("GROUNDLINE_ENROLLMENT_TOKEN")?,
            proxy_token: required_secret("GROUNDLINE_PROXY_TOKEN")?,
            owner_enrollment_enabled: std::env::var("GROUNDLINE_OWNER_ENROLLMENT_ENABLED")
                .is_ok_and(|value| value.eq_ignore_ascii_case("true")),
            latest_version,
            minimum_supported_version,
            retention_days,
            collector_max_events,
            collector_max_payload_bytes,
            dataset_max_rows,
            dataset_max_bytes,
        })
    }
}

fn constant_time_equal(left: &str, right: &str) -> bool {
    left.len() == right.len() && bool::from(left.as_bytes().ct_eq(right.as_bytes()))
}

fn bearer(headers: &HeaderMap) -> &str {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
}

#[derive(Clone)]
struct ClickHouse {
    client: reqwest::Client,
    endpoint: Url,
    database: String,
    authorization: HeaderValue,
}

impl ClickHouse {
    fn new(config: &Config) -> Result<Self, ApiError> {
        ensure_crypto_provider()?;
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| ApiError::storage())?;
        let raw = format!(
            "{}:{}",
            config.clickhouse_user,
            config.clickhouse_password.expose_secret()
        );
        let mut authorization = HeaderValue::from_str(&format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(raw.as_bytes())
        ))
        .map_err(|_| ApiError::storage())?;
        authorization.set_sensitive(true);
        Ok(Self {
            client,
            endpoint: config.clickhouse_url.clone(),
            database: config.clickhouse_database.clone(),
            authorization,
        })
    }

    async fn request(
        &self,
        query: &str,
        parameters: &[(&str, String)],
        body: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, ApiError> {
        self.request_at(query, parameters, body, true).await
    }

    async fn request_at(
        &self,
        query: &str,
        parameters: &[(&str, String)],
        body: Option<Vec<u8>>,
        include_database: bool,
    ) -> Result<Vec<u8>, ApiError> {
        let mut url = self.endpoint.clone();
        {
            let mut pairs = url.query_pairs_mut();
            if include_database {
                pairs.append_pair("database", &self.database);
            }
            pairs.append_pair("query", query);
            pairs.append_pair("date_time_input_format", "best_effort");
            for (key, value) in parameters {
                pairs.append_pair(&format!("param_{key}"), value);
            }
        }
        let body = body.unwrap_or_default();
        let request = self
            .client
            .post(url)
            .header(header::AUTHORIZATION, self.authorization.clone())
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_LENGTH, body.len())
            .body(body);
        let response = request.send().await.map_err(|_| ApiError::storage())?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|length| length > MAX_CLICKHOUSE_BYTES as u64)
        {
            return Err(ApiError::storage());
        }
        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| ApiError::storage())? {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|length| length > MAX_CLICKHOUSE_BYTES)
            {
                return Err(ApiError::storage());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    async fn ready(&self) -> bool {
        self.request("SELECT 1 FORMAT TabSeparated", &[], None)
            .await
            .is_ok_and(|value| value == b"1\n" || value == b"1")
    }

    async fn ensure_storage(&self, config: &Config) -> Result<(), ApiError> {
        for (index, query) in STORAGE_MIGRATIONS.iter().enumerate() {
            self.request_at(query, &[], None, index != 0).await?;
        }
        self.request(
            &format!(
                "ALTER TABLE groundline.basic_weekly MODIFY TTL received_at + toIntervalDay({})",
                config.retention_days
            ),
            &[],
            None,
        )
        .await?;
        for query in [
            "CREATE OR REPLACE VIEW groundline.basic_active AS SELECT events.* FROM (SELECT * FROM groundline.basic_weekly FINAL) AS events INNER JOIN (SELECT collector_id, current_generation FROM groundline.collectors FINAL WHERE revoked = 0) AS active ON events.collector_id = active.collector_id AND events.collection_generation = active.current_generation",
            "CREATE TABLE IF NOT EXISTS groundline.release_policy (policy_key LowCardinality(String), latest_version String, minimum_supported_version String, updated_at DateTime64(3, 'UTC')) ENGINE = ReplacingMergeTree(updated_at) ORDER BY policy_key",
            "ALTER TABLE groundline.release_policy ADD COLUMN IF NOT EXISTS retention_days UInt16 DEFAULT 365",
        ] {
            self.request(query, &[], None).await?;
        }
        let policy = json!({
            "policy_key":"stable",
            "latest_version":config.latest_version,
            "minimum_supported_version":config.minimum_supported_version,
            "retention_days":config.retention_days,
            "updated_at":Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        });
        let mut body = serde_json::to_vec(&policy).map_err(|_| ApiError::storage())?;
        body.push(b'\n');
        self.request(
            "INSERT INTO groundline.release_policy FORMAT JSONEachRow",
            &[],
            Some(body),
        )
        .await?;
        Ok(())
    }

    async fn json_row(
        &self,
        query: &str,
        parameters: &[(&str, String)],
    ) -> Result<Option<Value>, ApiError> {
        let bytes = self.request(query, parameters, None).await?;
        let line = bytes
            .split(|value| *value == b'\n')
            .find(|line| !line.is_empty());
        match line {
            Some(line) => serde_json::from_slice(line)
                .map(Some)
                .map_err(|_| ApiError::storage()),
            None => Ok(None),
        }
    }

    async fn json_rows(
        &self,
        query: &str,
        parameters: &[(&str, String)],
    ) -> Result<Vec<Value>, ApiError> {
        let bytes = self.request(query, parameters, None).await?;
        bytes
            .split(|value| *value == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).map_err(|_| ApiError::storage()))
            .collect()
    }

    async fn collector(&self, collector_id: Uuid) -> Result<Option<Collector>, ApiError> {
        let query = "SELECT collector_id, token_hash, current_generation, created_at, enrollment_schema_version, os_family, runtime_family, execution_mode, groundline_version FROM groundline.collectors FINAL WHERE collector_id = {collector_id:UUID} AND revoked = 0 ORDER BY updated_at DESC LIMIT 1 FORMAT JSONEachRow";
        self.json_row(query, &[("collector_id", collector_id.to_string())])
            .await?
            .map(Collector::from_value)
            .transpose()
    }

    async fn write_collector(&self, collector: &Collector) -> Result<(), ApiError> {
        let mut body = serde_json::to_vec(&collector.as_row()).map_err(|_| ApiError::storage())?;
        body.push(b'\n');
        self.request(
            "INSERT INTO groundline.collectors FORMAT JSONEachRow",
            &[],
            Some(body),
        )
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
struct Collector {
    collector_id: Uuid,
    token_hash: String,
    current_generation: u32,
    enrollment_schema_version: u8,
    created_at: String,
    os_family: String,
    runtime_family: String,
    execution_mode: String,
    groundline_version: String,
    revoked: u8,
}

impl Collector {
    fn from_value(value: Value) -> Result<Self, ApiError> {
        let collector_id = value
            .get("collector_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .unwrap_or(Uuid::nil());
        let token_hash = value
            .get("token_hash")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let current_generation = value
            .get("current_generation")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok());
        let enrollment_schema_version = value
            .get("enrollment_schema_version")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok());
        if collector_id.is_nil()
            || token_hash.len() != 64
            || !token_hash.bytes().all(|value| value.is_ascii_hexdigit())
            || current_generation.is_none()
            || !matches!(enrollment_schema_version, Some(1 | 2))
        {
            return Err(ApiError::storage());
        }
        Ok(Self {
            collector_id,
            token_hash,
            current_generation: current_generation.expect("checked"),
            enrollment_schema_version: enrollment_schema_version.expect("checked"),
            created_at: value
                .get("created_at")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            os_family: value
                .get("os_family")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
            runtime_family: value
                .get("runtime_family")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
            execution_mode: value
                .get("execution_mode")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
            groundline_version: value
                .get("groundline_version")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
            revoked: 0,
        })
    }

    fn as_row(&self) -> Value {
        json!({
            "collector_id":self.collector_id,
            "token_hash":self.token_hash,
            "current_generation":self.current_generation,
            "enrollment_schema_version":self.enrollment_schema_version,
            "created_at":self.created_at,
            "updated_at":Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            "revoked":self.revoked,
            "os_family":self.os_family,
            "runtime_family":self.runtime_family,
            "execution_mode":self.execution_mode,
            "groundline_version":self.groundline_version,
        })
    }
}

#[derive(Clone)]
struct AppState {
    config: Config,
    clickhouse: ClickHouse,
    rate_limits: Arc<Mutex<BTreeMap<RateLimitScope, VecDeque<Instant>>>>,
    health: Arc<tokio::sync::Mutex<HealthState>>,
    health_probe: Arc<tokio::sync::Mutex<()>>,
    ingestion_gate: Arc<tokio::sync::Mutex<()>>,
    collector_request_permits: Arc<tokio::sync::Semaphore>,
    collector_storage_permits: Arc<tokio::sync::Semaphore>,
    operator_storage_permits: Arc<tokio::sync::Semaphore>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RateLimitScope {
    Admin,
    Collector(Uuid),
    Enrollment,
    PreAuthGlobal,
    PreAuthPeer(IpAddr),
}

#[derive(Clone, Copy)]
enum StorageClass {
    Collector,
    Operator,
}

#[derive(Default)]
struct HealthState {
    requests: VecDeque<Instant>,
    readiness: Option<(Instant, bool)>,
}

impl AppState {
    fn rate_limit(&self, scope: RateLimitScope, limit: usize) -> Result<(), ApiError> {
        let mut scopes = self
            .rate_limits
            .lock()
            .map_err(|_| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"))?;
        charge_rate_limit(&mut scopes, scope, limit, Instant::now())
    }

    fn storage_permit(
        &self,
        class: StorageClass,
    ) -> Result<tokio::sync::OwnedSemaphorePermit, ApiError> {
        let permits = match class {
            StorageClass::Collector => &self.collector_storage_permits,
            StorageClass::Operator => &self.operator_storage_permits,
        };
        permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "storage_busy"))
    }
}

fn charge_rate_limit(
    scopes: &mut BTreeMap<RateLimitScope, VecDeque<Instant>>,
    scope: RateLimitScope,
    limit: usize,
    now: Instant,
) -> Result<(), ApiError> {
    let threshold = now - Duration::from_secs(60);
    for requests in scopes.values_mut() {
        expire_requests(requests, threshold);
    }
    scopes.retain(|_, requests| !requests.is_empty());
    if !scopes.contains_key(&scope) && scopes.len() >= MAX_ACTIVE_RATE_SCOPES {
        return Err(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "rate_limited"));
    }
    let requests = scopes.entry(scope).or_default();
    if requests.len() >= limit {
        return Err(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "rate_limited"));
    }
    requests.push_back(now);
    Ok(())
}

fn expire_requests(requests: &mut VecDeque<Instant>, threshold: Instant) {
    while requests.front().is_some_and(|value| *value <= threshold) {
        requests.pop_front();
    }
}

fn tailnet_address(address: IpAddr) -> bool {
    address.is_loopback()
        || matches!(address, IpAddr::V4(value) if Ipv4Net::new(Ipv4Addr::new(100, 64, 0, 0), 10).expect("fixed network").contains(&value))
}

fn private_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(value) => value.is_private(),
        IpAddr::V6(value) => value.is_unique_local(),
    }
}

fn require_tailnet(
    state: &AppState,
    peer: SocketAddr,
    headers: &HeaderMap,
) -> Result<IpAddr, ApiError> {
    if tailnet_address(peer.ip()) {
        return Ok(peer.ip());
    }
    if !private_address(peer.ip())
        || !constant_time_equal(
            headers
                .get("x-groundline-proxy-token")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default(),
            state.config.proxy_token.expose_secret(),
        )
    {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "invalid_auth"));
    }
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.contains(','))
        .and_then(|value| value.parse::<IpAddr>().ok());
    if !forwarded.is_some_and(tailnet_address) {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "invalid_auth"));
    }
    Ok(forwarded.expect("validated forwarded Tailnet address"))
}

fn admit_tailnet(
    state: &AppState,
    peer: SocketAddr,
    headers: &HeaderMap,
) -> Result<IpAddr, ApiError> {
    let effective_peer = require_tailnet(state, peer, headers)?;
    state.rate_limit(
        RateLimitScope::PreAuthPeer(effective_peer),
        MAX_PRE_AUTH_REQUESTS_PER_MINUTE,
    )?;
    state.rate_limit(
        RateLimitScope::PreAuthGlobal,
        MAX_PRE_AUTH_GLOBAL_REQUESTS_PER_MINUTE,
    )?;
    Ok(effective_peer)
}

async fn admit_collector_request(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    admit_tailnet(&state, peer, request.headers())?;
    let _request_permit = state
        .collector_request_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "request_busy"))?;
    let (parts, body) = request.into_parts();
    let body = tokio::time::timeout(
        REQUEST_BODY_TIMEOUT,
        axum::body::to_bytes(body, MAX_REQUEST_BYTES),
    )
    .await
    .map_err(|_| ApiError::new(StatusCode::REQUEST_TIMEOUT, "request_timeout"))?
    .map_err(|_| ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large"))?;
    Ok(next.run(Request::from_parts(parts, Body::from(body))).await)
}

fn require_token(headers: &HeaderMap, expected: &SecretString) -> Result<(), ApiError> {
    if constant_time_equal(bearer(headers), expected.expose_secret()) {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::UNAUTHORIZED, "invalid_auth"))
    }
}

fn require_enrollment(config: &Config, headers: &HeaderMap) -> Result<(), ApiError> {
    require_token(headers, &config.enrollment_token)?;
    if !config.owner_enrollment_enabled {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "enrollment_disabled"));
    }
    Ok(())
}

fn require_admin_report(config: &Config, headers: &HeaderMap) -> Result<(), ApiError> {
    require_token(headers, &config.admin_token)
}

async fn require_collector_token(
    state: &AppState,
    headers: &HeaderMap,
    collector_id: Uuid,
) -> Result<Collector, ApiError> {
    let supplied = format!("{:x}", Sha256::digest(bearer(headers).as_bytes()));
    let collector = state
        .clickhouse
        .collector(collector_id)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "invalid_auth"))?;
    if !constant_time_equal(&supplied, &collector.token_hash) {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "invalid_auth"));
    }
    Ok(collector)
}

async fn health(State(state): State<AppState>) -> Result<Response, ApiError> {
    let cached = {
        let mut health = state.health.lock().await;
        let now = Instant::now();
        expire_requests(&mut health.requests, now - Duration::from_secs(60));
        if health.requests.len() >= MAX_HEALTH_REQUESTS_PER_MINUTE {
            return Err(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "rate_limited"));
        }
        health.requests.push_back(now);
        health
            .readiness
            .filter(|(checked_at, _)| {
                now.duration_since(*checked_at) < Duration::from_secs(HEALTH_CACHE_SECONDS)
            })
            .map(|(_, ready)| ready)
    };
    let ready = if let Some(ready) = cached {
        ready
    } else {
        let _probe = state.health_probe.lock().await;
        let cached = {
            let health = state.health.lock().await;
            let now = Instant::now();
            health
                .readiness
                .filter(|(checked_at, _)| {
                    now.duration_since(*checked_at) < Duration::from_secs(HEALTH_CACHE_SECONDS)
                })
                .map(|(_, ready)| ready)
        };
        if let Some(ready) = cached {
            ready
        } else {
            let ready = state.clickhouse.ready().await;
            state.health.lock().await.readiness = Some((Instant::now(), ready));
            ready
        }
    };
    Ok(safe_response(
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        json!({
            "status":if ready {"PASS"} else {"FAIL"},
            "api_version":API_VERSION,
            "storage_ready":ready,
            "latest_version":state.config.latest_version,
            "minimum_supported_version":state.config.minimum_supported_version,
        }),
        if ready {
            "accepted"
        } else {
            "storage_unavailable"
        },
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Enrollment {
    schema_version: u8,
    kind: String,
    collector_instance_id: Uuid,
    collector_token: String,
    os_family: String,
    runtime_family: String,
    execution_mode: String,
    groundline_version: String,
}

async fn enroll(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<Enrollment>,
) -> Result<Response, ApiError> {
    require_tailnet(&state, peer, &headers)?;
    require_enrollment(&state.config, &headers)?;
    state.rate_limit(RateLimitScope::Enrollment, MAX_ENROLLMENTS_PER_MINUTE)?;
    let _storage_permit = state.storage_permit(StorageClass::Operator)?;
    if input.schema_version != 2
        || input.kind != "groundline-insights-owner-enrollment"
        || !(TOKEN_MIN_BYTES..=4096).contains(&input.collector_token.len())
        || !matches!(input.os_family.as_str(), "macos" | "windows" | "linux")
        || !matches!(input.runtime_family.as_str(), "codex_app" | "codex_cli")
        || !matches!(
            input.execution_mode.as_str(),
            "desktop" | "local_headless" | "remote_headless"
        )
        || groundline_contracts::version::strict_version(&input.groundline_version).is_err()
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_request"));
    }
    let token_hash = format!("{:x}", Sha256::digest(input.collector_token.as_bytes()));
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let existing = state
        .clickhouse
        .collector(input.collector_instance_id)
        .await?;
    let (outcome, status, created_at, generation) = match existing {
        Some(existing) => {
            if !constant_time_equal(&existing.token_hash, &token_hash) {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "collector_already_enrolled",
                ));
            }
            let same = existing.os_family == input.os_family
                && existing.runtime_family == input.runtime_family
                && existing.execution_mode == input.execution_mode
                && existing.groundline_version == input.groundline_version;
            (
                if same { "duplicate" } else { "updated" },
                StatusCode::OK,
                existing.created_at,
                existing.current_generation,
            )
        }
        None => ("accepted", StatusCode::CREATED, now, 0),
    };
    state
        .clickhouse
        .write_collector(&Collector {
            collector_id: input.collector_instance_id,
            token_hash,
            current_generation: generation,
            enrollment_schema_version: 2,
            created_at,
            os_family: input.os_family,
            runtime_family: input.runtime_family,
            execution_mode: input.execution_mode,
            groundline_version: input.groundline_version,
            revoked: 0,
        })
        .await?;
    Ok(safe_response(
        status,
        json!({
            "status":"PASS",
            "collector_instance_id":input.collector_instance_id,
            "outcome":outcome,
        }),
        outcome,
    ))
}

fn pointer_u64(value: &Value, pointer: &str) -> u64 {
    value.pointer(pointer).and_then(Value::as_u64).unwrap_or(0)
}

fn checked_u32_sum(left: u64, right: u64) -> Result<u64, ApiError> {
    left.checked_add(right)
        .filter(|value| *value <= u64::from(u32::MAX))
        .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_event"))
}

fn pointer_f64(value: &Value, pointer: &str) -> Value {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(Value::from)
        .unwrap_or(Value::Null)
}

fn pointer_str<'a>(value: &'a Value, pointer: &str, default: &'a str) -> &'a str {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or(default)
}

pub fn event_row(event: &Value, received_at: DateTime<Utc>) -> Result<Value, ApiError> {
    let encoded = serde_json::to_vec(event)
        .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_event"))?;
    validate_basic_event_bytes(&encoded)
        .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_event"))?;
    let root = event.pointer("/metrics/root").unwrap_or(&Value::Null);
    let delegated = event.pointer("/metrics/delegated").unwrap_or(&Value::Null);
    let guardian = event.pointer("/metrics/guardian").unwrap_or(&Value::Null);
    let model_effort = root
        .get("model_effort")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let model_families = model_effort
        .iter()
        .filter_map(|item| item.get("model_family").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let efforts = model_effort
        .iter()
        .filter_map(|item| item.get("effort").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let model_counts = model_effort
        .iter()
        .filter_map(|item| item.get("count").and_then(Value::as_u64))
        .collect::<Vec<_>>();
    let payload_json = String::from_utf8(encoded)
        .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_event"))?;
    let truncated_count = checked_u32_sum(
        pointer_u64(event, "/sample/delegated_truncated_count"),
        pointer_u64(event, "/sample/guardian_truncated_count"),
    )?;
    Ok(json!({
        "schema_version":5,
        "event_id":event["event_id"],
        "idempotency_key":event["idempotency_key"],
        "collector_id":event["collector"]["instance_id"],
        "collection_generation":pointer_u64(event, "/source/collection_generation"),
        "collection_trigger":pointer_str(event, "/source/collection_trigger", "manual"),
        "received_at":received_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        "groundline_version":event["source"]["groundline_version"],
        "os_family":event["collector"]["os_family"],
        "runtime_family":event["collector"]["runtime_family"],
        "execution_mode":event["collector"]["execution_mode"],
        "period_start":event["period"]["start_utc"],
        "period_end":event["period"]["end_utc"],
        "generated_at":event["period"]["generated_at_utc"],
        "selection_mode":event["sample"]["selection_mode"],
        "requested_days":pointer_u64(event, "/sample/requested_days"),
        "root_count":pointer_u64(event, "/sample/root_count"),
        "observed_root_count":pointer_u64(event, "/sample/observed_root_count"),
        "eligible_root_count":pointer_u64(event, "/sample/eligible_root_count"),
        "selected_root_count":pointer_u64(event, "/sample/selected_root_count"),
        "root_truncated_count":pointer_u64(event, "/sample/root_truncated_count"),
        "selection_coverage":pointer_f64(event, "/sample/selection_coverage"),
        "selected_recency_start":event["sample"]["selected_recency_start_utc"],
        "selected_recency_end":event["sample"]["selected_recency_end_utc"],
        "minimum_root_count":pointer_u64(event, "/sample/minimum_root_count"),
        "sample_sufficient":u8::from(event["sample"]["sample_sufficient"].as_bool().unwrap_or(false)),
        "delegated_count":pointer_u64(event, "/sample/delegated_count"),
        "guardian_count":pointer_u64(event, "/sample/guardian_count"),
        "guardian_incomplete_excluded_count":pointer_u64(event, "/sample/guardian_incomplete_excluded_count"),
        "unreadable_root_count":pointer_u64(event, "/sample/unreadable_completed_root_count"),
        "originator_unclassified_excluded_root_count":pointer_u64(event, "/sample/originator_unclassified_excluded_root_count"),
        "originator_source_fallback_root_count":pointer_u64(event, "/sample/originator_source_fallback_root_count"),
        "truncated_count":truncated_count,
        "capability_completed_root_coverage":u8::from(event["capabilities"]["completed_root_coverage"] == Value::Bool(true)),
        "capability_latency_completed_count":u8::from(event["capabilities"]["latency_completed_count"] == Value::Bool(true)),
        "capability_root_boundary_counts":u8::from(event["capabilities"]["root_boundary_counts"] == Value::Bool(true)),
        "capability_guardian_workspace_attribution":u8::from(event["capabilities"]["guardian_workspace_attribution"] == Value::Bool(true)),
        "root_status":root["status"],
        "delegated_status":delegated["status"],
        "guardian_status":guardian["status"],
        "model_families":model_families,
        "efforts":efforts,
        "model_effort_counts":model_counts,
        "usage_source":root["usage"]["source"],
        "delegated_usage_source":delegated["usage"]["source"],
        "guardian_usage_source":guardian["usage"]["source"],
        "input_tokens":pointer_u64(root, "/usage/input_tokens"),
        "cached_input_tokens":pointer_u64(root, "/usage/cached_input_tokens"),
        "output_tokens":pointer_u64(root, "/usage/output_tokens"),
        "reasoning_output_tokens":pointer_u64(root, "/usage/reasoning_output_tokens"),
        "total_tokens":pointer_u64(root, "/usage/total_tokens"),
        "non_cached_input_tokens":pointer_u64(root, "/usage/non_cached_input_tokens"),
        "cached_input_ratio":pointer_f64(root, "/usage/cached_input_ratio"),
        "cumulative_rollout_count":pointer_u64(root, "/usage/cumulative_rollout_count"),
        "fallback_rollout_count":pointer_u64(root, "/usage/fallback_rollout_count"),
        "delegated_fallback_rollout_count":pointer_u64(delegated, "/usage/fallback_rollout_count"),
        "guardian_fallback_rollout_count":pointer_u64(guardian, "/usage/fallback_rollout_count"),
        "task_started":pointer_u64(root, "/activity/task_started"),
        "task_completed":pointer_u64(root, "/activity/task_completed"),
        "turn_contexts":pointer_u64(root, "/activity/turn_contexts"),
        "compactions":pointer_u64(root, "/activity/compactions"),
        "user_messages_with_text":pointer_u64(root, "/activity/user_messages_with_text"),
        "latency_median_ms":pointer_f64(root, "/latency/median_ms"),
        "latency_p90_ms":pointer_f64(root, "/latency/p90_ms"),
        "latency_max_ms":pointer_f64(root, "/latency/max_ms"),
        "latency_completed_count":pointer_u64(root, "/latency/completed_count"),
        "long_turn_count":pointer_u64(root, "/latency/long_turn_count"),
        "verification_tool_calls":pointer_u64(root, "/quality_proxies/verification_tool_calls"),
        "verification_success_count":pointer_u64(root, "/quality_proxies/verification_success_count"),
        "verification_failure_count":pointer_u64(root, "/quality_proxies/verification_failure_count"),
        "verification_unresolved_count":pointer_u64(root, "/quality_proxies/verification_unresolved_count"),
        "tool_call_count":pointer_u64(root, "/quality_proxies/tool_call_count"),
        "short_message_count":pointer_u64(root, "/quality_proxies/short_message_count"),
        "broad_scope_message_count":pointer_u64(root, "/quality_proxies/broad_scope_message_count"),
        "nonzero_exit_count":pointer_u64(root, "/quality_proxies/failure_signals/nonzero_exit"),
        "timeout_count":pointer_u64(root, "/quality_proxies/failure_signals/timeout"),
        "rejected_count":pointer_u64(root, "/quality_proxies/failure_signals/rejected"),
        "exact_repeated_call_groups":pointer_u64(root, "/quality_proxies/exact_repeated_call_groups"),
        "calls_in_exact_repeated_groups":pointer_u64(root, "/quality_proxies/calls_in_exact_repeated_groups"),
        "boundary_review_recommended":u8::from(root["quality_proxies"]["task_boundary_review_recommended"].as_bool().unwrap_or(false)),
        "long_lived_root_session":u8::from(root["quality_proxies"]["long_lived_root_session"].as_bool().unwrap_or(false)),
        "boundary_review_root_count":pointer_u64(root, "/quality_proxies/boundary_review_root_count"),
        "long_lived_root_count":pointer_u64(root, "/quality_proxies/long_lived_root_count"),
        "delegated_total_tokens":pointer_u64(delegated, "/usage/total_tokens"),
        "guardian_total_tokens":pointer_u64(guardian, "/usage/total_tokens"),
        "guardian_review_count":pointer_u64(guardian, "/review_count"),
        "guardian_workspace_attributed_review_count":pointer_u64(guardian, "/signals/workspace_attributed_review_count"),
        "guardian_workspace_attribution_coverage":pointer_f64(guardian, "/signals/workspace_attribution_coverage"),
        "consent_receipt_id":event["consent"]["receipt_id"],
        "payload_json":payload_json,
    }))
}

async fn event_exists(state: &AppState, event_id: Uuid) -> Result<bool, ApiError> {
    state
        .clickhouse
        .request(
            "SELECT count() FROM groundline.basic_weekly FINAL WHERE event_id = {event_id:UUID} FORMAT TabSeparated",
            &[("event_id", event_id.to_string())],
            None,
        )
        .await
        .map(|value| value != b"0\n" && value != b"0")
}

fn reaches_ingestion_watermark(current: u64, candidate: u64, maximum: u64) -> bool {
    u128::from(current.saturating_add(candidate)) * 100
        >= u128::from(maximum) * u128::from(DATASET_INGESTION_WATERMARK_PERCENT)
}

async fn enforce_ingestion_capacity(
    state: &AppState,
    collector_id: Uuid,
    candidate_payload_bytes: u64,
) -> Result<(), ApiError> {
    let collector = state
        .clickhouse
        .json_row(
            "SELECT count() AS event_count, coalesce(sum(length(payload_json)), 0) AS payload_bytes FROM groundline.basic_weekly FINAL WHERE collector_id = {collector_id:UUID} FORMAT JSONEachRow",
            &[("collector_id", collector_id.to_string())],
        )
        .await?
        .unwrap_or_else(|| json!({}));
    if count(&collector, "event_count").saturating_add(1) > state.config.collector_max_events
        || count(&collector, "payload_bytes").saturating_add(candidate_payload_bytes)
            > state.config.collector_max_payload_bytes
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "collector_quota_exceeded",
        ));
    }

    let dataset = state
        .clickhouse
        .json_row(
            "SELECT coalesce(sum(rows), 0) AS row_count, coalesce(sum(bytes_on_disk), 0) AS disk_bytes FROM system.parts WHERE active AND database = 'groundline' AND table = 'basic_weekly' FORMAT JSONEachRow",
            &[],
        )
        .await?
        .unwrap_or_else(|| json!({}));
    if reaches_ingestion_watermark(
        count(&dataset, "row_count"),
        1,
        state.config.dataset_max_rows,
    ) || reaches_ingestion_watermark(
        count(&dataset, "disk_bytes"),
        candidate_payload_bytes,
        state.config.dataset_max_bytes,
    ) {
        return Err(ApiError::new(
            StatusCode::INSUFFICIENT_STORAGE,
            "dataset_capacity_reserved",
        ));
    }
    Ok(())
}

async fn ingest_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, ApiError> {
    let collector_header = headers
        .get("x-groundline-collector-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "invalid_auth"))?;
    let _storage_permit = state.storage_permit(StorageClass::Collector)?;
    let collector = require_collector_token(&state, &headers, collector_header).await?;
    state.rate_limit(
        RateLimitScope::Collector(collector_header),
        MAX_REQUESTS_PER_MINUTE,
    )?;
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        != Some("application/json")
    {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_content_type",
        ));
    }
    let event = validate_basic_event_bytes(&body)
        .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_event"))?;
    let event_id = event["event_id"]
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_event"))?;
    let collector_id = event["collector"]["instance_id"]
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_event"))?;
    if collector_header != collector_id
        || headers
            .get("idempotency-key")
            .and_then(|value| value.to_str().ok())
            != event["idempotency_key"].as_str()
    {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "invalid_auth"));
    }
    let generation = pointer_u64(&event, "/source/collection_generation");
    let next_generation = collector.current_generation.checked_add(1).map(u64::from);
    if generation != u64::from(collector.current_generation) && Some(generation) != next_generation
    {
        return Err(ApiError::new(StatusCode::CONFLICT, "invalid_generation"));
    }
    let candidate_payload_bytes = u64::try_from(body.len()).map_err(|_| ApiError::storage())?;
    let _ingestion_guard = state.ingestion_gate.lock().await;
    let duplicate = event_exists(&state, event_id).await?;
    if !duplicate {
        enforce_ingestion_capacity(&state, collector_id, candidate_payload_bytes).await?;
        let row = event_row(&event, Utc::now())?;
        let mut body = serde_json::to_vec(&row).map_err(|_| ApiError::storage())?;
        body.push(b'\n');
        state
            .clickhouse
            .request(
                "INSERT INTO groundline.basic_weekly FORMAT JSONEachRow",
                &[],
                Some(body),
            )
            .await?;
    }
    let outcome = if duplicate { "duplicate" } else { "accepted" };
    let mut payload = json!({
        "status":"PASS",
        "event_id":event_id,
        "outcome":outcome,
    });
    if let Some(advisory) = update_advisory(
        &state.config,
        headers
            .get("x-groundline-version")
            .and_then(|value| value.to_str().ok()),
    ) {
        payload
            .as_object_mut()
            .expect("payload object")
            .insert("update_advisory".to_owned(), advisory);
    }
    Ok(safe_response(
        if duplicate {
            StatusCode::OK
        } else {
            StatusCode::ACCEPTED
        },
        payload,
        outcome,
    ))
}

fn update_advisory(config: &Config, current: Option<&str>) -> Option<Value> {
    let current = current?;
    let current_version = groundline_contracts::version::strict_version(current).ok()?;
    let latest = groundline_contracts::version::strict_version(&config.latest_version).ok()?;
    let minimum =
        groundline_contracts::version::strict_version(&config.minimum_supported_version).ok()?;
    let status = if current_version < minimum {
        "update_required"
    } else if current_version < latest {
        "update_available"
    } else {
        "up_to_date"
    };
    Some(json!({
        "schema_version":1,
        "status":status,
        "current_version":current,
        "latest_version":config.latest_version,
        "minimum_supported_version":config.minimum_supported_version,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Activation {
    schema_version: u8,
    kind: String,
    collector_instance_id: Uuid,
    target_generation: u32,
    expected_event_count: u64,
}

async fn activate_generation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Activation>,
) -> Result<Response, ApiError> {
    if input.schema_version != 1
        || input.kind != "groundline-insights-generation-activation"
        || input.target_generation == 0
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_request"));
    }
    let _storage_permit = state.storage_permit(StorageClass::Collector)?;
    let mut collector =
        require_collector_token(&state, &headers, input.collector_instance_id).await?;
    state.rate_limit(
        RateLimitScope::Collector(input.collector_instance_id),
        MAX_REQUESTS_PER_MINUTE,
    )?;
    if input.target_generation == collector.current_generation {
        return Ok(safe_response(
            StatusCode::OK,
            json!({"status":"PASS","target_generation":input.target_generation,"outcome":"duplicate"}),
            "duplicate",
        ));
    }
    if collector.current_generation.checked_add(1) != Some(input.target_generation) {
        return Err(ApiError::new(StatusCode::CONFLICT, "invalid_generation"));
    }
    let count = state
        .clickhouse
        .request(
            "SELECT count() FROM groundline.basic_weekly FINAL WHERE collector_id = {collector_id:UUID} AND collection_generation = {generation:UInt32} FORMAT TabSeparated",
            &[
                ("collector_id", input.collector_instance_id.to_string()),
                ("generation", input.target_generation.to_string()),
            ],
            None,
        )
        .await?;
    let count = std::str::from_utf8(&count)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .ok_or_else(ApiError::storage)?;
    if count != input.expected_event_count {
        return Err(ApiError::new(StatusCode::CONFLICT, "generation_incomplete"));
    }
    collector.current_generation = input.target_generation;
    state.clickhouse.write_collector(&collector).await?;
    Ok(safe_response(
        StatusCode::CREATED,
        json!({"status":"PASS","target_generation":input.target_generation,"outcome":"accepted"}),
        "accepted",
    ))
}

async fn delete_collector(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(collector_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    require_tailnet(&state, peer, &headers)?;
    require_admin_report(&state.config, &headers)?;
    state.rate_limit(RateLimitScope::Admin, MAX_REQUESTS_PER_MINUTE)?;
    let _storage_permit = state.storage_permit(StorageClass::Operator)?;
    let confirmation = collector_id.to_string();
    if headers
        .get("x-groundline-delete-confirm")
        .and_then(|value| value.to_str().ok())
        != Some(confirmation.as_str())
    {
        return Err(ApiError::new(
            StatusCode::PRECONDITION_FAILED,
            "invalid_delete_confirmation",
        ));
    }
    for table in ["collectors", "basic_weekly"] {
        state
            .clickhouse
            .request(
                &format!("ALTER TABLE groundline.{table} DELETE WHERE collector_id = {{collector_id:UUID}} SETTINGS mutations_sync = 1"),
                &[("collector_id", collector_id.to_string())],
                None,
            )
            .await?;
    }
    Ok(safe_response(
        StatusCode::OK,
        json!({"status":"PASS","deleted":true}),
        "accepted",
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportQuery {
    days: u16,
}

fn count(row: &Value, key: &str) -> u64 {
    row.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn ratio(numerator: u64, denominator: u64) -> Value {
    if denominator == 0 {
        Value::Null
    } else {
        Value::from(((numerator as f64 / denominator as f64) * 10_000.0).round() / 10_000.0)
    }
}

fn distributions(rows: &[Value], dimension: &str, count_key: &str) -> Vec<Value> {
    rows.iter()
        .filter(|row| row.get("dimension").and_then(Value::as_str) == Some(dimension))
        .filter_map(|row| {
            let mut value = Map::new();
            value.insert("value".to_owned(), Value::from(row.get("value")?.as_str()?));
            value.insert(
                count_key.to_owned(),
                Value::from(row.get("count")?.as_u64()?),
            );
            Some(Value::Object(value))
        })
        .collect()
}

async fn weekly_report(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<ReportQuery>,
) -> Result<Response, ApiError> {
    require_tailnet(&state, peer, &headers)?;
    require_admin_report(&state.config, &headers)?;
    state.rate_limit(RateLimitScope::Admin, MAX_REQUESTS_PER_MINUTE)?;
    if !matches!(query.days, 7 | 30 | 90) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "invalid_request"));
    }
    let _storage_permit = state.storage_permit(StorageClass::Operator)?;
    let end = Utc::now();
    let start = end - ChronoDuration::days(i64::from(query.days));
    let params = [
        ("start", start.to_rfc3339_opts(SecondsFormat::Secs, true)),
        ("end", end.to_rfc3339_opts(SecondsFormat::Secs, true)),
    ];
    let summary = state
        .clickhouse
        .json_row(REPORT_SUMMARY_QUERY, &params)
        .await?
        .unwrap_or_else(|| json!({}));
    let fleet = state
        .clickhouse
        .json_row(REPORT_FLEET_QUERY, &params)
        .await?
        .unwrap_or_else(|| json!({}));
    let storage = state
        .clickhouse
        .json_row(REPORT_STORAGE_QUERY, &params)
        .await?
        .unwrap_or_else(|| json!({}));
    let event_cohorts = state
        .clickhouse
        .json_rows(REPORT_EVENT_COHORT_QUERY, &params)
        .await?;
    let install_cohorts = state
        .clickhouse
        .json_rows(REPORT_INSTALL_COHORT_QUERY, &[])
        .await?;
    let model_effort = state
        .clickhouse
        .json_rows(REPORT_MODEL_EFFORT_QUERY, &params)
        .await?;
    let event_count = count(&summary, "event_count");
    let observed_roots = count(&summary, "observed_root_total");
    let selected_roots = count(&summary, "selected_root_total");
    let eligible_roots = count(&summary, "eligible_root_total");
    let completed_turns = count(&summary, "completed_turn_count");
    let verification_calls = count(&summary, "verification_tool_call_count");
    let verification_success = count(&summary, "verification_success_count");
    let verification_failure = count(&summary, "verification_failure_count");
    let verification_unresolved = count(&summary, "verification_unresolved_count");
    let tool_calls = count(&summary, "tool_call_count");
    let user_messages = count(&summary, "user_messages_with_text");
    let mut quality = BTreeSet::<String>::new();
    if event_count == 0 {
        quality.insert("no_events".to_owned());
    }
    for (field, reason) in [
        ("unreadable_root_count", "unreadable_roots"),
        ("root_truncated_count", "completed_root_selection_truncated"),
        ("non_root_truncated_count", "non_root_selection_truncated"),
        ("originator_unclassified_count", "originator_gaps"),
        ("originator_source_fallback_count", "originator_gaps"),
        ("usage_fallback_count", "usage_fallback_present"),
        ("usage_missing_count", "usage_missing"),
        (
            "guardian_incomplete_excluded_count",
            "guardian_incomplete_excluded",
        ),
        ("component_nonpass_event_count", "component_nonpass_present"),
    ] {
        if count(&summary, field) > 0 {
            quality.insert(reason.to_owned());
        }
    }
    if event_count > 0 && observed_roots < REPORT_MINIMUM_ROOTS {
        quality.insert("insufficient_root_sample".to_owned());
    }
    if event_count > 0
        && (count(&summary, "latency_capable_event_count") < event_count || completed_turns == 0)
    {
        quality.insert("latency_denominator_unavailable".to_owned());
    }
    if event_count > 0 && count(&summary, "boundary_count_capable_event_count") < event_count {
        quality.insert("boundary_counts_unavailable".to_owned());
    }
    if count(&summary, "guardian_attribution_capable_event_count")
        < count(&summary, "guardian_attribution_applicable_event_count")
    {
        quality.insert("guardian_attribution_unavailable".to_owned());
    }
    if verification_unresolved > 0 {
        quality.insert("verification_outcome_incomplete".to_owned());
    }
    if count(&fleet, "metadata_unknown_installation_count") > 0 {
        quality.insert("enrollment_metadata_incomplete".to_owned());
    }
    if count(&fleet, "overdue_never_reported_installation_count") > 0 {
        quality.insert("fleet_reporting_incomplete".to_owned());
        quality.insert("initial_report_overdue".to_owned());
    }
    if count(&fleet, "stale_observed_installation_count") > 0 {
        quality.insert("stale_installation_reporting".to_owned());
    }
    if count(
        &fleet,
        "current_package_claim_unobserved_installation_count",
    ) > 0
    {
        quality.insert("current_package_observation_incomplete".to_owned());
    }
    if count(&storage, "duplicate_event_row_count") > 0 {
        quality.insert("physical_event_duplicates_detected".to_owned());
    }
    if count(&storage, "ttl_expired_event_row_count") > 0 {
        quality.insert("retention_cleanup_pending".to_owned());
    }
    if count(&storage, "overdue_delivery_event_count") > 0 {
        quality.insert("event_delivery_overdue".to_owned());
    } else if count(&storage, "delayed_delivery_event_count") > 0 {
        quality.insert("event_delivery_delayed".to_owned());
    }
    if count(&storage, "clock_skew_event_count") > 0 {
        quality.insert("event_clock_skew_detected".to_owned());
    }
    let sample_sufficient = count(&summary, "sample_sufficient_event_count");
    let sample_insufficient = count(&summary, "sample_insufficient_event_count");
    let mut comparison = BTreeSet::from(["comparison_baseline_not_included".to_owned()]);
    if !quality.is_empty() {
        comparison.insert("data_quality_not_pass".to_owned());
    }
    if event_count < REPORT_MINIMUM_EVENTS {
        comparison.insert("too_few_events".to_owned());
    }
    if observed_roots < REPORT_MINIMUM_ROOTS {
        comparison.insert("too_few_observed_roots".to_owned());
    }
    if distributions(&event_cohorts, "schema_version", "event_count").len() > 1 {
        comparison.insert("mixed_schema_versions".to_owned());
    }
    if distributions(&event_cohorts, "groundline_version", "event_count").len() > 1 {
        comparison.insert("mixed_groundline_versions".to_owned());
    }
    let latest_received = fleet
        .get("latest_received_at_utc")
        .cloned()
        .unwrap_or(Value::Null);
    let freshness = if event_count == 0 {
        "NO_DATA"
    } else if fleet.get("fresh") == Some(&Value::from(1)) {
        "FRESH"
    } else {
        "STALE"
    };
    let report = json!({
        "schema_version":3,
        "kind":"groundline-insights-weekly-report",
        "status":"PASS",
        "reason_code":"accepted",
        "generated_at_utc":end.to_rfc3339_opts(SecondsFormat::Secs, true),
        "requested_days":query.days,
        "source_contract":{
            "dataset":"basic_active","time_basis":"utc","metric_time_field":"period_end_or_generated_at",
            "freshness_time_field":"received_at","roster_source":"enrolled_installation_registry",
            "analysis_mode":"descriptive_single_period","query_set_version":3,"basic_aggregate_only":true
        },
        "collection_health":{
            "enrolled_installation_count":count(&fleet,"enrolled_installation_count"),
            "metadata_known_installation_count":count(&fleet,"metadata_known_installation_count"),
            "metadata_unknown_installation_count":count(&fleet,"metadata_unknown_installation_count"),
            "observed_installation_count":count(&fleet,"observed_installation_count"),
            "reporting_installation_count":count(&fleet,"reporting_installation_count"),
            "recent_installation_count":count(&fleet,"recent_installation_count"),
            "never_reported_installation_count":count(&fleet,"never_reported_installation_count"),
            "pending_initial_report_installation_count":count(&fleet,"pending_initial_report_installation_count"),
            "overdue_never_reported_installation_count":count(&fleet,"overdue_never_reported_installation_count"),
            "stale_observed_installation_count":count(&fleet,"stale_observed_installation_count"),
            "current_package_claim_installation_count":count(&fleet,"current_package_claim_installation_count"),
            "current_package_claim_unobserved_installation_count":count(&fleet,"current_package_claim_unobserved_installation_count"),
            "current_observed_installation_count":count(&fleet,"current_observed_installation_count"),
            "current_reporting_installation_count":count(&fleet,"current_reporting_installation_count"),
            "current_recent_installation_count":count(&fleet,"current_recent_installation_count"),
            "policy_latest_version":fleet.get("policy_latest_version").and_then(Value::as_str).unwrap_or(&state.config.latest_version),
            "roster_status":"AVAILABLE","latest_received_at_utc":latest_received,"freshness_status":freshness,
            "freshness_threshold_hours":REPORT_FRESHNESS_HOURS,"initial_report_grace_hours":INITIAL_REPORT_GRACE_HOURS,
            "stored_event_row_count":count(&storage,"stored_event_row_count"),"deduplicated_event_count":count(&storage,"deduplicated_event_count"),
            "duplicate_event_row_count":count(&storage,"duplicate_event_row_count"),"ttl_expired_event_row_count":count(&storage,"ttl_expired_event_row_count"),"delayed_delivery_event_count":count(&storage,"delayed_delivery_event_count"),
            "overdue_delivery_event_count":count(&storage,"overdue_delivery_event_count"),"clock_skew_event_count":count(&storage,"clock_skew_event_count"),
            "delivery_delay_threshold_hours":REPORT_DELIVERY_DELAY_HOURS,"delivery_overdue_threshold_hours":REPORT_DELIVERY_OVERDUE_HOURS,
            "clock_skew_tolerance_minutes":REPORT_CLOCK_SKEW_TOLERANCE_MINUTES
        },
        "coverage":{
            "event_count":event_count,"eligible_root_count":eligible_roots,"selected_root_count":selected_roots,"observed_root_count":observed_roots,
            "completed_turn_count":completed_turns,"unreadable_root_count":count(&summary,"unreadable_root_count"),
            "root_truncated_count":count(&summary,"root_truncated_count"),"non_root_truncated_count":count(&summary,"non_root_truncated_count"),
            "originator_unclassified_count":count(&summary,"originator_unclassified_count"),"originator_source_fallback_count":count(&summary,"originator_source_fallback_count"),
            "root_usage_applicable_event_count":count(&summary,"root_usage_applicable_event_count"),"root_usage_missing_event_count":count(&summary,"root_usage_missing_event_count"),
            "root_usage_fallback_event_count":count(&summary,"root_usage_fallback_event_count"),"delegated_usage_applicable_event_count":count(&summary,"delegated_usage_applicable_event_count"),
            "delegated_usage_missing_event_count":count(&summary,"delegated_usage_missing_event_count"),"delegated_usage_fallback_event_count":count(&summary,"delegated_usage_fallback_event_count"),
            "guardian_usage_applicable_event_count":count(&summary,"guardian_usage_applicable_event_count"),"guardian_usage_missing_event_count":count(&summary,"guardian_usage_missing_event_count"),
            "guardian_usage_fallback_event_count":count(&summary,"guardian_usage_fallback_event_count"),"guardian_incomplete_excluded_count":count(&summary,"guardian_incomplete_excluded_count"),
            "completed_root_coverage_applicable_event_count":count(&summary,"completed_root_coverage_applicable_event_count"),
            "completed_root_coverage_capable_event_count":count(&summary,"completed_root_coverage_capable_event_count"),
            "completed_root_selection_coverage":ratio(selected_roots,eligible_roots),"latency_capable_event_count":count(&summary,"latency_capable_event_count"),
            "boundary_count_capable_event_count":count(&summary,"boundary_count_capable_event_count"),
            "guardian_attribution_applicable_event_count":count(&summary,"guardian_attribution_applicable_event_count"),
            "guardian_attribution_capable_event_count":count(&summary,"guardian_attribution_capable_event_count"),
            "component_nonpass_event_count":count(&summary,"component_nonpass_event_count")
        },
        "weekly_metrics":{
            "tokens":{"input":count(&summary,"input_tokens"),"cached_input":count(&summary,"cached_input_tokens"),
                "non_cached_input":count(&summary,"non_cached_input_tokens"),"output":count(&summary,"output_tokens"),
                "reasoning_output":count(&summary,"reasoning_output_tokens"),"total":count(&summary,"total_tokens"),
                "delegated_total":count(&summary,"delegated_total_tokens"),"guardian_total":count(&summary,"guardian_total_tokens")},
            "workflow":{"compactions":count(&summary,"compactions"),"compactions_per_observed_root":ratio(count(&summary,"compactions"),observed_roots),
                "long_turn_count":count(&summary,"long_turn_count"),"long_turn_rate":ratio(count(&summary,"long_turn_count"),completed_turns),
                "exact_repeated_call_groups":count(&summary,"exact_repeated_call_groups"),"calls_in_exact_repeated_groups":count(&summary,"calls_in_exact_repeated_groups"),
                "repeated_call_rate":ratio(count(&summary,"calls_in_exact_repeated_groups"),tool_calls),"failure_signal_count":count(&summary,"failure_signal_count"),
                "failure_signal_rate":ratio(count(&summary,"failure_signal_count"),tool_calls),"tool_call_count":tool_calls,
                "user_messages_with_text":user_messages,"short_message_count":count(&summary,"short_message_count"),
                "short_message_rate":ratio(count(&summary,"short_message_count"),user_messages),"broad_scope_message_count":count(&summary,"broad_scope_message_count"),
                "broad_scope_message_rate":ratio(count(&summary,"broad_scope_message_count"),user_messages),
                "boundary_review_root_count":count(&summary,"boundary_review_root_count"),"long_lived_root_count":count(&summary,"long_lived_root_count")},
            "verification":{"tool_call_count":verification_calls,"success_count":verification_success,"failure_count":verification_failure,
                "unresolved_count":verification_unresolved,"outcome_coverage":ratio(verification_success.saturating_add(verification_failure),verification_calls)},
            "guardian":{"review_count":count(&summary,"guardian_review_total"),"workspace_attributed_review_count":count(&summary,"guardian_workspace_attributed_review_count"),
                "workspace_attribution_coverage":ratio(count(&summary,"guardian_workspace_attributed_review_count"),count(&summary,"guardian_review_total"))}
        },
        "cohorts":{
            "event_distributions":{"schema_version":distributions(&event_cohorts,"schema_version","event_count"),
                "groundline_version":distributions(&event_cohorts,"groundline_version","event_count"),
                "os_family":distributions(&event_cohorts,"os_family","event_count"),"runtime_family":distributions(&event_cohorts,"runtime_family","event_count"),
                "execution_mode":distributions(&event_cohorts,"execution_mode","event_count")},
            "installation_distributions":{"groundline_version":distributions(&install_cohorts,"groundline_version","installation_count"),
                "os_family":distributions(&install_cohorts,"os_family","installation_count"),"runtime_family":distributions(&install_cohorts,"runtime_family","installation_count"),
                "execution_mode":distributions(&install_cohorts,"execution_mode","installation_count")},
            "model_effort_context_distribution":model_effort,
            "model_effort_token_efficiency":{"status":"UNAVAILABLE","reason_code":"token_usage_not_attributed_to_model_effort","context_distribution_only":true}
        },
        "data_quality":{"status":if quality.is_empty(){"PASS"}else if quality.contains("no_events"){"FAIL"}else{"PARTIAL"},
            "reason_codes":quality,"sample_sufficient_event_count":sample_sufficient,"sample_insufficient_event_count":sample_insufficient},
        "comparison_readiness":{"status":"INSUFFICIENT","reason_codes":comparison,"minimum_event_count":REPORT_MINIMUM_EVENTS,"minimum_observed_root_count":REPORT_MINIMUM_ROOTS}
    });
    let encoded = serde_json::to_vec(&report).map_err(|_| ApiError::storage())?;
    if WeeklyReport::from_slice(&encoded).is_err() {
        #[cfg(test)]
        eprintln!("weekly_report_contract_rejected: {report}");
        return Err(ApiError::storage());
    }
    Ok(safe_response(StatusCode::OK, report, "accepted"))
}

const REPORT_SUMMARY_QUERY: &str = r#"SELECT count() AS event_count, sum(eligible_root_count) AS eligible_root_total, sum(selected_root_count) AS selected_root_total, sum(observed_root_count) AS observed_root_total, sum(task_completed) AS completed_turn_count, sum(unreadable_root_count) AS unreadable_root_count, sum(root_truncated_count) AS root_truncated_count, sum(truncated_count) AS non_root_truncated_count, sum(originator_unclassified_excluded_root_count) AS originator_unclassified_count, sum(originator_source_fallback_root_count) AS originator_source_fallback_count, countIf(observed_root_count > 0) AS root_usage_applicable_event_count, countIf(observed_root_count > 0 AND usage_source IN ('unavailable','unknown')) AS root_usage_missing_event_count, countIf(fallback_rollout_count > 0) AS root_usage_fallback_event_count, countIf(delegated_count > 0) AS delegated_usage_applicable_event_count, countIf(delegated_count > 0 AND delegated_usage_source IN ('unavailable','unknown')) AS delegated_usage_missing_event_count, countIf(delegated_fallback_rollout_count > 0) AS delegated_usage_fallback_event_count, countIf(guardian_count > 0) AS guardian_usage_applicable_event_count, countIf(guardian_count > 0 AND guardian_usage_source IN ('unavailable','unknown')) AS guardian_usage_missing_event_count, countIf(guardian_fallback_rollout_count > 0) AS guardian_usage_fallback_event_count, sum(guardian_incomplete_excluded_count) AS guardian_incomplete_excluded_count, countIf(selection_mode != 'activity_window') AS completed_root_coverage_applicable_event_count, countIf(capability_completed_root_coverage = 1 AND selection_mode != 'activity_window') AS completed_root_coverage_capable_event_count, countIf(capability_latency_completed_count = 1) AS latency_capable_event_count, countIf(capability_root_boundary_counts = 1) AS boundary_count_capable_event_count, countIf(guardian_review_count > 0) AS guardian_attribution_applicable_event_count, countIf(guardian_review_count > 0 AND capability_guardian_workspace_attribution = 1) AS guardian_attribution_capable_event_count, countIf(root_status != 'PASS' OR delegated_status != 'PASS' OR guardian_status != 'PASS') AS component_nonpass_event_count, countIf(sample_sufficient = 1) AS sample_sufficient_event_count, countIf(sample_sufficient = 0) AS sample_insufficient_event_count, countIf((observed_root_count > 0 AND usage_source IN ('unavailable','unknown')) OR (delegated_count > 0 AND delegated_usage_source IN ('unavailable','unknown')) OR (guardian_count > 0 AND guardian_usage_source IN ('unavailable','unknown'))) AS usage_missing_count, sum(fallback_rollout_count + delegated_fallback_rollout_count + guardian_fallback_rollout_count) AS usage_fallback_count, sum(input_tokens) AS input_tokens, sum(cached_input_tokens) AS cached_input_tokens, sum(non_cached_input_tokens) AS non_cached_input_tokens, sum(output_tokens) AS output_tokens, sum(reasoning_output_tokens) AS reasoning_output_tokens, sum(total_tokens) AS total_tokens, sum(delegated_total_tokens) AS delegated_total_tokens, sum(guardian_total_tokens) AS guardian_total_tokens, sum(compactions) AS compactions, sum(long_turn_count) AS long_turn_count, sum(exact_repeated_call_groups) AS exact_repeated_call_groups, sum(calls_in_exact_repeated_groups) AS calls_in_exact_repeated_groups, sum(nonzero_exit_count + timeout_count + rejected_count) AS failure_signal_count, sum(tool_call_count) AS tool_call_count, sum(user_messages_with_text) AS user_messages_with_text, sum(short_message_count) AS short_message_count, sum(broad_scope_message_count) AS broad_scope_message_count, sum(boundary_review_root_count) AS boundary_review_root_count, sum(long_lived_root_count) AS long_lived_root_count, sum(verification_tool_calls) AS verification_tool_call_count, sum(verification_success_count) AS verification_success_count, sum(verification_failure_count) AS verification_failure_count, sum(verification_unresolved_count) AS verification_unresolved_count, sum(guardian_review_count) AS guardian_review_total, sum(guardian_workspace_attributed_review_count) AS guardian_workspace_attributed_review_count FROM groundline.basic_active WHERE ifNull(period_end, generated_at) > parseDateTimeBestEffort({start:String}) AND ifNull(period_end, generated_at) <= parseDateTimeBestEffort({end:String}) FORMAT JSONEachRow"#;
const REPORT_FLEET_QUERY: &str = r#"WITH policy AS (SELECT argMax(latest_version, updated_at) AS latest_version FROM groundline.release_policy FINAL WHERE policy_key='stable'), enrolled AS (SELECT collector_id, created_at, enrollment_schema_version, os_family, runtime_family, execution_mode, groundline_version FROM groundline.collectors FINAL WHERE revoked=0), any_events AS (SELECT collector_id, max(received_at) AS last_seen FROM groundline.basic_active GROUP BY collector_id), reporting AS (SELECT collector_id FROM groundline.basic_active WHERE ifNull(period_end, generated_at) > parseDateTimeBestEffort({start:String}) AND ifNull(period_end, generated_at) <= parseDateTimeBestEffort({end:String}) GROUP BY collector_id), current_events AS (SELECT collector_id, received_at, ifNull(period_end, generated_at) AS event_time FROM groundline.basic_active CROSS JOIN policy WHERE groundline_version=policy.latest_version), current_observed AS (SELECT collector_id, max(received_at) AS last_seen FROM current_events GROUP BY collector_id), current_reporting AS (SELECT collector_id FROM current_events WHERE event_time > parseDateTimeBestEffort({start:String}) AND event_time <= parseDateTimeBestEffort({end:String}) GROUP BY collector_id) SELECT policy.latest_version AS policy_latest_version, count() AS enrolled_installation_count, countIf(enrollment_schema_version=2 AND os_family!='unknown' AND runtime_family!='unknown' AND execution_mode!='unknown' AND groundline_version!='unknown') AS metadata_known_installation_count, count() - metadata_known_installation_count AS metadata_unknown_installation_count, countIf(any_events.collector_id IS NOT NULL) AS observed_installation_count, countIf(reporting.collector_id IS NOT NULL) AS reporting_installation_count, countIf(any_events.last_seen >= now('UTC') - INTERVAL 7 DAY) AS recent_installation_count, countIf(any_events.collector_id IS NULL) AS never_reported_installation_count, countIf(any_events.collector_id IS NULL AND enrolled.created_at > now('UTC') - INTERVAL 24 HOUR) AS pending_initial_report_installation_count, countIf(any_events.collector_id IS NULL AND enrolled.created_at <= now('UTC') - INTERVAL 24 HOUR) AS overdue_never_reported_installation_count, countIf(any_events.collector_id IS NOT NULL AND any_events.last_seen < now('UTC') - INTERVAL 7 DAY) AS stale_observed_installation_count, countIf(enrolled.groundline_version=policy.latest_version) AS current_package_claim_installation_count, countIf(enrolled.groundline_version=policy.latest_version AND current_observed.collector_id IS NULL) AS current_package_claim_unobserved_installation_count, countIf(current_observed.collector_id IS NOT NULL) AS current_observed_installation_count, countIf(current_reporting.collector_id IS NOT NULL) AS current_reporting_installation_count, countIf(current_observed.last_seen >= now('UTC') - INTERVAL 7 DAY) AS current_recent_installation_count, if(countIf(any_events.last_seen IS NOT NULL)=0, CAST(NULL, 'Nullable(String)'), formatDateTime(max(any_events.last_seen), '%Y-%m-%dT%H:%i:%SZ', 'UTC')) AS latest_received_at_utc, toUInt8(max(any_events.last_seen) >= now('UTC') - INTERVAL 48 HOUR) AS fresh FROM enrolled CROSS JOIN policy LEFT JOIN any_events USING collector_id LEFT JOIN reporting USING collector_id LEFT JOIN current_observed USING collector_id LEFT JOIN current_reporting USING collector_id GROUP BY policy.latest_version FORMAT JSONEachRow"#;
const REPORT_STORAGE_QUERY: &str = r#"WITH policy AS (SELECT argMax(retention_days,updated_at) AS retention_days FROM groundline.release_policy FINAL WHERE policy_key='stable'), logical AS (SELECT event_id, received_at, generated_at FROM groundline.basic_active WHERE ifNull(period_end, generated_at) > parseDateTimeBestEffort({start:String}) AND ifNull(period_end, generated_at) <= parseDateTimeBestEffort({end:String})), active AS (SELECT collector_id,current_generation FROM groundline.collectors FINAL WHERE revoked=0), stored AS (SELECT events.event_id FROM groundline.basic_weekly events INNER JOIN active ON events.collector_id=active.collector_id AND events.collection_generation=active.current_generation INNER JOIN logical USING event_id) SELECT (SELECT count() FROM stored) AS stored_event_row_count, (SELECT count() FROM logical) AS deduplicated_event_count, stored_event_row_count-deduplicated_event_count AS duplicate_event_row_count, (SELECT count() FROM groundline.basic_weekly CROSS JOIN policy WHERE received_at < now('UTC') - toIntervalDay(policy.retention_days)) AS ttl_expired_event_row_count, (SELECT countIf(dateDiff('second',generated_at,received_at)>21600) FROM logical) AS delayed_delivery_event_count, (SELECT countIf(dateDiff('second',generated_at,received_at)>86400) FROM logical) AS overdue_delivery_event_count, (SELECT countIf(generated_at>received_at+INTERVAL 5 MINUTE) FROM logical) AS clock_skew_event_count FORMAT JSONEachRow"#;
const REPORT_EVENT_COHORT_QUERY: &str = r#"SELECT dimension,value,count() AS count FROM (SELECT 'schema_version' dimension,toString(schema_version) value FROM groundline.basic_active WHERE ifNull(period_end,generated_at)>parseDateTimeBestEffort({start:String}) AND ifNull(period_end,generated_at)<=parseDateTimeBestEffort({end:String}) UNION ALL SELECT 'groundline_version',groundline_version FROM groundline.basic_active WHERE ifNull(period_end,generated_at)>parseDateTimeBestEffort({start:String}) AND ifNull(period_end,generated_at)<=parseDateTimeBestEffort({end:String}) UNION ALL SELECT 'os_family',os_family FROM groundline.basic_active WHERE ifNull(period_end,generated_at)>parseDateTimeBestEffort({start:String}) AND ifNull(period_end,generated_at)<=parseDateTimeBestEffort({end:String}) UNION ALL SELECT 'runtime_family',runtime_family FROM groundline.basic_active WHERE ifNull(period_end,generated_at)>parseDateTimeBestEffort({start:String}) AND ifNull(period_end,generated_at)<=parseDateTimeBestEffort({end:String}) UNION ALL SELECT 'execution_mode',execution_mode FROM groundline.basic_active WHERE ifNull(period_end,generated_at)>parseDateTimeBestEffort({start:String}) AND ifNull(period_end,generated_at)<=parseDateTimeBestEffort({end:String})) GROUP BY dimension,value ORDER BY dimension,value FORMAT JSONEachRow"#;
const REPORT_INSTALL_COHORT_QUERY: &str = r#"SELECT dimension,value,count() AS count FROM (SELECT 'groundline_version' dimension,groundline_version value FROM groundline.collectors FINAL WHERE revoked=0 UNION ALL SELECT 'os_family',os_family FROM groundline.collectors FINAL WHERE revoked=0 UNION ALL SELECT 'runtime_family',runtime_family FROM groundline.collectors FINAL WHERE revoked=0 UNION ALL SELECT 'execution_mode',execution_mode FROM groundline.collectors FINAL WHERE revoked=0) GROUP BY dimension,value ORDER BY dimension,value FORMAT JSONEachRow"#;
const REPORT_MODEL_EFFORT_QUERY: &str = r#"SELECT tupleElement(item,1) AS model_family, tupleElement(item,2) AS effort, sum(tupleElement(item,3)) AS context_count FROM groundline.basic_active ARRAY JOIN arrayZip(model_families,efforts,model_effort_counts) AS item WHERE ifNull(period_end,generated_at)>parseDateTimeBestEffort({start:String}) AND ifNull(period_end,generated_at)<=parseDateTimeBestEffort({end:String}) GROUP BY model_family,effort ORDER BY model_family,effort FORMAT JSONEachRow"#;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("configuration_rejected")]
    Configuration,
    #[error("storage_unavailable")]
    Storage,
    #[error("listen_failed")]
    Listen,
    #[error("server_failed")]
    Server,
}

fn app(state: AppState) -> Router {
    let collector_routes = Router::new()
        .route("/v1/events", post(ingest_event))
        .route("/v1/generations/activate", post(activate_generation))
        .route_layer(from_fn_with_state(state.clone(), admit_collector_request));
    Router::new()
        .route("/healthz", get(health))
        .route("/v3/reports/weekly", get(weekly_report))
        .route("/v1/enroll", post(enroll))
        .route("/v1/collectors/{collector_id}", delete(delete_collector))
        .merge(collector_routes)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state)
}

pub async fn run() -> Result<(), RunError> {
    let config = Config::from_env().map_err(|_| RunError::Configuration)?;
    let clickhouse = ClickHouse::new(&config).map_err(|_| RunError::Configuration)?;
    clickhouse
        .ensure_storage(&config)
        .await
        .map_err(|_| RunError::Storage)?;
    let listen = config.listen;
    let state = AppState {
        config,
        clickhouse,
        rate_limits: Arc::new(Mutex::new(BTreeMap::new())),
        health: Arc::new(tokio::sync::Mutex::new(HealthState::default())),
        health_probe: Arc::new(tokio::sync::Mutex::new(())),
        ingestion_gate: Arc::new(tokio::sync::Mutex::new(())),
        collector_request_permits: Arc::new(tokio::sync::Semaphore::new(
            MAX_CONCURRENT_COLLECTOR_REQUESTS,
        )),
        collector_storage_permits: Arc::new(tokio::sync::Semaphore::new(
            MAX_CONCURRENT_COLLECTOR_STORAGE_REQUESTS,
        )),
        operator_storage_permits: Arc::new(tokio::sync::Semaphore::new(
            MAX_CONCURRENT_OPERATOR_STORAGE_REQUESTS,
        )),
    };
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .json()
        .with_target(false)
        .with_current_span(false)
        .without_time()
        .try_init()
        .ok();
    let app = app(state);
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .map_err(|_| RunError::Listen)?;
    tracing::info!(event = "server_started");
    serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .map_err(|_| RunError::Server)
}

#[cfg(test)]
mod tests {
    use axum::http::{Method, Request};
    use groundline_contracts::event::{CollectorIdentity, ConsentReceipt, build_basic_event};
    use tower::ServiceExt;

    use super::*;

    fn unit_config() -> Config {
        Config {
            listen: "127.0.0.1:8080".parse().expect("socket"),
            clickhouse_url: Url::parse("http://127.0.0.1:18123/").expect("url"),
            clickhouse_database: "groundline".to_owned(),
            clickhouse_user: "groundline".to_owned(),
            clickhouse_password: SecretString::from("x".repeat(32)),
            admin_token: SecretString::from("x".repeat(32)),
            enrollment_token: SecretString::from("e".repeat(32)),
            proxy_token: SecretString::from("p".repeat(32)),
            owner_enrollment_enabled: true,
            latest_version: env!("CARGO_PKG_VERSION").to_owned(),
            minimum_supported_version: env!("CARGO_PKG_VERSION").to_owned(),
            retention_days: DEFAULT_RETENTION_DAYS,
            collector_max_events: DEFAULT_COLLECTOR_MAX_EVENTS,
            collector_max_payload_bytes: DEFAULT_COLLECTOR_MAX_PAYLOAD_BYTES,
            dataset_max_rows: DEFAULT_DATASET_MAX_ROWS,
            dataset_max_bytes: DEFAULT_DATASET_MAX_BYTES,
        }
    }

    fn unit_state(collector_permits: usize, operator_permits: usize) -> AppState {
        let config = unit_config();
        AppState {
            clickhouse: ClickHouse::new(&config).expect("ClickHouse client"),
            config,
            rate_limits: Arc::new(Mutex::new(BTreeMap::new())),
            health: Arc::new(tokio::sync::Mutex::new(HealthState::default())),
            health_probe: Arc::new(tokio::sync::Mutex::new(())),
            ingestion_gate: Arc::new(tokio::sync::Mutex::new(())),
            collector_request_permits: Arc::new(tokio::sync::Semaphore::new(
                MAX_CONCURRENT_COLLECTOR_REQUESTS,
            )),
            collector_storage_permits: Arc::new(tokio::sync::Semaphore::new(collector_permits)),
            operator_storage_permits: Arc::new(tokio::sync::Semaphore::new(operator_permits)),
        }
    }

    fn clickhouse_test_config() -> Config {
        assert_eq!(
            std::env::var("GROUNDLINE_CLICKHOUSE_TEST_ALLOW_MUTATION").as_deref(),
            Ok("true"),
            "set GROUNDLINE_CLICKHOUSE_TEST_ALLOW_MUTATION=true only for an isolated test database"
        );
        let url = Url::parse(
            &std::env::var("GROUNDLINE_CLICKHOUSE_TEST_URL")
                .expect("GROUNDLINE_CLICKHOUSE_TEST_URL is required"),
        )
        .expect("valid ClickHouse test URL");
        let host = url.host_str().expect("ClickHouse test host");
        assert!(
            host == "localhost"
                || host
                    .parse::<IpAddr>()
                    .is_ok_and(|address| address.is_loopback()),
            "ClickHouse integration tests are restricted to loopback"
        );
        Config {
            listen: "127.0.0.1:8080".parse().expect("socket"),
            clickhouse_url: url,
            clickhouse_database: "groundline".to_owned(),
            clickhouse_user: std::env::var("GROUNDLINE_CLICKHOUSE_TEST_USER")
                .expect("GROUNDLINE_CLICKHOUSE_TEST_USER is required"),
            clickhouse_password: SecretString::from(
                std::env::var("GROUNDLINE_CLICKHOUSE_TEST_PASSWORD")
                    .expect("GROUNDLINE_CLICKHOUSE_TEST_PASSWORD is required"),
            ),
            admin_token: SecretString::from("a".repeat(32)),
            enrollment_token: SecretString::from("e".repeat(32)),
            proxy_token: SecretString::from("p".repeat(32)),
            owner_enrollment_enabled: true,
            latest_version: env!("CARGO_PKG_VERSION").to_owned(),
            minimum_supported_version: env!("CARGO_PKG_VERSION").to_owned(),
            retention_days: DEFAULT_RETENTION_DAYS,
            collector_max_events: DEFAULT_COLLECTOR_MAX_EVENTS,
            collector_max_payload_bytes: DEFAULT_COLLECTOR_MAX_PAYLOAD_BYTES,
            dataset_max_rows: DEFAULT_DATASET_MAX_ROWS,
            dataset_max_bytes: DEFAULT_DATASET_MAX_BYTES,
        }
    }

    fn local_request(
        method: Method,
        uri: &str,
        token: &str,
        payload: Option<&Value>,
        headers: &[(&str, String)],
    ) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {token}"));
        if payload.is_some() {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        for (name, value) in headers {
            builder = builder.header(*name, value);
        }
        let mut request = builder
            .body(
                payload
                    .map(|value| Body::from(serde_json::to_vec(value).expect("JSON")))
                    .unwrap_or_else(Body::empty),
            )
            .expect("request");
        request.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:41000".parse::<SocketAddr>().expect("peer"),
        ));
        request
    }

    async fn response_json(response: Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), MAX_CLICKHOUSE_BYTES)
            .await
            .expect("response body");
        serde_json::from_slice(&bytes).expect("response JSON")
    }

    fn integration_event(collector_id: Uuid) -> Value {
        let end = Utc::now();
        let start = end - ChronoDuration::minutes(1);
        let start = start.to_rfc3339_opts(SecondsFormat::Secs, true);
        let end = end.to_rfc3339_opts(SecondsFormat::Secs, true);
        let audit = json!({
            "kind":"groundline-codex-activity-audit",
            "schema":1,
            "status":"PASS",
            "scope":{
                "generated_at":end,
                "selection_mode":"activity_window",
                "requested_window_start":start,
                "requested_window_end":end,
                "requested_days":7,
                "completed_root_sample_count":1,
                "observed_root_sample_count":1,
                "eligible_root_count":1,
                "selected_root_count":1,
                "selection_coverage":1.0,
                "minimum_root_sample_count":1,
                "sample_sufficient":true,
                "delegated_rollout_count":0,
                "guardian_rollout_count":0
            },
            "root":{"status":"PASS"},
            "delegated":{"status":"PASS"},
            "guardian":{"status":"PASS"},
            "mutation_performed":false,
            "raw_content_emitted":false,
            "private_paths_emitted":false,
            "thread_ids_emitted":false,
            "rollout_paths_emitted":false,
            "secret_value_printed":false
        });
        build_basic_event(
            &audit,
            CollectorIdentity {
                instance_id: collector_id,
                os_family: "linux",
                runtime_family: "codex_cli",
                execution_mode: "local_headless",
            },
            ConsentReceipt {
                receipt_id: Uuid::new_v4(),
                accepted_at_utc: &start,
            },
            env!("CARGO_PKG_VERSION"),
            0,
            "manual",
        )
        .expect("valid integration event")
    }

    fn expand_grafana_time_filter(query: &str) -> Option<String> {
        let marker = "$$__timeFilter(";
        let mut remaining = query;
        let mut output = String::new();
        while let Some(start) = remaining.find(marker) {
            output.push_str(&remaining[..start]);
            let expression_start = start + marker.len();
            let mut depth = 1_u32;
            let mut end = None;
            for (offset, character) in remaining[expression_start..].char_indices() {
                match character {
                    '(' => depth = depth.checked_add(1)?,
                    ')' => {
                        depth = depth.checked_sub(1)?;
                        if depth == 0 {
                            end = Some(expression_start + offset);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let end = end?;
            let expression = &remaining[expression_start..end];
            output.push_str(&format!(
                "({expression} >= now('UTC') - INTERVAL 30 DAY AND {expression} <= now('UTC'))"
            ));
            remaining = &remaining[end + 1..];
        }
        output.push_str(remaining);
        Some(output)
    }

    fn collect_dashboard_queries(value: &Value, queries: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                if let Some(query) = object.get("rawSql").and_then(Value::as_str) {
                    queries.push(query.to_owned());
                }
                for value in object.values() {
                    collect_dashboard_queries(value, queries);
                }
            }
            Value::Array(values) => {
                for value in values {
                    collect_dashboard_queries(value, queries);
                }
            }
            _ => {}
        }
    }

    fn dashboard_queries() -> Vec<String> {
        let template = include_str!("../../../infrastructure/compose.template.yaml");
        let dashboard = template
            .split_once("  grafana_dashboard:\n    content: |\n")
            .expect("Grafana dashboard config")
            .1;
        let dashboard = dashboard
            .lines()
            .map(|line| line.strip_prefix("      ").expect("dashboard indentation"))
            .collect::<Vec<_>>()
            .join("\n");
        let dashboard: Value = serde_json::from_str(&dashboard).expect("dashboard JSON");
        let mut queries = Vec::new();
        collect_dashboard_queries(&dashboard, &mut queries);
        queries
    }

    #[test]
    fn constant_time_comparison_rejects_different_lengths() {
        assert!(constant_time_equal("same", "same"));
        assert!(!constant_time_equal("same", "different"));
    }

    #[test]
    fn enrollment_requires_the_owner_credential() {
        let mut config = unit_config();
        let mut headers = HeaderMap::new();
        assert!(require_enrollment(&config, &headers).is_err());

        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", "x".repeat(32))).unwrap(),
        );
        assert!(require_enrollment(&config, &headers).is_err());

        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", "e".repeat(32))).unwrap(),
        );
        assert!(require_enrollment(&config, &headers).is_ok());

        config.owner_enrollment_enabled = false;
        let error = require_enrollment(&config, &headers).expect_err("disabled enrollment");
        assert!(matches!(
            error,
            ApiError::Rejected {
                status: StatusCode::FORBIDDEN,
                reason: "enrollment_disabled"
            }
        ));

        headers.insert(
            "x-groundline-collector-id",
            HeaderValue::from_str(&Uuid::new_v4().to_string()).unwrap(),
        );
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", "e".repeat(32))).unwrap(),
        );
        assert!(require_admin_report(&config, &headers).is_err());
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", "x".repeat(32))).unwrap(),
        );
        assert!(require_admin_report(&config, &headers).is_ok());
    }

    #[test]
    fn authenticated_rate_budgets_are_isolated_by_role_and_collector() {
        let now = Instant::now();
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let mut scopes = BTreeMap::new();
        charge_rate_limit(&mut scopes, RateLimitScope::Collector(first), 1, now).unwrap();
        assert!(charge_rate_limit(&mut scopes, RateLimitScope::Collector(first), 1, now).is_err());
        charge_rate_limit(&mut scopes, RateLimitScope::Collector(second), 1, now).unwrap();
        charge_rate_limit(&mut scopes, RateLimitScope::Admin, 1, now).unwrap();
        charge_rate_limit(
            &mut scopes,
            RateLimitScope::Collector(first),
            1,
            now + Duration::from_secs(61),
        )
        .unwrap();
    }

    #[test]
    fn pre_auth_peer_budgets_are_bounded_and_do_not_consume_authenticated_scopes() {
        let now = Instant::now();
        let peer = IpAddr::V4(Ipv4Addr::new(100, 64, 0, 7));
        let collector = Uuid::new_v4();
        let mut scopes = BTreeMap::new();
        charge_rate_limit(&mut scopes, RateLimitScope::PreAuthPeer(peer), 1, now).unwrap();
        assert!(charge_rate_limit(&mut scopes, RateLimitScope::PreAuthPeer(peer), 1, now).is_err());
        charge_rate_limit(&mut scopes, RateLimitScope::Collector(collector), 1, now).unwrap();
        charge_rate_limit(&mut scopes, RateLimitScope::Admin, 1, now).unwrap();
    }

    #[test]
    fn saturated_collector_storage_is_shed_without_consuming_operator_capacity() {
        let state = unit_state(1, 1);
        let _collector = state
            .storage_permit(StorageClass::Collector)
            .expect("first collector permit");
        assert!(matches!(
            state.storage_permit(StorageClass::Collector),
            Err(ApiError::Rejected {
                status: StatusCode::SERVICE_UNAVAILABLE,
                reason: "storage_busy"
            })
        ));
        let _operator = state
            .storage_permit(StorageClass::Operator)
            .expect("reserved operator permit");
    }

    #[tokio::test]
    async fn invalid_collectors_are_budgeted_before_storage_and_operator_routes_remain_available() {
        let router = app(unit_state(0, 1));
        let collector_id = Uuid::new_v4();
        let headers = [("x-groundline-collector-id", collector_id.to_string())];
        for _ in 0..MAX_PRE_AUTH_REQUESTS_PER_MINUTE {
            let response = router
                .clone()
                .oneshot(local_request(
                    Method::POST,
                    "/v1/events",
                    &"invalid".repeat(8),
                    Some(&json!({})),
                    &headers,
                ))
                .await
                .expect("bounded response");
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        }
        let response = router
            .clone()
            .oneshot(local_request(
                Method::POST,
                "/v1/events",
                &"invalid".repeat(8),
                Some(&json!({"oversized":"x".repeat(MAX_REQUEST_BYTES)})),
                &headers,
            ))
            .await
            .expect("rate-limited response");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let response = router
            .oneshot(local_request(
                Method::GET,
                "/v3/reports/weekly?days=1",
                &"x".repeat(32),
                None,
                &[],
            ))
            .await
            .expect("operator response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn collector_concurrency_is_rejected_before_body_buffering() {
        let state = unit_state(1, 1);
        state.collector_request_permits.close();
        let response = app(state)
            .oneshot(local_request(
                Method::POST,
                "/v1/events",
                &"invalid".repeat(8),
                Some(&json!({"oversized":"x".repeat(MAX_REQUEST_BYTES)})),
                &[("x-groundline-collector-id", Uuid::new_v4().to_string())],
            ))
            .await
            .expect("bounded response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn effective_tailnet_peer_ignores_direct_xff_and_accepts_one_trusted_proxy_hop() {
        let state = unit_state(1, 1);
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("100.64.0.9"));
        let direct = require_tailnet(
            &state,
            "100.64.0.7:41000".parse().expect("direct peer"),
            &headers,
        )
        .expect("direct Tailnet peer");
        assert_eq!(direct, IpAddr::V4(Ipv4Addr::new(100, 64, 0, 7)));

        headers.insert(
            "x-groundline-proxy-token",
            HeaderValue::from_str(&"p".repeat(32)).expect("proxy token"),
        );
        let proxied = require_tailnet(
            &state,
            "192.168.1.10:41000".parse().expect("proxy peer"),
            &headers,
        )
        .expect("trusted proxy");
        assert_eq!(proxied, IpAddr::V4(Ipv4Addr::new(100, 64, 0, 9)));

        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("100.64.0.9, 100.64.0.10"),
        );
        assert!(
            require_tailnet(
                &state,
                "192.168.1.10:41000".parse().expect("proxy peer"),
                &headers,
            )
            .is_err()
        );
    }

    #[test]
    fn dataset_watermark_reserves_administrative_capacity() {
        assert!(!reaches_ingestion_watermark(88, 1, 100));
        assert!(reaches_ingestion_watermark(89, 1, 100));
        assert!(reaches_ingestion_watermark(u64::MAX, 1, u64::MAX));
    }

    #[test]
    fn only_loopback_and_tailnet_addresses_are_directly_accepted() {
        assert!(tailnet_address(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(tailnet_address(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(!tailnet_address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(!tailnet_address(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }

    #[test]
    fn advisory_is_monotonic() {
        let config = Config {
            listen: "127.0.0.1:8080".parse().expect("socket"),
            clickhouse_url: Url::parse("http://clickhouse:8123/").expect("url"),
            clickhouse_database: "groundline".to_owned(),
            clickhouse_user: "groundline".to_owned(),
            clickhouse_password: SecretString::from("x".repeat(32)),
            admin_token: SecretString::from("x".repeat(32)),
            enrollment_token: SecretString::from("e".repeat(32)),
            proxy_token: SecretString::from("x".repeat(32)),
            owner_enrollment_enabled: true,
            latest_version: "0.20.0".to_owned(),
            minimum_supported_version: "0.20.0".to_owned(),
            retention_days: DEFAULT_RETENTION_DAYS,
            collector_max_events: DEFAULT_COLLECTOR_MAX_EVENTS,
            collector_max_payload_bytes: DEFAULT_COLLECTOR_MAX_PAYLOAD_BYTES,
            dataset_max_rows: DEFAULT_DATASET_MAX_ROWS,
            dataset_max_bytes: DEFAULT_DATASET_MAX_BYTES,
        };
        assert_eq!(
            update_advisory(&config, Some("0.17.9"))
                .and_then(|value| value["status"].as_str().map(str::to_owned)),
            Some("update_required".to_owned())
        );
    }

    #[test]
    fn counters_and_generations_fail_closed_at_u32_boundaries() {
        assert!(checked_u32_sum(u64::from(u32::MAX), 1).is_err());
        assert_eq!(
            checked_u32_sum(u64::from(u32::MAX) - 1, 1).unwrap(),
            u64::from(u32::MAX)
        );
        assert_eq!(u32::MAX.checked_add(1), None);
    }

    #[tokio::test]
    #[ignore = "requires an isolated loopback ClickHouse and explicit mutation opt-in"]
    async fn clickhouse_schema_report_and_grafana_queries_are_executable() {
        let config = clickhouse_test_config();
        let clickhouse = ClickHouse::new(&config).unwrap();
        clickhouse.ensure_storage(&config).await.unwrap();
        let params = [
            ("start", "2026-01-01T00:00:00Z".to_owned()),
            ("end", "2026-02-01T00:00:00Z".to_owned()),
        ];
        for query in [
            REPORT_SUMMARY_QUERY,
            REPORT_FLEET_QUERY,
            REPORT_STORAGE_QUERY,
            REPORT_EVENT_COHORT_QUERY,
            REPORT_INSTALL_COHORT_QUERY,
            REPORT_MODEL_EFFORT_QUERY,
        ] {
            clickhouse.request(query, &params, None).await.unwrap();
        }
        let queries = dashboard_queries();
        assert!(queries.len() >= 10, "dashboard query inventory shrank");
        for query in queries {
            let query = expand_grafana_time_filter(&query).expect("bounded Grafana macro");
            clickhouse.request(&query, &[], None).await.unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "requires an isolated loopback ClickHouse and explicit mutation opt-in"]
    async fn clickhouse_collection_duplicate_report_and_deletion_are_end_to_end() {
        let mut config = clickhouse_test_config();
        config.collector_max_events = 1;
        let clickhouse = ClickHouse::new(&config).expect("ClickHouse client");
        clickhouse.ensure_storage(&config).await.expect("schema");
        let state = AppState {
            config: config.clone(),
            clickhouse: clickhouse.clone(),
            rate_limits: Arc::new(Mutex::new(BTreeMap::new())),
            health: Arc::new(tokio::sync::Mutex::new(HealthState::default())),
            health_probe: Arc::new(tokio::sync::Mutex::new(())),
            ingestion_gate: Arc::new(tokio::sync::Mutex::new(())),
            collector_request_permits: Arc::new(tokio::sync::Semaphore::new(
                MAX_CONCURRENT_COLLECTOR_REQUESTS,
            )),
            collector_storage_permits: Arc::new(tokio::sync::Semaphore::new(
                MAX_CONCURRENT_COLLECTOR_STORAGE_REQUESTS,
            )),
            operator_storage_permits: Arc::new(tokio::sync::Semaphore::new(
                MAX_CONCURRENT_OPERATOR_STORAGE_REQUESTS,
            )),
        };
        let router = app(state);
        let collector_id = Uuid::new_v4();
        let collector_token = "c".repeat(32);
        let enrollment = json!({
            "schema_version":2,
            "kind":"groundline-insights-owner-enrollment",
            "collector_instance_id":collector_id,
            "collector_token":collector_token,
            "os_family":"linux",
            "runtime_family":"codex_cli",
            "execution_mode":"local_headless",
            "groundline_version":env!("CARGO_PKG_VERSION")
        });
        let response = router
            .clone()
            .oneshot(local_request(
                Method::POST,
                "/v1/enroll",
                &"e".repeat(32),
                Some(&enrollment),
                &[],
            ))
            .await
            .expect("enroll response");
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response_json(response).await["outcome"], "accepted");

        let event = integration_event(collector_id);
        let event_id = event["event_id"].as_str().expect("event id");
        let event_headers = [
            ("x-groundline-collector-id", collector_id.to_string()),
            (
                "idempotency-key",
                event["idempotency_key"]
                    .as_str()
                    .expect("idempotency key")
                    .to_owned(),
            ),
            ("x-groundline-version", env!("CARGO_PKG_VERSION").to_owned()),
        ];
        let response = router
            .clone()
            .oneshot(local_request(
                Method::POST,
                "/v1/events",
                &collector_token,
                Some(&event),
                &event_headers,
            ))
            .await
            .expect("event response");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(response_json(response).await["outcome"], "accepted");

        let response = router
            .clone()
            .oneshot(local_request(
                Method::POST,
                "/v1/events",
                &collector_token,
                Some(&event),
                &event_headers,
            ))
            .await
            .expect("duplicate response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["outcome"], "duplicate");

        let second_event = integration_event(collector_id);
        let second_headers = [
            ("x-groundline-collector-id", collector_id.to_string()),
            (
                "idempotency-key",
                second_event["idempotency_key"]
                    .as_str()
                    .expect("idempotency key")
                    .to_owned(),
            ),
            ("x-groundline-version", env!("CARGO_PKG_VERSION").to_owned()),
        ];
        let response = router
            .clone()
            .oneshot(local_request(
                Method::POST,
                "/v1/events",
                &collector_token,
                Some(&second_event),
                &second_headers,
            ))
            .await
            .expect("quota response");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let physical_count = clickhouse
            .request(
                "SELECT count() FROM groundline.basic_weekly WHERE event_id = {event_id:UUID} FORMAT TabSeparated",
                &[("event_id", event_id.to_owned())],
                None,
            )
            .await
            .expect("physical event count");
        assert_eq!(physical_count, b"1\n");

        let report_end = Utc::now();
        let report_start = report_end - ChronoDuration::days(7);
        let report_params = [
            (
                "start",
                report_start.to_rfc3339_opts(SecondsFormat::Secs, true),
            ),
            ("end", report_end.to_rfc3339_opts(SecondsFormat::Secs, true)),
        ];
        for (name, query, params) in [
            ("summary", REPORT_SUMMARY_QUERY, report_params.as_slice()),
            ("fleet", REPORT_FLEET_QUERY, report_params.as_slice()),
            ("storage", REPORT_STORAGE_QUERY, report_params.as_slice()),
            (
                "event cohorts",
                REPORT_EVENT_COHORT_QUERY,
                report_params.as_slice(),
            ),
            ("install cohorts", REPORT_INSTALL_COHORT_QUERY, &[]),
            (
                "model effort",
                REPORT_MODEL_EFFORT_QUERY,
                report_params.as_slice(),
            ),
        ] {
            clickhouse
                .request(query, params, None)
                .await
                .unwrap_or_else(|_| panic!("{name} query failed with populated storage"));
        }

        let response = router
            .clone()
            .oneshot(local_request(
                Method::GET,
                "/v3/reports/weekly?days=7",
                &collector_token,
                None,
                &[("x-groundline-collector-id", collector_id.to_string())],
            ))
            .await
            .expect("collector report response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let response = router
            .clone()
            .oneshot(local_request(
                Method::GET,
                "/v3/reports/weekly?days=7",
                &"a".repeat(32),
                None,
                &[],
            ))
            .await
            .expect("report response");
        assert_eq!(response.status(), StatusCode::OK);
        let report = response_json(response).await;
        assert_eq!(report["schema_version"], 3);
        assert!(
            report["coverage"]["event_count"].as_u64().unwrap_or(0) >= 1,
            "{report}"
        );
        assert!(
            report["collection_health"]["reporting_installation_count"]
                .as_u64()
                .unwrap_or(0)
                >= 1,
            "{report}"
        );

        let response = router
            .clone()
            .oneshot(local_request(
                Method::DELETE,
                &format!("/v1/collectors/{collector_id}"),
                &"a".repeat(32),
                None,
                &[("x-groundline-delete-confirm", collector_id.to_string())],
            ))
            .await
            .expect("delete response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["deleted"], true);
        assert!(
            clickhouse
                .collector(collector_id)
                .await
                .expect("collector lookup")
                .is_none()
        );
    }
}
