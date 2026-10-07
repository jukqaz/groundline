# Privacy

GroundLine Core processes explicit local inputs and performs no network request.
Its audit commands open Codex state read-only and return aggregate counts without
prompt text, response text, task titles, repository names, filesystem paths,
configuration values, credentials, or database rows.

Delivery recording accepts explicit local manifests and evidence artifacts and
creates new owner-private receipts. Requested and observed effective selections,
verification, and resource gaps remain distinct. Receipt hashes and local outcome
records are not included in Insights uploads. See the
[delivery contract](../plugins/groundline/references/delivery-evidence.md).

`personal status` reads existing private trial state; `personal rollback` may
restore unchanged GroundLine-generated guidance while preserving user edits.
These recovery commands do not create or evaluate new trials, call a model, or
change Codex settings. See [personal recovery](../plugins/groundline/references/personal-recovery.md).

Requested environment and learning commands keep separate owner-private state.
Capture snapshots and outcome links carry digests and explicit scoped observations,
not native conversation bodies. Official text snapshots are supplied by native
tools; the Core comparison cache stores mappings, digests and freshness metadata.
Core does not fetch the text or authenticate its source. Portable bundles contain
selected managed guidance text and common baseline lineage, excluding device
bindings, paths and host exception fields. Managed text may itself contain private
information: review it before transport and keep bundles outside public source.
Native credentials, memory, sessions, databases and provider caches are not synced.
See [the learning loop](../plugins/groundline/references/learning-loop.md).

GroundLine Insights is separately installed, does not require Core, and remains
inactive until the owner configures and enables it. It writes only bounded
owner-private state under the Codex home and sends strict aggregate events to an
owner-selected HTTPS endpoint (or optional Tailnet endpoint). Its contracts exclude raw prompts, responses,
transcripts, commands, patches, paths, hostnames, repository names, task IDs,
rollout IDs, account identifiers, and IP addresses.

App and CLI registrations can share a random device-group UUID stored privately
in the same Codex home. This links the owner's installations without reading
hardware identifiers; copying that home also copies the group. Model/effort
token buckets contain only bounded labels and counters. Attribution uses native
owned response/turn links locally; those links are never uploaded. Unattributed
usage remains explicit. An optional purpose declaration contains only
`production`, `verification`, or `unclassified` and never relabels older windows.

The API retains bounded route/outcome counters for 30 days, collector retry
snapshots for 30 days, and server-start/version-registration observations for
365 days. Request bodies, identity labels, raw errors, URLs, and headers are
excluded from operational counters. Collector deletion removes its device
association, diagnostics, and lifecycle entries along with aggregate data.

The owner profile is stored without credentials. The enrollment credential and
per-collector token are separate private files and are never returned by status,
doctor, or receipt commands. The self-hosted service stores collector UUIDs and
aggregate events needed for fleet status and reports; deleting a collector is an
explicit authenticated operation. Deletion retains only a SHA-256 hash of the
random collector UUID and its retirement timestamp for the deployment lifetime,
so that the retired installation cannot automatically enroll again. Credentials
and aggregate rows are removed. See the [retirement policy](insights-operations.md#retired-installations).

The public repository and release packages contain no production endpoint,
credential, dataset path, infrastructure inventory, or deployment receipt.
Source qualification rejects common private-key, token, credential, environment,
and local database artifacts before packaging. This is defense in depth and does
not replace GitHub secret scanning or review.
Operators are responsible for their self-hosted ClickHouse and Grafana retention.
Installation profile, endpoint, enablement, backfill, and report-window choices
do not relax the fixed aggregate-only wire contract. See
[integrations and installation profiles](integrations.md).
