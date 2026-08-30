#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use groundline_runtime::local_file::open_bounded_regular_file;
use groundline_runtime::platform::{
    SUPPORTED_TARGETS, packaged_binary_path, packaged_insights_binary_path,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use thiserror::Error;

mod arm64_verify;
mod compose;
mod local_verify;
mod package;
mod release;
mod workflow;

const MAX_BINARY_BYTES: u64 = 128 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 4 * 1024;
const MAX_CHECKSUM_BYTES: u64 = 256;

#[derive(Debug, Parser)]
#[command(name = "cargo xtask", about = "GroundLine source-only build tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Package one already-built target with a checksum and strict manifest.
    PackageBinary {
        #[arg(long, value_enum)]
        product: Product,
        #[arg(long)]
        target: String,
        #[arg(long)]
        binary: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Verify the exact six-target package set before release promotion.
    VerifyPackageSet {
        #[arg(long, value_enum)]
        product: Product,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long)]
        json: bool,
    },
    /// Enforce the Rust-only source, split-plugin, privacy, and workflow contract.
    VerifySource {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Render a private self-hosted Insights compose file and a separate secret store.
    RenderCompose {
        #[arg(long, default_value = "infrastructure/truenas/compose.template.yaml")]
        template: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        secrets_file: PathBuf,
        #[arg(long)]
        dataset_root: String,
        #[arg(long)]
        tailscale_bind_ip: String,
        #[arg(long, default_value_t = 13000)]
        dashboard_port: u16,
        #[arg(long, default_value_t = 18080)]
        ingest_port: u16,
        #[arg(long)]
        image: String,
        #[arg(long)]
        access_url: String,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        json: bool,
    },
    /// Run the bounded, clean-commit local CI fallback and write a redacted receipt.
    VerifyLocal {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify the ARM64 Linux deployment controller with reusable bounded Docker caches.
    VerifyArm64 {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Inspect or remove only stale ARM64 verification cache volumes.
    PruneArm64Cache {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
    /// Plan or atomically advance the moving stable marketplace branch.
    PromoteStable {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long, default_value = "origin")]
        remote: String,
        #[arg(long)]
        release_tag: String,
        #[arg(long)]
        candidate_sha: String,
        #[arg(long)]
        source_sha: Option<String>,
        #[arg(long)]
        confirm: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Error)]
enum XtaskError {
    #[error("unsupported_target")]
    UnsupportedTarget,
    #[error("invalid_binary")]
    InvalidBinary,
    #[error("output_already_exists")]
    OutputAlreadyExists,
    #[error("invalid_package_set")]
    InvalidPackageSet,
    #[error("invalid_source")]
    InvalidSource,
    #[error("invalid_compose")]
    InvalidCompose,
    #[error("local_verification_failed")]
    LocalVerificationFailed,
    #[error("arm64_verification_failed")]
    Arm64VerificationFailed,
    #[error("invalid_release_channel")]
    InvalidReleaseChannel,
    #[error("io_failed")]
    Io(#[from] io::Error),
    #[error("manifest_failed")]
    Manifest(#[from] serde_json::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Product {
    Core,
    Insights,
}

impl Product {
    const fn name(self) -> &'static str {
        match self {
            Self::Core => "groundline",
            Self::Insights => "groundline-insights",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactManifest {
    schema_version: u8,
    kind: String,
    groundline_version: String,
    target: String,
    executable: String,
    size_bytes: u64,
    sha256: String,
}

fn executable_name(product: Product, target: &str) -> Result<String, XtaskError> {
    let path = match product {
        Product::Core => packaged_binary_path(target),
        Product::Insights => packaged_insights_binary_path(target),
    }
    .map_err(|_| XtaskError::UnsupportedTarget)?;
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_owned)
        .ok_or(XtaskError::UnsupportedTarget)
}

fn copy_and_sha256(source: &Path, destination: &Path) -> Result<(String, u64), XtaskError> {
    let mut reader = open_bounded_regular_file(source, 1, MAX_BINARY_BYTES)
        .map_err(|_| XtaskError::InvalidBinary)?;
    let mut writer = File::create(destination)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut size_bytes = 0_u64;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        size_bytes = size_bytes
            .checked_add(count as u64)
            .ok_or(XtaskError::InvalidBinary)?;
        if size_bytes > MAX_BINARY_BYTES {
            return Err(XtaskError::InvalidBinary);
        }
        writer.write_all(&buffer[..count])?;
        digest.update(&buffer[..count]);
    }
    writer.sync_all()?;
    Ok((format!("{:x}", digest.finalize()), size_bytes))
}

fn read_bounded(path: &Path, minimum: u64, maximum: u64) -> Result<Vec<u8>, XtaskError> {
    let mut reader = open_bounded_regular_file(path, minimum, maximum)
        .map_err(|_| XtaskError::InvalidPackageSet)?;
    let mut bytes = Vec::with_capacity(
        reader
            .metadata()
            .map_err(|_| XtaskError::InvalidPackageSet)?
            .len() as usize,
    );
    Read::by_ref(&mut reader)
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| XtaskError::InvalidPackageSet)?;
    if bytes.len() as u64 > maximum {
        return Err(XtaskError::InvalidPackageSet);
    }
    Ok(bytes)
}

fn sha256_file(path: &Path) -> Result<(String, u64), XtaskError> {
    let mut reader = open_bounded_regular_file(path, 1, MAX_BINARY_BYTES)
        .map_err(|_| XtaskError::InvalidPackageSet)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut size = 0_u64;
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|_| XtaskError::InvalidPackageSet)?;
        if count == 0 {
            break;
        }
        size = size
            .checked_add(count as u64)
            .ok_or(XtaskError::InvalidPackageSet)?;
        if size > MAX_BINARY_BYTES {
            return Err(XtaskError::InvalidPackageSet);
        }
        digest.update(&buffer[..count]);
    }
    Ok((format!("{:x}", digest.finalize()), size))
}

