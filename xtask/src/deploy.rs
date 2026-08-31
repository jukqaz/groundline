use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use regex::Regex;
use reqwest::header::{CONTENT_TYPE, HOST};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use url::{Host, Url};
use uuid::Uuid;

use groundline_runtime::local_file::{open_bounded_regular_file, private_for_current_user};

use crate::DeployError as XtaskError;

const MAX_CONFIG_BYTES: usize = 2 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_HEALTH_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_GRAFANA_RESPONSE_BYTES: usize = 512 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const AUTH_TIMEOUT: Duration = Duration::from_secs(30);
const INSPECTION_TIMEOUT: Duration = Duration::from_secs(30);
const HEALTH_RPC_TIMEOUT: Duration = Duration::from_secs(30);
const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const DEPLOY_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const STACK_VERIFY_TIMEOUT: Duration = Duration::from_secs(3 * 60);
const RPC_TIMEOUT: Duration = Duration::from_secs(600);
const JOB_POLL_DELAY: Duration = Duration::from_secs(1);
const POST_DEPLOY_HEALTH_TIMEOUT: Duration = Duration::from_secs(4 * 60);
const ROLLBACK_HEALTH_TIMEOUT: Duration = Duration::from_secs(3 * 60);
const HEALTH_ATTEMPTS: usize = 18;
const HEALTH_DELAY: Duration = Duration::from_secs(10);
const ROLLBACK_HEALTH_ATTEMPTS: usize = 12;
const PREFLIGHT_HEALTH_ATTEMPTS: usize = 3;
const PREFLIGHT_HEALTH_DELAY: Duration = Duration::from_secs(5);
const STACK_VERIFY_ATTEMPTS: usize = 24;
const STACK_VERIFY_DELAY: Duration = Duration::from_secs(5);
const GRAFANA_REFERENCE_REF_ID: &str = "H1";
const MANAGED_BLOCKS: &[&str] = &[
    "clickhouse_limits",
    "clickhouse_grafana_user",
    "grafana_datasource",
    "grafana_dashboard",
];
const FLEET_COUNT_FIELDS: &[&str] = &[
    "enrolled_installation_count",
    "metadata_known_installation_count",
    "metadata_unknown_installation_count",
    "observed_installation_count",
    "reporting_installation_count",
    "recent_installation_count",
    "never_reported_installation_count",
    "pending_initial_report_installation_count",
    "overdue_never_reported_installation_count",
    "stale_observed_installation_count",
    "current_package_claim_installation_count",
    "current_package_claim_unobserved_installation_count",
    "current_observed_installation_count",
    "current_reporting_installation_count",
    "current_recent_installation_count",
];
const STORAGE_COUNT_FIELDS: &[&str] = &[
    "stored_event_row_count",
    "deduplicated_event_count",
    "duplicate_event_row_count",
    "delayed_delivery_event_count",
    "overdue_delivery_event_count",
    "clock_skew_event_count",
];
const GRAFANA_FLEET_REFERENCE_QUERY: &str = r#"WITH
policy AS (
    SELECT argMax(latest_version, updated_at) AS latest_version
    FROM groundline.release_policy FINAL
    WHERE policy_key = 'stable'
),
enrolled AS (
    SELECT collector_id, created_at, enrollment_schema_version, os_family,
           runtime_family, execution_mode, groundline_version
    FROM groundline.collectors FINAL
    WHERE revoked = 0
)
SELECT
    (SELECT latest_version FROM policy) AS policy_latest_version,
    count() AS enrolled_installation_count,
    countIf(enrollment_schema_version = 2 AND os_family != 'unknown'
        AND runtime_family != 'unknown' AND execution_mode != 'unknown'
        AND groundline_version != 'unknown') AS metadata_known_installation_count,
    countIf(enrollment_schema_version != 2 OR os_family = 'unknown'
        OR runtime_family = 'unknown' OR execution_mode = 'unknown'
        OR groundline_version = 'unknown') AS metadata_unknown_installation_count,
    countIf(collector_id IN (
        SELECT collector_id FROM groundline.basic_active
    )) AS observed_installation_count,
    countIf(collector_id IN (
        SELECT collector_id FROM groundline.basic_active
        WHERE $__timeFilter(ifNull(period_end, generated_at))
    )) AS reporting_installation_count,
    countIf(collector_id IN (
        SELECT collector_id FROM groundline.basic_active
        WHERE received_at >= now('UTC') - INTERVAL 7 DAY
    )) AS recent_installation_count,
    countIf(collector_id NOT IN (
        SELECT collector_id FROM groundline.basic_active
    )) AS never_reported_installation_count,
    countIf(created_at > now('UTC') - INTERVAL 24 HOUR AND collector_id NOT IN (
        SELECT collector_id FROM groundline.basic_active
    )) AS pending_initial_report_installation_count,
    countIf(created_at <= now('UTC') - INTERVAL 24 HOUR AND collector_id NOT IN (
        SELECT collector_id FROM groundline.basic_active
    )) AS overdue_never_reported_installation_count,
    countIf(collector_id IN (
        SELECT collector_id FROM groundline.basic_active
    ) AND collector_id NOT IN (
        SELECT collector_id FROM groundline.basic_active
        WHERE received_at >= now('UTC') - INTERVAL 7 DAY
    )) AS stale_observed_installation_count,
    countIf(groundline_version = (SELECT latest_version FROM policy))
        AS current_package_claim_installation_count,
    countIf(groundline_version = (SELECT latest_version FROM policy)
        AND collector_id NOT IN (
            SELECT collector_id FROM groundline.basic_active
            WHERE groundline_version = (SELECT latest_version FROM policy)
        )) AS current_package_claim_unobserved_installation_count,
    countIf(collector_id IN (
        SELECT collector_id FROM groundline.basic_active
        WHERE groundline_version = (SELECT latest_version FROM policy)
    )) AS current_observed_installation_count,
    countIf(collector_id IN (
        SELECT collector_id FROM groundline.basic_active
        WHERE groundline_version = (SELECT latest_version FROM policy)
          AND $__timeFilter(ifNull(period_end, generated_at))
    )) AS current_reporting_installation_count,
    countIf(collector_id IN (
        SELECT collector_id FROM groundline.basic_active
        WHERE groundline_version = (SELECT latest_version FROM policy)
          AND received_at >= now('UTC') - INTERVAL 7 DAY
    )) AS current_recent_installation_count
FROM enrolled"#;

fn app_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[a-z][a-z0-9_-]{0,63}$").expect("fixed app regex"))
}

fn username_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[A-Za-z0-9._-]{1,64}$").expect("fixed user regex"))
}

fn image_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^ghcr\.io/jukqaz/groundline-insights-api@sha256:[0-9a-f]{64}$")
            .expect("fixed image regex")
    })
}

fn sha256_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[0-9a-f]{64}$").expect("fixed sha256 regex"))
}

fn stable_version_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
            .expect("fixed stable version regex")
    })
}

fn unresolved_placeholder_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"__[A-Z][A-Z0-9_]*__").expect("fixed unresolved placeholder regex")
    })
}

fn ensure_crypto_provider() -> Result<(), XtaskError> {
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
        Err(XtaskError::DeploymentFailed)
    }
}

fn current_image_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^ghcr\.io/jukqaz/groundline-insights-api(?::[A-Za-z0-9][A-Za-z0-9._-]{0,127}|@sha256:[0-9a-f]{64})$")
            .expect("fixed current image regex")
    })
}

fn valid_enrollment_token(value: &str) -> bool {
    (32..=4096).contains(&value.len())
}

fn required_env(name: &'static str, minimum: usize) -> Result<String, XtaskError> {
    match std::env::var(name) {
        Ok(value) if value.is_empty() => Err(XtaskError::MissingDeploymentInput(name)),
        Ok(value) if !(minimum..=4096).contains(&value.len()) => {
            Err(XtaskError::InvalidDeploymentInput(name))
        }
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Err(XtaskError::MissingDeploymentInput(name)),
        Err(std::env::VarError::NotUnicode(_)) => Err(XtaskError::InvalidDeploymentInput(name)),
    }
}

fn read_private_runtime_config(path: &Path) -> Result<String, XtaskError> {
    let mut file = open_bounded_regular_file(path, 1, MAX_CONFIG_BYTES as u64)
        .map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    if !private_for_current_user(&file) {
        return Err(XtaskError::InvalidRuntimeConfiguration);
    }
    let mut bytes = Vec::with_capacity(
        file.metadata()
            .map_err(|_| XtaskError::InvalidRuntimeConfiguration)?
            .len() as usize,
    );
    file.by_ref()
        .take(MAX_CONFIG_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(XtaskError::InvalidRuntimeConfiguration);
    }
    String::from_utf8(bytes).map_err(|_| XtaskError::InvalidRuntimeConfiguration)
}

fn line_key(line: &str) -> (&str, usize) {
    let trimmed = line.trim_start_matches(' ');
    (
        trimmed.split('#').next().unwrap_or_default().trim_end(),
        line.len() - trimmed.len(),
    )
}

