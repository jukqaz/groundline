# Security

Insights is optional and independently installed. Collection requires a valid
schema-7 profile, separate enrollment credential, explicit `worker enable`
consent, and hook trust. See the [contract](references/insights-contract.md) for
formats, limits, transactions, and recovery.

## Current boundaries

- Five fail-open lifecycle wrappers invoke only an existing packaged binary,
  read no hook input, emit no output, and never download/build code.
- The collector reads bounded Codex state read-only and writes atomic private
  state. Strict aggregates exclude prompts, responses, transcripts, commands,
  patches, paths, repositories, task/rollout/account IDs, hostnames, and IPs.
- Tailnet reachability grants no authority. Owner enrollment, per-collector,
  admin, and trusted-proxy credentials have separate scopes.
- Endpoints are owner-selected HTTPS origins; HTTP is limited to loopback
  development or Tailnet. Webhooks/exporters are unsupported. Missing API
  network-mode configuration preserves Tailnet restriction; new general HTTPS
  renders explicitly set `GROUNDLINE_REQUIRE_TAILNET=false`.
- Stop revokes policy and waits for active bounded requests/reads. Each new
  phase rechecks policy and consent under a shared lock. Transmitted requests
  cannot be recalled; unsent events and received ACKs remain preserved. Every
  running collector, including detached processes, must use the updated binary.
- Bearer clients reject redirects and ambient proxies. API logs use fixed
  reasons, never paths, headers, payloads, identifiers, credentials, or exception
  text. Real endpoints, secrets, dataset paths, infrastructure inventory, and
  receipts stay outside public Git, packages, and CI. The TrueNAS controller
  takes its enrollment credential only from owner-local environment state.
- No MCP server, cron, timer, global hook, daemon, account linking, or automatic
  experiment is installed. Collection uses non-overlapping windows, resumable
  sync, a bounded outbox, and an advisory lock; failures are non-fatal to Codex.

Local durability, API acceptance, ClickHouse visibility, and Grafana freshness
are separate proof. [Troubleshooting](references/operations-troubleshooting.md)
never treats a reset or deletion as a substitute for missing evidence.

## Supported versions and reporting

Security fixes target the latest stable release. Use GitHub private vulnerability
reporting with version, platform, minimal reproduction, and redacted outcome.
Never post credentials, private hostnames/paths, raw Codex content, or provider
authentication files in public issues.
