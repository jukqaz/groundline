# GroundLine

GroundLine is a public, local-first Codex plugin for repeatable task setup,
evidence-aware completion, project configuration audits, and aggregate usage
analysis. It complements Codex; it does not replace Codex execution, settings,
permissions, agents, worktrees, review, or upgrades.

## Privacy boundary

The public plugin has a deliberately small capability surface:

- no lifecycle hooks, background process, scheduler, or collector identity;
- no network client, upload destination, authentication token, or remote storage;
- no prompt, transcript, path, repository name, or configuration value emission;
- local audit commands open bounded regular files read-only and return aggregate
  counters or stable reason codes.

`groundline provider-smoke --plugin-root <path> --json` fails if an owner hook
manifest is present. Repository qualification rejects personal or secret markers,
Python runtime dependencies, duplicate package roots, and CI contract drift.

## Install and upgrade

Add `https://github.com/jukqaz/groundline.git` as a Codex marketplace and install
the `groundline` plugin. This installs Core only; it does not install or activate
`groundline-insights`. Codex owns refresh and upgrade. GroundLine does not
self-update or change plugin trust.

To apply GroundLine after installation, ask `$groundline:align-agent-home` to
inspect and repair evidenced mistakes in existing settings and active guidance,
with private backups and verification. Follow
[installation alignment](references/installation-alignment.md). Native package
installation alone does not execute this workflow or rewrite personal settings.

The repository's reviewed stable distribution includes `install.sh` and
`install.ps1` for installation and setup in one invocation. Its common baseline
is **gpt-6-astra / xhigh / Fast off**. The installed `groundline setup --catalog
<native-models.json> --apply` command applies it with private backups, native
context restoration, and bounded retired Core hook trust cleanup. Without
`--apply`, it previews without writes. Unsupported host catalog choices fail
explicitly. Other user settings and Insights state are preserved.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
```

Use [native upgrade](references/native-upgrade.md) to refresh the installed
package. Version tags contain source; use the binary-bearing `stable`
distribution for installation.

After an upgrade, verify the installed package and native artifact independently:

```console
groundline provider-smoke --plugin-root /path/to/installed/groundline --require-installed --json
groundline doctor --plugin-root /path/to/installed/groundline --json
```

The package supports Apple Silicon and Intel macOS, ARM64 and x86_64 Linux, and
ARM64 and x86_64 Windows. Release artifacts are built from the moving Rust
`stable` channel and include a strict manifest plus SHA-256 checksum.
Resolve the executable from the installed plugin's `bin/<target>` directory;
plugin installation does not by itself promise a user-shell `PATH` entry.

## Commands

```console
groundline platform --json
groundline project-audit --repo . --json
groundline config-audit --config /private/config.toml --catalog /private/models.json --json
groundline setup --catalog /private/models.json --apply
groundline config-repair --config /private/config.toml --catalog /private/models.json
groundline guidance audit --profile /private/review/profile.json --baseline /private/review/baseline.json --json
groundline audit weekly --days 7 --json
groundline efficiency batch --input batch.json --json
groundline efficiency compare --input comparison.json --json
```

`project-audit` counts Codex guidance, config, skills, agents, rules, plugins,
and `.worktreeinclude` without reading or returning their values. Audit commands
read the local Codex state store without modifying it. Efficiency commands accept
explicit JSON files and never transmit them.

`config-repair` previews bounded context-limit repairs and writes only with a
matching plan hash, `--apply`, and a new private backup. See
[configuration review](references/codex-configuration.md) for scope and recovery.
The existing `align-agent-home` skill handles installation alignment and
requested imported-skill maintenance.
`guidance snapshot` creates a new path-free private baseline without overwriting
skills or existing files. `guidance audit` freshly inventories profile-selected
roots, reports additions/removals, and optionally compares upstream checkouts.
The host-local profile and portable baseline use strict GroundLine contracts;
there is no personal-registry adapter or duplicate initialization command.
See [skill maintenance](references/skill-maintenance.md) for source review,
local-change preservation, and behavior-test routing. No language-specific SDK
or user-home verification script is required by the native commands.

The state database must be an owner-owned, non-symlinked regular file no larger
than 8 GiB. Audits read at most 100,000 thread metadata rows and accept rollout
files only below the canonical, non-symlinked `sessions` or `archived_sessions`
root. Streaming input is bounded to 1 GiB per rollout and 8 GiB per audit, with
at most 512 MiB of retained audit records. See [weekly audit](references/weekly-usage-audit.md)
for event windows, independent native/UI baselines, and incomplete history.

## Development

```console
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -p xtask -- verify-source --root . --json
```

Pull requests run only the fast lane. Full qualification and six-platform release
artifacts run for release tags or explicit manual requests, with concurrency cancellation, timeouts,
and short artifact retention.

See the repository [integration profiles](https://github.com/jukqaz/groundline/blob/main/docs/integrations.md),
[Privacy](https://github.com/jukqaz/groundline/blob/main/docs/privacy.md),
[Security](SECURITY.md), and
[release checklist](https://github.com/jukqaz/groundline/blob/main/docs/release-checklist.md).

## Personal workflow improvement

Use `$groundline:improve-personal-workflow` to combine Insights reports, native
audits, current model guidance, and private completion evidence. Core personal
commands review, trial, evaluate, and restore dedicated guidance while preserving
user edits. Missing evidence never authorizes a change. See [the contract](references/personal-improvement.md).
