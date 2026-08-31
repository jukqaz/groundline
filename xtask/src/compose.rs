use std::collections::BTreeMap;
use std::io::Read;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use groundline_runtime::local_file::{
    atomic_write_private, open_bounded_regular_file, private_for_current_user,
};
use regex::Regex;
use semver::Version;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;
use xtask::secret_store::{MAX_SECRET_STORE_BYTES, SECRET_KEYS, load_private_secret_store};

use super::XtaskError;

const MAX_TEMPLATE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_COMPATIBILITY_PROFILE_BYTES: u64 = 16 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityProfile {
    schema_version: u32,
    profile_name: String,
    clickhouse_image: String,
    nginx_image: String,
    grafana_image: String,
    grafana_clickhouse_plugin: String,
}

struct ValidatedCompatibilityProfile {
    profile: CompatibilityProfile,
    fingerprint: String,
    all_dependencies_pinned: bool,
}

pub struct RenderOptions<'a> {
    pub template: &'a Path,
    pub compatibility_profile: &'a Path,
    pub output: &'a Path,
    pub secrets_file: &'a Path,
    pub dataset_root: &'a str,
    pub tailscale_bind_ip: &'a str,
    pub dashboard_port: u16,
    pub ingest_port: u16,
    pub image: &'a str,
    pub allow_unpinned_dependencies: bool,
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
        return Ok((
            load_private_secret_store(path).map_err(|_| XtaskError::InvalidCompose)?,
            false,
        ));
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

fn normalized_dataset_root(value: &str) -> Option<String> {
    let normalized = value.replace('\\', "/");
    let normalized = normalized.trim_end_matches('/');
    let bytes = normalized.as_bytes();
    let unix_absolute = normalized.starts_with('/') && !normalized.starts_with("//");
    let windows_absolute =
        bytes.len() >= 4 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/';
    if normalized.len() > 512
        || !(unix_absolute || windows_absolute)
        || normalized.matches(':').count() != usize::from(windows_absolute)
        || normalized
            .bytes()
            .any(|byte| !byte.is_ascii_alphanumeric() && !b" /._-:".contains(&byte))
        || normalized
            .split('/')
            .any(|component| matches!(component, "." | ".."))
        || normalized.ends_with(':')
    {
        return None;
    }
    Some(normalized.to_owned())
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
            r"^[a-z0-9.-]+(?:/[a-z0-9._-]+)*(?::[A-Za-z0-9][A-Za-z0-9._-]{0,127})?(?:@sha256:[0-9a-f]{64})?$",
        )
        .expect("fixed image regex")
    })
}

fn image_is_valid(value: &str) -> bool {
    if !image_regex().is_match(value) {
        return false;
    }
    let name_and_tag = value.split_once('@').map_or(value, |(name, _)| name);
    value.contains("@sha256:")
        || name_and_tag
            .rsplit_once('/')
            .map_or(name_and_tag, |(_, last)| last)
            .contains(':')
}

fn image_is_immutable(value: &str) -> bool {
    image_is_valid(value) && value.contains("@sha256:")
}

fn image_repository(value: &str) -> &str {
    let name_and_tag = value.split_once('@').map_or(value, |(name, _)| name);
    name_and_tag
        .rsplit_once(':')
        .map_or(name_and_tag, |(name, _)| name)
}

fn profile_name_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn plugin_is_pinned(value: &str) -> bool {
    value
        .strip_prefix("grafana-clickhouse-datasource@")
        .is_some_and(|version| {
            Version::parse(version)
                .is_ok_and(|version| version.pre.is_empty() && version.build.is_empty())
        })
}