fn block_range(value: &str, name: &str) -> Result<(usize, usize), XtaskError> {
    let lines = value.split_inclusive('\n').collect::<Vec<_>>();
    let mut configs = None;
    let mut configs_end = lines.len();
    for (index, line) in lines.iter().enumerate() {
        let (key, indent) = line_key(line);
        if configs.is_none() && indent == 0 && key == "configs:" {
            configs = Some(index);
        } else if configs.is_some() && indent == 0 && !key.is_empty() {
            configs_end = index;
            break;
        }
    }
    let start = configs.ok_or(XtaskError::DeploymentFailed)?;
    let matches = ((start + 1)..configs_end)
        .filter(|index| {
            let (key, indent) = line_key(lines[*index]);
            indent == 2 && key == format!("{name}:")
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(XtaskError::DeploymentFailed);
    }
    let block_start = matches[0];
    let block_end = ((block_start + 1)..configs_end)
        .find(|index| {
            let (key, indent) = line_key(lines[*index]);
            indent <= 2 && !key.is_empty()
        })
        .unwrap_or(configs_end);
    Ok((block_start, block_end))
}

fn block(value: &str, name: &str) -> Result<String, XtaskError> {
    let lines = value.split_inclusive('\n').collect::<Vec<_>>();
    let (start, end) = block_range(value, name)?;
    let block = lines[start..end].concat();
    if block.trim().is_empty() || unresolved_placeholder_regex().is_match(&block) {
        return Err(XtaskError::DeploymentFailed);
    }
    Ok(block)
}

fn block_content(value: &str, name: &str) -> Result<String, XtaskError> {
    let source = block(value, name)?;
    let mut lines = source.split_inclusive('\n');
    let header = lines.next().ok_or(XtaskError::DeploymentFailed)?;
    let content_header = lines.next().ok_or(XtaskError::DeploymentFailed)?;
    let header_indent = header.len() - header.trim_start_matches(' ').len();
    let content_indent = content_header.len() - content_header.trim_start_matches(' ').len();
    if header_indent != 2 || content_indent != 4 || content_header.trim() != "content: |" {
        return Err(XtaskError::DeploymentFailed);
    }
    let body_indent = content_indent + 2;
    let mut output = String::new();
    for line in lines {
        if line.trim().is_empty() {
            output.push('\n');
            continue;
        }
        if line.len() < body_indent || !line.as_bytes()[..body_indent].iter().all(|b| *b == b' ') {
            return Err(XtaskError::DeploymentFailed);
        }
        output.push_str(&line[body_indent..]);
    }
    if output.trim().is_empty() || output.len() > MAX_CONFIG_BYTES {
        return Err(XtaskError::DeploymentFailed);
    }
    Ok(output)
}

fn template_minimum_supported_version(value: &str) -> Result<String, XtaskError> {
    let mut services_indent = None;
    let mut api_indent = None;
    let mut matches = Vec::new();
    for line in value.lines() {
        let (key, indent) = line_key(line);
        if key == "services:" {
            services_indent = Some(indent);
            api_indent = None;
        } else if services_indent.is_some_and(|services| indent <= services)
            && !key.is_empty()
            && key != "services:"
        {
            services_indent = None;
            api_indent = None;
        } else if services_indent.is_some() && key == "api:" {
            api_indent = Some(indent);
        } else if api_indent.is_some_and(|api| indent <= api) && !key.is_empty() && key != "api:" {
            api_indent = None;
        }
        if api_indent.is_some_and(|api| indent > api)
            && key.starts_with("GROUNDLINE_MINIMUM_SUPPORTED_VERSION:")
        {
            let version = key
                .split_once(':')
                .map(|(_, value)| value.trim().trim_matches(['\'', '"']))
                .filter(|value| stable_version_regex().is_match(value))
                .ok_or(XtaskError::DeploymentFailed)?;
            matches.push(version.to_owned());
        }
    }
    if matches.len() == 1 {
        Ok(matches.remove(0))
    } else {
        Err(XtaskError::DeploymentFailed)
    }
}

fn update_compose(
    current: &Value,
    template: &str,
    image: &str,
    enrollment_token: &SecretString,
) -> Result<Value, XtaskError> {
    if template.len() > MAX_CONFIG_BYTES || !image_regex().is_match(image) {
        return Err(XtaskError::DeploymentFailed);
    }
    if !valid_enrollment_token(enrollment_token.expose_secret()) {
        return Err(XtaskError::DeploymentFailed);
    }
    let current_bytes = serde_json::to_vec(current)?;
    if current_bytes.len() > MAX_CONFIG_BYTES || !current.is_object() {
        return Err(XtaskError::DeploymentFailed);
    }

    let minimum_supported_version = template_minimum_supported_version(template)?;
    let mut managed_contents = BTreeMap::new();
    for name in MANAGED_BLOCKS {
        managed_contents.insert(*name, block_content(template, name)?);
    }

    let mut updated = current.clone();
    {
        let api = updated
            .pointer_mut("/services/api")
            .and_then(Value::as_object_mut)
            .ok_or(XtaskError::DeploymentFailed)?;
        api.get("image")
            .and_then(Value::as_str)
            .filter(|value| current_image_regex().is_match(value))
            .ok_or(XtaskError::DeploymentFailed)?;
        api.insert("image".to_owned(), Value::String(image.to_owned()));

        let environment = api
            .get_mut("environment")
            .and_then(Value::as_object_mut)
            .ok_or(XtaskError::DeploymentFailed)?;
        if environment
            .get("GROUNDLINE_MINIMUM_SUPPORTED_VERSION")
            .is_some_and(|value| !value.is_string())
        {
            return Err(XtaskError::DeploymentFailed);
        }
        match environment.get("GROUNDLINE_ENROLLMENT_TOKEN") {
            Some(Value::String(value)) if valid_enrollment_token(value) => {}
            Some(_) => return Err(XtaskError::DeploymentFailed),
            None => {
                environment.insert(
                    "GROUNDLINE_ENROLLMENT_TOKEN".to_owned(),
                    Value::String(enrollment_token.expose_secret().to_owned()),
                );
            }
        }
        environment.remove("GROUNDLINE_INGEST_TOKEN");
        environment.insert(
            "GROUNDLINE_MINIMUM_SUPPORTED_VERSION".to_owned(),
            Value::String(minimum_supported_version),
        );
    }
    {
        let configs = updated
            .get_mut("configs")
            .and_then(Value::as_object_mut)
            .ok_or(XtaskError::DeploymentFailed)?;
        for (name, content) in managed_contents {
            let existing = configs
                .get(name)
                .and_then(Value::as_object)
                .ok_or(XtaskError::DeploymentFailed)?;
            if existing.len() != 1 || !existing.get("content").is_some_and(Value::is_string) {
                return Err(XtaskError::DeploymentFailed);
            }
            configs.insert(name.to_owned(), json!({"content":content}));
        }
    }
    if serde_json::to_vec(&updated)?.len() > MAX_CONFIG_BYTES {
        return Err(XtaskError::DeploymentFailed);
    }
    Ok(updated)
}

struct RpcClient<S> {
    stream: tokio_tungstenite::WebSocketStream<S>,
}

impl<S> RpcClient<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn call_bounded(
        &mut self,
        method: &str,
        params: Value,
        deadline: Duration,
    ) -> Result<Value, XtaskError> {
        let id = Uuid::new_v4().to_string();
        let request = serde_json::to_string(&json!({
            "jsonrpc":"2.0",
            "id":id,
            "method":method,
            "params":params,
        }))?;
        timeout(deadline, async {
            self.stream
                .send(Message::Text(request.into()))
                .await
                .map_err(|_| XtaskError::DeploymentFailed)?;
            loop {
                let message = self
                    .stream
                    .next()
                    .await
                    .ok_or(XtaskError::DeploymentFailed)?
                    .map_err(|_| XtaskError::DeploymentFailed)?;
                match message {
                    Message::Text(text) => {
                        if text.len() > MAX_RESPONSE_BYTES {
                            return Err(XtaskError::DeploymentFailed);
                        }
                        let response: Value = serde_json::from_str(&text)?;
                        if response.get("id").and_then(Value::as_str) != Some(id.as_str()) {
                            continue;
                        }
                        if response.get("error").is_some() {
                            return Err(XtaskError::DeploymentFailed);
                        }
                        return response
                            .get("result")
                            .cloned()
                            .ok_or(XtaskError::DeploymentFailed);
                    }
                    Message::Ping(value) => self
                        .stream
                        .send(Message::Pong(value))
                        .await
                        .map_err(|_| XtaskError::DeploymentFailed)?,
                    Message::Close(_) | Message::Binary(_) => {
                        return Err(XtaskError::DeploymentFailed);
                    }
                    _ => {}
                }
            }
        })
        .await
        .map_err(|_| XtaskError::DeploymentFailed)?
    }

    async fn call_job_success(&mut self, method: &str, params: Value) -> Result<bool, XtaskError> {
        self.call_job_success_bounded(method, params, RPC_TIMEOUT, JOB_POLL_DELAY)
            .await
    }

    async fn call_job_success_bounded(
        &mut self,
        method: &str,
        params: Value,
        deadline: Duration,
        poll_delay: Duration,
    ) -> Result<bool, XtaskError> {
        timeout(deadline, async {
            let job_id = self
                .call_bounded(method, params, HEALTH_RPC_TIMEOUT)
                .await?
                .as_u64()
                .ok_or(XtaskError::DeploymentFailed)?;
            loop {
                let job = self
                    .call_bounded(
                        "core.get_jobs",
                        json!([[["id", "=", job_id]], {"get": true}]),
                        HEALTH_RPC_TIMEOUT,
                    )
                    .await?;
                match job.get("state").and_then(Value::as_str) {
                    Some("SUCCESS") => return Ok(true),
                    Some("FAILED") | Some("ABORTED") => return Ok(false),
                    Some("WAITING") | Some("RUNNING") => sleep(poll_delay).await,
                    _ => return Err(XtaskError::DeploymentFailed),
                }
            }
        })
        .await
        .map_err(|_| XtaskError::DeploymentFailed)?
    }
}