fn directory_names(path: &Path) -> Result<BTreeSet<String>, XtaskError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| XtaskError::InvalidPackageSet)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(XtaskError::InvalidPackageSet);
    }
    fs::read_dir(path)
        .map_err(|_| XtaskError::InvalidPackageSet)?
        .map(|entry| {
            entry
                .map_err(|_| XtaskError::InvalidPackageSet)?
                .file_name()
                .into_string()
                .map_err(|_| XtaskError::InvalidPackageSet)
        })
        .collect()
}

#[cfg(unix)]
fn executable_contract(path: &Path, target: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;

    target.contains("-windows-")
        || path
            .metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn executable_contract(_path: &Path, _target: &str) -> bool {
    true
}

fn verify_package_set(root: &Path, version: &str, product: Product) -> Result<(), XtaskError> {
    if version != env!("CARGO_PKG_VERSION") {
        return Err(XtaskError::InvalidPackageSet);
    }
    let expected_targets = SUPPORTED_TARGETS
        .iter()
        .map(|target| (*target).to_owned())
        .collect::<BTreeSet<_>>();
    if directory_names(root)? != expected_targets {
        return Err(XtaskError::InvalidPackageSet);
    }

    for target in SUPPORTED_TARGETS {
        let executable = executable_name(product, target)?;
        let target_root = root.join(target);
        let expected_files = [executable.as_str(), "manifest.json"]
            .into_iter()
            .chain([format!("{executable}.sha256")].iter().map(String::as_str))
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        if directory_names(&target_root)? != expected_files {
            return Err(XtaskError::InvalidPackageSet);
        }

        let manifest: ArtifactManifest = serde_json::from_slice(&read_bounded(
            &target_root.join("manifest.json"),
            1,
            MAX_MANIFEST_BYTES,
        )?)?;
        if manifest.schema_version != 1
            || manifest.kind != "groundline-binary-artifact"
            || manifest.groundline_version != version
            || manifest.target != *target
            || manifest.executable != executable
        {
            return Err(XtaskError::InvalidPackageSet);
        }

        let binary = target_root.join(&executable);
        if !executable_contract(&binary, target) {
            return Err(XtaskError::InvalidPackageSet);
        }
        let (sha256, size_bytes) = sha256_file(&binary)?;
        if manifest.sha256 != sha256 || manifest.size_bytes != size_bytes {
            return Err(XtaskError::InvalidPackageSet);
        }
        let checksum = read_bounded(
            &target_root.join(format!("{executable}.sha256")),
            1,
            MAX_CHECKSUM_BYTES,
        )?;
        if checksum != format!("{sha256}  {executable}\n").as_bytes() {
            return Err(XtaskError::InvalidPackageSet);
        }
    }
    Ok(())
}

#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<(), XtaskError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<(), XtaskError> {
    Ok(())
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), XtaskError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), XtaskError> {
    Ok(())
}

