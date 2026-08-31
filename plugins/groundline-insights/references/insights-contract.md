# GroundLine Insights Contract

This document describes the current v0.20 contract. Historical schemas and
provider compatibility are not part of the active interface.

## Ownership

GroundLine Core owns offline guidance and local analysis. GroundLine Insights
owns the optional networked path: four Codex hooks, collector state, Tailnet
transport, the API, ClickHouse, Grafana, and generic self-hosting tools. It does
not require or install Core, install skills, change global Codex configuration,
or route models. Core-only, Insights-only, and combined installations are all
valid profiles.

Supported runtime families are `codex_app` and `codex_cli`. Supported execution
modes are `desktop`, `local_headless`, and `remote_headless`. Supported platforms
are macOS, Linux, and Windows on ARM64 and x86-64.

The current integration contract is deliberately narrow: Codex App/CLI are the
only collector sources, Tailnet is the only remote transport, the Rust/Axum API
is the ingestion service, ClickHouse is the storage and report backend, and
Grafana is the first-party dashboard. Docker Compose is the generic self-hosting
path and TrueNAS is one supported owner-run deployment path. Arbitrary Internet
endpoints, generic webhooks, third-party observability exporters, alternative
databases, and hosted GroundLine accounts are outside the current contract.

## Activation and local state

Installation is inert. An owner explicitly supplies a schema-7
`groundline-insights-owner-profile` input with:

- a Tailnet HTTP or HTTPS endpoint with no userinfo, query, fragment, or path;
- automatic activity checkpoints and initial history sync enabled;
- `all_activity`, a 900-second minimum interval, diagnostics disabled, and
  `native_hook_checkpoints`;
- an `enrollment_token` between 32 and 4096 bytes.

A missing policy means disabled. The complete policy is validated as a strict
private file, and enablement is rejected until both the sanitized profile and
enrollment credential are valid. Disabled lifecycle checkpoints exit without
spawning a detached worker. Worker status reports readiness and bounded blockers;
it never converts an unobservable Tailnet probe into a false disconnected state.
The exact previously shipped private policy-v1 and status-v3 records have one
bounded import path that preserves explicit enablement and watermarks without
making historical formats part of the public interface. Unknown state fails
closed, and the next explicit mutation writes the current compact format.

Configuration writes a sanitized profile without the token and a separate
private enrollment-credential file. Identity, consent, policy, status,
collector token, token metadata, checkpoints, and outbox entries are also
bounded private files below `~/.codex/groundline/insights`. Status and error
receipts return booleans and reason codes, never paths, endpoints, IDs, or
secret values.

Consent schema 2 states the network boundary directly: upload to the configured
owner service is enabled only while the separate owner policy is active, and
third-party upload remains disabled. An exact schema-1 consent created by the
previous public release is migrated without changing its receipt, acceptance
time, collection scope, or diagnostic setting; unknown consent shapes fail
closed.

## Enrollment and authentication

The `/v1/enroll` route requires all of the following:

1. a loopback/Tailnet peer, or a private trusted proxy presenting the configured
   proxy credential and one unambiguous Tailnet forwarded address;
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