fn health_url_allowed(url: &Url) -> bool {
    if url.username() != "" || url.password().is_some() || url.host().is_none() {
        return false;
    }
    match url.scheme() {
        "https" => true,
        "http" => matches!(url.host(), Some(Host::Ipv4(address)) if {
            let octets = address.octets();
            octets[0] == 100 && (64..=127).contains(&octets[1])
        }),
        _ => false,
    }
}

async fn bounded_json(mut response: reqwest::Response, maximum: usize) -> Option<Value> {
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|value| value > maximum as u64)
    {
        return None;
    }
    let mut bytes = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if bytes
                    .len()
                    .checked_add(chunk.len())
                    .is_none_or(|value| value > maximum)
                {
                    return None;
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    serde_json::from_slice(&bytes).ok()
}

#[derive(Debug)]
struct GrafanaQueryPlan {
    queries: Vec<Value>,
    semantic_refs: BTreeMap<&'static str, String>,
}

fn grafana_query_plan(template: &str) -> Result<GrafanaQueryPlan, XtaskError> {
    let dashboard: Value = serde_json::from_str(&block_content(template, "grafana_dashboard")?)?;
    let panels = dashboard
        .get("panels")
        .and_then(Value::as_array)
        .ok_or(XtaskError::DeploymentFailed)?;
    let expected_datasource = json!({
        "type":"grafana-clickhouse-datasource",
        "uid":"groundline-clickhouse",
    });
    let mut queries = Vec::new();
    for panel in panels {
        if panel.get("datasource") != Some(&expected_datasource) {
            return Err(XtaskError::DeploymentFailed);
        }
        let Some(targets) = panel.get("targets").and_then(Value::as_array) else {
            continue;
        };
        for target in targets {
            if target.get("datasource") != Some(&expected_datasource) {
                return Err(XtaskError::DeploymentFailed);
            }
            let raw_sql = target
                .get("rawSql")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or(XtaskError::DeploymentFailed)?
                .replace("$$__", "$__");
            let ref_id = format!("Q{}", queries.len() + 1);
            queries.push(json!({
                "refId":ref_id,
                "datasource":expected_datasource.clone(),
                "rawSql":raw_sql,
                "format":target.get("format").cloned().unwrap_or_else(|| json!(1)),
                "queryType":target.get("queryType").cloned().unwrap_or_else(|| json!("table")),
            }));
        }
    }
    if queries.is_empty() || queries.len() > 64 {
        return Err(XtaskError::DeploymentFailed);
    }
    let markers = [
        ("roster", ["AS reporting_status", "AS update_status"]),
        (
            "fleet",
            [
                "pending_initial_report_installation_count",
                "current_package_claim_unobserved_installation_count",
            ],
        ),
        (
            "storage",
            ["stored_event_row_count", "clock_skew_event_count"],
        ),
    ];
    let mut semantic_refs = BTreeMap::new();
    for (name, required) in markers {
        let matches = queries
            .iter()
            .filter(|query| {
                query
                    .get("rawSql")
                    .and_then(Value::as_str)
                    .is_some_and(|sql| required.iter().all(|marker| sql.contains(marker)))
            })
            .filter_map(|query| query.get("refId").and_then(Value::as_str))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(XtaskError::DeploymentFailed);
        }
        semantic_refs.insert(name, matches[0].to_owned());
    }
    queries.push(json!({
        "refId":GRAFANA_REFERENCE_REF_ID,
        "datasource":expected_datasource,
        "rawSql":GRAFANA_FLEET_REFERENCE_QUERY,
        "format":1,
        "queryType":"table",
    }));
    semantic_refs.insert("fleet_reference", GRAFANA_REFERENCE_REF_ID.to_owned());
    Ok(GrafanaQueryPlan {
        queries,
        semantic_refs,
    })
}

fn grafana_frame_rows(result: &Value) -> Option<Vec<serde_json::Map<String, Value>>> {
    let frames = result.get("frames")?.as_array()?;
    let mut rows = Vec::new();
    for frame in frames {
        let fields = frame.pointer("/schema/fields")?.as_array()?;
        let values = frame.pointer("/data/values")?.as_array()?;
        let names = fields
            .iter()
            .map(|field| field.get("name")?.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?;
        if names.is_empty()
            || names.iter().collect::<BTreeSet<_>>().len() != names.len()
            || names.len() != values.len()
        {
            return None;
        }
        let columns = values
            .iter()
            .map(Value::as_array)
            .collect::<Option<Vec<_>>>()?;
        let lengths = columns
            .iter()
            .map(|column| column.len())
            .collect::<BTreeSet<_>>();
        if lengths.len() != 1 {
            return None;
        }
        let mut frame_rows = vec![serde_json::Map::new(); columns[0].len()];
        for (name, column) in names.iter().zip(columns) {
            for (row, value) in frame_rows.iter_mut().zip(column) {
                row.insert(name.clone(), value.clone());
            }
        }
        rows.extend(frame_rows);
    }
    Some(rows)
}

fn nonnegative_count(value: &Value) -> Option<u64> {
    if let Some(value) = value.as_u64() {
        return Some(value);
    }
    let value = value.as_f64()?;
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= u64::MAX as f64 {
        Some(value as u64)
    } else {
        None
    }
}

fn semantic_count_row(
    results: &serde_json::Map<String, Value>,
    ref_id: &str,
    fields: &[&str],
) -> Option<BTreeMap<String, u64>> {
    let rows = grafana_frame_rows(results.get(ref_id)?)?;
    if rows.len() != 1 {
        return None;
    }
    fields
        .iter()
        .map(|name| Some(((*name).to_owned(), nonnegative_count(rows[0].get(*name)?)?)))
        .collect()
}

fn count(counts: &BTreeMap<String, u64>, name: &str) -> Option<u64> {
    counts.get(name).copied()
}

fn fleet_counts_consistent(counts: &BTreeMap<String, u64>) -> bool {
    let Some(enrolled) = count(counts, "enrolled_installation_count") else {
        return false;
    };
    let Some(observed) = count(counts, "observed_installation_count") else {
        return false;
    };
    let Some(reporting) = count(counts, "reporting_installation_count") else {
        return false;
    };
    let Some(recent) = count(counts, "recent_installation_count") else {
        return false;
    };
    let Some(never) = count(counts, "never_reported_installation_count") else {
        return false;
    };
    let Some(current_claim) = count(counts, "current_package_claim_installation_count") else {
        return false;
    };
    let Some(current_observed) = count(counts, "current_observed_installation_count") else {
        return false;
    };
    let Some(current_reporting) = count(counts, "current_reporting_installation_count") else {
        return false;
    };
    let Some(current_recent) = count(counts, "current_recent_installation_count") else {
        return false;
    };
    count(counts, "metadata_known_installation_count").and_then(|known| {
        count(counts, "metadata_unknown_installation_count")
            .and_then(|unknown| known.checked_add(unknown))
    }) == Some(enrolled)
        && observed.checked_add(never) == Some(enrolled)
        && reporting <= observed
        && recent <= reporting
        && count(counts, "pending_initial_report_installation_count").and_then(|pending| {
            count(counts, "overdue_never_reported_installation_count")
                .and_then(|overdue| pending.checked_add(overdue))
        }) == Some(never)
        && count(counts, "stale_observed_installation_count")
            .and_then(|stale| stale.checked_add(recent))
            == Some(observed)
        && current_claim <= enrolled
        && count(
            counts,
            "current_package_claim_unobserved_installation_count",
        )
        .is_some_and(|value| value <= current_claim)
        && current_observed <= enrolled
        && current_reporting <= current_observed
        && current_recent <= current_reporting
}

fn roster_rows_consistent(rows: &[serde_json::Map<String, Value>], enrolled: u64) -> bool {
    if rows.len() as u64 != enrolled {
        return false;
    }
    let reporting_statuses = [
        "초기 보고 대기",
        "최초 보고 지연",
        "최근 보고",
        "보고 지연",
        "Tailnet/수집 확인 필요",
    ];
    let update_statuses = ["미확인", "지원 중단", "업데이트 필요", "최신"];
    rows.iter().all(|row| {
        let reporting = row.get("reporting_status").and_then(Value::as_str);
        let update = row.get("update_status").and_then(Value::as_str);
        if !reporting.is_some_and(|value| reporting_statuses.contains(&value))
            || !update.is_some_and(|value| update_statuses.contains(&value))
            || ["os", "runtime", "execution", "installed_version"]
                .iter()
                .any(|name| {
                    row.get(*name)
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty)
                })
        {
            return false;
        }
        let last_seen = row.get("last_seen").unwrap_or(&Value::Null);
        if last_seen.as_i64() == Some(0)
            || last_seen
                .as_str()
                .is_some_and(|value| value.starts_with("1970-01-01"))
        {
            return false;
        }
        !last_seen.is_null()
            || (matches!(reporting, Some("초기 보고 대기" | "최초 보고 지연"))
                && update == Some("미확인"))
    })
}