fn package_binary(
    product: Product,
    target: &str,
    binary: &Path,
    output: &Path,
) -> Result<(), XtaskError> {
    if !SUPPORTED_TARGETS.contains(&target) {
        return Err(XtaskError::UnsupportedTarget);
    }
    if output.exists() {
        return Err(XtaskError::OutputAlreadyExists);
    }
    let executable = executable_name(product, target)?;
    let parent = output.parent().ok_or(XtaskError::InvalidBinary)?;
    fs::create_dir_all(parent)?;
    let staging = TempDir::new_in(parent)?;
    let staged_output = staging.path().join(target);
    fs::create_dir(&staged_output)?;
    let staged_binary = staged_output.join(&executable);
    let (checksum, size_bytes) = copy_and_sha256(binary, &staged_binary)?;
    mark_executable(&staged_binary)?;

    let mut checksum_file = File::create(staged_output.join(format!("{executable}.sha256")))?;
    writeln!(checksum_file, "{checksum}  {executable}")?;
    checksum_file.sync_all()?;
    drop(checksum_file);

    let manifest = ArtifactManifest {
        schema_version: 1,
        kind: "groundline-binary-artifact".to_owned(),
        groundline_version: env!("CARGO_PKG_VERSION").to_owned(),
        target: target.to_owned(),
        executable,
        size_bytes,
        sha256: checksum,
    };
    let mut manifest_file = File::create(staged_output.join("manifest.json"))?;
    serde_json::to_writer_pretty(&mut manifest_file, &manifest)?;
    writeln!(manifest_file)?;
    manifest_file.sync_all()?;
    drop(manifest_file);
    sync_parent_directory(&staged_output)?;

    fs::rename(staged_output, output)?;
    sync_parent_directory(parent)?;
    Ok(())
}

