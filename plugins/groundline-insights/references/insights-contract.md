# GroundLine Insights Contract

The current contract rejects unsupported formats without converting or deleting
state. Historical records remain readable where explicitly stated below.

## Ownership

Core owns offline guidance and analysis. Insights independently owns five Codex
hooks, private collector state, HTTPS transport, the Rust/Axum API, ClickHouse,
Grafana, and self-hosting tools. It installs no Core dependency or skills,
changes no global Codex settings, and routes no models.

Supported runtime families are `codex_app` and `codex_cli`. Supported execution
modes are `desktop`, `local_headless`, and `remote_headless`. Supported platforms
are Apple Silicon macOS (ARM64) and Linux on ARM64 and x86-64.

Codex App/CLI are the only sources. HTTPS is the default transport; Tailnet is
optional. Docker Compose is the generic self-hosting path and TrueNAS an optional
owner-run path. Webhooks, third-party exporters, alternative databases, and
hosted GroundLine accounts are unsupported.

## Activation and local state

Installation is inert. An owner explicitly supplies a schema-7
`groundline-insights-owner-profile` input with:

- an HTTPS endpoint (or optional Tailnet/loopback HTTP endpoint) with no userinfo, query, fragment, or path;
- automatic activity checkpoints and initial history sync enabled;
- `all_activity`, a 900-second minimum interval, diagnostics disabled, and
  `native_hook_checkpoints`;
- an `enrollment_token` between 32 and 4096 bytes.

Missing policy means disabled; disabled checkpoints spawn no worker. Enable
validates existing policy/status and requires a valid sanitized profile and
enrollment credential before creating consent/policy.
Only compact policy schema 1, status schema 4, and consent schema 2 are accepted.
Unsupported kinds, versions, or shapes return `unsupported_local_state` without
conversion or lost watermarks, including on enable. Explicit disable still works.

Status returns readiness and bounded blockers. Ordinary HTTPS skips Tailnet
probing with `tailnet_required: false` and `not_required`; only Tailnet endpoints
gate readiness on it. An unobservable probe stays unknown, never disconnected.

Configuration writes a sanitized profile without the token and a separate
private enrollment-credential file. Identity, consent, policy, status,
collector token, token metadata, checkpoints, and outbox entries are also
bounded private files below `~/.codex/groundline/insights`. Status and error
receipts return booleans and reason codes, never paths, endpoints, IDs, or
secret values.

Consent permits upload only to the configured owner service while separate
policy is active; third-party upload remains disabled. Explicit enable creates
missing consent and quarantines unconsented pending events, but preserves a valid
receipt. Invalid or older consent is neither broadened, converted, nor archived.
Before replacing unsupported state, stop collection, preserve state/outbox, and
obtain approval for a fresh setup. Replaying old events into a newly consented
outbox or resetting watermarks needs separate data authorization.

Stop revokes policy immediately and waits for any active bounded request or
collection read before reporting success. Each subsequent phase/request checks
policy and consent under a process-shared lock. Already transmitted requests
cannot be recalled; unsent events and received ACKs are preserved. This requires
the updated executable on every collector process, including detached hooks.

## Enrollment and authentication

Every due cycle checks `/healthz` before enrollment/upload, including with a
cached collector token. `ingest_capabilities` must advertise Basic schema 5 and
revision 10 or newer, not an exact package version. Revision 10 adds dynamic
model identities and explicit response observations. Output signals may overlap
or refer to earlier-window calls;
they are bounded output proxies, not failed-call counts. Cache ratios, disjoint
windows, coherent usage totals, and provenance remain validated.

Re-enroll once per due cycle with the existing identity/token and use the returned
`current_generation` for new events. Never infer generation zero from credentials
or overwrite a prepared event after generation changes. Missing/incompatible
capabilities require API upgrade and operator retry; unready storage is retryable.
Health preflight sends no credentials and uses bounded readiness caching/rates.
`worker check-server` exposes that read-only check for installation: no profile
means `NOT_CONFIGURED`, no network, and no writes; configured failures exit nonzero.

## Collection transactions

The first collection covers the preceding seven days. Existing valid cursors
are preserved; `history_sync` retries the current window, not all historical
data. The newest update/recency timestamp prunes old candidates; stale sidebar
ordering cannot exclude an active turn. Continuing after the requested end does
not remove historical events. Records decide event time.

A private `collection-window.json` (at most 128 KiB) freezes the start/end and
counts attempts before reading. Only complete owned-scope reads prepare an
aggregate. Statistical sample insufficiency and a known inherited prefix remain
quality caveats, not failed owned-scope reads. Missing files, unknown ownership,
invalid metrics, and exhausted bounds stop the window without advancing it.
After three attempts, automatic reading stops until explicit operator retry.

