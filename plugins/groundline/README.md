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

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
```

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
groundline audit weekly --days 7 --json
groundline efficiency batch --input batch.json --json
groundline efficiency compare --input comparison.json --json
```

`project-audit` counts Codex guidance, config, skills, agents, rules, plugins,
and `.worktreeinclude` without reading or returning their values. Audit commands
read the local Codex state store without modifying it. Efficiency commands accept
explicit JSON files and never transmit them.

## Development

```console
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -p xtask -- verify-source --root . --json
```

Pull requests run only the fast lane. Full qualification and six-platform release
artifacts are explicit manual workflows, with concurrency cancellation, timeouts,
and short artifact retention.

See the repository [integration profiles](../../docs/integrations.md),
[Privacy](../../docs/privacy.md), [Security](../../SECURITY.md), and
[release checklist](../../docs/release-checklist.md).
