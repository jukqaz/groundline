# GroundLine Insights

GroundLine Insights is the optional self-hosted data companion shipped from the
same public monorepo as GroundLine Core. It is independently installable and
does not require Core. It owns only the networked surface:

- four fail-open Codex lifecycle hooks;
- owner-private identity, consent, checkpoint, credential, and outbox state;
- Tailnet-only collection and owner reports;
- a Rust/Axum API, ClickHouse schema, Grafana dashboards, and generic deployment
  tooling.

It does not package GroundLine skills, alter global Codex configuration, install
a daemon or scheduler, or replace Codex permissions and execution.

## Install and upgrade

Register the monorepo once and install the Insights plugin. This does not install
Core; install `groundline@groundline` separately only when Core skills and local
audits are also wanted.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline-insights@groundline --json
codex plugin marketplace upgrade groundline --json
```

An upgrade that changes `hooks/hooks.json` requires a fresh Codex review of the
new hook hash. GroundLine never grants trust to itself. `codex plugin list`
proves installation and enablement, not that a changed hook was reviewed or
dispatched; verify a new-task lifecycle receipt separately.

See [integrations and installation profiles](../../docs/integrations.md) for the
Core-only, Insights-only, and combined choices.

The packaged executable is `groundline-insights` (`groundline-insights.exe` on
Windows). macOS, Linux, and Windows are supported on ARM64 and x86-64. Resolve
the executable from the installed plugin's `bin/<target>` directory; a Codex
plugin installation does not by itself promise a user-shell `PATH` entry.

```console
groundline-insights provider-smoke --require-installed --json
groundline-insights worker status
groundline-insights worker run-once
groundline-insights insights fetch-report \
  --admin-token-file /owner-private/admin-report-token \
  --days 7 --json
```

## Owner configuration

Installation does not activate collection. `worker configure` accepts a reviewed
schema-7 input containing a Tailnet endpoint and an owner-issued
`enrollment_token`. It writes a sanitized profile and the credential to separate
owner-private files under `~/.codex/groundline/insights`; the secret is never
printed or copied into the plugin. First-contact enrollment requires both
Tailnet reachability and that credential. Each collector then uses its own token.
The fleet-wide CLI report is an administrative operation: it requires an
explicit owner-private file containing only the separate admin token. Collector
tokens cannot fetch it. Keep that file off collector-only hosts and never commit
or print it.

```console
cp references/owner-profile.example.json owner-profile.json
# Replace the example endpoint and REPLACE_ME with owner-private values.
groundline-insights worker configure --input owner-profile.json
groundline-insights worker enable
groundline-insights worker run-once
```

`worker enable` is the explicit owner-service upload consent boundary. A legacy
receipt whose contract disabled network upload is never broadened in place. The
worker reports `reconsent_required`; running `worker enable` issues a new receipt
and moves any legacy pending events into owner-private quarantine rather than
uploading or deleting them.

`worker status` reports the operational lane separately: `collection_state`,
`ready_to_collect`, and bounded `blocking_reason_codes` distinguish an intentional
disabled state, missing or invalid configuration, an unverified/disconnected
Tailnet, a pending first collection, a seven-day stale collector, clock skew, and
an active collector. A Tailnet probe with `tailnet_connected: null` is unverified,
not proof of disconnection.

The input file is owner-private operational material and must not be committed.
The checked-in example deliberately contains an invalid short token and cannot
activate collection unchanged.
Collection uses bounded aggregate counters from Codex's read-only state database.
The wire contract excludes raw prompts, responses, transcripts, commands, patches,
paths, hostnames, repository names, task IDs, rollout IDs, account identifiers,
and IP addresses.

The supported collector runtimes are Codex App and Codex CLI. The worker accepts
only a Tailnet IPv4 or `*.ts.net` endpoint, and every operator supplies their own
private service and credentials. The official service path is the Rust/Axum API,
ClickHouse storage, strict 7/30/90-day CLI JSON reports, and the provisioned
Grafana dashboard. Docker Compose is the public self-hosting preview; TrueNAS is
an optional operator overlay, not a requirement for the plugin. A production
deployment still needs fresh-host, immutable-image, and external TLS evidence.

The API is the canonical ClickHouse schema migrator. Collection tables use
`ReplacingMergeTree`; report and Grafana queries read the `basic_active` view,
which uses `FINAL` for logically deduplicated results. A retry is idempotent at
the API boundary, while the storage report keeps any physical duplicate rows
observable instead of hiding data-quality drift.
The default service contract retains 365 days, caps each collector at 4,096
retained events and 256 MiB of logical payload, and stops ingest at 90% of the
configured dataset row or byte ceiling so administrative operations retain
capacity. Operators may change only the documented bounded environment values.
Local delivery is independently bounded to 256 queued events, 16 MiB, and
16-event upload batches. Retryable failures use durable capped backoff; permanent
remote rejection requires operator action. Grafana's TTL cleanup panel reports
rows whose retention deadline passed but which still await ClickHouse's
background merge. It never runs `OPTIMIZE` or deletes data.

## Evidence lanes

Marketplace refresh, package checksum, four effective hooks, lifecycle dispatch,
accepted upload, ClickHouse visibility, Grafana query frames, image publication,
deployment, and stable promotion must be proven independently. Operational
endpoints, credentials, dataset paths, and receipts remain outside public Git and
public CI never receives production credentials.

See [operations troubleshooting](references/operations-troubleshooting.md),
[native upgrade](references/native-upgrade.md), and the repository
[self-hosting guide](../../docs/self-hosting.md) and
[privacy policy](../../docs/privacy.md). Unsupported sinks and future adapter
requirements are listed in the [integration matrix](../../docs/integrations.md).

License: MIT.