The exact prepared event is persisted before enqueue, then the durable outbox
is written before committing the collection cursor. ACK status is persisted
before deleting delivered entries. Restarting reuses the prepared event, never a
new content hash from changed inputs. No partial aggregate is uploaded or later
added again. Old already-published windows are not replayed or corrected by this
update; retrospective repair needs a separate generation/activation decision.

## Enrollment requirements

The `/v1/enroll` route requires all of the following:

1. in optional `GROUNDLINE_REQUIRE_TAILNET=true` mode, a loopback/Tailnet peer or a private authenticated proxy with one Tailnet forwarded address; in general HTTPS mode, normal peers are admitted before token authentication and bounded rate limits;
2. owner enrollment enabled on the service;
3. `Authorization: Bearer <owner enrollment credential>`;
4. a strict schema-2 enrollment body with one collector UUID, one proposed
   collector token, supported platform/runtime enums, and a strict stable
   GroundLine version.

The enrollment credential is distinct from the proxy, admin, and collector
tokens. A collector UUID cannot be rebound to a different collector token.
Retired IDs remain denied. A current collector row marked `revoked` also
rejects enrollment without changing its token, generation, or stored row,
including with a valid owner enrollment credential. The normal API does not
create this revoked-only state; this guard covers externally modified storage.
After enrollment, event upload and collector-scoped operations require the
per-collector token. Administrative reports and deletion use the admin token.
The CLI requires that admin token through an explicit owner-private token file;
an authenticated collector token never authorizes the fleet-wide report.
Comparisons are constant-time and request bodies, responses, and rate windows
are bounded.

`POST /v1/enroll/check` uses the same selected network policy and owner enrollment checks without
reading or writing collector records, event history, or ClickHouse. It requires
no enrollment body and returns `enrollment_credential_verified: true` only after
authentication and rate checks succeed. Its `mutation_performed: false` refers
to persistent application data; transient rate budgets still apply.

The API distinguishes `enrollment_credential_rejected`,
`proxy_authentication_rejected`, and `tailnet_peer_rejected` with HTTP 401.
Disabled enrollment returns 403 `enrollment_disabled`; a collector identity
already bound to another token returns 409 `collector_already_enrolled`.
Retired IDs return 403 `collector_retired`; externally revoked rows return 403
`collector_revoked`. Retirement takes precedence when both states exist.
The worker preserves only these allowlisted, status-matched reasons. Other
401/403 replies become `remote_authentication_rejected`; arbitrary response text
is never emitted. These permanent failures still require an operator retry.

## Collection and transport

Hooks ignore input, persist bounded private lifecycle markers, and detach a
fail-open checkpoint process. The worker coalesces work and acknowledges only its
atomically claimed marker generation after durable handling; later captures stay
pending. Collection and delivery retries have independent cadence. The outbox
caps at 256 events/16 MiB and 16 uploads per cycle with capped exponential backoff;
permanent rejection requires operator action.

Worker status computes local scope and separate capture, worker, collection and
delivery observations from existing state. It does not infer native installation,
trust or dispatch from markers, and it cannot establish cloud or account-wide
coverage. Old records and unknown phases remain visible. These labels do not
change profile, policy, consent, status or receipt schemas, nor claim a server
receipt was rechecked. Core remains hook-free and offline.

Read-only Codex SQLite produces schema-5 `groundline-insights-basic-weekly` events:
aggregate usage, lifecycle, latency, verification, boundary counters, and bounded
platform/runtime dimensions.

Model observations use the dynamic shared `PUBLIC_MODEL_PATTERN` grammar, not
a fixed model catalog. Public labels have at most 96 lowercase ASCII bytes;
versions and snapshots retain their exact IDs. Valid model IDs outside that
public grammar become `private-<64 lowercase hex>`, computed with SHA-256 over
`groundline-model-id-v1\0` followed by the trimmed exact ID bytes. Private names,
paths, credentials, and conversation content never enter the public dimensions;
no original-name/key registry is uploaded or installed. Identity is explicit:
`public_model_id`, `opaque_model_id`, `historical_family`, `unknown`, or
`overflow`. Historical `astra`/`sol`/`luna`, `gpt-6`, and `other` remain family
observations. Previously normalized base labels such as `gpt-6-sol` also remain
historical when the component lacks response-count observations, because an
older snapshot may have been collapsed. They remain separate from new exact-ID
groups and are not retrospectively interpreted as exact model IDs.
Optimization eligibility remains a separate policy using the current native
catalog; observation labels neither route models nor prove account availability.

