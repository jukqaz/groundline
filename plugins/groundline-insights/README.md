# GroundLine Insights

GroundLine Insights is the optional self-hosted data companion shipped from the
same public monorepo as GroundLine Core. It owns only the networked surface:

- four fail-open Codex lifecycle hooks;
- owner-private identity, consent, checkpoint, credential, and outbox state;
- Tailnet-only collection and owner reports;
- a Rust/Axum API, ClickHouse schema, Grafana dashboards, and generic deployment
  tooling.

It does not package GroundLine skills, alter global Codex configuration, install
a daemon or scheduler, or replace Codex permissions and execution.

## Install and upgrade

Register the monorepo once and install the Insights plugin from the same
marketplace as Core:

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline-insights@groundline --json
codex plugin marketplace upgrade groundline --json
```

The packaged executable is `groundline-insights` (`groundline-insights.exe` on
Windows). macOS, Linux, and Windows are supported on ARM64 and x86-64.

```console
groundline-insights provider-smoke --require-installed --json
groundline-insights worker status
groundline-insights worker run-once
groundline-insights insights fetch-report --days 7 --json
```

## Owner configuration

Installation does not activate collection. `worker configure` accepts a reviewed
schema-7 input containing a Tailnet endpoint and an owner-issued
`enrollment_token`. It writes a sanitized profile and the credential to separate
owner-private files under `~/.codex/groundline/insights`; the secret is never
printed or copied into the plugin. First-contact enrollment requires both
Tailnet reachability and that credential. Each collector then uses its own token.

```console
groundline-insights worker configure --input owner-profile.json
groundline-insights worker enable
groundline-insights worker run-once
```

The input file is owner-private operational material and must not be committed.
Collection uses bounded aggregate counters from Codex's read-only state database.
The wire contract excludes raw prompts, responses, transcripts, commands, patches,
paths, hostnames, repository names, task IDs, rollout IDs, account identifiers,
and IP addresses.

The API is the canonical ClickHouse schema migrator. Collection tables use
`ReplacingMergeTree`; report and Grafana queries read the `basic_active` view,
which uses `FINAL` for logically deduplicated results. A retry is idempotent at
the API boundary, while the storage report keeps any physical duplicate rows
observable instead of hiding data-quality drift.

## Evidence lanes

Marketplace refresh, package checksum, four effective hooks, lifecycle dispatch,
accepted upload, ClickHouse visibility, Grafana query frames, image publication,
deployment, and stable promotion must be proven independently. Operational
endpoints, credentials, dataset paths, and receipts remain outside public Git and
public CI never receives production credentials.

See [operations troubleshooting](references/operations-troubleshooting.md),
[native upgrade](references/native-upgrade.md), and the repository
[privacy policy](../../docs/privacy.md).

License: MIT.