fn grafana_semantic_response_ready(
    results: &serde_json::Map<String, Value>,
    refs: &BTreeMap<&'static str, String>,
) -> bool {
    if refs.keys().copied().collect::<BTreeSet<_>>()
        != BTreeSet::from(["fleet", "fleet_reference", "roster", "storage"])
    {
        return false;
    }
    let Some(fleet) = semantic_count_row(results, &refs["fleet"], FLEET_COUNT_FIELDS) else {
        return false;
    };
    let Some(reference) = semantic_count_row(results, &refs["fleet_reference"], FLEET_COUNT_FIELDS)
    else {
        return false;
    };
    let Some(storage) = semantic_count_row(results, &refs["storage"], STORAGE_COUNT_FIELDS) else {
        return false;
    };
    let Some(roster) = results.get(&refs["roster"]).and_then(grafana_frame_rows) else {
        return false;
    };
    let Some(enrolled) = count(&fleet, "enrolled_installation_count") else {
        return false;
    };
    fleet == reference
        && fleet_counts_consistent(&fleet)
        && roster_rows_consistent(&roster, enrolled)
        && count(&storage, "stored_event_row_count")
            .zip(count(&storage, "deduplicated_event_count"))
            .is_some_and(|(stored, deduplicated)| stored >= deduplicated)
        && count(&storage, "duplicate_event_row_count")
            == count(&storage, "stored_event_row_count").and_then(|stored| {
                count(&storage, "deduplicated_event_count")
                    .and_then(|deduplicated| stored.checked_sub(deduplicated))
            })
        && count(&storage, "delayed_delivery_event_count")
            .zip(count(&storage, "deduplicated_event_count"))
            .is_some_and(|(delayed, deduplicated)| delayed <= deduplicated)
        && count(&storage, "overdue_delivery_event_count")
            .zip(count(&storage, "delayed_delivery_event_count"))
            .is_some_and(|(overdue, delayed)| overdue <= delayed)
        && count(&storage, "clock_skew_event_count")
            .zip(count(&storage, "deduplicated_event_count"))
            .is_some_and(|(skew, deduplicated)| skew <= deduplicated)
}

fn grafana_query_ready(value: &Value, plan: &GrafanaQueryPlan) -> bool {
    let Some(results) = value.get("results").and_then(Value::as_object) else {
        return false;
    };
    let expected = plan
        .queries
        .iter()
        .filter_map(|query| query.get("refId").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if results.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return false;
    }
    results.values().all(|result| {
        result.get("status").and_then(Value::as_u64) == Some(200)
            && result.get("error").is_none()
            && result.get("frames").is_some_and(Value::is_array)
    }) && grafana_semantic_response_ready(results, &plan.semantic_refs)
}

async fn grafana_datasource_healthy(
    client: &reqwest::Client,
    base: &Url,
    public_host: &str,
    template: &str,
    admin_password: Option<&SecretString>,
) -> bool {
    let Ok(plan) = grafana_query_plan(template) else {
        return false;
    };
    let mut url = base.clone();
    url.set_path("/api/ds/query");
    url.set_query(None);
    url.set_fragment(None);
    let payload = json!({
        "queries":plan.queries,
        "from":"now-90d",
        "to":"now"
    });
    let Ok(body) = serde_json::to_vec(&payload) else {
        return false;
    };
    if body.len() > MAX_CONFIG_BYTES {
        return false;
    }
    let mut request = client
        .post(url)
        .header(HOST, public_host)
        .header(CONTENT_TYPE, "application/json")
        .body(body);
    if let Some(password) = admin_password {
        request = request.basic_auth("groundline-admin", Some(password.expose_secret()));
    }
    let Ok(response) = request.send().await else {
        return false;
    };
    bounded_json(response, MAX_GRAFANA_RESPONSE_BYTES)
        .await
        .as_ref()
        .is_some_and(|value| grafana_query_ready(value, &plan))
}

fn http_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .ok()
}

async fn service_healthy(
    client: &reqwest::Client,
    url: &str,
    kind: &str,
    public_host: &str,
    template: &str,
    require_datasource: bool,
    grafana_admin_password: Option<&SecretString>,
) -> bool {
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    if !health_url_allowed(&url) {
        return false;
    }
    let mut request = client.get(url.clone());
    if kind == "grafana" {
        request = request.header(HOST, public_host);
    }
    let Ok(response) = request.send().await else {
        return false;
    };
    let Some(value) = bounded_json(response, MAX_HEALTH_RESPONSE_BYTES).await else {
        return false;
    };
    match kind {
        "api" => {
            value.get("status").and_then(Value::as_str) == Some("PASS")
                && value.get("storage_ready").and_then(Value::as_bool) == Some(true)
        }
        "grafana" => {
            (value.get("database").and_then(Value::as_str) == Some("ok")
                || value.get("status").and_then(Value::as_str) == Some("ok"))
                && (!require_datasource
                    || grafana_datasource_healthy(
                        client,
                        &url,
                        public_host,
                        template,
                        grafana_admin_password,
                    )
                    .await)
        }
        _ => false,
    }
}

fn access_url_allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username() == ""
        && url.password().is_none()
        && url.host().is_some()
        && url.port().is_none_or(|port| port == 443)
        && matches!(url.path(), "" | "/")
        && url.query().is_none()
        && url.fragment().is_none()
}

async fn access_gate_healthy(client: &reqwest::Client, raw_url: &str) -> bool {
    let Ok(url) = Url::parse(raw_url) else {
        return false;
    };
    if !access_url_allowed(&url) {
        return false;
    }
    client.get(url).send().await.ok().is_some_and(|response| {
        matches!(
            response.status().as_u16(),
            301 | 302 | 303 | 307 | 308 | 401 | 403
        )
    })
}

async fn app_running<S>(client: &mut RpcClient<S>, app_name: &str) -> bool
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    client
        .call_bounded(
            "app.get_instance",
            json!([app_name, {}]),
            HEALTH_RPC_TIMEOUT,
        )
        .await
        .ok()
        .and_then(|value| {
            value
                .get("state")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .as_deref()
        == Some("RUNNING")
}

struct HealthInputs<'a> {
    app_name: &'a str,
    api_url: &'a str,
    grafana_url: &'a str,
    access_url: &'a str,
    public_host: &'a str,
    template: &'a str,
    grafana_admin_password: &'a SecretString,
}

async fn wait_for_health<S>(
    rpc: &mut RpcClient<S>,
    inputs: &HealthInputs<'_>,
    attempts: usize,
    delay: Duration,
    require_datasource: bool,
) -> bool
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let Some(client) = http_client() else {
        return false;
    };
    for attempt in 0..attempts {
        if app_running(rpc, inputs.app_name).await
            && service_healthy(
                &client,
                inputs.api_url,
                "api",
                inputs.public_host,
                inputs.template,
                false,
                None,
            )
            .await
            && service_healthy(
                &client,
                inputs.grafana_url,
                "grafana",
                inputs.public_host,
                inputs.template,
                require_datasource,
                Some(inputs.grafana_admin_password),
            )
            .await
            && access_gate_healthy(&client, inputs.access_url).await
        {
            return true;
        }
        if attempt + 1 < attempts {
            tokio::time::sleep(delay).await;
        }
    }
    false
}

trait DeploymentOps {
    async fn update(&mut self, config: &Value) -> bool;
    async fn healthy(&mut self, require_datasource: bool) -> bool;
}

struct LiveDeploymentOps<'a, S> {
    rpc: &'a mut RpcClient<S>,
    health: HealthInputs<'a>,
}

