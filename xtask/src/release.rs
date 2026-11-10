//! Independent, source-bound product distributions and atomic channel promotion.
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use groundline_contracts::version::{release_display_name, strict_version};
use groundline_runtime::platform::SUPPORTED_TARGETS;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{Product, XtaskError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Distribution {
    pub kind: String,
    pub schema: u8,
    pub product: String,
    pub plugin: String,
    pub marketplace: String,
    pub channel: String,
    pub version: String,
    pub release_tag: String,
    pub source_sha: String,
}

impl Product {
    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Insights => "insights",
        }
    }
    pub(super) const fn marketplace(self) -> &'static str {
        match self {
            Self::Core => "groundline-core",
            Self::Insights => "groundline",
        }
    }
    pub(super) const fn channel(self) -> &'static str {
        match self {
            Self::Core => "core-stable",
            Self::Insights => "stable",
        }
    }
}

pub fn parse_tag(tag: &str) -> Result<(Product, Version), XtaskError> {
    let (product, version) = if let Some(v) = tag.strip_prefix("core-v") {
        (Product::Core, v)
    } else if let Some(v) = tag.strip_prefix("insights-v") {
        (Product::Insights, v)
    } else {
        return Err(XtaskError::InvalidReleaseChannel);
    };
    release_display_name(version).map_err(|_| XtaskError::InvalidReleaseChannel)?;
    Ok((
        product,
        strict_version(version).map_err(|_| XtaskError::InvalidReleaseChannel)?,
    ))
}

fn sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|v| v.is_ascii_hexdigit() && !v.is_ascii_uppercase())
}

fn git(repo: &Path, args: &[&str]) -> Result<String, XtaskError> {
    let result = Command::new("git")
        .arg("-c")
        .arg("core.fsmonitor=false")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|_| XtaskError::InvalidReleaseChannel)?;
    if !result.status.success() || result.stdout.len() > 4 * 1024 * 1024 {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    String::from_utf8(result.stdout)
        .map(|v| v.trim().to_owned())
        .map_err(|_| XtaskError::InvalidReleaseChannel)
}