Each root/delegated component independently admits at most 128 model/effort
context pairs and 128 attributed-token buckets. The bound is independent of the
number of public models. Context collection reserves one overflow row and
preserves excess context counts. Usage overflow retains all six counters in the
unattributed residual. `overflow_response_count` is a subset of
`unattributed_response_count`; adding both would double-count.

Context counts describe native turn-context records. Observed responses count
unique owned native usage records, with model/effort attribution only when an
explicit turn link is coherent. Neither is a completed-task count. Response
counts absent from historical events remain unobserved, not zero. All six token
counters conserve the authoritative component totals, including cache-write
input. Missing links, conflicting contexts, or inconsistent native/UI baselines
remain unattributed; totals are never allocated by context frequency. Usage
provenance remains a shared bounded allowlist with distinct native response-only
and mixed-source labels.

Contract revision 10 requires API support before updated collectors are enabled;
rejected events stay operator-visible. Event envelope schema 5 and existing
stored event sources remain unchanged. The supported revision-9 trust migration
checks the exact prior definition, verifies source hashes and row counts, and
uses an atomic swap without rewriting payloads, IDs, periods, or counters.
Unknown definitions are rejected. Migration does not reconstruct older IDs or
response counts.

Activity samples count selected ongoing or completed roots with
`completed_root_coverage=false`; weekly samples require a final completed turn.
Streaming reads cap decoded input at 1 GiB per rollout and 8 GiB per invocation,
retaining at most 512 MiB of audit records. Native thread totals and UI totals
use independent baselines; valid native totals take precedence without adding
the streams. Selected-source resets inside the window and uncovered response
suffixes remain incomplete. Resets before the window do not poison its later
baseline. Read failures and
unread shared-history prefixes remain partial evidence, never a
claim that all provider history was collected.

The strict raw-content exclusions are listed in [Security](../SECURITY.md).
Clients reject redirects and ambient proxies, apply a fixed timeout, and contact
only the validated endpoint.

## Storage and reporting

The API is the only active ClickHouse schema migrator. It validates event
structure before insertion and uses event IDs for API-level idempotency.
Collector metadata and events are stored separately in `ReplacingMergeTree`
tables. Reports and Grafana read the `basic_active` view with `FINAL`, while
storage counters expose any physical duplicate excess caused by a race or
external writer. Logical deduplication is therefore part of the read contract;
physical duplicates remain an observable quality signal.

`basic_current` contains receipts for the current collector generations.
`basic_active` selects only current Codex aggregates with observed usage for each
present component, coherent counters, and readable, classified input.
`basic_quarantined` contains the complementary current receipts. A lifecycle
window can legitimately have activity before provider usage appears; receiving
that window must not stall collection or turn unknown usage into measured zero.
Its receipt remains idempotent, but it does not enter analytical metrics.
Reports expose `collection_health.quarantined_event_count` and the
`events_quarantined` quality reason rather than hiding exclusions behind PASS.
These views do not mutate or repair native Codex task databases.

The producer and API reject token splits larger than their totals, cache or
reasoning counts larger than their parents, and inconsistent usage provenance.
A single declarative projection produces the 88 payload-derived storage columns
and their ClickHouse consistency constraint. It covers metadata, all components,
nullable metrics, timestamps, and model arrays, in addition to root token bounds.
The API verifies the canonical SHA-256 digest and UUIDv5 before storing a row;
`insights validate-event --input <file> --json` performs the same offline check.
Cache ratios use the provider counters with four-decimal ties-to-even rounding;
a zero input denominator requires JSON null, never a synthetic zero ratio.
Total-only provider records remain valid; missing splits are never fabricated.
Admission requires start < end <= generated time and rejects generated times
more than five minutes ahead of receipt. Different IDs for overlapping periods
in the same collector and generation are rejected. The check uses canonical
microsecond boundaries; adjacent and out-of-order disjoint windows remain valid.
The duplicate-ID check runs first, preserving ordinary delivery retries.

Historical cleanup is an explicit owner operation. Before applying a stricter
TTL, audit existing payloads, back up exact affected rows and DDL, and verify the
backup checksum. Repair only deterministic derived values whose counters are
available; changing a payload requires a new canonical digest and UUID. Preserve
an old-to-new ID map and validate every replacement before writing. Remove only
captured irrecoverable or superseded IDs, then prove untouched rows and all
retained source counters are unchanged. Startup never silently rewrites history.

