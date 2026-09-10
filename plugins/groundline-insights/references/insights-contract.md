# GroundLine Insights Contract

This document describes the current v0.20 contract. Historical schemas and
provider compatibility are not part of the active interface.

## Ownership

GroundLine Core owns offline guidance and local analysis. GroundLine Insights
owns the optional networked path: four Codex hooks, collector state, HTTPS
transport, the API, ClickHouse, Grafana, and generic self-hosting tools. It does
not require or install Core, install skills, change global Codex configuration,
or route models. Core-only, Insights-only, and combined installations are all
valid profiles.

Supported runtime families are `codex_app` and `codex_cli`. Supported execution
modes are `desktop`, `local_headless`, and `remote_headless`. Supported platforms
are macOS, Linux, and Windows on ARM64 and x86-64.

The current integration contract is deliberately narrow: Codex App/CLI are the
only collector sources, HTTPS is the default remote transport, with optional Tailnet access, the Rust/Axum API
is the ingestion service, ClickHouse is the storage and report backend, and
Grafana is the first-party dashboard. Docker Compose is the generic self-hosting
path and TrueNAS is one supported owner-run deployment path. Generic webhooks,
third-party observability exporters, alternative
databases, and hosted GroundLine accounts are outside the current contract.

## Activation and local state

Installation is inert. An owner explicitly supplies a schema-7
`groundline-insights-owner-profile` input with:

- an HTTPS endpoint (or optional Tailnet/loopback HTTP endpoint) with no userinfo, query, fragment, or path;
- automatic activity checkpoints and initial history sync enabled;
- `all_activity`, a 900-second minimum interval, diagnostics disabled, and
  `native_hook_checkpoints`;
- an `enrollment_token` between 32 and 4096 bytes.

A missing policy means disabled. The complete policy is validated as a strict
private file, and enablement is rejected until both the sanitized profile and
enrollment credential are valid. Disabled lifecycle checkpoints exit without
spawning a detached worker. Worker status reports readiness and bounded blockers;
it never converts an unobservable Tailnet probe into a false disconnected state.
For ordinary HTTPS endpoints, tailnet_required is false and the probe is skipped;
not_required is a bounded Tailnet status. Only explicit Tailnet endpoints gate readiness on that probe.
Only the current compact policy schema 1, status schema 4, and consent schema 2
are accepted. The former private policy shape and status schema 3 are not
imported. Unsupported versions, kinds, or shapes fail closed with
`unsupported_local_state`; reading or enabling never converts them or discards
their watermarks. Explicit disable remains available to revoke collection.

Configuration writes a sanitized profile without the token and a separate
private enrollment-credential file. Identity, consent, policy, status,
collector token, token metadata, checkpoints, and outbox entries are also
bounded private files below `~/.codex/groundline/insights`. Status and error
receipts return booleans and reason codes, never paths, endpoints, IDs, or
secret values.

Consent schema 2 states the network boundary directly: upload to the configured
owner service is enabled only while the separate owner policy is active, and
third-party upload remains disabled. Consent schema 1 is unsupported and is not
converted or archived automatically, including by `worker enable`. With no
existing consent, explicit enable creates a receipt and quarantines unconsented
pending events. Re-enabling an existing valid receipt preserves it. Invalid
current consent requires operator review and is never silently broadened.
Before replacing unsupported state, stop collection, preserve the original
state/outbox, and obtain explicit approval for a fresh setup. Do not restore old
pending events into a newly consented outbox or reset collection watermarks
without a separate data-authorization decision.

Stop revokes policy immediately and waits for any active bounded request or
collection read before reporting success. Each subsequent phase/request checks
policy and consent under a process-shared lock. Already transmitted requests
cannot be recalled; unsent events and received ACKs are preserved. This requires
the updated executable on every collector process, including detached hooks.

## Enrollment and authentication

