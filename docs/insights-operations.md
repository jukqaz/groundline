# Insights metrics and operations

Insights uses ClickHouse for deduplicated aggregate storage and Grafana for
inspection. Core performs no network collection. App and CLI are runtime
dimensions, not physical device identities. No hostname, account, IP address,
prompt, or transcript is needed for these dashboards.

## Dashboard contract

The overview shows the entire enrolled fleet, including installations with no
accepted event. Follow an installation's analysis link to preserve the selected
time range and inspect its usage. The analysis dashboard applies OS, runtime,
reported version, and installation filters to every panel. An opaque SHA-256
installation key is used for links; the four-character display suffix is not a
unique identifier. A random installation UUID is not a person or device count.

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

## Trust-column migration

`trusted_event_v5` is a materialized UInt8 result of the current envelope and
payload/projection validation predicate. Inserts compute it in ClickHouse.
Explicit values and unknown JSON fields are rejected. Startup checks its type,
materialized kind, and predicate fingerprint. Changing the validation predicate
requires a new explicit migration; reusing old trust decisions is rejected.

Raw envelopes, counters, event IDs, generations, and timestamps stay intact.
`FINAL`, active-generation selection, retirement filtering, quarantine, and TTL
remain in the query path. Existing parts calculate the new column lazily until
materialized. Startup does not launch an unbounded materialization job.

Before materializing old parts, take a consistent recoverable backup and rehearse
on a separate database. Compare original-column hashes, trusted/quarantined
counts, and all dashboard results. Then scope `MATERIALIZE COLUMN
trusted_event_v5` to inventoried partitions, await mutation completion, and
repeat those comparisons. Check disk headroom and concurrent receipts; never
mount production storage into a rehearsal. An image rollback alone does not
undo schema or TTL changes.

## Diagnostic logging

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
Inconsistent native/UI counter baselines, missing links, and conflicting contexts
remain unattributed. No event's total is spread over context frequencies.

## Verification and remaining contracts

TrueNAS deployment transports the two dashboards as gzip/base64 inline configs
to fit its 64 KiB authenticated WebSocket request limit. The public Compose
template remains readable. The deploy operator uses `flate2` with its Rust
backend because the standard library does not provide gzip, and validates
decoded size, checksum, trailing bytes, JSON, and the mounted target before
query verification. Dashboard JSON and Compose dollar escaping are preserved.
Grafana decodes these files into its container-local temporary directory before
its original `/run.sh`; startup stops if decoding fails. An owner-defined
command or entrypoint is rejected rather than overwritten. Preflight checks
both the candidate and rollback request sizes without increasing server limits.

Source checks execute both dashboards, annotations, and all six variable queries against
ClickHouse. Deployment verification executes them through the authenticated
Grafana datasource and independently reconciles fleet/roster/storage semantics.
Browser verification must also cover All, single/multiple selections, empty
results, and a roster-to-analysis link with its time range preserved.
Health responses alone do not prove this path.

Deploy the API advertising contract revision 7 before upgrading collectors.
Verify source, package, install, and a fresh receipt independently. A Windows
build or server-reported version does not prove that device's installed files.
Use a consistent temporary backup and an isolated restore for schema changes.
Backup schedules, external copies, and Garage integration are separate owner
choices; their absence does not turn a local restore into NAS-failure protection.

References: [ClickHouse column migration](https://clickhouse.com/docs/reference/statements/alter/column),
[query profiling](https://clickhouse.com/docs/concepts/features/performance/troubleshoot/sampling-query-profiler),
[Grafana ClickHouse variables](https://grafana.com/docs/plugins/grafana-clickhouse-datasource/latest/template-variables/).
