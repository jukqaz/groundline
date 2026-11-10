use groundline_runtime::platform::SUPPORTED_TARGETS;
use std::collections::BTreeSet;
use std::path::Path;

use super::XtaskError;
use super::package::regular_bytes;

fn text(path: &Path) -> Result<String, XtaskError> {
    String::from_utf8(regular_bytes(path)?).map_err(|_| XtaskError::InvalidSource)
}

fn release_matrices_match_supported_targets(workflow: &str) -> bool {
    let Ok(document) = serde_saphyr::from_str::<serde_json::Value>(workflow) else {
        return false;
    };
    let expected = SUPPORTED_TARGETS.iter().copied().collect::<BTreeSet<_>>();
    ["native-setup", "artifacts"].iter().all(|job| {
        let Some(rows) = document["jobs"][*job]["strategy"]["matrix"]["include"].as_array() else {
            return false;
        };
        rows.len() == SUPPORTED_TARGETS.len()
            && rows
                .iter()
                .filter_map(|row| row["target"].as_str())
                .collect::<BTreeSet<_>>()
                == expected
    })
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

fn public_build_metadata_is_bounded(workflow: &str) -> bool {
    let Ok(document) = serde_saphyr::from_str::<serde_json::Value>(workflow) else {
        return false;
    };
    let Some(jobs) = document.get("jobs").and_then(serde_json::Value::as_object) else {
        return false;
    };
    let mut builds = 0;
    for job in jobs.values() {
        let Some(steps) = job.get("steps").and_then(serde_json::Value::as_array) else {
            continue;
        };
        let mut private_builder = false;
        for step in steps {
            let action = step
                .get("uses")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if action.starts_with("docker/setup-buildx-action@") {
                let options = step["with"]["driver-opts"].as_str().unwrap_or("");
                let privacy_options: Vec<_> = options
                    .lines()
                    .map(str::trim)
                    .filter(|line| line.starts_with("provenance-add-gha"))
                    .collect();
                if step["with"]["driver"] != "docker-container"
                    || privacy_options != ["provenance-add-gha=false"]
                    || step["with"].get("endpoint").is_some()
                {
                    return false;
                }
                private_builder = true;
            }
            if action.starts_with("docker/build-push-action@") {
                builds += 1;
                // A different builder could bypass the preceding privacy setup.
                if !private_builder
                    || step["with"].get("builder").is_some()
                    || step["env"]["DOCKER_BUILD_RECORD_UPLOAD"] != "false"
                    || step["with"]["provenance"] != "mode=max"
                    || step["with"]["sbom"] != true
                {
                    return false;
                }
            }
        }
    }
    builds > 0
}

fn native_core_command_is_complete(line: &str) -> bool {
    let Some(args) = shlex::split(line) else {
        return false;
    };
    let args: Vec<_> = args.iter().map(String::as_str).collect();
    if !args.starts_with(&[
        "cargo",
        "test",
        "--locked",
        "-p",
        "groundline-cli",
        "--target",
        "${{ matrix.target }}",
    ]) {
        return false;
    }
    let mut rest = args[7..].iter().copied();
    let mut required = std::collections::BTreeSet::from([
        "improvement_cli",
        "setup_cli",
        "config_repair_cli",
        "install_cli",
    ]);
    while let Some(arg) = rest.next() {
        match arg {
            "--lib" => {}
            "--test" => match rest.next() {
                Some(
                    name @ ("improvement_cli" | "setup_cli" | "config_repair_cli" | "install_cli"),
                ) => {
                    required.remove(name);
                }
                _ => return false,
            },
            "--" => {
                return required.is_empty()
                    && rest.all(|arg| matches!(arg, "--show-output" | "--nocapture"));
            }
            _ => return false,
        }
    }
    required.is_empty()
}

fn native_core_checks_are_required(workflow: &str) -> bool {
    let Ok(document) = serde_saphyr::from_str::<serde_json::Value>(workflow) else {
        return false;
    };
    ["native-setup", "artifacts"].into_iter().all(|job| {
        document["jobs"][job]["steps"]
            .as_array()
            .is_some_and(|steps| {
                steps.iter().any(|step| {
                    if step.get("if").is_some() || step.get("continue-on-error").is_some() {
                        return false;
                    }
                    step["run"]
                        .as_str()
                        .is_some_and(|run| run.lines().any(native_core_command_is_complete))
                })
            })
    })
}

const SCHEMA_RESET_ENV: &str = "GROUNDLINE_CLICKHOUSE_TEST_ALLOW_SCHEMA_RESET";

fn clickhouse_schema_reset_is_scoped(workflow: &str) -> bool {
    // Exactly one occurrence also rejects shell exports or an extra workflow,
    // job, or unrelated step environment that would widen this permission.
    if workflow.matches(SCHEMA_RESET_ENV).count() != 1 {
        return false;
    }
    let Ok(document) = serde_saphyr::from_str::<serde_json::Value>(workflow) else {
        return false;
    };
    document["jobs"]["qualification"]["steps"]
        .as_array()
        .is_some_and(|steps| {
            steps.iter().any(|step| {
                step["name"] == "Exercise the isolated ClickHouse and Grafana integration lane"
                    && step["if"] == "needs.fast.outputs.product == 'insights'"
                    && step.get("continue-on-error").is_none()
                    && step["env"][SCHEMA_RESET_ENV] == "true"
                    && step["env"]["GROUNDLINE_CLICKHOUSE_TEST_ALLOW_MUTATION"] == "true"
                    && step["env"]["GROUNDLINE_CLICKHOUSE_TEST_URL"] == "http://127.0.0.1:8123"
            })
        })
}

const PRODUCT_TAGS: &str =
    "startsWith(github.ref, 'refs/tags/core-v') || startsWith(github.ref, 'refs/tags/insights-v')";
const INSIGHTS_SCOPE: &str = "needs.fast.outputs.product == 'insights'";

fn independent_releases_are_scoped(workflow: &str) -> bool {
    let Ok(doc) = serde_saphyr::from_str::<serde_json::Value>(workflow) else {
        return false;
    };
    let jobs = &doc["jobs"];
    if doc["on"]["push"]["tags"] != serde_json::json!(["core-v*.*.*", "insights-v*.*.*"])
        || jobs["publish-image"]["if"] != "startsWith(github.ref, 'refs/tags/insights-v')"
        || jobs["publish-release"]["if"] != PRODUCT_TAGS
        || jobs["artifacts"]["env"]["RELEASE_PRODUCT"] != "${{ needs.fast.outputs.product }}"
        || jobs["artifacts"]["env"]["RELEASE_PLUGIN"] != "${{ needs.fast.outputs.plugin }}"
        || jobs["qualification"]["if"]
            != format!("github.event_name == 'workflow_dispatch' || {PRODUCT_TAGS}")
        || jobs["promote-stable"]["if"]
            != format!(
                "always() && ({PRODUCT_TAGS}) && needs.artifacts.result == 'success' && needs.publish-release.result == 'success' && (startsWith(github.ref, 'refs/tags/core-v') || needs.publish-image.result == 'success')"
            )
    {
        return false;
    }
    let Some(steps) = jobs["qualification"]["steps"].as_array() else {
        return false;
    };
    for name in [
        "Prepare one coherent compatibility profile",
        "Install the pinned Ubuntu Noble musl toolchain",
        "Validate the release-tested and selected compatibility profiles",
        "Start the selected ClickHouse integration service",
        "Exercise the isolated ClickHouse and Grafana integration lane",
        "Validate the generic compose template",
        "Exercise the rendered ClickHouse, API, and Grafana stack",
    ] {
        let matches: Vec<_> = steps.iter().filter(|s| s["name"] == name).collect();
        if matches.len() != 1
            || matches[0]["if"] != INSIGHTS_SCOPE
            || matches[0].get("continue-on-error").is_some()
        {
            return false;
        }
    }
    for name in [
        "Run the portable Rust suite once",
        "Deny Rust warnings once",
        "Verify canonical packages and public source",
        "Deny vulnerable, unlicensed, or untrusted crates",
    ] {
        let Some(step) = steps.iter().find(|s| s["name"] == name) else {
            return false;
        };
        if step.get("if").is_some() || step.get("continue-on-error").is_some() {
            return false;
        }
        if name == "Deny vulnerable, unlicensed, or untrusted crates"
            && (step["env"]["CARGO_DENY_VERSION"] != "0.20.2"
                || step["env"]["CARGO_DENY_SHA256"]
                    != "9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f")
        {
            return false;
        }
    }
    let Some(steps) = jobs["artifacts"]["steps"].as_array() else {
        return false;
    };
    for name in [
        "Build the Linux Insights API from the same moving stable toolchain",
        "Stage the integrity-checked Insights API image input",
    ] {
        let Some(step) = steps.iter().find(|s| s["name"] == name) else {
            return false;
        };
        if step["if"]
            != "env.RELEASE_PRODUCT == 'insights' && contains(matrix.target, 'unknown-linux-musl')"
            || step.get("continue-on-error").is_some()
        {
            return false;
        }
    }
    let Some(build) = steps
        .iter()
        .find(|s| s["name"] == "Build the selected release binary and host packager")
    else {
        return false;
    };
    if build.get("if").is_some()
        || build.get("continue-on-error").is_some()
        || !build["run"]
            .as_str()
            .is_some_and(|run| run.contains("--bin \"$RELEASE_PLUGIN\""))
    {
        return false;
    }
    let Some(steps) = jobs["promote-stable"]["steps"].as_array() else {
        return false;
    };
    steps.iter().any(|s| {
        s["name"] == "Download the selected exact three-target packages"
            && s["with"]["path"] == "${{ runner.temp }}/groundline-channel-packages"
    })
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
    let independent_release_scope = independent_releases_are_scoped(&rust);
    super::compose::verify_compatibility_profile(
        &root.join("infrastructure/compatibility.json"),
        false,
    )?;
    for required in [
        "workflow_dispatch:",
        "release_product:",
        "build_release_artifacts:",
        "clickhouse_image:",
        "nginx_image:",
        "grafana_image:",
        "grafana_clickhouse_plugin:",
        "pull_request:",
        "cancel-in-progress: true",
        "release tag must select core or insights explicitly",
        "cargo test --locked -p xtask --all-targets",
        "cargo test --locked -p groundline-cli",
        "cargo test --workspace --all-features --locked",
        "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
        "cargo run --locked -p xtask -- verify-source --root . --json",
        "cargo run --locked -p xtask -- verify-history --root . --json",
        "cargo run --locked -p xtask -- stage-distribution",
        "cargo run --locked -p xtask -- promote-stable",
        "cargo run --locked -p xtask -- verify-package-set",
        "--remap-path-prefix=%s=/_groundline --remap-path-prefix=%s=/_cargo_home",
        "musl-tools=1.2.4-2",
        "--noproxy",
        "Launch both downloaded API artifacts before publishing",
        "sha256sum --check groundline-insights-api.sha256",
        "--test improvement_cli --test setup_cli --test config_repair_cli --test install_cli",
    ] {
        if !rust.contains(required) {
            return Err(XtaskError::InvalidSource);
        }
    }
    if rust.contains("\n  push:\n    branches:")
        || rust.contains("self-hosted")
        || rust.contains("pull_request_target:")
        || rust.contains("schedule:")
        || rust.contains("apps/desktop")
        || rust.contains("groundline-desktop")
        || rust.contains("\n  desktop:")
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
        || !independent_release_scope
        || !actions_are_pinned(&rust)
        || !public_build_metadata_is_bounded(&rust)
        || !native_core_checks_are_required(&rust)
        || !release_matrices_match_supported_targets(&rust)
        || !clickhouse_schema_reset_is_scoped(&rust)
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
        || !api_dockerfile
            .contains("COPY --chmod=0755 image-context/groundline-insights-api-${TARGETARCH}")
        || !rust.contains("Launch both downloaded API artifacts before publishing")
        || !rust.contains("groundline_insights_api_failed: configuration_rejected")
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

    use super::{
        SCHEMA_RESET_ENV, actions_are_pinned, clickhouse_schema_reset_is_scoped,
        external_action_is_pinned, native_core_checks_are_required,
        public_build_metadata_is_bounded, release_matrices_match_supported_targets,
        verify_ci_cost_contract,
    };

    #[test]
    fn release_and_native_matrices_require_the_exact_three_supported_targets() {
        let workflow = include_str!("../../.github/workflows/rust.yml");
        assert!(release_matrices_match_supported_targets(workflow));
        for job in ["native-setup", "artifacts"] {
            for change in ["intel", "missing", "duplicate"] {
                let mut document: serde_json::Value = serde_saphyr::from_str(workflow).unwrap();
                let rows = document["jobs"][job]["strategy"]["matrix"]["include"]
                    .as_array_mut()
                    .unwrap();
                match change {
                    "intel" => rows[0]["target"] = "x86_64-apple-darwin".into(),
                    "missing" => {
                        rows.remove(0);
                    }
                    _ => {
                        rows[1] = rows[0].clone();
                    }
                }
                assert!(!release_matrices_match_supported_targets(
                    &document.to_string()
                ));
            }
        }
    }

    #[test]
    fn schema_reset_requires_the_disposable_insights_step_only() {
        let workflow = include_str!("../../.github/workflows/rust.yml");
        assert!(clickhouse_schema_reset_is_scoped(workflow));
        for action in 0..7 {
            let mut doc: serde_json::Value = serde_saphyr::from_str(workflow).unwrap();
            let step = doc["jobs"]["qualification"]["steps"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|s| {
                    s["name"] == "Exercise the isolated ClickHouse and Grafana integration lane"
                })
                .unwrap();
            match action {
                0 => {
                    step["env"]
                        .as_object_mut()
                        .unwrap()
                        .remove(SCHEMA_RESET_ENV);
                }
                1 => step["env"][SCHEMA_RESET_ENV] = "false".into(),
                2 => {
                    step["env"]["GROUNDLINE_CLICKHOUSE_TEST_URL"] =
                        "https://clickhouse.example.invalid".into()
                }
                3 => step["if"] = "always()".into(),
                4 => {
                    step.as_object_mut().unwrap().remove("if");
                }
                5 => step["continue-on-error"] = true.into(),
                _ => doc["env"][SCHEMA_RESET_ENV] = "true".into(),
            }
            assert!(
                !clickhouse_schema_reset_is_scoped(&doc.to_string()),
                "action {action}"
            );
        }
    }

    #[test]
    fn core_checks_cannot_disappear_from_either_native_job() {
        let workflow = include_str!("../../.github/workflows/rust.yml");
        assert!(native_core_checks_are_required(workflow));
        for job in ["native-setup", "artifacts"] {
            for removed in [
                " --test improvement_cli",
                " --test setup_cli",
                " --test config_repair_cli",
                " --test install_cli",
            ] {
                let mut document: serde_json::Value = serde_saphyr::from_str(workflow).unwrap();
                for step in document["jobs"][job]["steps"].as_array_mut().unwrap() {
                    if let Some(run) = step["run"].as_str() {
                        step["run"] = run.replace(removed, "").into();
                    }
                }
                assert!(!native_core_checks_are_required(&document.to_string()));
            }
        }
        for extra in [
            " --no-run",
            " -- --skip receipt",
            " -- --list",
            " -- --exact",
            " -- --ignored",
            " nonexistent_filter",
            " || true",
        ] {
            assert!(!native_core_checks_are_required(&workflow.replace(
                "--test improvement_cli",
                &format!("--test improvement_cli{extra}")
            )));
        }
    }

    #[test]
    fn public_builds_exclude_raw_events_without_dropping_attestations() {
        let workflow = include_str!("../../.github/workflows/rust.yml");
        assert!(public_build_metadata_is_bounded(workflow));
        for action in 0..7 {
            let mut doc: serde_json::Value = serde_saphyr::from_str(workflow).unwrap();
            for step in doc["jobs"]["publish-image"]["steps"]
                .as_array_mut()
                .unwrap()
            {
                let name = step["uses"].as_str().unwrap_or("").to_owned();
                if name.starts_with("docker/setup-buildx-action@") {
                    match action {
                        0 => step["with"]["driver-opts"] = "provenance-add-gha=true".into(),
                        4 => step["with"]["driver"] = "remote".into(),
                        5 => step["with"]["endpoint"] = "remote-builder".into(),
                        _ => {}
                    }
                }
                if name.starts_with("docker/build-push-action@") {
                    match action {
                        1 => step["env"]["DOCKER_BUILD_RECORD_UPLOAD"] = "true".into(),
                        2 => step["with"]["provenance"] = false.into(),
                        3 => step["with"]["sbom"] = false.into(),
                        6 => step["with"]["builder"] = "unreviewed-builder".into(),
                        _ => {}
                    }
                }
            }
            assert!(
                !public_build_metadata_is_bounded(&doc.to_string()),
                "action {action}"
            );
        }
        assert!(!public_build_metadata_is_bounded("jobs: {}"));
    }

    #[test]
    fn core_releases_cannot_publish_or_depend_on_insights_images() {
        let workflow = include_str!("../../.github/workflows/rust.yml");
        assert!(super::independent_releases_are_scoped(workflow));
        for action in 0..5 {
            let mut doc: serde_json::Value = serde_saphyr::from_str(workflow).unwrap();
            match action {
                0 => doc["jobs"]["publish-image"]["if"] = "always()".into(),
                1 => doc["jobs"]["promote-stable"]["if"] = "success()".into(),
                2 => doc["on"]["push"]["tags"] = serde_json::json!(["v*.*.*"]),
                3 => {
                    doc["jobs"]["artifacts"]["env"]["RELEASE_PLUGIN"] = "groundline-insights".into()
                }
                _ => {
                    doc["jobs"]["qualification"]["steps"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|s| {
                            s["name"] == "Exercise the rendered ClickHouse, API, and Grafana stack"
                        })
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .remove("if");
                }
            }
            assert!(
                !super::independent_releases_are_scoped(&doc.to_string()),
                "action {action}"
            );
        }
    }

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
