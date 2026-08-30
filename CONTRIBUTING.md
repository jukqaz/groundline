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

## Change Rules

- Keep dependencies minimal and justify every new network or platform surface.
- Keep Codex as the only supported runtime.
- Preserve ARM64 and x86_64 support on macOS, Linux, and Windows.
- Keep Core read-only and offline by default. Keep Insights opt-in, Tailnet-only,
  authenticated, and privacy-bounded.
- Do not commit provider auth files, sessions, shell snapshots, logs, caches,
  raw prompts, transcripts, or secret values.
- Do not add lifecycle hooks, network clients, background workers, or collector
  identities to `plugins/groundline`; keep those capabilities isolated in
  `plugins/groundline-insights` and its feature-gated Rust modules.

## Verification

Run the smallest credible gate for the change. For release-sized changes, run:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo deny check --show-stats
cargo run --locked -p xtask -- verify-source --root . --json
actionlint .github/workflows/rust.yml
```

Report every skipped target build as `UNVERIFIED` and include the exact command.

## Pull Requests

- Use one logical intent per pull request.
- Explain the user-facing behavior change.
- Include verification commands and results.
- Call out any mutation boundary, provider runtime path, or external command
  change explicitly.
