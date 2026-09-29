# Integrations and installation profiles

GroundLine provides independent Core and Insights plugins for macOS and Linux
on ARM64 and x86-64. Use the [installation guide](installation.md) for commands,
updates, and recovery. There is no separate GroundLine Desktop app.

## Choose a profile

| Profile | Install | External service | Intended use |
| --- | --- | --- | --- |
| Core only | `groundline` | None | Local guidance, audits, and evidence contracts |
| Insights only | `groundline-insights` | Owner-operated Insights service | Collector or operations nodes that do not need Core skills |
| Core and Insights | Both plugins | Owner-operated Insights service | Local GroundLine workflows plus private aggregate reporting |

Installing one plugin does not install or activate its sibling. Refreshing their
shared marketplace can update both **already installed** plugins; the installer
preserves enabled/disabled states and checks existing Insights API compatibility.

## Current Insights integrations

Insights uses this direct path:

```text
Native Codex App / CLI -> trusted plugin hooks + read-only native activity
                      -> private aggregate outbox -> owner HTTPS Insights API
                      -> ClickHouse -> Grafana / owner JSON reports
```

Core, an inference proxy, and a custom model provider are not required. Insights
does not repair Codex settings or inference credentials. Removing a wrapper or
old Desktop app is not a reason to reset identity, consent, cursors, or outboxes.
Native hooks run collection while the computer is awake and the server is
reachable; there is no periodic delivery without hook activity.

## Runtime and source selection

Use the intended `CODEX_HOME`: another home is another source, not a migration.
App and CLI share the connection profile but retain separate collection state
and consent. Hooks supply the native origin. Select the intended source for
manual commands, for example:

```console
GROUNDLINE_RUNTIME_FAMILY=codex_app GROUNDLINE_EXECUTION_MODE=desktop groundline-insights worker status
GROUNDLINE_RUNTIME_FAMILY=codex_cli GROUNDLINE_EXECUTION_MODE=local_headless groundline-insights worker status
```

`remote_headless` is also supported. Unsupported runtime/mode/origin overrides
fail before state writes. Disabling one source does not disable the other.
See [Codex compatibility](codex-compatibility.md#known-evidence-limits) for native
activity selection, missing sources, and imported or shared histories.

| Surface | Status | Contract |
| --- | --- | --- |
| Codex App | Built in | Four fail-open lifecycle checkpoints after explicit activation |
| Codex CLI | Built in | Desktop, local headless, and remote headless runtime metadata |
| HTTPS | Default transport | Owner-selected HTTPS origin with certificate verification and no redirects |
| Tailscale/Tailnet | Optional transport | Tailnet IPv4 or `*.ts.net`; only these endpoints require a local Tailnet probe |
| GroundLine Insights API | Built in | Rust/Axum enrollment, upload, report, and administration API |
| ClickHouse | Required storage | API-owned schema migration, idempotent event ingestion, and fixed report views |
| CLI JSON reports | Built in | Strict 7, 30, or 90-day owner reports using a separate admin-token file; collector tokens are rejected |
| Grafana | First-party dashboard | Provisioned ClickHouse datasource and fixed GroundLine dashboard queries |
| Docker Compose | Public self-hosting preview | Generic placeholder-only service topology, authenticated Grafana, and private rendered secrets |
| TrueNAS | Optional operator overlay | Owner-run preflight/apply controller layered over the generic deployment contract; no private inventory is shipped |

[Self-hosting](self-hosting.md) covers server deployment, authenticated access,
and stack verification. Installing the plugin does not install Docker services
or connect to a maintainer endpoint. Production evidence requires the selected
image digest and actual host, storage, dashboard, and external access checks.

## Private-owner deployment boundary

The supported model is bring-your-own service, not registration for a shared
maintainer service. Each independent owner runs a separate Insights instance
with their own storage and credentials, and configures only the Codex homes they
intend to collect. Collector UUIDs distinguish installations within that owner
instance; they are not a multi-tenant account or tenant-isolation boundary.
There is no public signup, automatic service discovery, or default maintainer
endpoint. Keep a personal deployment's configuration and data outside the public
repository and release packages.

| Credential | Where it belongs | Purpose |
| --- | --- | --- |
| TrueNAS management API key, when used | Private operator credential store | Inspect and deploy NAS apps; never install it on collector-only hosts |
| Insights enrollment credential | Private server configuration and authorized collector setup | Register a collector with the chosen owner service |
| Per-collector token | That collector's private local state | Authenticated collector-scoped delivery and operations |
| Insights admin token and Grafana login | Private owner operations and dashboard access | Owner-wide reports and dashboard administration, not collector enrollment |

Installing or configuring Insights does not grant collection consent. Enable it
explicitly with [Insights setup](installation.md#add-insights-in-the-same-flow).
Plugin updates do not update the private service. Collection windows, retries,
and owner reports are documented in [operations](insights-operations.md#collector-verification-and-retries).

## Not currently supported

GroundLine Insights does not currently provide:

- Claude, Hermes, Antigravity, or generic provider collectors;
- generic webhook, Slack, OpenTelemetry, or Prometheus exports;
- PostgreSQL, SQLite, S3, or pluggable storage backends;
- Grafana Cloud account provisioning or a hosted GroundLine SaaS;
- raw prompt, response, transcript, command, patch, path, repository, task, or
  account export.
