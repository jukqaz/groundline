# GroundLine

GroundLine is one public Rust monorepo with two independently installable Codex
plugins. Codex remains responsible for execution, settings, permissions, agents,
worktrees, review, compaction, and upgrades.

| Plugin | Purpose | Default network behavior |
| --- | --- | --- |
| `groundline` | Local guidance, project audits, evidence boundaries, and aggregate usage analysis | Offline; no hooks or collector identity |
| `groundline-insights` | Optional self-hosted aggregate collection, ClickHouse reports, and Grafana dashboards | Disabled until an owner profile and enrollment credential are configured |

The plugin packages are canonical under `plugins/`. Shared Rust contracts and
runtime code live under `crates/`; generic self-hosting assets live under
`infrastructure/` and `services/`. Real endpoints, credentials, dataset paths,
deployment receipts, and infrastructure inventories must remain outside Git.

## Install and upgrade

Register this repository once on the moving `stable` branch, then choose a
profile. The plugin IDs are independent; installing one never installs or
activates the other.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
```

Core only (the offline default):

```console
codex plugin add groundline@groundline --json
```

Insights only (for an owner-operated collector or operations node):

```console
codex plugin add groundline-insights@groundline --json
```

Run both `plugin add` commands only when the combined profile is desired.

Refresh the single marketplace snapshot to adopt a newer release:

```console
codex plugin marketplace upgrade groundline --json
codex plugin list --json
```

An immutable release tag can be used instead of `stable` for rollback or a
frozen installation. Marketplace refresh, installed package checksums, hook
trust, collector upload, ClickHouse visibility, Grafana frames, image
publication, deployment, and stable promotion are separate evidence lanes.

## Privacy and security

Core never installs lifecycle hooks or performs network requests. Insights owns
exactly four fail-open Codex lifecycle hooks. It reads bounded aggregate counters
from Codex's SQLite state in read-only mode and rejects raw prompts, responses,
transcripts, commands, patches, paths, repository names, task IDs, rollout IDs,
account identifiers, hostnames, and IP addresses from its wire contract.

Tailnet reachability is not authorization. First-contact enrollment additionally
requires an owner-issued credential stored in a private file outside the plugin.
Each collector then uses a distinct token. The public repository contains only
placeholders and generic deployment templates.

## Development

Use the fast lane while editing:

```console
cargo fmt --all -- --check
cargo test --locked -p xtask --all-targets
cargo test --locked -p groundline-cli --test cli_contract
cargo test --locked -p groundline-insights-cli --test cli_contract
actionlint
```

Run the complete gate once after the change is frozen:

```console
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -p xtask -- verify-source --root . --json
git diff --check
```

The GitHub workflow uses one fast pull-request lane. Full qualification and the
six-platform two-product artifact matrix run only for a manual request or a
release tag, with cancellation, timeouts, and bounded retention. No self-hosted
runner or production credential is required by public CI.

See [integrations and installation profiles](docs/integrations.md),
[Privacy](docs/privacy.md), [Security](SECURITY.md), and the
[release checklist](docs/release-checklist.md).