impl<S> DeploymentOps for LiveDeploymentOps<'_, S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn update(&mut self, config: &Value) -> bool {
        self.rpc
            .call_job_success(
                "app.update",
                json!([
                    self.health.app_name,
                    {"custom_compose_config":config}
                ]),
            )
            .await
            .unwrap_or(false)
    }

    async fn healthy(&mut self, require_datasource: bool) -> bool {
        let (attempts, delay) = if require_datasource {
            (HEALTH_ATTEMPTS, HEALTH_DELAY)
        } else {
            (ROLLBACK_HEALTH_ATTEMPTS, HEALTH_DELAY)
        };
        let phase_timeout = if require_datasource {
            POST_DEPLOY_HEALTH_TIMEOUT
        } else {
            ROLLBACK_HEALTH_TIMEOUT
        };
        timeout(
            phase_timeout,
            wait_for_health(self.rpc, &self.health, attempts, delay, require_datasource),
        )
        .await
        .unwrap_or(false)
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ApplyFailure {
    VerificationFailed,
    RolledBack,
    RollbackFailed,
}

async fn apply_with_rollback<O>(
    ops: &mut O,
    current: &Value,
    updated: &Value,
) -> Result<bool, ApplyFailure>
where
    O: DeploymentOps,
{
    if current == updated {
        return if ops.healthy(true).await {
            Ok(false)
        } else {
            Err(ApplyFailure::VerificationFailed)
        };
    }
    if ops.update(updated).await && ops.healthy(true).await {
        return Ok(true);
    }
    if !ops.update(current).await || !ops.healthy(false).await {
        return Err(ApplyFailure::RollbackFailed);
    }
    Err(ApplyFailure::RolledBack)
}

struct RuntimeInputs {
    uri: String,
    username: String,
    api_key: SecretString,
    enrollment_token: SecretString,
    grafana_admin_password: SecretString,
    app_name: String,
    api_health: String,
    grafana_health: String,
    access_url: String,
    public_host: String,
    template: String,
    query_count: usize,
}

impl RuntimeInputs {
    fn load(compose_template: &Path) -> Result<Self, XtaskError> {
        let uri = required_env("GROUNDLINE_TRUENAS_URI", 1)?;
        let username = required_env("GROUNDLINE_TRUENAS_USERNAME", 1)?;
        let api_key = SecretString::from(required_env("GROUNDLINE_TRUENAS_API_KEY", 32)?);
        let enrollment_token =
            SecretString::from(required_env("GROUNDLINE_INSIGHTS_ENROLLMENT_TOKEN", 32)?);
        let grafana_admin_password = SecretString::from(required_env(
            "GROUNDLINE_INSIGHTS_GRAFANA_ADMIN_PASSWORD",
            32,
        )?);
        let app_name = std::env::var("GROUNDLINE_TRUENAS_APP_NAME")
            .unwrap_or_else(|_| "groundline-insights".to_owned());
        let api_health = required_env("GROUNDLINE_INSIGHTS_API_HEALTH_URL", 1)?;
        let grafana_health = required_env("GROUNDLINE_INSIGHTS_GRAFANA_HEALTH_URL", 1)?;
        let access_url = required_env("GROUNDLINE_INSIGHTS_ACCESS_URL", 1)?;
        let parsed = Url::parse(&uri).map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
        let api_url =
            Url::parse(&api_health).map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
        let grafana_url =
            Url::parse(&grafana_health).map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
        let access =
            Url::parse(&access_url).map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
        if parsed.scheme() != "wss"
            || parsed.path() != "/api/current"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.username() != ""
            || parsed.password().is_some()
            || !app_regex().is_match(&app_name)
            || !username_regex().is_match(&username)
            || !health_url_allowed(&api_url)
            || !health_url_allowed(&grafana_url)
            || !access_url_allowed(&access)
        {
            return Err(XtaskError::InvalidRuntimeConfiguration);
        }
        let public_host = access
            .host_str()
            .map(str::to_owned)
            .ok_or(XtaskError::InvalidRuntimeConfiguration)?;
        let access_origin = format!("https://{public_host}");
        let template = read_private_runtime_config(compose_template)?;
        let template = template
            .replace("__INSIGHTS_ACCESS_URL__", &access_origin)
            .replace("__INSIGHTS_ACCESS_HOST__", &public_host);
        if template.len() > MAX_CONFIG_BYTES || unresolved_placeholder_regex().is_match(&template) {
            return Err(XtaskError::InvalidRuntimeConfiguration);
        }
        let query_count = grafana_query_plan(&template)
            .map_err(|_| XtaskError::InvalidRuntimeConfiguration)?
            .queries
            .len();
        Ok(Self {
            uri,
            username,
            api_key,
            enrollment_token,
            grafana_admin_password,
            app_name,
            api_health,
            grafana_health,
            access_url,
            public_host,
            template,
            query_count,
        })
    }

    fn health(&self) -> HealthInputs<'_> {
        HealthInputs {
            app_name: &self.app_name,
            api_url: &self.api_health,
            grafana_url: &self.grafana_health,
            access_url: &self.access_url,
            public_host: &self.public_host,
            template: &self.template,
            grafana_admin_password: &self.grafana_admin_password,
        }
    }
}

type LiveRpcClient = RpcClient<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect_authenticated(inputs: &RuntimeInputs) -> Result<LiveRpcClient, XtaskError> {
    let mut config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
    config.max_message_size = Some(MAX_RESPONSE_BYTES);
    config.max_frame_size = Some(MAX_RESPONSE_BYTES);
    let (stream, _) = timeout(
        CONNECT_TIMEOUT,
        tokio_tungstenite::connect_async_with_config(&inputs.uri, Some(config), false),
    )
    .await
    .map_err(|_| XtaskError::ConnectFailed)?
    .map_err(|_| XtaskError::ConnectFailed)?;
    let mut client = RpcClient { stream };
    let auth = client
        .call_bounded(
            "auth.login_ex",
            json!([{
                "mechanism":"API_KEY_PLAIN",
                "username":inputs.username,
                "api_key":inputs.api_key.expose_secret(),
                "login_options":{"user_info":false},
            }]),
            AUTH_TIMEOUT,
        )
        .await
        .map_err(|_| XtaskError::AuthenticationFailed)?;
    if auth.get("response_type").and_then(Value::as_str) != Some("SUCCESS") {
        return Err(XtaskError::AuthenticationFailed);
    }
    Ok(client)
}

async fn inspect_current<S>(client: &mut RpcClient<S>, app_name: &str) -> Result<Value, XtaskError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let app = client
        .call_bounded(
            "app.get_instance",
            json!([app_name, {"extra":{"retrieve_config":true}}]),
            INSPECTION_TIMEOUT,
        )
        .await
        .map_err(|_| XtaskError::InspectionFailed)?;
    if app.get("state").and_then(Value::as_str) != Some("RUNNING") {
        return Err(XtaskError::InspectionFailed);
    }
    let config = app
        .get("config")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or(XtaskError::InvalidCurrentConfiguration)?;
    let serialized =
        serde_json::to_vec(&config).map_err(|_| XtaskError::InvalidCurrentConfiguration)?;
    if serialized.is_empty() || serialized.len() > MAX_CONFIG_BYTES {
        return Err(XtaskError::InvalidCurrentConfiguration);
    }
    Ok(config)
}

fn config_sha256(config: &Value) -> Result<String, XtaskError> {
    let serialized = serde_json::to_vec(config)?;
    if serialized.is_empty() || serialized.len() > MAX_CONFIG_BYTES {
        return Err(XtaskError::InvalidCurrentConfiguration);
    }
    Ok(format!("{:x}", Sha256::digest(serialized)))
}

async fn deploy_async(
    image: &str,
    compose_template: &Path,
    expected_current_config_sha256: &str,
) -> Result<Value, XtaskError> {
    if !image_regex().is_match(image) {
        return Err(XtaskError::InvalidRuntimeConfiguration);
    }
    if !sha256_regex().is_match(expected_current_config_sha256) {
        return Err(XtaskError::InvalidRuntimeConfiguration);
    }
    let inputs = RuntimeInputs::load(compose_template)?;
    let mut client = connect_authenticated(&inputs).await?;
    let current = inspect_current(&mut client, &inputs.app_name).await?;
    let current_config_sha256 = config_sha256(&current)?;
    if expected_current_config_sha256 != current_config_sha256 {
        return Err(XtaskError::InvalidCurrentConfiguration);
    }
    let image_digest = image
        .split_once('@')
        .map(|(_, digest)| digest)
        .ok_or(XtaskError::InvalidRuntimeConfiguration)?;
    let updated = update_compose(&current, &inputs.template, image, &inputs.enrollment_token)
        .map_err(|_| XtaskError::InvalidCurrentConfiguration)?;
    let mut ops = LiveDeploymentOps {
        rpc: &mut client,
        health: inputs.health(),
    };
    let changed = apply_with_rollback(&mut ops, &current, &updated)
        .await
        .map_err(|error| match error {
            ApplyFailure::VerificationFailed => XtaskError::VerificationFailed,
            ApplyFailure::RolledBack => XtaskError::RolledBack,
            ApplyFailure::RollbackFailed => XtaskError::RollbackFailed,
        })?;
    Ok(json!({
        "kind":"groundline-insights-deployment",
        "schema":3,
        "status":"PASS",
        "image_digest":image_digest,
        "changed":changed,
        "rollback_performed":false,
        "api_health_verified":true,
        "grafana_health_verified":true,
        "grafana_query_count":inputs.query_count,
        "grafana_semantics_verified":true,
        "access_gate_verified":true,
        "preflight_config_matched":true,
        "websocket_api":"json-rpc-2.0",
        "authentication":"api_key_plain_over_wss",
        "secret_value_printed":false,
        "private_url_printed":false,
    }))
}

