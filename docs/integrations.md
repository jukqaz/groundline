# Integrations and installation profiles

GroundLine ships two independent Codex plugins from one marketplace. Installing
one plugin never installs or activates the other.

## Choose a profile

| Profile | Install | External service | Intended use |
| --- | --- | --- | --- |
| Core only | `groundline` | None | Local guidance, audits, and evidence contracts |
| Insights only | `groundline-insights` | Owner-operated Insights service | Collector or operations nodes that do not need Core skills |
| Core and Insights | Both plugins | Owner-operated Insights service | Local GroundLine workflows plus private aggregate reporting |

Register the marketplace once, then install only the selected plugin IDs:

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
codex plugin add groundline-insights@groundline --json
```

The two `plugin add` commands are alternatives unless the user deliberately
chooses the combined profile. Refreshing the shared marketplace does not opt a
user into an uninstalled sibling plugin.

## Current Insights integrations

Insights uses this direct path:

```text
Native Codex App / CLI -> trusted plugin hooks + read-only native activity
                      -> private aggregate outbox -> Tailnet Insights API
                      -> ClickHouse -> Grafana / owner JSON reports
```

No inference proxy, generated model catalog, custom provider, or Core plugin is
required. Insights does not read or repair Codex `config.toml`, model catalogs,
inference credentials, or proxy configuration. Removing an inference wrapper
does not require changing the Insights API, ClickHouse, or Grafana integration.
Restore native Codex startup separately if the wrapper left provider overrides;
do not reset Insights identity, consent, cursors, or pending events as a shortcut.
Use the same native `CODEX_HOME`; an explicitly different home is a different
source, not an automatic state migration.

`doctor` and `worker status` discover the highest numeric `state_<n>.sqlite`
through the same native reader, rather than requiring `state_5.sqlite`.
An unavailable source is reported as `native_activity_unavailable`, with
`ready_to_collect: false`. Presence is not schema validation, API acceptance,
or dashboard freshness. Existing pending delivery and operator-action reasons
retain priority; a missing source must not delete or prevent draining the outbox.

| Surface | Status | Contract |
| --- | --- | --- |
| Codex App | Built in | Four fail-open lifecycle checkpoints after explicit activation |
| Codex CLI | Built in | Desktop, local headless, and remote headless runtime metadata |
| Tailscale/Tailnet | Required transport | A Tailnet IPv4 address or `*.ts.net` HTTPS endpoint; arbitrary public endpoints are rejected |
| GroundLine Insights API | Built in | Rust/Axum enrollment, upload, report, and administration API |
| ClickHouse | Required storage | API-owned schema migration, idempotent event ingestion, and fixed report views |
| CLI JSON reports | Built in | Strict 7, 30, or 90-day owner reports using a separate admin-token file; collector tokens are rejected |
| Grafana | First-party dashboard | Provisioned ClickHouse datasource and fixed GroundLine dashboard queries |
| Docker Compose | Public self-hosting preview | Generic placeholder-only service topology, authenticated Grafana, and private rendered secrets |
| TrueNAS | Optional operator overlay | Owner-run preflight/apply controller layered over the generic deployment contract; no private inventory is shipped |

See [self-hosting GroundLine Insights](self-hosting.md) for the versioned source
checkout, private render, real stack, semantic Grafana verification, and
collector enrollment sequence. Server deployment runs from a source checkout;
installing the Codex plugin alone does not install Docker services.

The collector and API contracts are release-qualified independently from the
public Compose preview. A production claim additionally requires the exact
release image digest, fresh-host stack verification, and an external
TLS/Tailnet authentication check for that operator deployment.

Every operator supplies their own private endpoint, enrollment credential,
storage, retention, and access control. Installing the public plugin does not
connect a user to the maintainer's ClickHouse, Grafana, or Tailnet.

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

A person and an LLM use the same documented `worker configure`, `enable`,
`run-once`, and `status` commands. The owner supplies the service address and
enrollment credential; neither installation nor configuration is collection
consent. Enable collection explicitly after reviewing its scope. Server
deployment is a separate operator step, and upgrading a public plugin does not
upgrade or reconfigure anyone's private service.

## User-selectable operations

An owner can choose whether to install Insights, when to enable or disable it,
which HTTPS or optional Tailnet endpoint to use, when to explicitly retry collection,
and whether to consume strict JSON reports or the supplied Grafana
dashboard. Report windows are 7, 30, or 90 days.

The first collection covers seven days; subsequent runs use the saved cursor.
`worker backfill-history --confirm-rebuild` retries that same collection path;
it does not rewind the cursor, replay all history, or replace existing server
events. Unrecoverable historical gaps need an explicit owner decision and a
preserved gap record before changing the collection boundary.

The current privacy contract intentionally fixes aggregate-only collection,
native hook checkpoints, a 900-second minimum checkpoint interval, disabled
diagnostics, no ambient proxy discovery, no redirects, and ClickHouse-backed
storage. These are safety invariants, not user preferences.

## Not currently supported

GroundLine Insights does not currently provide:

- Claude, Hermes, Antigravity, or generic provider collectors;
- arbitrary Internet, webhook, Slack, OpenTelemetry, or Prometheus exports;
- PostgreSQL, SQLite, S3, or pluggable storage backends;
- Grafana Cloud account provisioning or a hosted GroundLine SaaS;
- raw prompt, response, transcript, command, patch, path, repository, task, or
  account export.

Future integrations should be explicit adapters with a versioned contract,
disabled-by-default activation, owner-supplied credentials, bounded payloads,
and dedicated source, package, runtime, storage, and dashboard verification.
