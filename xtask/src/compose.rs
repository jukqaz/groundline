use std::collections::BTreeMap;
use std::io::Read;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use groundline_runtime::local_file::{
    atomic_write_private, open_bounded_regular_file, private_for_current_user,
};
use regex::Regex;
use serde_json::{Value, json};
use url::Url;
use uuid::Uuid;

use super::XtaskError;

const MAX_TEMPLATE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SECRETS_BYTES: u64 = 16 * 1024;
const SECRET_KEYS: &[&str] = &[
    "ADMIN_TOKEN",
    "CLICKHOUSE_PASSWORD",
    "ENROLLMENT_TOKEN",
    "GRAFANA_ADMIN_PASSWORD",
    "GRAFANA_READER_PASSWORD",
    "PROXY_TOKEN",
];

pub struct RenderOptions<'a> {
    pub template: &'a Path,
    pub output: &'a Path,
    pub secrets_file: &'a Path,
    pub dataset_root: &'a str,
    pub tailscale_bind_ip: &'a str,
    pub dashboard_port: u16,
    pub ingest_port: u16,
    pub image: &'a str,
    pub access_url: &'a str,
    pub overwrite: bool,
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, XtaskError> {
    let mut file =
        open_bounded_regular_file(path, 1, maximum).map_err(|_| XtaskError::InvalidCompose)?;
    let mut bytes = Vec::with_capacity(file.metadata()?.len() as usize);
    file.by_ref().take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(XtaskError::InvalidCompose);
    }
    Ok(bytes)
}

fn generated_secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn secrets(path: &Path) -> Result<(BTreeMap<String, String>, bool), XtaskError> {
    if path.exists() {
        let file = open_bounded_regular_file(path, 1, MAX_SECRETS_BYTES)
            .map_err(|_| XtaskError::InvalidCompose)?;
        if !private_for_current_user(&file) {
            return Err(XtaskError::InvalidCompose);
        }
        drop(file);
        let values: BTreeMap<String, String> =
            serde_json::from_slice(&read_bounded(path, MAX_SECRETS_BYTES)?)?;
        if values.len() != SECRET_KEYS.len()
            || !SECRET_KEYS.iter().all(|key| {
                values
                    .get(*key)
                    .is_some_and(|value| (32..=4096).contains(&value.len()))
            })
        {
            return Err(XtaskError::InvalidCompose);
        }
        return Ok((values, false));
    }
    let values = SECRET_KEYS
        .iter()
        .map(|key| ((*key).to_owned(), generated_secret()))
        .collect::<BTreeMap<_, _>>();
    let mut encoded = serde_json::to_vec_pretty(&values)?;
    encoded.push(b'\n');
    atomic_write_private(path, &encoded)?;
    Ok((values, true))
}

fn valid_dataset_root(value: &str) -> bool {
    value.starts_with("/mnt/")
        && !value.contains("..")
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte))
}

fn valid_tailnet_ip(value: &str) -> bool {
    value.parse::<Ipv4Addr>().is_ok_and(|address| {
        let octets = address.octets();
        octets[0] == 100 && (64..=127).contains(&octets[1])
    })
}

fn valid_port(value: u16) -> bool {
    value >= 1024
}

fn access_origin_and_host(value: &str) -> Option<(String, String)> {
    let url = Url::parse(value).ok()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return None;
    }
    let host = url.host_str()?.to_owned();
    let origin = format!(
        "https://{host}{}",
        url.port().map_or(String::new(), |port| format!(":{port}"))
    );
    Some((origin, host))
}

fn image_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"^[a-z0-9.-]+(?:/[a-z0-9._-]+)+(?::[A-Za-z0-9][A-Za-z0-9._-]{0,127}|@sha256:[0-9a-f]{64})$",
        )
        .expect("fixed image regex")
    })
}

fn unresolved_placeholder_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"__[A-Z][A-Z0-9_]*__").expect("fixed placeholder regex"))
}

fn stable_target(path: &Path) -> Result<PathBuf, XtaskError> {
    let parent = path.parent().ok_or(XtaskError::InvalidCompose)?;
    let file_name = path.file_name().ok_or(XtaskError::InvalidCompose)?;
    let parent = if parent.exists() {
        std::fs::canonicalize(parent).map_err(|_| XtaskError::InvalidCompose)?
    } else {
        parent.to_owned()
    };
    Ok(parent.join(file_name))
}