async fn preflight_async(compose_template: &Path) -> Result<Value, XtaskError> {
    let inputs = RuntimeInputs::load(compose_template)?;
    let mut client = connect_authenticated(&inputs).await?;
    let current = inspect_current(&mut client, &inputs.app_name).await?;
    let probe_image = format!(
        "ghcr.io/jukqaz/groundline-insights-api@sha256:{}",
        "0".repeat(64)
    );
    update_compose(
        &current,
        &inputs.template,
        &probe_image,
        &inputs.enrollment_token,
    )
    .map_err(|_| XtaskError::InvalidCurrentConfiguration)?;
    if !wait_for_health(
        &mut client,
        &inputs.health(),
        PREFLIGHT_HEALTH_ATTEMPTS,
        PREFLIGHT_HEALTH_DELAY,
        true,
    )
    .await
    {
        return Err(XtaskError::VerificationFailed);
    }
    Ok(json!({
        "kind":"groundline-insights-deployment-preflight",
        "schema":1,
        "status":"PASS",
        "current_config_sha256":config_sha256(&current)?,
        "mutation_started":false,
        "api_health_verified":true,
        "grafana_health_verified":true,
        "grafana_query_count":inputs.query_count,
        "grafana_semantics_verified":true,
        "access_gate_verified":true,
        "configuration_printed":false,
        "private_url_printed":false,
        "secret_value_printed":false,
    }))
}

fn deployment_runtime() -> Result<tokio::runtime::Runtime, XtaskError> {
    ensure_crypto_provider().map_err(|_| XtaskError::RuntimeFailed)?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| XtaskError::RuntimeFailed)
}

async fn verify_stack_async(
    api_health: &str,
    grafana_health: &str,
    access_url: &str,
    compose_template: &Path,
    secrets_file: &Path,
) -> Result<Value, XtaskError> {
    let api_url = Url::parse(api_health).map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    let grafana_url =
        Url::parse(grafana_health).map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    let access = Url::parse(access_url).map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    if !health_url_allowed(&api_url)
        || !health_url_allowed(&grafana_url)
        || !access_url_allowed(&access)
    {
        return Err(XtaskError::InvalidRuntimeConfiguration);
    }
    let public_host = access
        .host_str()
        .ok_or(XtaskError::InvalidRuntimeConfiguration)?;
    let metadata = std::fs::symlink_metadata(compose_template)
        .map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > MAX_CONFIG_BYTES as u64
    {
        return Err(XtaskError::InvalidRuntimeConfiguration);
    }
    let template = std::fs::read_to_string(compose_template)
        .map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    let plan =
        grafana_query_plan(&template).map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    let grafana_admin_password =
        crate::secret_store::load_private_secret(secrets_file, "GRAFANA_ADMIN_PASSWORD")
            .map_err(|_| XtaskError::InvalidRuntimeConfiguration)?;
    let client = http_client().ok_or(XtaskError::RuntimeFailed)?;
    for attempt in 0..STACK_VERIFY_ATTEMPTS {
        let api_ready = service_healthy(
            &client,
            api_health,
            "api",
            public_host,
            &template,
            false,
            None,
        )
        .await;
        let grafana_ready = api_ready
            && service_healthy(
                &client,
                grafana_health,
                "grafana",
                public_host,
                &template,
                true,
                Some(&grafana_admin_password),
            )
            .await;
        if grafana_ready {
            return Ok(json!({
                "kind":"groundline-insights-stack-verification",
                "schema":1,
                "status":"PASS",
                "api_health_verified":true,
                "grafana_health_verified":true,
                "grafana_authentication_verified":true,
                "grafana_datasource_verified":true,
                "grafana_query_count":plan.queries.len(),
                "grafana_semantics_verified":true,
                "access_origin_validated":true,
                "external_tls_gate_verified":false,
                "mutation_performed":false,
                "private_url_printed":false,
                "secret_value_printed":false,
            }));
        }
        if attempt + 1 < STACK_VERIFY_ATTEMPTS {
            sleep(STACK_VERIFY_DELAY).await;
        }
    }
    Err(XtaskError::VerificationFailed)
}

pub fn verify_stack(
    api_health: &str,
    grafana_health: &str,
    access_url: &str,
    compose_template: &Path,
    secrets_file: &Path,
) -> Result<Value, XtaskError> {
    let runtime = deployment_runtime()?;
    runtime.block_on(async {
        timeout(
            STACK_VERIFY_TIMEOUT,
            verify_stack_async(
                api_health,
                grafana_health,
                access_url,
                compose_template,
                secrets_file,
            ),
        )
        .await
        .map_err(|_| XtaskError::RuntimeFailed)?
    })
}

pub fn preflight(compose_template: &Path) -> Result<Value, XtaskError> {
    let runtime = deployment_runtime()?;
    runtime.block_on(async {
        timeout(PREFLIGHT_TIMEOUT, preflight_async(compose_template))
            .await
            .map_err(|_| XtaskError::RuntimeFailed)?
    })
}