Every due worker cycle checks `/healthz` before enrollment or upload, even when
a collector token is already cached. The API advertises Basic envelope schema
versions and a semantic allowlist revision in `ingest_capabilities`. Collectors
require schema 5 and revision 3 or newer, not an exact package version. Revision 3
includes the authoritative `current_generation` in the enrollment response.
Re-enroll once per due cycle with the existing identity and token, and use that
generation when staging new events. Never infer zero from a cached credential
or overwrite a prepared event after a generation changes. Missing
or incompatible capabilities require an API upgrade and explicit operator retry;
unready storage remains a retryable service failure. Credentials are not sent
by this preflight, and the existing bounded readiness cache and rate limit apply.

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
The worker preserves only these allowlisted, status-matched reasons. Other
401/403 replies become `remote_authentication_rejected`; arbitrary response text
is never emitted. These permanent failures still require an operator retry.

## Collection and transport

Hooks ignore hook input, persist one bounded private marker per lifecycle event,
and detach one fail-open checkpoint process. The worker atomically claims the
current marker generation, coalesces concurrent work, and acknowledges only the
claimed generation after it has durably handled the cycle; a later capture stays
pending. Accepted delivery advances durable collection state before its outbox
file is removed.
Collection cadence and delivery retry cadence are independent. The outbox is
limited to 256 events and 16 MiB, uploads at most 16 events per cycle, and uses
capped exponential backoff. Permanent remote rejection pauses automatic retry
for operator action.
It opens Codex SQLite read-only and produces schema-5
`groundline-insights-basic-weekly` events. The event contract contains aggregate
usage, lifecycle, latency, verification, and boundary counters plus
low-cardinality platform/runtime fields.

Model/effort dimensions are shared Rust allowlists used by normalization,
ingestion, weekly reports, and comparisons. Astra has its own family label;
unknown model IDs remain `other`. These labels do not route models or prove
account availability. Usage provenance is also a shared bounded allowlist;
native response-only usage and mixed-source aggregates have distinct labels.
An API must support newly introduced labels before updated collectors are
enabled; rejected events stay operator-visible.

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

The contract rejects raw prompts, responses, transcripts, commands, patches,
paths, repository names, task IDs, rollout IDs, account identifiers, hostnames,
and IP addresses. The client disables ambient HTTP proxy discovery, rejects
redirects, applies a fixed timeout, and contacts only the validated endpoint.

## Storage and reporting

The API is the only active ClickHouse schema migrator. It validates event
structure before insertion and uses event IDs for API-level idempotency.
Collector metadata and events are stored separately in `ReplacingMergeTree`
tables. Reports and Grafana read the `basic_active` view with `FINAL`, while
storage counters expose any physical duplicate excess caused by a race or
external writer. Logical deduplication is therefore part of the read contract;
physical duplicates remain an observable quality signal.

The service applies an owner-configured retention TTL, retained per-collector
event and logical-payload quotas, and dataset row/byte watermarks. Ingest stops at
90% of the configured dataset ceilings to reserve capacity for administration.
Duplicate retries do not consume quota, and quota check plus insert is serialized
within the supported single API instance.
TTL cleanup is eventual because ClickHouse removes expired rows during
background merges. The release policy stores the active retention window, and
the report plus Grafana expose rows that have passed that deadline without
triggering `OPTIMIZE` or manual deletion.

Reports are schema-3 `groundline-insights-weekly-report` documents with fixed
7, 30, or 90-day windows, sufficiency and coverage signals, bounded
distributions, update advisories, and fleet/storage counters.

Grafana panels use the provisioned ClickHouse datasource and fixed query
templates. Dashboard availability, datasource health, query execution, report
generation, and collector upload are separate evidence lanes.

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

1. Core zero-hook and Insights four-hook package invariants;
2. missing, wrong, and correct enrollment credentials;
3. profile/secret separation and private permissions;
4. strict event, report, platform, runtime, version, and size contracts;
5. symlink, redirect, proxy, rate-limit, integer-boundary, and malformed input
   rejection;
6. API-owned ClickHouse migrations, enrollment, accepted and duplicate upload,
   report generation, every Grafana query, and authenticated collector deletion;
7. macOS, Linux, and Windows packages on ARM64 and x86-64;
8. source privacy scanning, pinned CI actions, bounded timeouts, and exact stable
   artifact promotion.

Source tests do not prove an installed plugin, hook dispatch, upload,
ClickHouse/Grafana visibility, image publication, production deployment, or
stable promotion. Unobserved lanes remain `UNVERIFIED`.
