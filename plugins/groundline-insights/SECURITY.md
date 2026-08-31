# Security

GroundLine Insights is the optional public-source, owner-operated data companion
for GroundLine. It is independently installable, is not the Core guidance plugin,
and remains inactive until the owner configures a schema-7 profile, a separate
enrollment credential, and hook trust.

## Current boundaries

- Exactly four fail-open Codex lifecycle hooks invoke the packaged Rust binary
  only when it exists. The wrappers read no hook input, emit no output, and
  never download or build code.
- The Rust collector opens bounded Codex state read-only, writes atomic private
  local state, and emits strict aggregate contracts. Raw prompts, responses,
  transcripts, commands, patches, paths, repository names, task IDs, rollout
  IDs, account identifiers, hostnames, and IP addresses are excluded.
- Tailnet reachability is not authorization. Enrollment also requires the
  owner-issued credential; the server then authenticates each collector with a
  distinct token. Admin and trusted-proxy credentials remain separate.
- Collector endpoints are restricted to Tailnet IPv4 or `*.ts.net`; arbitrary
  public Internet, webhook, and observability-export endpoints are unsupported.
- Bearer clients reject redirects and ambient proxy discovery. The API uses
  fixed reason codes and must not log paths, headers, payloads, identifiers,
  credentials, or exception text.
- The public repository, release workflow, and plugin packages contain no real
  endpoint, credential, dataset path, infrastructure inventory, or deployment
  receipt. The TrueNAS controller receives its enrollment credential only from
  owner-local environment state and emits redacted receipts.
- No MCP server, cron entry, timer, global hook, OS daemon, account linking, or
  automatic experiment is installed.

The owner worker uses non-overlapping activity windows, a resumable initial
history sync, a bounded outbox, and a cross-platform advisory lock. Failure is
non-fatal to Codex and remains distinct from server acceptance, ClickHouse
visibility, and Grafana freshness.

## Supported versions and reporting

Security fixes target the latest stable release. Report vulnerabilities through
GitHub private vulnerability reporting. Never include credentials, private
hostnames, private paths, raw Codex content, or provider authentication files in
a public issue. Include the affected version, platform, minimal reproduction,
and redacted outcome.