pub fn deploy(
    image: &str,
    compose_template: &Path,
    expected_current_config_sha256: &str,
) -> Result<Value, XtaskError> {
    let runtime = deployment_runtime()?;
    runtime.block_on(async {
        timeout(
            DEPLOY_TIMEOUT,
            deploy_async(image, compose_template, expected_current_config_sha256),
        )
        .await
        .map_err(|_| XtaskError::DeploymentTimedOut)?
    })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use secrecy::SecretString;
    use serde_json::json;
    use tokio::io::duplex;
    use tokio_tungstenite::WebSocketStream;
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::tungstenite::protocol::Role;
    use url::Url;

    use super::{
        ApplyFailure, DeploymentOps, RpcClient, access_url_allowed, apply_with_rollback,
        config_sha256, deploy, ensure_crypto_provider, grafana_query_plan, grafana_query_ready,
        health_url_allowed, inspect_current, read_private_runtime_config, sha256_regex,
        update_compose, verify_stack,
    };

    fn enrollment_credential() -> SecretString {
        SecretString::from("e".repeat(32))
    }

    fn compose(image: &str, dashboard: &str) -> String {
        format!(
            "services:\n  api:\n    image: {image}\n    environment:\n      GROUNDLINE_INGEST_TOKEN: \"legacy\"\n      GROUNDLINE_MINIMUM_SUPPORTED_VERSION: \"0.13.0\"\n  other:\n    image: other:1\nconfigs:\n  clickhouse_limits:\n    content: |\n      limits\n  clickhouse_grafana_user:\n    content: |\n      user\n  grafana_datasource:\n    content: |\n      datasource\n  grafana_dashboard:\n    content: |\n      {dashboard}\nnetworks:\n  data:\n"
        )
    }

    fn current_config(image: &str, dashboard: &str) -> serde_json::Value {
        json!({
            "name":"groundline-insights",
            "services":{
                "api":{
                    "image":image,
                    "environment":{
                        "GROUNDLINE_INGEST_TOKEN":"legacy",
                        "PRESERVED_SECRET":"unchanged"
                    }
                },
                "other":{"image":"other:1"}
            },
            "configs":{
                "clickhouse_limits":{"content":"limits-old\n"},
                "clickhouse_grafana_user":{"content":"user-old\n"},
                "clickhouse_schema":{"content":"schema-unchanged\n"},
                "grafana_datasource":{"content":"datasource-old\n"},
                "grafana_dashboard":{"content":dashboard}
            },
            "networks":{"data":{}}
        })
    }

    #[test]
    fn migration_changes_only_the_api_image_and_allowlisted_blocks() {
        let current = current_config("ghcr.io/jukqaz/groundline-insights-api:0.19.3", "old\n");
        let template = compose("__INSIGHTS_API_IMAGE__", "new").replace("0.13.0", "0.18.0");
        let image = format!(
            "ghcr.io/jukqaz/groundline-insights-api@sha256:{}",
            "a".repeat(64)
        );
        let updated =
            update_compose(&current, &template, &image, &enrollment_credential()).unwrap();
        assert_eq!(
            updated
                .pointer("/services/api/image")
                .and_then(|value| value.as_str()),
            Some(image.as_str())
        );
        assert_eq!(
            updated.pointer("/services/other/image"),
            current.pointer("/services/other/image")
        );
        assert_eq!(
            updated.pointer("/configs/grafana_dashboard/content"),
            Some(&json!("new\n"))
        );
        assert_eq!(
            updated.pointer("/configs/clickhouse_schema"),
            current.pointer("/configs/clickhouse_schema")
        );
        assert!(
            updated
                .pointer("/services/api/environment/GROUNDLINE_INGEST_TOKEN")
                .is_none()
        );
        assert_eq!(
            updated.pointer("/services/api/environment/GROUNDLINE_MINIMUM_SUPPORTED_VERSION"),
            Some(&json!("0.18.0"))
        );
        assert_eq!(
            updated.pointer("/services/api/environment/PRESERVED_SECRET"),
            current.pointer("/services/api/environment/PRESERVED_SECRET")
        );
        assert_eq!(
            updated.pointer("/services/api/environment/GROUNDLINE_ENROLLMENT_TOKEN"),
            Some(&json!("e".repeat(32)))
        );
    }

    #[test]
    fn runtime_config_reader_requires_a_private_bounded_regular_file() {
        use groundline_runtime::local_file::atomic_write_private;
        use tempfile::tempdir;

        let root = tempdir().expect("temporary directory");
        let config = root.path().join("compose.yaml");
        atomic_write_private(&config, b"services: {}\n").expect("private config");
        assert_eq!(
            read_private_runtime_config(&config).expect("read private config"),
            "services: {}\n"
        );
        assert!(read_private_runtime_config(root.path()).is_err());

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&config, root.path().join("compose-link.yaml"))
                .expect("config symlink");
            assert!(read_private_runtime_config(&root.path().join("compose-link.yaml")).is_err());
        }
    }

    #[test]
    fn migration_uses_the_immutable_template_version_not_the_controller_version() {
        let current = current_config("ghcr.io/jukqaz/groundline-insights-api:0.19.3", "old\n");
        let template = compose("__INSIGHTS_API_IMAGE__", "new").replace("0.13.0", "1.2.3");
        let image = format!(
            "ghcr.io/jukqaz/groundline-insights-api@sha256:{}",
            "a".repeat(64)
        );
        let updated =
            update_compose(&current, &template, &image, &enrollment_credential()).unwrap();
        assert_eq!(
            updated.pointer("/services/api/environment/GROUNDLINE_MINIMUM_SUPPORTED_VERSION"),
            Some(&json!("1.2.3"))
        );
        assert_ne!(
            updated.pointer("/services/api/environment/GROUNDLINE_MINIMUM_SUPPORTED_VERSION"),
            Some(&json!(env!("CARGO_PKG_VERSION")))
        );
    }

    #[test]
    fn migration_preserves_an_existing_enrollment_credential() {
        let mut current = current_config("ghcr.io/jukqaz/groundline-insights-api:0.19.3", "old\n");
        current["services"]["api"]["environment"]["GROUNDLINE_ENROLLMENT_TOKEN"] =
            json!("p".repeat(32));
        let template = compose("__INSIGHTS_API_IMAGE__", "new");
        let image = format!(
            "ghcr.io/jukqaz/groundline-insights-api@sha256:{}",
            "a".repeat(64)
        );
        let updated =
            update_compose(&current, &template, &image, &enrollment_credential()).unwrap();
        assert_eq!(
            updated.pointer("/services/api/environment/GROUNDLINE_ENROLLMENT_TOKEN"),
            Some(&json!("p".repeat(32)))
        );
    }

    #[test]
    fn migration_rejects_a_malformed_existing_enrollment_credential() {
        let mut current = current_config("ghcr.io/jukqaz/groundline-insights-api:0.19.3", "old\n");
        current["services"]["api"]["environment"]["GROUNDLINE_ENROLLMENT_TOKEN"] = json!("short");
        let template = compose("__INSIGHTS_API_IMAGE__", "new");
        let image = format!(
            "ghcr.io/jukqaz/groundline-insights-api@sha256:{}",
            "a".repeat(64)
        );
        assert!(update_compose(&current, &template, &image, &enrollment_credential()).is_err());
    }

    #[test]
    fn health_urls_allow_https_and_tailnet_http_only() {
        assert!(health_url_allowed(
            &Url::parse("https://groundline.example/health").unwrap()
        ));
        assert!(health_url_allowed(
            &Url::parse("http://100.64.0.1:18080/healthz").unwrap()
        ));
        assert!(!health_url_allowed(
            &Url::parse("http://100.63.255.255/health").unwrap()
        ));
        assert!(!health_url_allowed(
            &Url::parse("http://192.168.1.2/health").unwrap()
        ));
        assert!(!health_url_allowed(
            &Url::parse("https://user:secret@groundline.example/health").unwrap()
        ));
    }

    #[test]
    fn stack_verifier_rejects_public_plain_http_before_network_access() {
        assert!(
            verify_stack(
                "http://192.168.1.2:18080/healthz",
                "http://192.168.1.2:13000/api/health",
                "https://insights.example.invalid",
                Path::new("infrastructure/compose.template.yaml"),
                Path::new("missing-secrets.json"),
            )
            .is_err()
        );
    }

    fn frame(rows: &[serde_json::Value]) -> serde_json::Value {
        let first = rows[0].as_object().unwrap();
        let names = first.keys().cloned().collect::<Vec<_>>();
        assert!(rows.iter().all(|row| {
            row.as_object()
                .is_some_and(|value| value.keys().eq(names.iter()))
        }));
        json!({
            "status":200,
            "frames":[{
                "schema":{"fields":names.iter().map(|name| json!({"name":name})).collect::<Vec<_>>()},
                "data":{"values":names.iter().map(|name| rows.iter().map(|row| row[name].clone()).collect::<Vec<_>>()).collect::<Vec<_>>()}
            }]
        })
    }

    fn valid_grafana_response() -> (serde_json::Value, super::GrafanaQueryPlan) {
        let template = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../infrastructure/compose.template.yaml"),
        )
        .unwrap();
        let plan = grafana_query_plan(&template).unwrap();
        let mut results = plan
            .queries
            .iter()
            .map(|query| {
                (
                    query["refId"].as_str().unwrap().to_owned(),
                    frame(&[json!({"value":1})]),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let fleet = json!({
            "enrolled_installation_count":2,
            "metadata_known_installation_count":1,
            "metadata_unknown_installation_count":1,
            "observed_installation_count":1,
            "reporting_installation_count":1,
            "recent_installation_count":1,
            "never_reported_installation_count":1,
            "pending_initial_report_installation_count":0,
            "overdue_never_reported_installation_count":1,
            "stale_observed_installation_count":0,
            "current_package_claim_installation_count":1,
            "current_package_claim_unobserved_installation_count":0,
            "current_observed_installation_count":1,
            "current_reporting_installation_count":1,
            "current_recent_installation_count":1
        });
        let roster = [
            json!({
                "execution":"unknown","install_code":"0001","installed_version":"unknown",
                "last_seen":null,"os":"unknown","reporting_status":"최초 보고 지연",
                "runtime":"unknown","update_status":"미확인"
            }),
            json!({
                "execution":"desktop","install_code":"0002","installed_version":"0.18.0",
                "last_seen":1786316400000_i64,"os":"macos","reporting_status":"최근 보고",
                "runtime":"codex_app","update_status":"최신"
            }),
        ];
        results.insert(
            plan.semantic_refs["fleet"].clone(),
            frame(std::slice::from_ref(&fleet)),
        );
        results.insert(
            plan.semantic_refs["fleet_reference"].clone(),
            frame(&[fleet]),
        );
        results.insert(plan.semantic_refs["roster"].clone(), frame(&roster));
        results.insert(
            plan.semantic_refs["storage"].clone(),
            frame(&[json!({
                "clock_skew_event_count":1,
                "deduplicated_event_count":3,
                "delayed_delivery_event_count":2,
                "duplicate_event_row_count":1,
                "overdue_delivery_event_count":1,
                "stored_event_row_count":4
            })]),
        );
        (json!({"results":results}), plan)
    }

    #[test]
    fn grafana_gate_executes_every_panel_and_checks_semantics() {
        let (response, plan) = valid_grafana_response();
        assert_eq!(plan.queries.len(), 20);
        assert!(grafana_query_ready(&response, &plan));

        let mut drifted = response;
        let reference = plan.semantic_refs["fleet_reference"].clone();
        drifted["results"][&reference]["frames"][0]["data"]["values"][0][0] = json!(3);
        assert!(!grafana_query_ready(&drifted, &plan));
    }

    #[test]
    fn deploy_runtime_boundary_returns_an_error_instead_of_panicking() {
        assert!(
            deploy(
                "invalid-image",
                Path::new("missing-compose.yaml"),
                &"a".repeat(64)
            )
            .is_err()
        );
    }

    #[test]
    fn preflight_config_fingerprint_is_strict_and_content_bound() {
        let first = config_sha256(&json!({"value":"first"})).unwrap();
        let second = config_sha256(&json!({"value":"second"})).unwrap();
        assert!(sha256_regex().is_match(&first));
        assert!(sha256_regex().is_match(&second));
        assert_ne!(first, second);
        assert!(!sha256_regex().is_match(&first.to_uppercase()));
    }

    #[test]
    fn deploy_installs_the_xtask_tls_crypto_provider() {
        ensure_crypto_provider().unwrap();
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }

    #[test]
    fn access_gate_rejects_ambiguous_or_credential_bearing_urls() {
        assert!(access_url_allowed(
            &Url::parse("https://insights.example.com/").unwrap()
        ));
        assert!(access_url_allowed(
            &Url::parse("https://insights.example.com:443/").unwrap()
        ));
        assert!(!access_url_allowed(
            &Url::parse("https://insights.example.com:8443/").unwrap()
        ));
        assert!(!access_url_allowed(
            &Url::parse("https://user:password@insights.example.com/").unwrap()
        ));
        assert!(!access_url_allowed(
            &Url::parse("https://insights.example.com/?token=secret").unwrap()
        ));
    }

    #[test]
    fn image_replacement_is_fail_closed() {
        let current = current_config("untrusted.example/groundline-insights:1", "old\n");
        let template = compose("__INSIGHTS_API_IMAGE__", "new");
        let image = format!(
            "ghcr.io/jukqaz/groundline-insights-api@sha256:{}",
            "b".repeat(64)
        );
        assert!(update_compose(&current, &template, &image, &enrollment_credential()).is_err());
    }

    struct FakeOps {
        updates: Vec<serde_json::Value>,
        update_results: VecDeque<bool>,
        health_results: VecDeque<bool>,
        datasource_requirements: Vec<bool>,
    }

    impl DeploymentOps for FakeOps {
        async fn update(&mut self, config: &serde_json::Value) -> bool {
            self.updates.push(config.clone());
            self.update_results.pop_front().unwrap_or(false)
        }

        async fn healthy(&mut self, require_datasource: bool) -> bool {
            self.datasource_requirements.push(require_datasource);
            self.health_results.pop_front().unwrap_or(false)
        }
    }

    #[tokio::test]
    async fn failed_update_attempts_and_verifies_exact_rollback() {
        let mut ops = FakeOps {
            updates: Vec::new(),
            update_results: VecDeque::from([false, true]),
            health_results: VecDeque::from([true]),
            datasource_requirements: Vec::new(),
        };
        let result = apply_with_rollback(&mut ops, &json!("original"), &json!("updated")).await;
        assert_eq!(result, Err(ApplyFailure::RolledBack));
        assert_eq!(ops.updates, [json!("updated"), json!("original")]);
        assert_eq!(ops.datasource_requirements, [false]);
    }

    #[tokio::test]
    async fn failed_post_update_health_rolls_back_without_new_dashboard_semantics() {
        let mut ops = FakeOps {
            updates: Vec::new(),
            update_results: VecDeque::from([true, true]),
            health_results: VecDeque::from([false, true]),
            datasource_requirements: Vec::new(),
        };
        let result = apply_with_rollback(&mut ops, &json!("original"), &json!("updated")).await;
        assert_eq!(result, Err(ApplyFailure::RolledBack));
        assert_eq!(ops.updates, [json!("updated"), json!("original")]);
        assert_eq!(ops.datasource_requirements, [true, false]);
    }

    #[tokio::test]
    async fn rollback_failure_is_distinct_and_fail_closed() {
        let mut ops = FakeOps {
            updates: Vec::new(),
            update_results: VecDeque::from([true, false]),
            health_results: VecDeque::from([false]),
            datasource_requirements: Vec::new(),
        };
        let result = apply_with_rollback(&mut ops, &json!("original"), &json!("updated")).await;
        assert_eq!(result, Err(ApplyFailure::RollbackFailed));
        assert_eq!(ops.updates, [json!("updated"), json!("original")]);
    }

    #[tokio::test]
    async fn already_current_still_requires_full_semantic_health() {
        let mut ops = FakeOps {
            updates: Vec::new(),
            update_results: VecDeque::new(),
            health_results: VecDeque::from([true]),
            datasource_requirements: Vec::new(),
        };
        assert_eq!(
            apply_with_rollback(&mut ops, &json!("same"), &json!("same")).await,
            Ok(false)
        );
        assert!(ops.updates.is_empty());
        assert_eq!(ops.datasource_requirements, [true]);
    }

    #[tokio::test]
    async fn rpc_waits_through_job_events_for_the_final_result() {
        let (client_io, server_io) = duplex(16 * 1024);
        let client_stream = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server_stream =
            WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let server = tokio::spawn(async move {
            let request = server_stream.next().await.unwrap().unwrap();
            let Message::Text(request) = request else {
                panic!("expected text request");
            };
            let request: serde_json::Value = serde_json::from_str(&request).unwrap();
            let id = request["id"].as_str().unwrap();
            server_stream
                .send(Message::Text(
                    json!({
                        "jsonrpc":"2.0","method":"collection_update",
                        "params":{"msg":"changed","collection":"core.get_jobs","fields":{"state":"RUNNING"}}
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            server_stream
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{"state":"RUNNING"}})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        });
        let mut client = RpcClient {
            stream: client_stream,
        };
        let result = client
            .call_bounded("app.update", json!([]), Duration::from_secs(1))
            .await
            .unwrap();
        server.await.unwrap();
        assert_eq!(result["state"], "RUNNING");
    }

    #[tokio::test]
    async fn rpc_job_waits_for_the_terminal_job_state() {
        let (client_io, server_io) = duplex(16 * 1024);
        let client_stream = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server_stream =
            WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let server = tokio::spawn(async move {
            for (index, job_state) in [None, Some("RUNNING"), Some("SUCCESS")]
                .into_iter()
                .enumerate()
            {
                let request = server_stream.next().await.unwrap().unwrap();
                let Message::Text(request) = request else {
                    panic!("expected text request");
                };
                let request: serde_json::Value = serde_json::from_str(&request).unwrap();
                let id = request["id"].as_str().unwrap();
                if index == 0 {
                    assert_eq!(request["method"], "app.update");
                    server_stream
                        .send(Message::Text(
                            json!({"jsonrpc":"2.0","id":id,"result":42})
                                .to_string()
                                .into(),
                        ))
                        .await
                        .unwrap();
                } else {
                    assert_eq!(request["method"], "core.get_jobs");
                    assert_eq!(request["params"], json!([[["id", "=", 42]], {"get":true}]));
                    server_stream
                        .send(Message::Text(
                            json!({
                                "jsonrpc":"2.0",
                                "id":id,
                                "result":{
                                    "state":job_state.unwrap(),
                                    "result":{"state":"DEPLOYING"}
                                }
                            })
                            .to_string()
                            .into(),
                        ))
                        .await
                        .unwrap();
                }
            }
        });
        let mut client = RpcClient {
            stream: client_stream,
        };
        let result = client
            .call_job_success_bounded(
                "app.update",
                json!([]),
                Duration::from_secs(1),
                Duration::ZERO,
            )
            .await
            .unwrap();
        server.await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn rpc_job_reports_a_terminal_failure() {
        let (client_io, server_io) = duplex(16 * 1024);
        let client_stream = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server_stream =
            WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let server = tokio::spawn(async move {
            for response in [json!(7), json!({"state":"FAILED","error":"redacted"})] {
                let request = server_stream.next().await.unwrap().unwrap();
                let Message::Text(request) = request else {
                    panic!("expected text request");
                };
                let request: serde_json::Value = serde_json::from_str(&request).unwrap();
                let id = request["id"].as_str().unwrap();
                server_stream
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":id,"result":response})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
            }
        });
        let mut client = RpcClient {
            stream: client_stream,
        };
        let result = client
            .call_job_success_bounded(
                "app.update",
                json!([]),
                Duration::from_secs(1),
                Duration::ZERO,
            )
            .await
            .unwrap();
        server.await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn inspection_accepts_the_documented_structured_app_config() {
        let (client_io, server_io) = duplex(16 * 1024);
        let client_stream = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server_stream =
            WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let expected = current_config("ghcr.io/jukqaz/groundline-insights:0.17.9", "old\n");
        let server_expected = expected.clone();
        let server = tokio::spawn(async move {
            let request = server_stream.next().await.unwrap().unwrap();
            let Message::Text(request) = request else {
                panic!("expected text request");
            };
            let request: serde_json::Value = serde_json::from_str(&request).unwrap();
            assert_eq!(request["method"], "app.get_instance");
            assert_eq!(request["params"][0], "groundline-insights");
            assert_eq!(request["params"][1]["extra"]["retrieve_config"], true);
            let id = request["id"].as_str().unwrap();
            server_stream
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":id,"result":{"state":"RUNNING","config":server_expected}})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        });
        let mut client = RpcClient {
            stream: client_stream,
        };
        let actual = inspect_current(&mut client, "groundline-insights")
            .await
            .unwrap();
        server.await.unwrap();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn rpc_deadline_is_not_extended_by_unrelated_messages() {
        let (client_io, server_io) = duplex(16 * 1024);
        let client_stream = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server_stream =
            WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let server = tokio::spawn(async move {
            let _ = server_stream.next().await;
            loop {
                if server_stream
                    .send(Message::Text(r#"{"id":"unrelated","result":true}"#.into()))
                    .await
                    .is_err()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
        let mut client = RpcClient {
            stream: client_stream,
        };
        let result = client
            .call_bounded("test.method", json!([]), Duration::from_millis(25))
            .await;
        server.abort();
        assert!(result.is_err());
    }
}