pub fn render(options: RenderOptions<'_>) -> Result<Value, XtaskError> {
    let output_path = stable_target(options.output)?;
    let secrets_path = stable_target(options.secrets_file)?;
    if (output_path.exists() && !options.overwrite)
        || !valid_dataset_root(options.dataset_root)
        || !valid_tailnet_ip(options.tailscale_bind_ip)
        || !valid_port(options.dashboard_port)
        || !valid_port(options.ingest_port)
        || !image_regex().is_match(options.image)
    {
        return Err(XtaskError::InvalidCompose);
    }
    let (access_origin, access_host) =
        access_origin_and_host(options.access_url).ok_or(XtaskError::InvalidCompose)?;
    let source = String::from_utf8(read_bounded(options.template, MAX_TEMPLATE_BYTES)?)
        .map_err(|_| XtaskError::InvalidCompose)?;
    let (secrets, secrets_created) = secrets(&secrets_path)?;
    let mut replacements = secrets
        .iter()
        .map(|(key, value)| (format!("__{key}__"), value.clone()))
        .collect::<BTreeMap<_, _>>();
    replacements.extend([
        (
            "__DATASET_ROOT__".to_owned(),
            options.dataset_root.trim_end_matches('/').to_owned(),
        ),
        (
            "__TAILSCALE_BIND_IP__".to_owned(),
            options.tailscale_bind_ip.to_owned(),
        ),
        (
            "__DASHBOARD_PORT__".to_owned(),
            options.dashboard_port.to_string(),
        ),
        (
            "__INGEST_PORT__".to_owned(),
            options.ingest_port.to_string(),
        ),
        (
            "__INSIGHTS_API_IMAGE__".to_owned(),
            options.image.to_owned(),
        ),
        ("__INSIGHTS_ACCESS_URL__".to_owned(), access_origin),
        ("__INSIGHTS_ACCESS_HOST__".to_owned(), access_host),
    ]);
    let mut rendered = source;
    for (placeholder, value) in replacements {
        rendered = rendered.replace(&placeholder, &value);
    }
    if unresolved_placeholder_regex().is_match(&rendered)
        || rendered.len() > MAX_TEMPLATE_BYTES as usize
    {
        return Err(XtaskError::InvalidCompose);
    }
    atomic_write_private(&output_path, rendered.as_bytes())?;
    let output_file = open_bounded_regular_file(&output_path, 1, MAX_TEMPLATE_BYTES)
        .map_err(|_| XtaskError::InvalidCompose)?;
    let secrets_file = open_bounded_regular_file(&secrets_path, 1, MAX_SECRETS_BYTES)
        .map_err(|_| XtaskError::InvalidCompose)?;
    Ok(json!({
        "status":"PASS",
        "output_written":true,
        "secrets_created":secrets_created,
        "secret_value_printed":false,
        "output_private":private_for_current_user(&output_file),
        "secrets_private":private_for_current_user(&secrets_file),
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{RenderOptions, render};

    #[test]
    fn rendering_is_private_exact_and_refuses_overwrite() {
        let root = tempdir().expect("temporary directory");
        let template = root.path().join("compose.yaml");
        let output = root.path().join("rendered.yaml");
        let secrets = root.path().join("secrets.json");
        fs::write(
            &template,
            "__CLICKHOUSE_PASSWORD__ __GRAFANA_READER_PASSWORD__ __GRAFANA_ADMIN_PASSWORD__ __ADMIN_TOKEN__ __ENROLLMENT_TOKEN__ __PROXY_TOKEN__ __DATASET_ROOT__ __TAILSCALE_BIND_IP__ __DASHBOARD_PORT__ __INGEST_PORT__ __INSIGHTS_API_IMAGE__ __INSIGHTS_ACCESS_URL__ __INSIGHTS_ACCESS_HOST__ $__timeFilter(received_at)",
        )
        .unwrap();
        let options = || RenderOptions {
            template: &template,
            output: &output,
            secrets_file: &secrets,
            dataset_root: "/mnt/tank/apps/groundline",
            tailscale_bind_ip: "100.64.0.1",
            dashboard_port: 13000,
            ingest_port: 18080,
            image: "ghcr.io/jukqaz/groundline-insights-api:0.20.0",
            access_url: "https://insights.example.invalid",
            overwrite: false,
        };
        let result = render(options()).expect("render");
        assert_eq!(result["status"], "PASS");
        let rendered = fs::read_to_string(&output).unwrap();
        assert!(rendered.contains("$__timeFilter(received_at)"));
        assert!(!super::unresolved_placeholder_regex().is_match(&rendered));
        assert!(render(options()).is_err());
        assert!(
            render(RenderOptions {
                output: &root.path().join("other"),
                access_url: "http://insights.example.invalid",
                ..options()
            })
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn rendering_accepts_an_existing_symlinked_parent_after_canonicalization() {
        use std::os::unix::fs::symlink;

        let root = tempdir().expect("temporary directory");
        let real = root.path().join("real");
        let alias = root.path().join("alias");
        fs::create_dir(&real).unwrap();
        symlink(&real, &alias).unwrap();
        let template = root.path().join("compose.yaml");
        fs::write(
            &template,
            "__CLICKHOUSE_PASSWORD__ __GRAFANA_READER_PASSWORD__ __GRAFANA_ADMIN_PASSWORD__ __ADMIN_TOKEN__ __ENROLLMENT_TOKEN__ __PROXY_TOKEN__ __DATASET_ROOT__ __TAILSCALE_BIND_IP__ __DASHBOARD_PORT__ __INGEST_PORT__ __INSIGHTS_API_IMAGE__ __INSIGHTS_ACCESS_URL__ __INSIGHTS_ACCESS_HOST__",
        )
        .unwrap();
        let output = alias.join("rendered.yaml");
        let secrets = alias.join("secrets.json");
        let result = render(RenderOptions {
            template: &template,
            output: &output,
            secrets_file: &secrets,
            dataset_root: "/mnt/tank/apps/groundline",
            tailscale_bind_ip: "100.64.0.1",
            dashboard_port: 13000,
            ingest_port: 18080,
            image: "ghcr.io/jukqaz/groundline-insights-api:0.20.0",
            access_url: "https://insights.example.invalid",
            overwrite: false,
        })
        .unwrap();
        assert_eq!(result["status"], "PASS");
        assert!(output.is_file());
        assert!(secrets.is_file());
    }

    #[test]
    fn rendering_rejects_public_networks_and_unbounded_paths() {
        let root = tempdir().expect("temporary directory");
        let template = root.path().join("compose.yaml");
        fs::write(&template, "template").unwrap();
        assert!(
            render(RenderOptions {
                template: &template,
                output: &root.path().join("out"),
                secrets_file: &root.path().join("secrets"),
                dataset_root: "/mnt/../etc",
                tailscale_bind_ip: "192.168.1.1",
                dashboard_port: 13000,
                ingest_port: 18080,
                image: "latest",
                access_url: "https://insights.example.invalid",
                overwrite: false,
            })
            .is_err()
        );
    }
}
