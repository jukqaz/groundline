use groundline_runtime::{insights, local_file::atomic_write_private};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
use url::Url;
use zeroize::Zeroizing;

const TEMPLATE: &str = include_str!("../../../../infrastructure/compose.template.yaml");
const COMPATIBILITY: &str = include_str!("../../../../infrastructure/compatibility.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupInput {
    pub api_url: String,
    pub grafana_url: String,
    pub dataset_root: String,
    pub api_port: u16,
    pub grafana_port: u16,
    pub api_image: String,
    pub mode: String,
    pub tailnet_ip: String,
    pub clickhouse_password: String,
    pub grafana_password: String,
    pub enrollment_key: String,
}

fn validate(input: &SetupInput) -> Result<(), String> {
    insights::report_url(&input.api_url, 7).map_err(|_| "invalid_api_url")?;
    let grafana = Url::parse(&input.grafana_url).map_err(|_| "invalid_grafana_url")?;
    if grafana.scheme() != "https"
        || grafana.host_str().is_none()
        || !grafana.username().is_empty()
        || grafana.password().is_some()
        || grafana.path() != "/"
        || grafana.query().is_some()
        || grafana.fragment().is_some()
    {
        return Err("invalid_grafana_url".into());
    }
    if input.api_port < 1024 || input.grafana_port < 1024 || input.api_port == input.grafana_port {
        return Err("invalid_ports".into());
    }
    if !matches!(input.mode.as_str(), "https" | "tailscale") {
        return Err("invalid_connection_mode".into());
    }
    if input.mode == "https"
        && (!input.api_url.starts_with("https://")
            || insights::endpoint_requires_tailnet(&input.api_url))
    {
        return Err("invalid_api_url".into());
    }
    if input.mode == "tailscale" && !insights::endpoint_requires_tailnet(&input.api_url) {
        return Err("tailnet_endpoint_required".into());
    }
    for value in [
        &input.clickhouse_password,
        &input.grafana_password,
        &input.enrollment_key,
    ] {
        if !value.is_empty()
            && (!(32..=128).contains(&value.len())
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)))
        {
            return Err("invalid_service_password".into());
        }
    }
    Ok(())
}
fn random_secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub fn export_to(input: SetupInput, directory: &Path) -> Result<Value, String> {
    validate(&input)?;
    let mut secret_values: BTreeMap<String, String> = xtask::secret_store::SECRET_KEYS
        .iter()
        .map(|key| ((*key).into(), random_secret()))
        .collect();
    for (key, mut value) in [
        ("CLICKHOUSE_PASSWORD", input.clickhouse_password),
        ("GRAFANA_ADMIN_PASSWORD", input.grafana_password),
        ("ENROLLMENT_TOKEN", input.enrollment_key),
    ] {
        if !value.is_empty() {
            secret_values.insert(key.into(), std::mem::take(&mut value));
        }
    }
    let staging = tempfile::tempdir().map_err(|_| "compose_export_failed")?;
    let template = staging.path().join("template.yaml");
    let compatibility = staging.path().join("compatibility.json");
    let secrets = staging.path().join("secrets.json");
    let output = staging.path().join("compose.yaml");
    atomic_write_private(&template, TEMPLATE.as_bytes()).map_err(|_| "compose_export_failed")?;
    atomic_write_private(&compatibility, COMPATIBILITY.as_bytes())
        .map_err(|_| "compose_export_failed")?;
    let encoded = Zeroizing::new(
        serde_json::to_vec_pretty(&secret_values).map_err(|_| "compose_export_failed")?,
    );
    atomic_write_private(&secrets, &encoded).map_err(|_| "compose_export_failed")?;
    let receipt = xtask::compose::render(xtask::compose::RenderOptions {
        template: &template,
        compatibility_profile: &compatibility,
        output: &output,
        secrets_file: &secrets,
        dataset_root: &input.dataset_root,
        bind_ip: if input.mode == "tailscale" {
            &input.tailnet_ip
        } else {
            "127.0.0.1"
        },
        require_tailnet: input.mode == "tailscale",
        dashboard_port: input.grafana_port,
        ingest_port: input.api_port,
        image: &input.api_image,
        allow_unpinned_dependencies: false,
        access_url: &input.grafana_url,
        overwrite: false,
    })
    .map_err(|_| "invalid_compose")?;
    std::fs::create_dir(directory).map_err(|_| "export_directory_exists")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "compose_export_failed")?;
    }
    for name in ["compose.yaml", "secrets.json"] {
        let data = Zeroizing::new(
            std::fs::read(staging.path().join(name)).map_err(|_| "compose_export_failed")?,
        );
        atomic_write_private(&directory.join(name), &data).map_err(|_| "compose_export_failed")?;
    }
    let instructions = format!(
        "# GroundLine Insights 서버 구성\n\nAPI: {}\nGrafana: {}\n\n1. 서버에서 데이터 경로의 clickhouse, grafana 하위 폴더를 준비합니다.\n2. docker compose -f compose.yaml config --quiet 로 구성 문법을 검사합니다.\n3. 검토 후 docker compose -f compose.yaml up -d 로 실행합니다.\n4. 호스트의 HTTPS 프록시에서 API 주소를 127.0.0.1:{}, Grafana 주소를 127.0.0.1:{}에 연결합니다. Tailscale 모드에서는 선택한 Tailnet IP에 연결합니다.\n5. HTTPS /healthz의 storage_ready와 Grafana 로그인을 확인합니다.\n6. secrets.json의 ENROLLMENT_TOKEN을 클라이언트의 등록키 입력란에 직접 입력합니다. GRAFANA_ADMIN_PASSWORD는 groundline-admin 로그인에 사용합니다.\n\ncompose.yaml과 secrets.json은 자격증명이 포함된 비공개 파일입니다. Git이나 채팅에 올리지 마세요. 이 생성 작업은 서버를 배포하거나 데이터 수집을 켜지 않습니다.\n",
        input.api_url, input.grafana_url, input.api_port, input.grafana_port
    );
    atomic_write_private(&directory.join("README.ko.md"), instructions.as_bytes())
        .map_err(|_| "compose_export_failed")?;
    atomic_write_private(
        &directory.join("connection.json"),
        &serde_json::to_vec_pretty(&json!({
            "schema":1,"kind":"groundline-connection","api_url":input.api_url,"grafana_url":input.grafana_url
        }))
        .map_err(|_| "compose_export_failed")?,
    )
    .map_err(|_| "compose_export_failed")?;
    Ok(
        json!({"status":"PASS","directory":directory.display().to_string(),"files":["compose.yaml","secrets.json","connection.json","README.ko.md"],"deployed":false,"receipt":receipt}),
    )
}
pub fn export(input: SetupInput) -> Result<Value, String> {
    let base = dirs::download_dir().ok_or("downloads_unavailable")?;
    export_to(
        input,
        &base.join(format!("GroundLine-{}", uuid::Uuid::new_v4().simple())),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> SetupInput {
        SetupInput {
            api_url: "https://insights.example.com".into(),
            grafana_url: "https://grafana.example.com".into(),
            dataset_root: "/srv/groundline".into(),
            api_port: 18080,
            grafana_port: 13000,
            api_image: format!(
                "ghcr.io/jukqaz/groundline-insights-api@sha256:{}",
                "a".repeat(64)
            ),
            mode: "https".into(),
            tailnet_ip: String::new(),
            clickhouse_password: String::new(),
            grafana_password: String::new(),
            enrollment_key: String::new(),
        }
    }
    #[test]
    fn exports_the_real_compose_contract_without_exposing_clickhouse_or_secrets() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("deployment");
        let receipt = export_to(input(), &output).unwrap();
        let compose = std::fs::read_to_string(output.join("compose.yaml")).unwrap();
        assert!(compose.contains("127.0.0.1:18080:8080"));
        assert!(compose.contains("GROUNDLINE_REQUIRE_TAILNET: \"false\""));
        assert!(!compose.contains("__BIND_IP__"));
        assert!(!compose.contains("__CLICKHOUSE_PASSWORD__"));
        assert!(compose.contains("X-Forwarded-For $$remote_addr;"));
        assert!(!compose.contains(":8123:"));
        let values =
            xtask::secret_store::load_private_secret_store(&output.join("secrets.json")).unwrap();
        for secret in values.values() {
            assert!(!receipt.to_string().contains(secret));
        }
        assert!(export_to(input(), &output).is_err());
    }
    #[test]
    fn service_inputs_reject_injection_and_mismatched_modes() {
        let mut value = input();
        value.clickhouse_password = format!("{}{}", "a".repeat(32), "$oops");
        assert_eq!(validate(&value).unwrap_err(), "invalid_service_password");
        let mut value = input();
        value.mode = "tailscale".into();
        assert_eq!(validate(&value).unwrap_err(), "tailnet_endpoint_required");
        let mut value = input();
        value.api_port = value.grafana_port;
        assert_eq!(validate(&value).unwrap_err(), "invalid_ports");
    }
}
