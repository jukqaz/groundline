# GroundLine

[한국어](README.ko.md)

GroundLine is one public Rust monorepo with two independently installable Codex
plugins. Codex remains responsible for execution, settings, permissions, agents,
worktrees, review, compaction, and upgrades.

| Plugin | Purpose | Default network behavior |
| --- | --- | --- |
| `groundline` | Local guidance, project audits, evidence boundaries, and aggregate usage analysis | Offline; no hooks or collector identity |
| `groundline-insights` | Optional aggregate collection plus a public self-hosting preview for ClickHouse and Grafana | Disabled until an owner profile and enrollment credential are configured |

The plugin packages are canonical under `plugins/`. Shared Rust contracts and
runtime code live under `crates/`; generic self-hosting assets live under
`infrastructure/` and `services/`. Real endpoints, credentials, dataset paths,
deployment receipts, and infrastructure inventories must remain outside Git.
The Compose template has no baked-in infrastructure versions: a strict
compatibility profile selects a release-tested or newer candidate dependency
set, and the newer set must pass the same live stack verifier.

Insights is bring-your-own service: independent owners use separate private
instances, storage, and credentials. Public installation does not enroll a user
in the maintainer's service. See the [private-owner boundary](docs/integrations.md#private-owner-deployment-boundary).

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

`main` and version tags contain source; `stable` includes the verified native
`bin` trees required for plugin installation. Do not substitute a source tag for
the binary distribution. A frozen installation needs a verified packaged
revision. If refresh leaves an installed version unchanged, run `plugin add`
again for that same plugin ID, then verify its version and checksum.
Marketplace refresh, installed package checksums, hook
trust, collector upload, ClickHouse visibility, Grafana frames, image
publication, deployment, and stable promotion are separate evidence lanes.

## Maintain personal skills

For Codex configuration/model posture, use `groundline config-audit --config
<config.toml> --catalog <native-models.json> --json` (or `--catalog -` for stdin).
It checks selected fields against the supplied native catalog without printing
values or changing settings; native strict doctor validates effective config.
See [configuration review](plugins/groundline/references/codex-configuration.md).

Ask `$groundline:align-agent-home` to review or update imported skills. Core
handles local source tracking, drift/metadata checks, and private fingerprint
receipts with `groundline guidance audit|snapshot`. A host-local profile selects
roots; a separate path-free baseline records comparisons. Codex reviews upstream
changes, applies the authorized patch, and runs affected behavior tests. User
skills, settings, and provenance stay outside this public repository; plugin
upgrades never overwrite them. See [skill maintenance](plugins/groundline/references/skill-maintenance.md).

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
cargo test --locked -p groundline-contracts -p groundline-runtime --lib --all-features
cargo test --locked -p groundline-cli --test cli_contract
cargo test --locked -p groundline-insights-cli --test cli_contract
actionlint
```

Run the complete gate once after the change is frozen:

```console
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -p xtask -- verify-source --root . --json
cargo run --locked -p xtask -- verify-history --root . --json
cargo run --locked -p xtask -- verify-compatibility-profile --json
git diff --check
```

The GitHub workflow uses one fast pull-request lane. Full qualification and the
six-platform two-product artifact matrix run only for a manual request or a
release tag, with cancellation, timeouts, and bounded retention. No self-hosted
runner or production credential is required by public CI.

For an opt-in parsing/statistics benchmark, run
`cargo run --locked -p groundline-contracts --example audit_benchmark`.
It generates synthetic records, runs five measurements, and emits timing plus
an aggregate fingerprint without reading private data or using the network.
Compare the same build profile and machine, and require an identical fingerprint
before accepting a speedup. This benchmark is not run automatically by CI and
does not measure provider tokens, billing, or installed-plugin latency.

See [integrations and installation profiles](docs/integrations.md),
[Codex compatibility and update boundaries](docs/codex-compatibility.md),
[Insights self-hosting](docs/self-hosting.md), [Privacy](docs/privacy.md),
[Security](SECURITY.md), [changes](CHANGELOG.md), and the
[release checklist](docs/release-checklist.md).

## Personal workflow improvement

Use `$groundline:improve-personal-workflow` to combine Insights reports, native
audits, current model guidance, and private completion evidence. Core personal
commands review, trial, evaluate, and restore dedicated guidance while preserving
user edits. Missing evidence never authorizes a change. See [the contract](plugins/groundline/references/personal-improvement.md).
