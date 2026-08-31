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

| Surface | Status | Contract |
| --- | --- | --- |
| Codex App | Built in | Four fail-open lifecycle checkpoints after explicit activation |
| Codex CLI | Built in | Desktop, local headless, and remote headless runtime metadata |
| Tailscale/Tailnet | Required transport | A Tailnet IPv4 address or `*.ts.net` HTTPS endpoint; arbitrary public endpoints are rejected |
| GroundLine Insights API | Built in | Rust/Axum enrollment, upload, report, and administration API |
| ClickHouse | Required storage | API-owned schema migration, idempotent event ingestion, and fixed report views |
| CLI JSON reports | Built in | Strict 7, 30, or 90-day owner reports |
| Grafana | First-party dashboard | Provisioned ClickHouse datasource and fixed GroundLine dashboard queries |
| Docker Compose | First-party self-hosting | Generic placeholder-only service topology and private rendered secrets |
| TrueNAS | Supported operator path | Owner-run preflight/apply controller layered over the generic deployment contract |

Every operator supplies their own private endpoint, enrollment credential,
storage, retention, and access control. Installing the public plugin does not
connect a user to the maintainer's ClickHouse, Grafana, or Tailnet.

## User-selectable operations

An owner can choose whether to install Insights, when to enable or disable it,
which private Tailnet endpoint to use, whether to run an explicit initial
backfill, and whether to consume strict JSON reports or the supplied Grafana
dashboard. Report windows are 7, 30, or 90 days.

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
