# Privacy

GroundLine Core processes explicit local inputs and performs no network request.
Its audit commands open Codex state read-only and return aggregate counts without
prompt text, response text, task titles, repository names, filesystem paths,
configuration values, credentials, or database rows.

GroundLine Insights is separately installed, does not require Core, and remains
inactive until the owner configures and enables it. It writes only bounded
owner-private state under the Codex home and sends strict aggregate events to an
owner-selected Tailnet endpoint. Its contracts exclude raw prompts, responses,
transcripts, commands, patches, paths, hostnames, repository names, task IDs,
rollout IDs, account identifiers, and IP addresses.

The owner profile is stored without credentials. The enrollment credential and
per-collector token are separate private files and are never returned by status,
doctor, or receipt commands. The self-hosted service stores collector UUIDs and
aggregate events needed for fleet status and reports; deleting a collector is an
explicit authenticated operation.

The public repository and release packages contain no production endpoint,
credential, dataset path, infrastructure inventory, or deployment receipt.
Source qualification rejects common private-key, token, credential, environment,
and local database artifacts before packaging. This is defense in depth and does
not replace GitHub secret scanning or review.
Operators are responsible for their self-hosted ClickHouse and Grafana retention.
Installation profile, endpoint, enablement, backfill, and report-window choices
do not relax the fixed aggregate-only wire contract. See
[integrations and installation profiles](integrations.md).