fn commit(repo: &Path, reference: &str) -> Result<String, XtaskError> {
    let value = git(
        repo,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )?;
    if !sha(&value) {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    Ok(value)
}

fn version_at(repo: &Path, reference: &str, product: Product) -> Result<Version, XtaskError> {
    let path = format!(
        "{reference}:plugins/{}/.codex-plugin/plugin.json",
        product.name()
    );
    let value: Value = serde_json::from_str(&git(repo, &["show", &path])?)?;
    strict_version(
        value["version"]
            .as_str()
            .ok_or(XtaskError::InvalidReleaseChannel)?,
    )
    .map_err(|_| XtaskError::InvalidReleaseChannel)
}

pub fn info(tag: &str) -> Result<Value, XtaskError> {
    let (product, version) = parse_tag(tag)?;
    Ok(
        json!({"product":product.key(),"plugin":product.name(),"marketplace":product.marketplace(),
        "channel":product.channel(),"version":version.to_string(),
        "title":format!("GroundLine {} {}", if product == Product::Core {"Core"} else {"Insights"},
            release_display_name(&version.to_string()).map_err(|_| XtaskError::InvalidReleaseChannel)?)}),
    )
}

fn descriptor(tag: &str, source: &str) -> Result<Distribution, XtaskError> {
    let (product, version) = parse_tag(tag)?;
    if !sha(source) {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    Ok(Distribution {
        kind: "groundline-plugin-distribution".to_owned(),
        schema: 1,
        product: product.key().to_owned(),
        plugin: product.name().to_owned(),
        marketplace: product.marketplace().to_owned(),
        channel: product.channel().to_owned(),
        version: version.to_string(),
        release_tag: tag.to_owned(),
        source_sha: source.to_owned(),
    })
}

fn catalog(source: &str, product: Product) -> Result<Value, XtaskError> {
    let mut value: Value = serde_json::from_str(source)?;
    let entries = value["plugins"]
        .as_array_mut()
        .ok_or(XtaskError::InvalidReleaseChannel)?;
    entries.retain(|v| v["name"] == product.name());
    if entries.len() != 1
        || entries[0]["source"]["source"] != "local"
        || entries[0]["source"]["path"] != format!("./plugins/{}", product.name())
    {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    value["name"] = json!(product.marketplace());
    Ok(value)
}

fn expected_files(product: Product) -> BTreeSet<String> {
    let mut expected = BTreeSet::from([
        "distribution.json".to_owned(),
        ".agents/plugins/marketplace.json".to_owned(),
    ]);
    for target in SUPPORTED_TARGETS {
        for name in [
            product.name().to_owned(),
            format!("{}.sha256", product.name()),
            "manifest.json".to_owned(),
        ] {
            expected.insert(format!("plugins/{}/bin/{target}/{name}", product.name()));
        }
    }
    expected
}

pub fn stage(repo: &Path, packages: &Path, release_tag: &str) -> Result<Value, XtaskError> {
    let (product, version) = parse_tag(release_tag)?;
    let source = commit(repo, "HEAD")?;
    if !git(repo, &["status", "--porcelain", "--untracked-files=all"])?.is_empty()
        || version_at(repo, &source, product)? != version
    {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    super::verify_package_set(packages, &version.to_string(), product)?;
    let descriptor = descriptor(release_tag, &source)?;
    let index_path = repo.join(".agents/plugins/marketplace.json");
    let index = catalog(&std::fs::read_to_string(&index_path)?, product)?;
    let bin = repo.join("plugins").join(product.name()).join("bin");
    if bin.exists() || repo.join("distribution.json").exists() {
        return Err(XtaskError::OutputAlreadyExists);
    }
    std::fs::create_dir(&bin)?;
    for target in SUPPORTED_TARGETS {
        let target_dir = bin.join(target);
        std::fs::create_dir(&target_dir)?;
        for name in [
            product.name().to_owned(),
            format!("{}.sha256", product.name()),
            "manifest.json".to_owned(),
        ] {
            std::fs::copy(packages.join(target).join(&name), target_dir.join(name))?;
        }
    }
    std::fs::write(
        index_path,
        format!("{}\n", serde_json::to_string_pretty(&index)?),
    )?;
    std::fs::write(
        repo.join("distribution.json"),
        format!("{}\n", serde_json::to_string_pretty(&descriptor)?),
    )?;
    Ok(
        json!({"kind":"groundline-distribution-stage","schema":1,"status":"PASS",
        "product":product.key(),"channel":product.channel(),"source_sha":source,
        "generated_paths":expected_files(product)}),
    )
}

fn generated_commit(
    repo: &Path,
    source: &str,
    candidate: &str,
    tag: &str,
    product: Product,
) -> Result<bool, XtaskError> {
    if git(repo, &["show", "-s", "--format=%P", candidate])? != source {
        return Ok(false);
    }
    let changed = git(repo, &["diff", "--name-only", source, candidate])?
        .lines()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if changed != expected_files(product)
        || !git(
            repo,
            &["diff", "--diff-filter=D", "--name-only", source, candidate],
        )?
        .is_empty()
    {
        return Ok(false);
    }
    let recorded: Distribution = serde_json::from_str(&git(
        repo,
        &["show", &format!("{candidate}:distribution.json")],
    )?)?;
    if recorded != descriptor(tag, source)? {
        return Ok(false);
    }
    let original = git(
        repo,
        &[
            "show",
            &format!("{source}:.agents/plugins/marketplace.json"),
        ],
    )?;
    let actual: Value = serde_json::from_str(&git(
        repo,
        &[
            "show",
            &format!("{candidate}:.agents/plugins/marketplace.json"),
        ],
    )?)?;
    Ok(actual == catalog(&original, product)?)
}

pub struct PromotionOptions<'a> {
    pub repo: &'a Path,
    pub remote: &'a str,
    pub release_tag: &'a str,
    pub candidate_sha: &'a str,
    pub source_sha: Option<&'a str>,
    pub confirm: bool,
}

pub fn promote_stable(options: PromotionOptions<'_>) -> Result<Value, XtaskError> {
    let repo = options
        .repo
        .canonicalize()
        .map_err(|_| XtaskError::InvalidReleaseChannel)?;
    let (product, version) = parse_tag(options.release_tag)?;
    if options.remote.is_empty()
        || !options
            .remote
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"_-".contains(&v))
        || !sha(options.candidate_sha)
    {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    let candidate = commit(&repo, options.candidate_sha)?;
    let source = commit(&repo, options.source_sha.unwrap_or(&candidate))?;
    if commit(&repo, &format!("refs/tags/{}", options.release_tag))? != source
        || version_at(&repo, &source, product)? != version
        || version_at(&repo, &candidate, product)? != version
        || !generated_commit(&repo, &source, &candidate, options.release_tag, product)?
    {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    // Qualification validates package bytes before the generated commit. The
    // promotion gate also checks them from an isolated checkout of that commit.
    let checkout = tempfile::tempdir()?;
    let bin_path = format!("plugins/{}/bin", product.name());
    let tree = git(&repo, &["ls-tree", "-r", "-l", &candidate, "--", &bin_path])?;
    let rows: Vec<_> = tree.lines().collect();
    if rows.len() != 9
        || rows.iter().any(|row| {
            let fields: Vec<_> = row.split_whitespace().collect();
            fields.len() != 5
                || !["100644", "100755"].contains(&fields[0])
                || fields[1] != "blob"
                || fields[3]
                    .parse::<u64>()
                    .map_or(true, |size| size == 0 || size > super::MAX_BINARY_BYTES)
        })
    {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    let mut archive = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["archive", "--format=tar", &candidate, "--", &bin_path])
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    let input = archive
        .stdout
        .take()
        .ok_or(XtaskError::InvalidReleaseChannel)?;
    let extraction = Command::new("tar")
        .args(["-xf", "-", "-C"])
        .arg(checkout.path())
        .stdin(std::process::Stdio::from(input))
        .status();
    if !extraction.as_ref().is_ok_and(|s| s.success()) {
        let _ = archive.kill();
    }
    let archived = archive.wait()?;
    if !extraction?.success() || !archived.success() {
        return Err(XtaskError::InvalidReleaseChannel);
    }
    super::verify_package_set(
        &checkout
            .path()
            .join("plugins")
            .join(product.name())
            .join("bin"),
        &version.to_string(),
        product,
    )?;
    let reference = format!("refs/heads/{}", product.channel());
    let output = git(&repo, &["ls-remote", "--heads", options.remote, &reference])?;
    let current = if output.is_empty() {
        None
    } else {
        let row: Vec<_> = output.split_whitespace().collect();
        if row.len() != 2 || !sha(row[0]) || row[1] != reference {
            return Err(XtaskError::InvalidReleaseChannel);
        }
        Some(row[0].to_owned())
    };
    if let Some(current) = &current {
        git(&repo, &["fetch", "--no-tags", options.remote, &reference])?;
        if current != &candidate && version_at(&repo, current, product)? >= version {
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
    if options.confirm && action != "noop" {
        git(
            &repo,
            &[
                "push",
                options.remote,
                &format!("{candidate}:{reference}"),
                &format!(
                    "--force-with-lease={reference}:{}",
                    current.as_deref().unwrap_or_default()
                ),
            ],
        )?;
    }
    Ok(
        json!({"kind":"groundline-stable-promotion","schema":3,"status":"PASS","product":product.key(),
        "channel":product.channel(),"action":action,"candidate_version":version.to_string(),
        "mutation_performed":options.confirm && action!="noop","race_guard":"force_with_lease",
        "tag_target_verified":true,"generated_artifact_commit_verified":true,"version_monotonic":true,
        "secret_value_printed":false,"private_path_printed":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    struct Fixture {
        _temp: tempfile::TempDir,
        repo: std::path::PathBuf,
        packages: std::path::PathBuf,
        source: String,
        tag: String,
        product: Product,
    }
    impl Fixture {
        fn new(product: Product) -> Self {
            let temp = tempfile::tempdir().unwrap();
            let repo = temp.path().join("source");
            let packages = temp.path().join("packages");
            std::fs::create_dir_all(repo.join(".agents/plugins")).unwrap();
            git(&repo, &["init"]).unwrap();
            git(&repo, &["config", "user.name", "Fixture"]).unwrap();
            git(&repo, &["config", "user.email", "fixture@example.invalid"]).unwrap();
            git(&repo, &["config", "commit.gpgsign", "false"]).unwrap();
            let version = env!("CARGO_PKG_VERSION");
            let mut entries = Vec::new();
            for p in [Product::Core, Product::Insights] {
                let manifest = repo.join("plugins").join(p.name()).join(".codex-plugin");
                std::fs::create_dir_all(&manifest).unwrap();
                std::fs::write(
                    manifest.join("plugin.json"),
                    json!({"name":p.name(),"version":version}).to_string(),
                )
                .unwrap();
                entries.push(json!({"name":p.name(),"source":{"source":"local","path":format!("./plugins/{}",p.name())}}));
            }
            std::fs::write(
                repo.join(".agents/plugins/marketplace.json"),
                json!({"name":"groundline","plugins":entries}).to_string(),
            )
            .unwrap();
            std::fs::write(repo.join("README.md"), "Public fixture.\n").unwrap();
            git(&repo, &["add", "."]).unwrap();
            git(&repo, &["commit", "-m", "source fixture"]).unwrap();
            let source = commit(&repo, "HEAD").unwrap();
            let tag = format!("{}-v{version}", product.key());
            git(&repo, &["tag", &tag]).unwrap();
            for target in SUPPORTED_TARGETS {
                let root = packages.join(target);
                std::fs::create_dir_all(&root).unwrap();
                let bytes = b"synthetic executable package fixture";
                let hash = format!("{:x}", Sha256::digest(bytes));
                let binary = root.join(product.name());
                std::fs::write(&binary, bytes).unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(binary, std::fs::Permissions::from_mode(0o755))
                        .unwrap();
                }
                std::fs::write(
                    root.join(format!("{}.sha256", product.name())),
                    format!("{hash}  {}\n", product.name()),
                )
                .unwrap();
                std::fs::write(root.join("manifest.json"),json!({"schema_version":1,"kind":"groundline-binary-artifact","groundline_version":version,"target":target,"executable":product.name(),"size_bytes":bytes.len(),"sha256":hash}).to_string()).unwrap();
            }
            let remote = temp.path().join("remote.git");
            let result = Command::new("git")
                .args(["init", "--bare"])
                .arg(&remote)
                .output()
                .unwrap();
            assert!(result.status.success());
            git(
                &repo,
                &["remote", "add", "origin", remote.to_str().unwrap()],
            )
            .unwrap();
            // A Core publication must not move the existing Insights channel.
            git(
                &repo,
                &["push", "origin", &format!("{source}:refs/heads/stable")],
            )
            .unwrap();
            Self {
                _temp: temp,
                repo,
                packages,
                source,
                tag,
                product,
            }
        }
        fn stage(&self) -> String {
            stage(&self.repo, &self.packages, &self.tag).unwrap();
            git(&self.repo, &["add", "."]).unwrap();
            git(&self.repo, &["commit", "-m", "generated distribution"]).unwrap();
            commit(&self.repo, "HEAD").unwrap()
        }
        fn promote(&self, candidate: &str, confirm: bool) -> Result<Value, XtaskError> {
            promote_stable(PromotionOptions {
                repo: &self.repo,
                remote: "origin",
                release_tag: &self.tag,
                candidate_sha: candidate,
                source_sha: Some(&self.source),
                confirm,
            })
        }
        fn amend(&self) -> String {
            git(&self.repo, &["add", "."]).unwrap();
            git(&self.repo, &["commit", "--amend", "--no-edit"]).unwrap();
            commit(&self.repo, "HEAD").unwrap()
        }
    }
    #[test]
    fn products_have_disjoint_channels_and_exact_artifact_surfaces() {
        let core = expected_files(Product::Core);
        let insights = expected_files(Product::Insights);
        assert_eq!(core.len(), 11);
        assert_eq!(insights.len(), 11);
        assert!(core.iter().all(|p| !p.contains("groundline-insights")));
        assert!(
            insights
                .iter()
                .all(|p| !p.contains("plugins/groundline/bin"))
        );
        assert_ne!(Product::Core.channel(), Product::Insights.channel());
        assert_ne!(Product::Core.marketplace(), Product::Insights.marketplace());
    }
    #[test]
    fn only_explicit_product_tags_are_current_publication_inputs() {
        for tag in [
            "v2026.1110.1",
            "core-v0.29.0",
            "other-v2026.1110.1",
            "core-v2026.229.1",
            "insights-v2026.1110.0",
        ] {
            assert!(parse_tag(tag).is_err(), "{tag}");
        }
        assert_eq!(parse_tag("core-v2026.1110.1").unwrap().0, Product::Core);
        assert_eq!(
            parse_tag("insights-v2026.1110.1").unwrap().0,
            Product::Insights
        );
    }
    #[test]
    fn core_channel_promotes_verified_bytes_and_never_moves_insights() {
        let f = Fixture::new(Product::Core);
        let candidate = f.stage();
        assert_eq!(f.promote(&candidate, false).unwrap()["action"], "create");
        assert!(
            git(
                &f.repo,
                &["ls-remote", "--heads", "origin", "refs/heads/core-stable"]
            )
            .unwrap()
            .is_empty()
        );
        assert_eq!(
            f.promote(&candidate, true).unwrap()["mutation_performed"],
            true
        );
        assert_eq!(f.promote(&candidate, true).unwrap()["action"], "noop");
        assert!(
            git(
                &f.repo,
                &["ls-remote", "--heads", "origin", "refs/heads/stable"]
            )
            .unwrap()
            .starts_with(&f.source)
        );
    }
    #[test]
    fn insights_channel_remains_separately_publishable() {
        let f = Fixture::new(Product::Insights);
        let candidate = f.stage();
        // Replace only the fixture's initial same-version channel with an absent
        // one. Real equal-version replacements are deliberately rejected.
        git(&f.repo, &["push", "origin", ":refs/heads/stable"]).unwrap();
        assert_eq!(
            f.promote(&candidate, true).unwrap()["channel"],
            f.product.channel()
        );
        assert!(
            git(
                &f.repo,
                &["ls-remote", "--heads", "origin", "refs/heads/core-stable"]
            )
            .unwrap()
            .is_empty()
        );
    }
    #[test]
    fn promotion_rejects_changed_source_catalog_descriptor_packages_and_equal_version() {
        for action in [
            "source",
            "catalog",
            "descriptor",
            "binary",
            "extra",
            "equal",
        ] {
            let f = Fixture::new(Product::Core);
            let mut candidate = f.stage();
            match action {
                "source" => std::fs::write(f.repo.join("README.md"), "changed source").unwrap(),
                "catalog" => std::fs::write(
                    f.repo.join(".agents/plugins/marketplace.json"),
                    json!({"name":"groundline","plugins":[]}).to_string(),
                )
                .unwrap(),
                "descriptor" => {
                    let path = f.repo.join("distribution.json");
                    let mut d: Value =
                        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                    d["source_sha"] = json!("0".repeat(40));
                    std::fs::write(path, d.to_string()).unwrap();
                }
                "binary" => std::fs::write(
                    f.repo
                        .join("plugins/groundline/bin")
                        .join(SUPPORTED_TARGETS[0])
                        .join("groundline"),
                    b"tampered",
                )
                .unwrap(),
                "extra" => {
                    std::fs::write(f.repo.join("plugins/groundline/bin/extra"), b"extra").unwrap()
                }
                "equal" => {
                    git(
                        &f.repo,
                        &[
                            "push",
                            "origin",
                            &format!("{}:refs/heads/core-stable", f.source),
                        ],
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            }
            if action != "equal" {
                candidate = f.amend();
            }
            assert!(f.promote(&candidate, true).is_err(), "{action}");
        }
    }
    #[test]
    fn staging_rejects_dirty_source_wrong_product_and_incomplete_target_set() {
        let f = Fixture::new(Product::Core);
        std::fs::write(f.repo.join("unreviewed"), b"dirty").unwrap();
        assert!(stage(&f.repo, &f.packages, &f.tag).is_err());
        std::fs::remove_file(f.repo.join("unreviewed")).unwrap();
        assert!(
            stage(
                &f.repo,
                &f.packages,
                &format!("insights-v{}", env!("CARGO_PKG_VERSION"))
            )
            .is_err()
        );
        std::fs::remove_dir_all(f.packages.join(SUPPORTED_TARGETS[0])).unwrap();
        assert!(stage(&f.repo, &f.packages, &f.tag).is_err());
        assert!(!f.repo.join("distribution.json").exists());
    }
}
