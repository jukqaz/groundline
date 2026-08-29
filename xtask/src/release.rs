use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use regex::Regex;
use semver::Version;
use serde_json::{Value, json};

use super::XtaskError;

const CHANNEL: &str = "stable";
const MANIFEST_PATH: &str = ".codex-plugin/plugin.json";
const RUST_TARGETS: &[&str] = &[
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
    "aarch64-pc-windows-msvc",
    "x86_64-pc-windows-msvc",
];

pub struct PromotionOptions<'a> {
    pub repo: &'a Path,
    pub remote: &'a str,
    pub release_tag: &'a str,
    pub candidate_sha: &'a str,
    pub source_sha: Option<&'a str>,
    pub confirm: bool,
}

fn sha_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[0-9a-f]{40}$").expect("fixed SHA regex"))
}

fn remote_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[A-Za-z0-9._-]+$").expect("fixed remote regex"))
}

fn git(repo: &Path, arguments: &[&str]) -> Result<String, XtaskError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|_| XtaskError::InvalidReleaseChannel)?;
    if !output.status.success() || output.stdout.len() > 4 * 1024 * 1024 {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| XtaskError::InvalidReleaseChannel)
}

fn commit(repo: &Path, revision: &str) -> Result<String, XtaskError> {
    let revision = format!("{revision}^{{commit}}");
    let value = git(repo, &["rev-parse", "--verify", &revision])?;
    if sha_regex().is_match(&value) {
        Ok(value)
    } else {
        Err(XtaskError::InvalidReleaseChannel)
    }
}

fn version_at(repo: &Path, revision: &str) -> Result<Version, XtaskError> {
    let spec = format!("{revision}:{MANIFEST_PATH}");
    let raw = git(repo, &["show", &spec])?;
    let value: Value = serde_json::from_str(&raw)?;
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .ok_or(XtaskError::InvalidReleaseChannel)?;
    let parsed = Version::parse(version).map_err(|_| XtaskError::InvalidReleaseChannel)?;
    if parsed.to_string() == version && parsed.pre.is_empty() && parsed.build.is_empty() {
        Ok(parsed)
    } else {
        Err(XtaskError::InvalidReleaseChannel)
    }
}

fn expected_artifacts() -> BTreeSet<String> {
    let mut expected = BTreeSet::new();
    for target in RUST_TARGETS {
        let executable = if target.contains("windows") {
            "groundline.exe"
        } else {
            "groundline"
        };
        for name in [
            executable.to_owned(),
            format!("{executable}.sha256"),
            "manifest.json".to_owned(),
        ] {
            expected.insert(format!("plugins/groundline/bin/{target}/{name}"));
        }
    }
    expected
}

fn generated_artifact_commit(
    repo: &Path,
    source: &str,
    candidate: &str,
) -> Result<bool, XtaskError> {
    if commit(repo, &format!("{candidate}^"))? != source {
        return Ok(false);
    }
    let changed = git(repo, &["diff", "--name-only", source, candidate])?
        .lines()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let deleted = git(
        repo,
        &["diff", "--diff-filter=D", "--name-only", source, candidate],
    )?;
    Ok(deleted.is_empty() && changed == expected_artifacts())
}

fn remote_channel(repo: &Path, remote: &str) -> Result<Option<String>, XtaskError> {
    let reference = format!("refs/heads/{CHANNEL}");
    let output = git(repo, &["ls-remote", "--heads", remote, &reference])?;
    if output.is_empty() {
        return Ok(None);
    }
    let rows = output.split_whitespace().collect::<Vec<_>>();
    if rows.len() != 2 || rows[1] != reference || !sha_regex().is_match(rows[0]) {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    Ok(Some(rows[0].to_owned()))
}

pub fn promote_stable(options: PromotionOptions<'_>) -> Result<Value, XtaskError> {
    let repo = options
        .repo
        .canonicalize()
        .map_err(|_| XtaskError::InvalidReleaseChannel)?;
    if !repo.join(".git").exists()
        || !remote_regex().is_match(options.remote)
        || !options.release_tag.starts_with('v')
        || !sha_regex().is_match(options.candidate_sha)
    {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    let release_version =
        Version::parse(&options.release_tag[1..]).map_err(|_| XtaskError::InvalidReleaseChannel)?;
    if release_version.to_string() != options.release_tag[1..]
        || !release_version.pre.is_empty()
        || !release_version.build.is_empty()
    {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    let candidate = commit(&repo, options.candidate_sha)?;
    let source = commit(&repo, options.source_sha.unwrap_or(&candidate))?;
    if commit(&repo, &format!("refs/tags/{}", options.release_tag))? != source {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    let source_version = version_at(&repo, &source)?;
    let candidate_version = version_at(&repo, &candidate)?;
    if source_version != candidate_version || candidate_version != release_version {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    let generated = candidate != source && generated_artifact_commit(&repo, &source, &candidate)?;
    if !generated {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    let current = remote_channel(&repo, options.remote)?;
    if let Some(current) = &current {
        git(
            &repo,
            &[
                "fetch",
                "--no-tags",
                options.remote,
                &format!(
                    "+refs/heads/{CHANNEL}:refs/remotes/{}/{CHANNEL}",
                    options.remote
                ),
            ],
        )?;
        let current_version =
            version_at(&repo, &format!("refs/remotes/{}/{CHANNEL}", options.remote))?;
        if current != &candidate && current_version >= candidate_version {
            return Err(XtaskError::InvalidReleaseChannel);
        }
    }
    let action = if current.as_deref() == Some(candidate.as_str()) {
        "noop"
    } else if current.is_some() {
        "advance"
    } else {
        "create"
    };
    let mut mutation_performed = false;
    if options.confirm && action != "noop" {
        let lease = format!(
            "--force-with-lease=refs/heads/{CHANNEL}:{}",
            current.as_deref().unwrap_or_default()
        );
        git(
            &repo,
            &[
                "push",
                options.remote,
                &format!("{candidate}:refs/heads/{CHANNEL}"),
                &lease,
            ],
        )?;
        mutation_performed = true;
    }
    Ok(json!({
        "kind":"groundline-stable-promotion",
        "schema":2,
        "status":"PASS",
        "channel":CHANNEL,
        "action":action,
        "candidate_version":candidate_version.to_string(),
        "mutation_required":action != "noop",
        "mutation_performed":mutation_performed,
        "race_guard":"force_with_lease",
        "tag_target_verified":true,
        "generated_artifact_commit_verified":true,
        "version_monotonic":true,
        "secret_value_printed":false,
        "private_path_printed":false,
    }))
}

#[cfg(test)]
mod tests {
    use super::expected_artifacts;

    #[test]
    fn stable_release_requires_exactly_six_immutable_artifact_sets() {
        let expected = expected_artifacts();
        assert_eq!(expected.len(), 18);
        assert!(expected.contains("plugins/groundline/bin/aarch64-pc-windows-msvc/groundline.exe"));
        assert!(expected.contains("plugins/groundline/bin/x86_64-apple-darwin/groundline.sha256"));
    }
}