fn run(cli: Cli) -> Result<(), XtaskError> {
    match cli.command {
        Command::PackageBinary {
            product,
            target,
            binary,
            output,
        } => package_binary(product, &target, &binary, &output),
        Command::VerifyPackageSet {
            product,
            root,
            version,
            json: json_output,
        } => {
            verify_package_set(&root, &version, product)?;
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "kind": "groundline-package-set-verification",
                        "schema": 1,
                        "status": "PASS",
                        "groundline_version": version,
                        "product": product.name(),
                        "target_count": SUPPORTED_TARGETS.len(),
                        "artifact_file_count": SUPPORTED_TARGETS.len() * 3,
                        "mutation_performed": false,
                        "private_paths_emitted": false,
                    }))?
                );
            }
            Ok(())
        }
        Command::VerifySource {
            root,
            json: json_output,
        } => {
            let result = package::verify_source(&root)?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Ok(())
        }
        Command::RenderCompose {
            template,
            output,
            secrets_file,
            dataset_root,
            tailscale_bind_ip,
            dashboard_port,
            ingest_port,
            image,
            access_url,
            overwrite,
            json: json_output,
        } => {
            let result = compose::render(compose::RenderOptions {
                template: &template,
                output: &output,
                secrets_file: &secrets_file,
                dataset_root: &dataset_root,
                tailscale_bind_ip: &tailscale_bind_ip,
                dashboard_port,
                ingest_port,
                image: &image,
                access_url: &access_url,
                overwrite,
            })?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Ok(())
        }
        Command::VerifyLocal {
            root,
            json: json_output,
        } => {
            let result = local_verify::verify(&root)?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Ok(())
        }
        Command::VerifyArm64 {
            root,
            json: json_output,
        } => {
            let outcome = arm64_verify::verify(&root);
            if json_output {
                println!("{}", serde_json::to_string_pretty(&outcome.receipt)?);
            }
            if outcome.success {
                Ok(())
            } else {
                Err(XtaskError::Arm64VerificationFailed)
            }
        }
        Command::PruneArm64Cache {
            root,
            confirm,
            json: json_output,
        } => {
            let outcome = arm64_verify::prune(&root, confirm);
            if json_output {
                println!("{}", serde_json::to_string_pretty(&outcome.receipt)?);
            }
            if outcome.success {
                Ok(())
            } else {
                Err(XtaskError::Arm64VerificationFailed)
            }
        }
        Command::PromoteStable {
            repo,
            remote,
            release_tag,
            candidate_sha,
            source_sha,
            confirm,
            json: json_output,
        } => {
            let result = release::promote_stable(release::PromotionOptions {
                repo: &repo,
                remote: &remote,
                release_tag: &release_tag,
                candidate_sha: &candidate_sha,
                source_sha: source_sha.as_deref(),
                confirm,
            })?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use tempfile::tempdir;

    use super::{Product, SUPPORTED_TARGETS, XtaskError, package_binary, verify_package_set};

    #[test]
    fn every_target_gets_one_bounded_reproducible_artifact() {
        let root = tempdir().expect("temporary directory");
        let binary = root.path().join("input-binary");
        fs::write(&binary, b"bounded-test-binary").expect("test binary");
        for target in SUPPORTED_TARGETS {
            let output = root.path().join("dist").join(target);
            package_binary(Product::Insights, target, &binary, &output).expect("packaged target");
            let executable = if target.ends_with("windows-msvc") {
                "groundline-insights.exe"
            } else {
                "groundline-insights"
            };
            assert_eq!(
                fs::read(output.join(executable)).unwrap(),
                b"bounded-test-binary"
            );
            let checksum = fs::read_to_string(output.join(format!("{executable}.sha256"))).unwrap();
            assert!(checksum.ends_with(&format!("  {executable}\n")));
            let manifest: Value =
                serde_json::from_slice(&fs::read(output.join("manifest.json")).unwrap()).unwrap();
            assert_eq!(manifest["target"], *target);
            assert_eq!(manifest["executable"], executable);
            assert_eq!(manifest["size_bytes"], 19);
            let mut digest = Sha256::new();
            digest.update(fs::read(output.join(executable)).unwrap());
            let expected = format!("{:x}", digest.finalize());
            assert_eq!(manifest["sha256"], expected);
            assert_eq!(checksum, format!("{expected}  {executable}\n"));
        }
    }

    #[test]
    fn package_refuses_symlinks_and_existing_outputs() {
        let root = tempdir().expect("temporary directory");
        let binary = root.path().join("input-binary");
        fs::write(&binary, b"binary").expect("test binary");
        let output = root.path().join("dist").join(SUPPORTED_TARGETS[0]);
        package_binary(Product::Core, SUPPORTED_TARGETS[0], &binary, &output)
            .expect("first package");
        assert!(matches!(
            package_binary(Product::Core, SUPPORTED_TARGETS[0], &binary, &output),
            Err(XtaskError::OutputAlreadyExists)
        ));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let link = root.path().join("linked-binary");
            symlink(&binary, &link).expect("test symlink");
            assert!(matches!(
                package_binary(
                    Product::Core,
                    SUPPORTED_TARGETS[1],
                    &link,
                    &root.path().join("dist").join(SUPPORTED_TARGETS[1]),
                ),
                Err(XtaskError::InvalidBinary)
            ));
        }
    }

    #[test]
    fn package_set_requires_exact_targets_and_matching_bytes() {
        let root = tempdir().expect("temporary directory");
        let binary = root.path().join("input-binary");
        let dist = root.path().join("dist");
        fs::write(&binary, b"bounded-test-binary").expect("test binary");
        for target in SUPPORTED_TARGETS {
            package_binary(Product::Core, target, &binary, &dist.join(target))
                .expect("packaged target");
        }

        verify_package_set(&dist, env!("CARGO_PKG_VERSION"), Product::Core)
            .expect("valid package set");
        assert!(matches!(
            verify_package_set(&dist, "9.9.9", Product::Core),
            Err(XtaskError::InvalidPackageSet)
        ));

        fs::write(
            dist.join(SUPPORTED_TARGETS[0]).join("groundline.sha256"),
            b"tampered\n",
        )
        .expect("tamper checksum");
        assert!(matches!(
            verify_package_set(&dist, env!("CARGO_PKG_VERSION"), Product::Core),
            Err(XtaskError::InvalidPackageSet)
        ));
    }
}