fn load_compatibility_profile(
    path: &Path,
    allow_unpinned_dependencies: bool,
) -> Result<ValidatedCompatibilityProfile, XtaskError> {
    let bytes = read_bounded(path, MAX_COMPATIBILITY_PROFILE_BYTES)?;
    let fingerprint = format!("{:x}", Sha256::digest(&bytes));
    let profile = serde_json::from_slice::<CompatibilityProfile>(&bytes)
        .map_err(|_| XtaskError::InvalidCompose)?;
    let images = [
        profile.clickhouse_image.as_str(),
        profile.nginx_image.as_str(),
        profile.grafana_image.as_str(),
    ];
    let images_are_valid = images.iter().all(|image| image_is_valid(image));
    let repositories_are_expected = [
        (
            profile.clickhouse_image.as_str(),
            "clickhouse/clickhouse-server",
        ),
        (profile.nginx_image.as_str(), "nginx"),
        (profile.grafana_image.as_str(), "grafana/grafana"),
    ]
    .iter()
    .all(|(image, expected)| image_repository(image) == *expected);
    let images_are_immutable = images.iter().all(|image| image_is_immutable(image));
    let plugin_is_pinned = plugin_is_pinned(&profile.grafana_clickhouse_plugin);
    let plugin_is_latest = profile.grafana_clickhouse_plugin == "grafana-clickhouse-datasource";
    let all_dependencies_pinned = images_are_immutable && plugin_is_pinned;
    if profile.schema_version != 1
        || !profile_name_is_valid(&profile.profile_name)
        || !images_are_valid
        || !repositories_are_expected
        || (!plugin_is_pinned && !plugin_is_latest)
        || (!all_dependencies_pinned && !allow_unpinned_dependencies)
    {
        return Err(XtaskError::InvalidCompose);
    }
    Ok(ValidatedCompatibilityProfile {
        profile,
        fingerprint,
        all_dependencies_pinned,
    })
}

