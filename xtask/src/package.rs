use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use aho_corasick::AhoCorasick;
use serde::Deserialize;
use serde_json::{Value, json};
use walkdir::WalkDir;

use super::XtaskError;

const MAX_SOURCE_FILE_BYTES: u64 = 4 * 1024 * 1024;
const PRIVATE_MARKERS: &[&[u8]] = &[
    b"@gmail.com",
    b"/Users/",
    b"/home/",
    b"/root/",
    b"C:\\Users\\",
    b"BEGIN PRIVATE KEY",
    b"BEGIN OPENSSH PRIVATE KEY",
    b"BEGIN RSA PRIVATE KEY",
    b"BEGIN EC PRIVATE KEY",
    b"BEGIN DSA PRIVATE KEY",
    b"BEGIN PGP PRIVATE KEY BLOCK",
    b"github_pat_",
    b"ghp_",
    b"gho_",
    b"ghu_",
    b"ghs_",
    b"ghr_",
    b"glpat-",
    b"sk_live_",
    b"rk_live_",
    b"sk-proj-",
    b"xoxb-",
    b"xoxp-",
];
static PRIVATE_MATCHER: OnceLock<AhoCorasick> = OnceLock::new();
const CORE_SKILLS: &[&str] = &[
    "align-agent-home",
    "audit-agent-history",
    "close-live-work",
    "evaluate-ai-usage-maturity",
    "package-agent-task",
    "reconcile-current-state",
];
const HOOK_EVENTS: &[&str] = &["PostCompact", "SessionEnd", "SessionStart", "Stop"];

#[derive(Debug, Deserialize)]
struct PluginManifest {
    name: String,
    version: String,
    repository: String,
}

pub(super) fn regular_bytes(path: &Path) -> Result<Vec<u8>, XtaskError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| XtaskError::InvalidSource)?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_SOURCE_FILE_BYTES
    {
        return Err(XtaskError::InvalidSource);
    }
    fs::read(path).map_err(|_| XtaskError::InvalidSource)
}

fn relative_files(root: &Path) -> Result<BTreeSet<PathBuf>, XtaskError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| XtaskError::InvalidSource)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(XtaskError::InvalidSource);
    }
    let mut files = BTreeSet::new();
    for entry in WalkDir::new(root).follow_links(false).sort_by_file_name() {
        let entry = entry.map_err(|_| XtaskError::InvalidSource)?;
        if entry.file_type().is_symlink() {
            return Err(XtaskError::InvalidSource);
        }
        if entry.file_type().is_file() {
            files.insert(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| XtaskError::InvalidSource)?
                    .to_path_buf(),
            );
        }
    }
    Ok(files)
}

fn source_scan_path(root: &Path, path: &Path) -> bool {
    !matches!(
        path.file_name().and_then(|value| value.to_str()),
        Some(".git" | "target" | "dist")
    ) && !path.starts_with(root.join("plugins/groundline/bin"))
        && !path.starts_with(root.join("plugins/groundline-insights/bin"))
}

fn contains_private_marker(bytes: &[u8]) -> bool {
    PRIVATE_MATCHER
        .get_or_init(|| AhoCorasick::new(PRIVATE_MARKERS).expect("fixed private markers"))
        .is_match(bytes)
}

fn private_source_name(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return true;
    };
    let lower = name.to_ascii_lowercase();
    (lower == ".env" || (lower.starts_with(".env.") && lower != ".env.example"))
        || matches!(
            path.extension()
                .and_then(|value| value.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some(
                "pem"
                    | "key"
                    | "p12"
                    | "pfx"
                    | "jks"
                    | "keystore"
                    | "mobileprovision"
                    | "sqlite"
                    | "sqlite3"
                    | "db"
            )
        )
        || matches!(
            lower.as_str(),
            ".netrc" | ".npmrc" | ".pypirc" | "id_rsa" | "id_ed25519"
        )
        || lower == "credentials.json"
        || lower == "service-account.json"
        || lower.ends_with("-secrets.json")
}

fn manifest(path: &Path, expected_name: &str) -> Result<PluginManifest, XtaskError> {
    let manifest: PluginManifest = serde_json::from_slice(&regular_bytes(path)?)?;
    if manifest.name != expected_name
        || manifest.version != env!("CARGO_PKG_VERSION")
        || manifest.repository != "https://github.com/jukqaz/groundline"
    {
        return Err(XtaskError::InvalidSource);
    }
    Ok(manifest)
}

fn verify_core(root: &Path) -> Result<(), XtaskError> {
    let package = root.join("plugins/groundline");
    manifest(&package.join(".codex-plugin/plugin.json"), "groundline")?;
    if package.join("hooks").exists()
        || package.join("scripts").exists()
        || !package.join("README.md").is_file()
        || !package.join("README.ko.md").is_file()
    {
        return Err(XtaskError::InvalidSource);
    }
    let skills = fs::read_dir(package.join("skills"))
        .map_err(|_| XtaskError::InvalidSource)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| XtaskError::InvalidSource)?;
    let names = skills
        .into_iter()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            entry
                .file_name()
                .into_string()
                .map_err(|_| XtaskError::InvalidSource)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if names
        != CORE_SKILLS
            .iter()
            .map(|value| (*value).to_owned())
            .collect()
    {
        return Err(XtaskError::InvalidSource);
    }
    Ok(())
}