Trusted records retain the owner-configured TTL (365 days by default).
Quarantined receipts expire seven days after receipt. A single conditional TTL
expression implements both deadlines; the `basic_retention` view exposes the
same expression for reports and Grafana. Quarantine is bounded diagnostic
storage, not a permanent archive of unusable measurements.
Default per-collector retention is capped at 4,096 events and 256 MiB of logical
payload. Dataset row/byte watermarks stop ingest at 90% of configured ceilings
to reserve administrative capacity. Operators may change documented bounded values.
Duplicate retries do not consume quota, and quota check plus insert is serialized
within the supported single API instance.
TTL cleanup is eventual because ClickHouse removes expired rows during
background merges. The release policy stores the active retention window, and
the report plus Grafana expose rows that have passed that deadline without
triggering `OPTIMIZE` or manual deletion.

Reports are schema-3 `groundline-insights-weekly-report` documents with fixed
7, 30, or 90-day windows, sufficiency and coverage signals, bounded
distributions, update advisories, and fleet/storage counters. Basic event bodies
are capped at 256 KiB and reports at 1 MiB. Report `query_set_version` 4 describes
the new observation semantics without changing event schema 5 or API schema 3.

`cohorts.model_usage_patterns` groups model identity, effort, root/delegated, and
explicit purpose. It exposes context counts, nullable observed response counts,
all six token fields, and per-cohort coverage. Period aggregation caps retained
labeled rows at 128 per component/purpose cohort and retains the excess in an
additional overflow row and coverage counters: at most 774 rows across the six
cohorts. Historical response-unobserved windows
remain visible alongside measured windows. Unknown/overflow context counts,
unattributed responses, and unattributed tokens accompany comparisons.
Response attribution uses observed responses only; tokens are not a price or
subscription-quota measure, and these statistics do not establish causal
improvement. Model-specific latency, tool use, and quality require actual turn
attribution; whole-event totals are never duplicated across model rows.

Grafana panels use the provisioned ClickHouse datasource and fixed query
templates. Dashboard availability, datasource health, query execution, report
generation, and collector upload are separate evidence lanes.
Model-pattern panels read API-owned `model_usage_patterns` and
`model_usage_coverage` views with the existing OS/runtime/version/installation/
device/purpose filters. Zero-token unknown residuals do not fabricate observed
windows; only matching contexts, observed responses, or positive token counters
produce model-pattern rows.

## Deployment boundary

The repository contains a generic compose template with placeholders only. The
renderer creates a separate private secrets file containing independent
ClickHouse, Grafana, admin, enrollment, and proxy credentials. Production
endpoints, credentials, dataset paths, TrueNAS inventory, and deployment
receipts remain outside Git.

Infrastructure versions are supplied by a strict compatibility profile rather
than hard-coded into the template. The checked-in profile is the release-tested
default. An operator or manual CI run may supply a complete newer four-component
profile; partial sets fail closed, and any unpinned discovery run requires an
explicit qualification-only override. Passing the live stack verifier does not
mutate the default profile or deploy the candidate.

The deployment controller validates every required input before mutation,
accepts only digest-pinned Insights API images, compares the current app
configuration with the preflight fingerprint, applies one bounded update, and
verifies API, ClickHouse, and Grafana evidence. Rollback outcome is reported
separately from code validation. `preflight` and `apply` require an explicit
owner-rendered private `--compose-template`; passing the public placeholder
template is rejected rather than guessed or partially rendered. The controller
opens that rendered file without following symbolic links, enforces a bounded
size, and requires it to be private to the current user.

Both `preflight` and `apply` require the owner-local
`GROUNDLINE_INSIGHTS_ENROLLMENT_TOKEN` and
`GROUNDLINE_INSIGHTS_GRAFANA_ADMIN_PASSWORD` environment variables. They are
never repository or release-workflow secrets. The Grafana credential is used
only for authenticated datasource and dashboard semantics checks. The
controller preserves an existing valid `GROUNDLINE_ENROLLMENT_TOKEN` in the
TrueNAS app configuration and uses the local input only when migrating an
installation that does not have one. Malformed existing values fail closed,
and receipts never contain either value.

## Required verification

Release qualification covers:

1. Core zero-hook and Insights five-hook package invariants;
2. missing, wrong, and correct enrollment credentials;
3. profile/secret separation and private permissions;
4. strict event, report, platform, runtime, version, and size contracts;
5. symlink, redirect, proxy, rate-limit, integer-boundary, and malformed input
   rejection;
6. API-owned ClickHouse migrations, enrollment, accepted and duplicate upload,
   report generation, every Grafana query, and authenticated collector deletion;
7. Apple Silicon macOS (ARM64) and Linux packages on ARM64 and x86-64;
8. source privacy scanning, pinned CI actions, bounded timeouts, and exact stable
   artifact promotion.

Source tests do not prove an installed plugin, hook dispatch, upload,
ClickHouse/Grafana visibility, image publication, production deployment, or
stable promotion. Unobserved lanes remain `UNVERIFIED`.
