# Security policy

## Supported version

Security fixes target the latest stable GroundLine release.

## Product boundaries

GroundLine Core is offline and hook-free. It rejects symlinked or oversized
bounded inputs, opens Codex SQLite read-only, and emits aggregates or reason codes.

GroundLine Insights is a separately installed opt-in plugin and does not require
Core. It owns exactly four fail-open Codex lifecycle hooks, owner-private local
state, a no-proxy/no-redirect HTTPS client, an authenticated Axum API,
ClickHouse, and Grafana. Collectors accept an owner-selected HTTPS origin;
Tailnet access is optional. Plain HTTP is limited to loopback development and
Tailnet endpoints. Generic exporters are unsupported. TLS terminates at the
operator's HTTPS proxy. New Compose renders explicitly select general HTTPS;
an absent API network-mode variable preserves the existing Tailnet restriction.
Tailnet reachability alone never authorizes enrollment: first contact also
requires an owner-issued enrollment credential, then every collector uses a
distinct token. Administrative and trusted-proxy tokens remain separate.

Collection stop revokes policy and waits for the active request or collection
read before returning success. Every new phase/request rechecks policy and
consent under a process-shared lock. Already transmitted requests cannot be
recalled; unsent events stay local. Upgrade all collector processes to apply
this boundary, since an already running older executable cannot use the new lock.

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