pub fn verify_compatibility_profile(
    path: &Path,
    allow_unpinned_dependencies: bool,
) -> Result<Value, XtaskError> {
    let validated = load_compatibility_profile(path, allow_unpinned_dependencies)?;
    Ok(json!({
        "status":"PASS",
        "profile_name":validated.profile.profile_name,
        "profile_sha256":validated.fingerprint,
        "all_dependencies_pinned":validated.all_dependencies_pinned,
        "unpinned_dependencies_allowed":allow_unpinned_dependencies,
    }))
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
    let template_path =
        std::fs::canonicalize(options.template).map_err(|_| XtaskError::InvalidCompose)?;
    let output_path = stable_target(options.output)?;
    let secrets_path = stable_target(options.secrets_file)?;
    let dataset_root =
        normalized_dataset_root(options.dataset_root).ok_or(XtaskError::InvalidCompose)?;
    let image_is_immutable = image_is_immutable(options.image);
    let compatibility = load_compatibility_profile(
        options.compatibility_profile,
        options.allow_unpinned_dependencies,
    )?;
    if output_path == secrets_path
        || output_path == template_path
        || secrets_path == template_path
        || (output_path.exists() && !options.overwrite)
        || !valid_tailnet_ip(options.tailscale_bind_ip)
        || !valid_port(options.dashboard_port)
        || !valid_port(options.ingest_port)
        || !image_is_valid(options.image)
        || (!image_is_immutable && !options.allow_unpinned_dependencies)
    {
        return Err(XtaskError::InvalidCompose);
    }
    let (access_origin, access_host) =
        access_origin_and_host(options.access_url).ok_or(XtaskError::InvalidCompose)?;
    let source = String::from_utf8(read_bounded(&template_path, MAX_TEMPLATE_BYTES)?)
        .map_err(|_| XtaskError::InvalidCompose)?;
    for placeholder in SECRET_KEYS.iter().copied().chain([
        "DATASET_ROOT",
        "TAILSCALE_BIND_IP",
        "DASHBOARD_PORT",
        "INGEST_PORT",
        "INSIGHTS_API_IMAGE",
        "CLICKHOUSE_IMAGE",
        "NGINX_IMAGE",
        "GRAFANA_IMAGE",
        "GRAFANA_CLICKHOUSE_PLUGIN",
        "INSIGHTS_ACCESS_URL",
        "INSIGHTS_ACCESS_HOST",
    ]) {
        if !source.contains(&format!("__{placeholder}__")) {
            return Err(XtaskError::InvalidCompose);
        }
    }
    let (secrets, secrets_created) = secrets(&secrets_path)?;
    let mut replacements = secrets
        .iter()
        .map(|(key, value)| (format!("__{key}__"), value.clone()))
        .collect::<BTreeMap<_, _>>();
    replacements.extend([
        ("__DATASET_ROOT__".to_owned(), dataset_root),
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
        (
            "__CLICKHOUSE_IMAGE__".to_owned(),
            compatibility.profile.clickhouse_image,
        ),
        (
            "__NGINX_IMAGE__".to_owned(),
            compatibility.profile.nginx_image,
        ),
        (
            "__GRAFANA_IMAGE__".to_owned(),
            compatibility.profile.grafana_image,
        ),
        (
            "__GRAFANA_CLICKHOUSE_PLUGIN__".to_owned(),
            compatibility.profile.grafana_clickhouse_plugin,
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
    let secrets_file = open_bounded_regular_file(&secrets_path, 1, MAX_SECRET_STORE_BYTES)
        .map_err(|_| XtaskError::InvalidCompose)?;
    let output_private = private_for_current_user(&output_file);
    let secrets_private = private_for_current_user(&secrets_file);
    if !output_private || !secrets_private {
        return Err(XtaskError::InvalidCompose);
    }
    Ok(json!({
        "status":"PASS",
        "output_written":true,
        "secrets_created":secrets_created,
        "api_image_is_immutable":image_is_immutable,
        "compatibility_profile":compatibility.profile.profile_name,
        "compatibility_profile_sha256":compatibility.fingerprint,
        "infrastructure_dependencies_pinned":compatibility.all_dependencies_pinned,
        "all_dependencies_pinned":compatibility.all_dependencies_pinned && image_is_immutable,
        "unpinned_dependencies_allowed":options.allow_unpinned_dependencies,
        "secret_value_printed":false,
        "output_private":output_private,
        "secrets_private":secrets_private,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::{RenderOptions, normalized_dataset_root, render, verify_compatibility_profile};

    const TEST_IMAGE_DIGEST: &str = "ghcr.io/jukqaz/groundline-insights-api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const TEST_TEMPLATE: &str = "__CLICKHOUSE_PASSWORD__ __GRAFANA_READER_PASSWORD__ __GRAFANA_ADMIN_PASSWORD__ __ADMIN_TOKEN__ __ENROLLMENT_TOKEN__ __PROXY_TOKEN__ __DATASET_ROOT__ __TAILSCALE_BIND_IP__ __DASHBOARD_PORT__ __INGEST_PORT__ __INSIGHTS_API_IMAGE__ __CLICKHOUSE_IMAGE__ __NGINX_IMAGE__ __GRAFANA_IMAGE__ __GRAFANA_CLICKHOUSE_PLUGIN__ __INSIGHTS_ACCESS_URL__ __INSIGHTS_ACCESS_HOST__";

    fn compatibility_profile(root: &Path) -> PathBuf {
        let path = root.join("compatibility.json");
        fs::write(
            &path,
            format!(
                r#"{{
  "schema_version": 1,
  "profile_name": "test-profile",
  "clickhouse_image": "clickhouse/clickhouse-server@sha256:{digest}",
  "nginx_image": "nginx@sha256:{digest}",
  "grafana_image": "grafana/grafana@sha256:{digest}",
  "grafana_clickhouse_plugin": "grafana-clickhouse-datasource@4.21.1"
}}"#,
                digest = "b".repeat(64)
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn dataset_roots_are_portable_absolute_and_injection_safe() {
        assert_eq!(
            normalized_dataset_root("/srv/groundline-insights/"),
            Some("/srv/groundline-insights".to_owned())
        );
        assert_eq!(
            normalized_dataset_root(r"D:\GroundLine Data\insights"),
            Some("D:/GroundLine Data/insights".to_owned())
        );
        assert_eq!(
            normalized_dataset_root("/Volumes/External SSD/groundline insights"),
            Some("/Volumes/External SSD/groundline insights".to_owned())
        );
        for invalid in [
            "relative/data",
            "/",
            "C:/",
            "//server/share",
            "/srv/../private",
            "/srv/groundline\nother",
        ] {
            assert!(normalized_dataset_root(invalid).is_none(), "{invalid}");
        }
    }

    #[test]
    fn public_template_requires_authenticated_grafana_and_bounded_storage_init() {
        let template = include_str!("../../infrastructure/compose.template.yaml");
        assert!(template.contains(r#"GF_AUTH_ANONYMOUS_ENABLED: "false""#));
        assert!(!template.contains(r#"GF_AUTH_ANONYMOUS_ENABLED: "true""#));
        for disabled in [
            r#"GF_ANALYTICS_REPORTING_ENABLED: "false""#,
            r#"GF_ANALYTICS_CHECK_FOR_UPDATES: "false""#,
            r#"GF_ANALYTICS_CHECK_FOR_PLUGIN_UPDATES: "false""#,
            r#"GF_PLUGINS_PREINSTALL_AUTO_UPDATE: "false""#,
        ] {
            assert!(template.contains(disabled), "{disabled}");
        }
        for placeholder in [
            "__CLICKHOUSE_IMAGE__",
            "__NGINX_IMAGE__",
            "__GRAFANA_IMAGE__",
            "__GRAFANA_CLICKHOUSE_PLUGIN__",
        ] {
            assert!(template.contains(placeholder), "{placeholder}");
        }
        assert_eq!(template.matches("__GRAFANA_IMAGE__").count(), 2);
        assert!(!template.contains("clickhouse/clickhouse-server:"));
        assert!(!template.contains("grafana/grafana:"));
        let ingress = template
            .split_once("  api-ingress:\n")
            .and_then(|(_, value)| value.split_once("  grafana-storage-init:\n"))
            .map(|(value, _)| value)
            .expect("bounded API ingress section");
        assert!(ingress.contains("      - ingress-host\n"));
        assert!(!ingress.contains("      - egress\n"));
        let init = template
            .split_once("  grafana-storage-init:\n")
            .and_then(|(_, value)| value.split_once("  grafana:\n"))
            .map(|(value, _)| value)
            .expect("Grafana storage initializer");
        for required in [
            "    user: \"0:0\"",
            "    read_only: true",
            "      - ALL",
            "      - CHOWN",
            "      - DAC_OVERRIDE",
            "      - FOWNER",
            "    network_mode: none",
            "chmod 0750 /var/lib/grafana",
        ] {
            assert!(init.contains(required), "{required}");
        }
    }

    #[test]
    fn compatibility_profiles_are_strict_and_latest_requires_an_override() {
        let root = tempdir().expect("temporary directory");
        let pinned = compatibility_profile(root.path());
        let verified = verify_compatibility_profile(&pinned, false).unwrap();
        assert_eq!(verified["all_dependencies_pinned"], true);

        let latest = root.path().join("latest.json");
        fs::write(
            &latest,
            r#"{
  "schema_version": 1,
  "profile_name": "latest-candidate",
  "clickhouse_image": "clickhouse/clickhouse-server:latest",
  "nginx_image": "nginx:latest",
  "grafana_image": "grafana/grafana:latest",
  "grafana_clickhouse_plugin": "grafana-clickhouse-datasource"
}"#,
        )
        .unwrap();
        assert!(verify_compatibility_profile(&latest, false).is_err());
        let verified = verify_compatibility_profile(&latest, true).unwrap();
        assert_eq!(verified["all_dependencies_pinned"], false);

        let unknown = root.path().join("unknown.json");
        fs::write(
            &unknown,
            fs::read_to_string(&pinned)
                .unwrap()
                .replace("\n}", ",\n  \"unexpected\": true\n}"),
        )
        .unwrap();
        assert!(verify_compatibility_profile(&unknown, true).is_err());

        let prerelease = root.path().join("prerelease.json");
        fs::write(
            &prerelease,
            fs::read_to_string(&pinned).unwrap().replace(
                "grafana-clickhouse-datasource@4.21.1",
                "grafana-clickhouse-datasource@4.22.0-beta.1",
            ),
        )
        .unwrap();
        assert!(verify_compatibility_profile(&prerelease, true).is_err());

        let substituted_repository = root.path().join("substituted-repository.json");
        fs::write(
            &substituted_repository,
            fs::read_to_string(&pinned).unwrap().replace(
                "clickhouse/clickhouse-server@",
                "untrusted.example/clickhouse-server@",
            ),
        )
        .unwrap();
        assert!(verify_compatibility_profile(&substituted_repository, true).is_err());
    }

    #[test]
    fn rendering_is_private_exact_and_refuses_overwrite() {
        let root = tempdir().expect("temporary directory");
        let template = root.path().join("compose.yaml");
        let compatibility_profile = compatibility_profile(root.path());
        let output = root.path().join("rendered.yaml");
        let secrets = root.path().join("secrets.json");
        fs::write(
            &template,
            format!("{TEST_TEMPLATE} $__timeFilter(received_at)"),
        )
        .unwrap();
        let options = || RenderOptions {
            template: &template,
            compatibility_profile: &compatibility_profile,
            output: &output,
            secrets_file: &secrets,
            dataset_root: "/mnt/tank/apps/groundline",
            tailscale_bind_ip: "100.64.0.1",
            dashboard_port: 13000,
            ingest_port: 18080,
            image: TEST_IMAGE_DIGEST,
            allow_unpinned_dependencies: false,
            access_url: "https://insights.example.invalid",
            overwrite: false,
        };
        let result = render(options()).expect("render");
        assert_eq!(result["status"], "PASS");
        assert_eq!(result["api_image_is_immutable"], true);
        assert_eq!(result["infrastructure_dependencies_pinned"], true);
        assert_eq!(result["all_dependencies_pinned"], true);
        assert_eq!(result["unpinned_dependencies_allowed"], false);
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
        let compatibility_profile = compatibility_profile(root.path());
        fs::write(&template, TEST_TEMPLATE).unwrap();
        let output = alias.join("rendered.yaml");
        let secrets = alias.join("secrets.json");
        let result = render(RenderOptions {
            template: &template,
            compatibility_profile: &compatibility_profile,
            output: &output,
            secrets_file: &secrets,
            dataset_root: "/mnt/tank/apps/groundline",
            tailscale_bind_ip: "100.64.0.1",
            dashboard_port: 13000,
            ingest_port: 18080,
            image: TEST_IMAGE_DIGEST,
            allow_unpinned_dependencies: false,
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
                compatibility_profile: &root.path().join("missing-profile.json"),
                output: &root.path().join("out"),
                secrets_file: &root.path().join("secrets"),
                dataset_root: "/mnt/../etc",
                tailscale_bind_ip: "192.168.1.1",
                dashboard_port: 13000,
                ingest_port: 18080,
                image: "latest",
                allow_unpinned_dependencies: false,
                access_url: "https://insights.example.invalid",
                overwrite: false,
            })
            .is_err()
        );
    }

    #[test]
    fn unpinned_dependencies_require_an_explicit_development_override() {
        let root = tempdir().expect("temporary directory");
        let template = root.path().join("compose.yaml");
        let compatibility_profile = root.path().join("compatibility.json");
        fs::write(&template, TEST_TEMPLATE).unwrap();
        fs::write(
            &compatibility_profile,
            r#"{
  "schema_version": 1,
  "profile_name": "latest-candidate",
  "clickhouse_image": "clickhouse/clickhouse-server:latest",
  "nginx_image": "nginx:latest",
  "grafana_image": "grafana/grafana:latest",
  "grafana_clickhouse_plugin": "grafana-clickhouse-datasource"
}"#,
        )
        .unwrap();
        let allowed_output = root.path().join("allowed.yaml");
        let rejected_output = root.path().join("rejected.yaml");
        let secrets = root.path().join("secrets.json");
        let options = |allow_unpinned_dependencies| RenderOptions {
            template: &template,
            compatibility_profile: &compatibility_profile,
            output: if allow_unpinned_dependencies {
                &allowed_output
            } else {
                &rejected_output
            },
            secrets_file: &secrets,
            dataset_root: "/mnt/tank/apps/groundline",
            tailscale_bind_ip: "100.64.0.1",
            dashboard_port: 13000,
            ingest_port: 18080,
            image: "local/groundline-insights-api:ci",
            allow_unpinned_dependencies,
            access_url: "https://insights.example.invalid",
            overwrite: false,
        };
        assert!(render(options(false)).is_err());
        let result = render(options(true)).expect("explicit unpinned dependency override");
        assert_eq!(result["api_image_is_immutable"], false);
        assert_eq!(result["infrastructure_dependencies_pinned"], false);
        assert_eq!(result["all_dependencies_pinned"], false);
        assert_eq!(result["unpinned_dependencies_allowed"], true);
    }

    #[test]
    fn renderer_rejects_template_output_and_secret_path_aliases() {
        let root = tempdir().expect("temporary directory");
        let template = root.path().join("compose.yaml");
        let compatibility_profile = compatibility_profile(root.path());
        fs::write(&template, TEST_TEMPLATE).unwrap();
        let shared = root.path().join("shared.yaml");
        macro_rules! options {
            ($output:expr, $secrets_file:expr, $overwrite:expr) => {
                RenderOptions {
                    template: &template,
                    compatibility_profile: &compatibility_profile,
                    output: $output,
                    secrets_file: $secrets_file,
                    dataset_root: "/mnt/groundline",
                    tailscale_bind_ip: "100.64.0.1",
                    dashboard_port: 13000,
                    ingest_port: 18080,
                    image: TEST_IMAGE_DIGEST,
                    allow_unpinned_dependencies: false,
                    access_url: "https://insights.example.invalid",
                    overwrite: $overwrite,
                }
            };
        }
        assert!(render(options!(&shared, &shared, false)).is_err());
        assert!(render(options!(&template, &root.path().join("secrets.json"), true)).is_err());
        assert!(
            render(options!(
                &root.path().join("rendered.yaml"),
                &template,
                false
            ))
            .is_err()
        );
    }
}
