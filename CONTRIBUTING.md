# Contributing

GroundLine is a small local-first plugin and Rust workspace. Changes should
keep the runtime easy for humans to run and easy for LLM agents to inspect.

## Local Setup

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
cargo run --locked -p xtask -- verify-source --root . --json
```

No package install is required for the core checks.

Developer and agent guidance is maintained in English. Keep Korean installation
and operator documentation aligned with the English behavior and commands;
structured JSON keys and reason codes remain English.

## Change Rules

- Keep dependencies minimal and justify every new network or platform surface.
- Keep Codex as the only supported runtime.
- Preserve ARM64 and x86_64 support on macOS, Linux, and Windows.
- Keep Core read-only and offline by default. Keep Insights opt-in,
  authenticated, and privacy-bounded over HTTPS with optional Tailnet access.
- Do not commit provider auth files, sessions, shell snapshots, logs, caches,
  raw prompts, transcripts, or secret values.
- Do not add lifecycle hooks, network clients, background workers, or collector
  identities to `plugins/groundline`; keep those capabilities isolated in
  `plugins/groundline-insights` and its feature-gated Rust modules.

## Verification

Run the smallest credible gate for the change. For release-sized changes, run:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo deny check --show-stats
cargo run --locked -p xtask -- verify-source --root . --json
cargo run --locked -p xtask -- verify-history --root . --json
cargo run --locked -p xtask -- verify-compatibility-profile --json
actionlint .github/workflows/rust.yml
git diff --check
```

For documentation-only edits, check links and `verify-source`; rerun the relevant
xtask checks if workflow metadata changed. Do not repeat the full Rust suite for
prose alone. See the [release checklist](docs/release-checklist.md) for isolated
ClickHouse, package, image, and deployment verification.

Report unexecuted platform or live checks as `UNVERIFIED`.

## Pull Requests

- Use one logical intent per pull request.
- Explain the user-facing behavior change.
- Include verification commands and results.
- Call out any mutation boundary, provider runtime path, or external command
  change explicitly.

`main` owns source and `stable` owns the verified binary distribution. A source
release tag does not include the generated `bin` trees. Retire completed topic
branches only after checking the merged PR and resulting tree; preserve open
PRs, active worktrees, unreconciled changes, release tags, and recoverable work.
