# Insights metrics and operations

Insights uses ClickHouse for deduplicated aggregate storage and Grafana for
inspection. Core performs no network collection. App and CLI are runtime
dimensions, not physical device identities. No hostname, account, IP address,
prompt, or transcript is needed for these dashboards.

## Collector verification and retries

After [installation](installation.md), select the intended
[runtime and Codex home](integrations.md#runtime-and-source-selection). These
commands inspect state and server compatibility; neither grants consent:

```console
groundline-insights worker status
groundline-insights worker check-server --json
```

Current collectors require Basic schema 5 and ingest contract revision 8 or
newer. Deploy the API first. A missing profile returns `NOT_CONFIGURED` without
network access; malformed profiles or incompatible/unreachable APIs fail without
changing local state. A compatible health response does not prove authentication,
accepted delivery, or hook trust. After explicit enablement and a real Codex
task, verify a fresh acknowledgement, matching ClickHouse row, and Grafana result.
An old acknowledgement does not establish a fresh upload.

The privacy contract fixes aggregate-only collection, native checkpoints with a
900-second minimum interval, disabled diagnostics, no ambient proxy discovery,
and no redirects. These are not user-selectable relaxations.

The first collection covers seven days; subsequent runs use the saved cursor.
An incomplete window stays frozen and stops automatic reads after three
attempts. Fix the reported cause before explicit `worker run-once`.
`worker backfill-history --confirm-rebuild` uses that same path: it does not rewind
the cursor or replace historical events. Preserve gap evidence and get an
explicit owner decision before changing a collection boundary. Disabling with
`worker disable` preserves evidence and pending data; reinstalling is not a reset.
Detailed failure codes are in [troubleshooting](../plugins/groundline-insights/references/operations-troubleshooting.md).

For a 7, 30, or 90-day owner report, use a private file containing only the
administrative report token; collector tokens are rejected:

```console
groundline-insights insights fetch-report --days 7 --admin-token-file /private/admin-token --json
```

## Dashboard contract

The overview shows the entire enrolled fleet, including installations with no
accepted event. Follow an installation's analysis link to preserve the selected
time range and inspect its usage. The analysis dashboard applies OS, runtime,
reported version, and installation filters to every panel. An opaque SHA-256
installation key is used for links; the four-character display suffix is not a
unique identifier. A random installation UUID is not a person or device count.

The installation roster exposes one **status** combining reporting freshness
and the authenticated package registration. The attention count uses the same
decisions. A latest package awaiting its first new-version aggregate is a
waiting state, not an update failure. Initial reporting grace and this waiting
state do not require attention; missing metadata, reporting delays, and outdated
or unsupported package versions do. Reporting age alone cannot establish that
a device is offline or that collection is disabled. Retiring an installation
removes it from the roster and usage data; it does not end platform support.

| Metric | Unit and denominator | Interpretation |
| --- | --- | --- |
| Observed roots | Sum of observed root windows | Period aggregate, not unique people or devices |
| Component nonpass | Aggregate windows | Root nonpass; or an observed/unreadable optional component that is nonpass. An unused optional component with `INSUFFICIENT_EVIDENCE` alone is excluded |
| Usage missing | Windows with an observed component and unavailable/unknown usage | Missing measurements are not measured zero usage |
| Tokens per completed turn | Root provider tokens / completed turns | Descriptive ratio; not price or a causal model comparison |
| Cache ratio | Cached input / input tokens | NULL for a zero denominator; not a whole-prompt cache hit rate |
| Mean window p90 | Mean of eligible window p90 durations | Not the pooled p90 and not model inference latency |
| Model/effort contexts | Native turn-context counts | Kept separate from response-attributed tokens |
| Model/effort tokens | Owned native response counters plus an explicit unattributed residual | Model/effort comes from an explicit turn link; all six token counters conserve the authoritative root/delegated total. No per-model completed-turn denominator is available |
| Verification success | Detected successes / detected successes plus failures | Tool-result proxy; unresolved calls remain separate |
| Delivery delay | Receipt minus generation, seconds | Over six hours is delayed, over 24 hours overdue; generation more than five minutes ahead is clock skew |
| TTL backlog | Physical rows past their retention deadline | Eventual background cleanup; not a reason for routine `OPTIMIZE FINAL` |

Time-series points group whole aggregate windows by their period end, falling
back to generation time. The interval follows Grafana's selected range with a
one-hour minimum. This display does not reconstruct activity within a source
window. Cohort comparisons require at least ten observed roots and retain
schema/version/runtime dimensions. Coverage and missingness must accompany any
comparison. Ratios and model labels do not establish improvement causally.

## Retired installations

Authenticated collector deletion first records the SHA-256 hash of its random
UUID and retirement time, then removes registry credentials and aggregate rows.
The deny record is retained for the deployment's lifetime: expiring it while an
old installation still holds enrollment credentials would permit re-enrollment.
No token, hostname, OS, runtime, or event content is retained in that record.
Deleting it is a separate owner-authorized state reset, not routine retention.

The same ID receives `403 collector_retired` on enrollment; a new installation
with a new UUID can enroll normally. The client preserves its local state and
outbox and requires operator action rather than silently generating a new ID.
Repeating DELETE finishes an interrupted deletion without reactivating the ID.
Retirement, activation, enrollment, and ingestion share the single API writer's
serialization gate. Multiple API writers need a separate concurrency design.
Already deleted IDs cannot be reconstructed from absent rows; use only a
verified owner-held retirement inventory when migrating earlier cleanups.

Windows is no longer a supported runtime. Historical Windows observations and
OS labels remain readable; removing support does not rewrite or delete history.

### Returning to a retired device

Retirement is scoped to the previous random installation ID, never an operating
system, machine, or user. A fresh macOS or Linux installation gets a
new ID and can enroll normally. Its dashboard status starts at initial-report
waiting and becomes normal after a current supported package reports.

Reinstalling plugin files alone deliberately preserves existing local state;
it does not replace a retired ID. If `collector_retired` remains after a
reinstall, request a fresh registration for that runtime. Stop its worker,
inventory and preserve its pending data, and explicitly authorize a fresh setup
of only that retired runtime's Insights state. Preserve Codex history, global
configuration, the shared owner profile, and every other runtime. Then enable
and verify the fresh registration using the current installer/setup flow.
Do not remove the server deny record or silently replay the retired outbox.

## Trust-column migration

`trusted_event_v5` is a materialized UInt8 result of the current envelope and
payload/projection validation predicate. Inserts compute it in ClickHouse.
Explicit values and unknown JSON fields are rejected. Startup checks its type,
materialized kind, predicate fingerprint, and parsed SQL expression. ClickHouse
may print that expression with different spacing and parentheses; the API
compares its `EXPLAIN AST` result with the source expression. A matching comment
alone cannot authorize a changed definition. Unknown schema states fail closed.

The one supported transition is from the previous released fingerprint
`d653ba15120d6bdb9b5d0d4077c2dd6fd0eb7aeb00f225c996f82964871ae316`.
Startup first makes `basic_current` return no rows, which also guards its active
and quarantined views. It switches TTL to the new predicate directly, then adds
`trusted_event_v5_revalidated` with a pending marker. Existing parts calculate
that new materialized column from their original fields, never from the old
stored trust bit. The API checks every row's new result, row count, and a hash
of event ID, collector ID, generation, receipt time, idempotency key, and raw
payload before and after one atomic metadata operation that replaces the old
column. It marks the new definition complete only after that check. The staged
column and pending comment let startup resume after a crash; repeated startup
is idempotent. The raw event fields are not rewritten by the transition.

The verification scans have a 20-second server limit and two threads. A scan
that does not finish leaves the published event views guarded and startup fails;
it does not launch an unbounded background materialization. Plan a maintenance
window for a large table. This is a per-scan limit, not a bound on the preceding
metadata changes or total startup; the API's 30-second ClickHouse request
timeout can also stop an overloaded migration before its scans. Stop old API
writers before upgrading, take a
recoverable backup, and inventory TTL-expired rows and disk headroom. Retention
merges may remove already expired rows, while concurrent inserts can change the
row count/hash between checks; either can require operator investigation. Do
not reset or overwrite an unknown column to make startup pass. Rehearse against
a separate database and compare original rows, active/quarantined counts, and
reports before applying the package to production.

`FINAL`, active-generation selection, retirement filtering, quarantine, and TTL
remain in the restored query path. Existing parts calculate the new column
lazily until an optional, separately planned materialization. For that later
maintenance, scope `MATERIALIZE COLUMN trusted_event_v5` to inventoried
partitions, await mutation completion, and repeat source-row and view checks.
Never mount production storage into a rehearsal. An image rollback alone does
not undo schema or TTL changes.

## Dependency upgrades and recovery

Use a complete compatibility JSON for ClickHouse, Nginx, Grafana, and the
ClickHouse datasource plugin. Images need exact digests and the plugin an exact
stable version. Validate before rendering with the same profile:

```console
cargo run --locked -p xtask -- verify-compatibility-profile --profile /private/compatibility.json --json
```

For discovery only, explicit moving image tags and the bare plugin ID
`grafana-clickhouse-datasource` require `--allow-unpinned-dependencies` on both
validation and rendering. After all stack/dashboard/variable/semantic queries
pass, record resolved digests and plugin version, rerender pinned, and repeat.
The manual CI workflow's four candidate inputs are all-or-none; passing does not
promote the default profile, publish artifacts, or deploy an owner service.

Before a production change:

1. Record image/plugin versions, configuration fingerprint, and application
   table counts. API-only updates do not update the other services.
2. Test the exact image on the actual CPU **before mounting storage**:
   `docker run --rm --network none --entrypoint clickhouse <image> --version`.
   CI on another CPU does not prove compatibility; qualify a supported profile
   or change hardware rather than silently substituting an image.
3. Pause ingestion and Grafana; preserve a consistent backup of ClickHouse data,
   Grafana DB/plugins, and private configuration. Rehearse on an isolated copy,
   never a production data mount, and compare every application table.
4. Before a stricter event contract or quarantine TTL, inventory stored events
   with `groundline-insights insights validate-event --input /private/event.json --json`.
   Preserve original data; do not fabricate measurements to repair invalid
   history. Deletion requires owner approval of the exact scope. Keep ingestion
   stopped between inventory and replacement.
5. Repeat [stack and external access checks](self-hosting.md#4-start-and-verify-the-real-stack),
   verify the installed datasource version, and match a fresh accepted upload to
   storage and dashboard output. Preserve credentials, consent, network policy,
   and retention. Restore a matching backup before restarting an older database
   image; an image-only rollback does not undo schema or TTL changes.

A complete lifecycle event may arrive before usage. Keep its observed lifecycle
counts for normal retention; unavailable usage is unmeasured, not zero cost.
Incomplete reads/incoherent provenance remain quarantined. Classification does
not rewrite event IDs, payloads, counters, or collection periods.

## Diagnostic logging

Defaults retain events for 365 days, cap each collector at 4,096 events and
256 MiB logical payload, and reserve 10% of the two-million-row/64 GiB dataset
ceiling for administration. Change `GROUNDLINE_RETENTION_DAYS`,
`GROUNDLINE_COLLECTOR_MAX_EVENTS`, `GROUNDLINE_COLLECTOR_MAX_PAYLOAD_BYTES`,
`GROUNDLINE_DATASET_MAX_ROWS`, or `GROUNDLINE_DATASET_MAX_BYTES` only in private
server configuration and within API bounds. Diagnostic TTL is separate from
event retention; template changes require actual configuration/TTL verification.



The 4 GiB ClickHouse container permits the server to use half its allocation,
leaving the other half for untracked/runtime overhead. A quarter-allocation
limit can reject even readiness queries after restore and dashboard workloads
raise process RSS above 1 GiB. Validate this bound on the deployed CPU and
workload; increasing the internal ratio does not increase the container limit.

Normal operation keeps server Information and higher messages in rotated logs
(20 MiB, three archived files) and Warning and higher in `system.text_log`.
Text-log TTL still expires older low-severity entries after seven days and
remaining severities after 30 days. Trace and processor-profile records keep
seven days. Query/metric logs remain disabled under the existing policy.

The default and Grafana profiles disable periodic CPU/wall sampling, allocation
profiling steps, and processor profiling. For a bounded owner diagnostic query,
enable only the necessary settings in that query's `SETTINGS` clause; they end
with the query. Preserve errors and crash diagnosis. Do not leave a global
debug/trace override enabled after the investigation. Verify effective settings
and log growth on the deployed CPU-qualified image before claiming a reduction.
Do not drop old `_0` system tables by name alone: inventory rows, dates, TTL,
backup needs, and active writers first.

## Screen-only monitoring

Status is inspected in Grafana. There are no external contact points, automatic
messages, or notification delivery claims. A datasource error is a failed query,
not a healthy zero; row-limit overflow throws instead of silently truncating.
No accepted events means unobserved. A notebook's receipt gap alone cannot
distinguish inactivity, sleep, disabled collection, or a network/service outage.
Inspect the installation detail and endpoint probes before labeling an outage.
After recovery, require a fresh accepted event and a matching dashboard result.

The API records bounded route/outcome counts and handler response duration every
60 seconds in `api_observations`, with a 30-day TTL. Its heartbeat measures the
API-to-DB observation path; after 180 seconds the screen asks for investigation.
No samples is unobserved. Request counts, mean/max handler duration, authentication
rejection, capacity limits, and server errors are distinct. Duration stops when
the handler produces a response, excluding client transfer and rendering.
No event/collector IDs, URLs, tokens, raw errors, or headers are metric labels.
Repeated metric delivery uses the same sample ID and queries use `FINAL`.
After three unacknowledged attempts, `dropped_requests` reports the number of
requests whose metric delivery is unconfirmed, not necessarily absent in storage.
Counters still in memory at process termination can be lost; this is operational
sampling, not an accounting ledger or pooled latency percentile.

Enrollment also reports bounded retry attempts, operator-required state, and the
previous cycle's pending count. `collector_diagnostics` retains the last reported
snapshot for 30 days. It cannot expose current state while the client is offline.
Collector deletion removes those snapshots and collector lifecycle observations.

`lifecycle` records actual API starts and the first authenticated registration of
each collector/package version, retained for 365 days. Grafana annotations and
the history table distinguish these observations. They do not claim a file's
installation time or reconstruct deployments before this instrumentation.

## Device groups and explicit purpose

App and CLI in one Codex home share `groundline/insights/device.json`, a private
random UUID created atomically under a common file lock. No hardware identifier
is read. Enrollment binds that group without changing collector identities,
tokens, consent, generations, or outboxes. A conflicting binding is rejected for
operator investigation; unsupported local state is never reset automatically.
Profiles copied between machines copy the group too. The dashboard therefore
counts verified local-profile groups and leaves missing groups unlinked; it is
not a hardware census. Do not infer or seed historical links from OS/IP/timing.

`groundline-insights worker purpose --value verification` declares the purpose
of future complete collection windows in the current Codex home. `production`
and `unclassified` are the other accepted values. The default is unclassified.
A window started before the latest declaration remains unclassified, including
backfill. Purpose does not infer task intent, change consent, or start collection.

The current v5 event envelope's optional `analysis` observation is advertised by
contract revision 7. When absent, existing accepted envelopes remain unchanged
and their tokens are projected as unattributed. Present observations have strict
keys, bounded labels, and exact token-conservation checks in Rust and ClickHouse.
The SQL bucket limit uses the same `MAX_MODEL_CONTEXTS` constant as Rust; the
current catalog permits 90 distinct non-unknown model/effort pairs.
Inconsistent native/UI counter baselines, missing links, and conflicting contexts
remain unattributed. No event's total is spread over context frequencies.

## Verification and remaining contracts

The optional TrueNAS controller sends compressed dashboard configs within its
64 KiB request limit and checks decoded size, checksum, JSON, mount targets, and
candidate/rollback request sizes. It refuses owner command/entrypoint overrides.
The public Compose template stays readable; this transport does not change
Grafana queries or prove deployment success.

Source checks execute both dashboards, annotations, and all six variable queries against
ClickHouse. Deployment verification executes them through the authenticated
Grafana datasource and independently reconciles fleet/roster/storage semantics.
Browser verification must also cover All, single/multiple selections, empty
results, and a roster-to-analysis link with its time range preserved.
Health responses alone do not prove this path.

Revision 8 adds generation-specific GPT-6 tier labels. Historical labels and
events are preserved; older, mixed, and unversioned cohorts are not optimization
targets. This change does not require rewriting existing ClickHouse rows.
Verify source, package, install, and a fresh receipt independently. A local
build or server-reported version does not prove that device's installed files.
Use a consistent temporary backup and an isolated restore for schema changes.
Backup schedules, external copies, and Garage integration are separate owner
choices; their absence does not turn a local restore into NAS-failure protection.

References: [ClickHouse column migration](https://clickhouse.com/docs/reference/statements/alter/column),
[query profiling](https://clickhouse.com/docs/concepts/features/performance/troubleshoot/sampling-query-profiler),
[Grafana ClickHouse variables](https://grafana.com/docs/plugins/grafana-clickhouse-datasource/latest/template-variables/).
