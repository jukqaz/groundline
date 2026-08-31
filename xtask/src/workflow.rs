use std::path::Path;

use super::XtaskError;
use super::package::regular_bytes;

fn text(path: &Path) -> Result<String, XtaskError> {
    String::from_utf8(regular_bytes(path)?).map_err(|_| XtaskError::InvalidSource)
}

fn external_action_is_pinned(line: &str) -> bool {
    let line = line.trim();
    let Some(reference) = line
        .strip_prefix("- uses: ")
        .or_else(|| line.strip_prefix("uses: "))
    else {
        return true;
    };
    if reference.starts_with("./") {
        return true;
    }
    let Some((_, revision)) = reference.split_once('@') else {
        return false;
    };
    revision.split_whitespace().next().is_some_and(|value| {
        value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn actions_are_pinned(workflow: &str) -> bool {
    workflow
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("uses: ") || line.starts_with("- uses: ")
        })
        .all(external_action_is_pinned)
}

pub fn verify_ci_cost_contract(root: &Path) -> Result<(), XtaskError> {
    let workflows = root.join(".github/workflows");
    let entries = std::fs::read_dir(&workflows)
        .map_err(|_| XtaskError::InvalidSource)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| XtaskError::InvalidSource)?;
    if entries.len() != 1 || !workflows.join("rust.yml").is_file() {
        return Err(XtaskError::InvalidSource);
    }

    let rust = text(&workflows.join("rust.yml"))?;
    let setup = text(&root.join(".github/actions/setup-rust-stable/action.yml"))?;
    let compose = text(&root.join("infrastructure/compose.template.yaml"))?;
    let api_dockerfile = text(&root.join("services/insights-api/Dockerfile"))?;
    let dockerignore = text(&root.join(".dockerignore"))?;
    let stable_promotion_cleans_staging = rust
        .split_once("\n  promote-stable:")
        .and_then(|(_, stable)| {
            Some((
                stable.find("rm -rf distribution")?,
                stable.find("cargo run --locked -p xtask -- verify-source --root . --json")?,
            ))
        })
        .is_some_and(|(cleanup, verification)| cleanup < verification);
    super::compose::verify_compatibility_profile(
        &root.join("infrastructure/compatibility.json"),
        false,
    )?;
    for required in [
        "workflow_dispatch:",
        "build_release_artifacts:",
        "clickhouse_image:",
        "nginx_image:",
        "grafana_image:",
        "grafana_clickhouse_plugin:",
        "pull_request:",
        "push:\n    tags:",
        "concurrency:",
        "cancel-in-progress: true",
        "Reject an invalid or version-mismatched release tag before expensive work",
        "release tag must be strict vMAJOR.MINOR.PATCH",
        "name: fast source checks",
        "name: full source qualification",
        "if: github.event_name == 'workflow_dispatch'",
        "needs: fast",
        "cargo test --locked -p xtask --all-targets",
        "cargo test --locked -p groundline-cli --test cli_contract",
        "cargo test --locked -p groundline-insights-cli --test cli_contract",
        "cargo test --workspace --all-features --locked",
        "cargo test --locked -p groundline-insights-api --lib clickhouse_",
        r#"GROUNDLINE_CLICKHOUSE_TEST_ALLOW_MUTATION: "true""#,
        "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
        "cargo run --locked -p xtask -- verify-source --root . --json",
        "cargo run --locked -p xtask -- verify-history --root . --json",
        "fetch-depth: 0",
        "name: Exercise the rendered ClickHouse, API, and Grafana stack",
        "name: Prepare one coherent compatibility profile",
        "name: Validate the release-tested and selected compatibility profiles",
        "name: Start the selected ClickHouse integration service",
        "name: Stop the selected ClickHouse integration service",
        "all four compatibility candidate inputs must be supplied together",
        "cargo run --locked -p xtask -- verify-compatibility-profile",
        "cargo run --locked -p xtask --bin groundline-deploy -- verify-stack",
        "--secrets-file \"$secrets_path\"",
        "docker compose --project-name \"$project\" -f \"$compose_path\" up --detach --wait --wait-timeout 240",
        "test \"$unauthenticated_status\" = \"302\"",
        "grep --quiet --ignore-case '^location: .*/login' \"$unauthenticated_headers\"",
        "--noproxy '*'",
        "--connect-timeout 5",
        "--max-time 10",
        "retention-days: 14",
        "name: promote both plugins to stable",
        "rm -rf distribution",
        "--product core",
        "--product insights",
        "linux/amd64,linux/arm64",
        "cargo run --locked -p xtask -- render-compose",
        "--compatibility-profile \"$COMPATIBILITY_PROFILE\"",
        "cargo run --locked -p xtask -- promote-stable",
        "uses: ./.github/actions/setup-rust-stable",
        "name: publish attested multi-architecture Insights API image\n    if: startsWith(github.ref, 'refs/tags/v')\n    needs: artifacts",
        "name: Remap GitHub runner paths from release binaries",
        "--remap-path-prefix=%s=/_groundline --remap-path-prefix=%s=/_cargo_home",
        "name: Stage the integrity-checked Insights API image input",
        "sha256sum --check groundline-insights-api.sha256",
        "GROUNDLINE_RUSTC_VERSION=${{ steps.image-inputs.outputs.rustc-version }}",
        "GROUNDLINE_BUILD_PACKAGES=${{ steps.image-inputs.outputs.musl-packages }}",
        "musl-tools=1.2.4-2",
    ] {
        if !rust.contains(required) {
            return Err(XtaskError::InvalidSource);
        }
    }
    if rust.contains("\n  push:\n    branches:")
        || rust.contains("self-hosted")
        || rust.contains("pull_request_target:")
        || rust.contains("schedule:")
        || rust.contains("permissions: write-all")
        || rust.contains("GROUNDLINE_TRUSTED")
        || rust.contains("RUSTUP_TOOLCHAIN: \"1.")
        || rust.contains("rust-version: \"1.")
        || rust.contains("sync-package")
        || rust.contains("chmod 0777")
        || rust.contains("--allow-mutable-image")
        || rust.contains("apt-get install --yes musl-tools\n")
        || rust.matches("--allow-unpinned-dependencies").count() != 3
        || rust
            .matches("cargo test --workspace --all-features --locked")
            .count()
            != 1
        || rust
            .matches("cargo clippy --workspace --all-targets --all-features --locked")
            .count()
            != 1
        || rust
            .matches("cargo run --locked -p xtask -- verify-history --root . --json")
            .count()
            != 1
        || rust
            .matches("cargo run --locked -p xtask --bin groundline-deploy -- verify-stack")
            .count()
            != 1
        || rust.matches("runs-on:").count() != rust.matches("timeout-minutes:").count()
        || !stable_promotion_cleans_staging
        || !actions_are_pinned(&rust)
        || !setup.contains("using: composite")
        || !setup.contains("rustup toolchain install")
        || setup.contains("curl ")
        || !compose.contains(r#"GF_AUTH_ANONYMOUS_ENABLED: "false""#)
        || compose.contains(r#"GF_AUTH_ANONYMOUS_ENABLED: "true""#)
        || !compose.contains(r#"GF_ANALYTICS_REPORTING_ENABLED: "false""#)
        || !compose.contains(r#"GF_ANALYTICS_CHECK_FOR_UPDATES: "false""#)
        || !compose.contains(r#"GF_ANALYTICS_CHECK_FOR_PLUGIN_UPDATES: "false""#)
        || !compose.contains(r#"GF_PLUGINS_PREINSTALL_AUTO_UPDATE: "false""#)
        || !compose.contains("grafana-storage-init:")
        || !compose.contains("condition: service_completed_successfully")
        || !compose.contains("network_mode: none")
        || !compose.contains(r#"GROUNDLINE_RETENTION_DAYS: "365""#)
        || !compose.contains(r#"GROUNDLINE_COLLECTOR_MAX_EVENTS: "4096""#)
        || !compose.contains(r#"GROUNDLINE_DATASET_MAX_BYTES: "68719476736""#)
        || !api_dockerfile.starts_with("FROM alpine:3.23@sha256:")
        || api_dockerfile.contains("FROM rust:")
        || api_dockerfile.contains("apk add")
        || !api_dockerfile.contains("COPY image-context/groundline-insights-api-${TARGETARCH}")
        || !dockerignore.starts_with("**\n")
        || !dockerignore.contains("!image-context/groundline-insights-api-amd64")
        || !dockerignore.contains("!image-context/groundline-insights-api-arm64")
    {
        return Err(XtaskError::InvalidSource);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{actions_are_pinned, external_action_is_pinned, verify_ci_cost_contract};

    #[test]
    fn repository_ci_cost_contract_is_current() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a repository parent")
            .to_path_buf();
        verify_ci_cost_contract(&root).unwrap();
    }

    #[test]
    fn external_actions_require_full_commit_shas() {
        assert!(external_action_is_pinned(
            "uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7"
        ));
        assert!(external_action_is_pinned(
            "uses: ./.github/actions/setup-rust-stable"
        ));
        assert!(!external_action_is_pinned("uses: actions/checkout@v7"));
        assert!(!actions_are_pinned(
            "steps:\n  - uses: actions/checkout@main\n"
        ));
    }
}
