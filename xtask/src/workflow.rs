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
    for required in [
        "workflow_dispatch:",
        "build_release_artifacts:",
        "pull_request:",
        "push:\n    tags:",
        "concurrency:",
        "cancel-in-progress: true",
        "name: fast source checks",
        "name: full source qualification",
        "if: github.event_name == 'workflow_dispatch'",
        "needs: fast",
        "cargo test --locked -p xtask --all-targets",
        "cargo test --locked -p groundline-cli --test cli_contract",
        "cargo test --workspace --all-features --locked",
        "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
        "cargo run --locked -p xtask -- verify-source --root . --json",
        "retention-days: 14",
        "name: promote binary stable channel",
        "git merge-base --is-ancestor",
        "cargo run --locked -p xtask -- promote-stable",
        "uses: ./.github/actions/setup-rust-stable",
    ] {
        if !rust.contains(required) {
            return Err(XtaskError::InvalidSource);
        }
    }
    if rust.contains("\n  push:\n    branches:")
        || rust.contains("self-hosted")
        || rust.contains("GROUNDLINE_TRUSTED")
        || rust.contains("infrastructure/")
        || rust.contains("services/")
        || rust.contains("hooks/**")
        || rust
            .matches("cargo test --workspace --all-features --locked")
            .count()
            != 1
        || rust
            .matches("cargo clippy --workspace --all-targets --all-features --locked")
            .count()
            != 1
        || rust.matches("runs-on:").count() != rust.matches("timeout-minutes:").count()
        || !actions_are_pinned(&rust)
        || !setup.contains("using: composite")
        || !setup.contains("rustup toolchain install")
        || setup.contains("curl ")
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