fn verify_insights(root: &Path) -> Result<(), XtaskError> {
    let package = root.join("plugins/groundline-insights");
    manifest(
        &package.join(".codex-plugin/plugin.json"),
        "groundline-insights",
    )?;
    if package.join("skills").exists() || package.join("scripts").exists() {
        return Err(XtaskError::InvalidSource);
    }
    let hooks: Value = serde_json::from_slice(&regular_bytes(&package.join("hooks/hooks.json"))?)?;
    let events = hooks
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or(XtaskError::InvalidSource)?;
    let names = events.keys().cloned().collect::<BTreeSet<_>>();
    if names
        != HOOK_EVENTS
            .iter()
            .map(|value| (*value).to_owned())
            .collect()
    {
        return Err(XtaskError::InvalidSource);
    }
    let text = serde_json::to_string(&hooks)?;
    if !text.contains("groundline-insights")
        || !text.contains("checkpoint")
        || text.contains("groundline checkpoint")
        || text.contains("python")
    {
        return Err(XtaskError::InvalidSource);
    }
    Ok(())
}

pub fn verify_source(root: &Path) -> Result<Value, XtaskError> {
    let root = root.canonicalize().map_err(|_| XtaskError::InvalidSource)?;
    for removed_duplicate in [".codex-plugin", "skills", "references", "assets"] {
        if root.join(removed_duplicate).exists() {
            return Err(XtaskError::InvalidSource);
        }
    }
    verify_core(&root)?;
    verify_insights(&root)?;
    if !regular_bytes(&root.join("rust-toolchain.toml"))?
        .windows(b"channel = \"stable\"".len())
        .any(|window| window == b"channel = \"stable\"")
    {
        return Err(XtaskError::InvalidSource);
    }
    super::workflow::verify_ci_cost_contract(&root)?;

    let mut scanned = 0_usize;
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| source_scan_path(&root, entry.path()))
    {
        let entry = entry.map_err(|_| XtaskError::InvalidSource)?;
        if entry.file_type().is_symlink() {
            return Err(XtaskError::InvalidSource);
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if private_source_name(entry.path()) {
            return Err(XtaskError::InvalidSource);
        }
        if entry.path().extension().and_then(|value| value.to_str()) == Some("py") {
            return Err(XtaskError::InvalidSource);
        }
        let bytes = regular_bytes(entry.path())?;
        if entry.path() != root.join("xtask/src/package.rs") && contains_private_marker(&bytes) {
            return Err(XtaskError::InvalidSource);
        }
        scanned += 1;
    }

    for package in [
        root.join("plugins/groundline"),
        root.join("plugins/groundline-insights"),
    ] {
        if relative_files(&package)?.is_empty() {
            return Err(XtaskError::InvalidSource);
        }
    }

    Ok(json!({
        "kind":"groundline-rust-source-verification",
        "schema":3,
        "status":"PASS",
        "groundline_version":env!("CARGO_PKG_VERSION"),
        "canonical_plugin_roots":true,
        "plugin_count":2,
        "core_hook_count":0,
        "insights_hook_count":4,
        "python_source_count":0,
        "private_marker_count":0,
        "moving_rust_stable":true,
        "ci_cost_contract":true,
        "scanned_source_file_count":scanned,
    }))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{contains_private_marker, private_source_name, regular_bytes, source_scan_path};

    #[test]
    fn source_reads_reject_symlinks_and_generated_binary_trees() {
        let root = tempdir().expect("temporary directory");
        let file = root.path().join("file");
        std::fs::write(&file, b"ok").unwrap();
        assert_eq!(regular_bytes(&file).unwrap(), b"ok");
        assert!(!source_scan_path(
            root.path(),
            &root
                .path()
                .join("plugins/groundline-insights/bin/target/binary")
        ));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&file, root.path().join("link")).unwrap();
            assert!(regular_bytes(&root.path().join("link")).is_err());
        }
    }

    #[test]
    fn public_source_guard_rejects_private_artifact_names_and_secret_markers() {
        for path in [
            ".env",
            ".env.production",
            "credentials.json",
            "owner-secrets.json",
            ".netrc",
            "id_ed25519",
            "service-account.json",
            "release.keystore",
            "collector.sqlite3",
            "certificate.pem",
        ] {
            assert!(private_source_name(std::path::Path::new(path)), "{path}");
        }
        for path in [".env.example", "schema.sql", "example.json"] {
            assert!(!private_source_name(std::path::Path::new(path)), "{path}");
        }
        for marker in [
            b"-----BEGIN PRIVATE KEY-----".as_slice(),
            b"github_pat_example".as_slice(),
            b"glpat-example".as_slice(),
            b"sk-proj-example".as_slice(),
            b"xoxb-example".as_slice(),
        ] {
            assert!(contains_private_marker(marker));
        }
    }
}
