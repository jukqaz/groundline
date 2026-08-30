# Security policy

## Supported version

Security fixes target the latest stable GroundLine release.

## Product boundaries

GroundLine Core is offline and hook-free. It rejects symlinked or oversized
bounded inputs, opens Codex SQLite read-only, and emits aggregates or reason codes.

GroundLine Insights is a separate opt-in plugin. It owns exactly four fail-open
Codex lifecycle hooks, owner-private local state, a no-proxy/no-redirect Tailnet
client, an authenticated Axum API, ClickHouse, and Grafana. Tailnet reachability
alone never authorizes enrollment: first contact also requires an owner-issued
enrollment credential, then every collector uses a distinct token. Administrative
and trusted-proxy tokens remain separate.

All secret files are outside the plugin, opened as bounded regular files, and
required to be private to the current user where the platform exposes permission
checks. Public templates contain placeholders only. Logs and error receipts must
not echo endpoints, headers, tokens, collector IDs, payloads, private paths, or
exception text.

Source qualification checks both canonical plugin manifests, the Core zero-hook
invariant, the Insights four-hook invariant, personal/secret markers, pinned CI
actions, bounded jobs, and the moving Rust stable channel. These controls reduce
accidental exposure but do not replace review or live deployment validation.

## Reporting

Use GitHub private vulnerability reporting. Never include credentials, private
hostnames, private paths, raw prompts, transcripts, or personal data in a public
issue. Include the version, platform, minimal reproduction, and redacted output.
