# GroundLine Insights Operations Troubleshooting

Use bounded evidence before changing state. Core, inference proxies, and model
catalogs are not dependencies; do not install them or alter model/provider
settings to repair collection. Native startup and owner-service faults are separate.

## Read-only sequence

```bash
groundline-insights platform --json
groundline-insights doctor --json
groundline-insights provider-smoke --require-installed --json
groundline-insights tailnet-status --json
groundline-insights worker status
codex plugin list --json
```

Keep evidence separate: provider smoke checks manifest/target/hooks/size/SHA-256;
the owning App's hook list checks effective configuration/trust; a lifecycle
receipt proves dispatch; outbox proves local durability; accepted upload proves
API acknowledgement, not Grafana freshness. Tailnet status is local only, and
`tailnet_connected: null` remains unverified rather than disconnected.

## Common actions

- `enrollment_credential_rejected`: the owner API rejected the enrollment
  credential. Keep the collector identity, token, consent, and outbox. Review the
  owner-issued enrollment credential through the normal private configuration
  flow, then perform one explicit retry. A collector token or a locally valid
  credential file does not establish enrollment authorization.
- `proxy_authentication_rejected` / `tailnet_peer_rejected`: inspect the trusted
  ingress boundary and effective Tailnet peer. Do not rotate enrollment
  credentials based on these errors.
- `enrollment_disabled` / `collector_already_enrolled`: review the server's owner
  enrollment policy or collector identity/token ownership respectively. Do not
  reset an identity or discard pending data to bypass a conflict.
- `remote_authentication_rejected`: an HTTP 401/403 response did not provide a
  recognized, matching reason code. The runtime intentionally does not echo
  arbitrary server details. Use server-side evidence before assigning blame to
  an enrollment credential, proxy, or collector token.
- `native_activity_unavailable` / `codex_state_store_unavailable`: check the
  intended native `CODEX_HOME` and source permissions. The highest numeric
  `state_<n>.sqlite` must be a nonempty, owned regular file; a newer unusable
  store does not fall back to an older one. Use native Codex to create activity
  when the home is new. Do not manufacture/edit a database, copy proxy state,
  or reset Insights. Presence does not prove required columns or complete rollouts.

- `collection_incomplete`: the frozen window has not been published and its
  cursor has not advanced. Inspect bounded audit counts, restore missing or
  malformed inputs, and retry once with `worker run-once`. Never edit Codex's
  database or delete the collection receipt to skip evidence.
  Explicit imported non-Codex tasks are reported locally as
  `non_codex_excluded_rollout_count` and do not block the Codex window. Unknown
  originators remain blockers. An older collector may count imported tasks as
  unclassified; update the collector before retrying the preserved window.
- `collection_operator_action_required`: three read attempts failed (including
  interrupted attempts). Automatic hooks stop reading this window. Fix the
  cause, then explicitly run `worker run-once`; this does not reset history.
  Known native shared prefixes are an intentional scope exclusion, not a retry
  failure. Unknown ownership, malformed metrics, and read limits are blockers.
- `api_upgrade_required`: update the owner API first, then run the candidate's
  read-only `worker check-server` against the [current capabilities](insights-contract.md#enrollment-and-authentication).
  Retry `worker run-once` explicitly. Cached credentials bypass no check; pending
  events stay local without downgrade. Enrollment must return the active
  generation; reuse identity/token and never assume generation zero.
- `invalid_owner_profile`: install the reviewed owner-local schema-7 input with
  `groundline-insights worker configure --input <profile.json>`; it must contain
  an enrollment credential, and neither the real endpoint nor credential belongs
  in the plugin. Start from `owner-profile.example.json`; its short placeholder
  token is intentionally invalid until replaced in an owner-private copy.
- `runtime_binary_missing` / `invalid_artifact_checksum`: stop using the bad
  cache and follow [native upgrade](native-upgrade.md). Hooks never build/download;
  refresh of a pinned source does not select a new release.
- Tailnet disconnected: connect Tailscale, then run an explicitly authorized
  `groundline-insights worker run-once`.
- delayed/overdue outbox: preserve the outbox, restore Tailnet/API availability,
  and retry once. Automatic hooks honor `delivery_next_attempt_utc`; permanent
  rejection sets `delivery_operator_required`. Do not delete evidence.
- `unsupported_local_state`: compare the complete [state contract](insights-contract.md#activation-and-local-state),
  not just schema numbers: an old schema-1 policy may lack `updated_at_utc` and
  contain forbidden fields. Inventory each runtime partition/cursor/pending data.
  `worker disable` still revokes collection. Stop, preserve state/outbox, and obtain
  approval before fresh setup; never delete or replay data to bypass this error.
- `reconsent_required`: review the current owner-service destination. If consent
  is missing, explicit `worker enable` creates it and quarantines unconsented
  pending events. An invalid existing receipt is not repaired by enable; preserve
  it for review before any separately authorized reset. Never move quarantined
  events into the active outbox without a separate data-authorization decision.
- `outbox_capacity_exceeded`: collection stops producing new events while the
  bounded worker drains existing 16-event batches. Restore the endpoint and
  inspect status; never run a recursive upload or delete the outbox to hide it.
- changed hook hash: review and trust it in Codex. GroundLine never approves
  itself. `plugin list` does not prove hook trust or dispatch.

Read `worker status` from the operational fields. `status: PASS` with
`collection_state: disabled` is an intentional inert installation. `status: WARN`
identifies a collection blocker through `blocking_reason_codes`; it is not a
package-integrity failure. Enabling without a valid owner profile and enrollment
credential is rejected before identity, consent, or policy state is created.
Seven days without a successful collection becomes `collection_state: stale`;
a success timestamp more than five minutes in the future becomes `clock_skew`.

`collection_scope` describes supported local App/CLI state only. Account-wide
activity and other machines are unobserved; cloud orchestration is unsupported.
The declared `remote_headless` dimension does not establish cloud hook support.
Orchestration location stays unknown without evidence from the native host.

`execution_evidence` separates native installation/enablement/trust/dispatch,
bounded capture markers, worker hook handling, collection and acknowledgement.
The status command cannot authenticate native dispatch: manually invoked hook
commands can create the same markers. Inspect Codex's effective hook support and
trusted hash, then a fresh native event and its durable handling. A pending
marker is not a processed capture; a historical `last_hook_at_utc` is not proof
that all four events ran. Capture inspection is read-only and bounded to the
existing eight slots; unreadable markers report unavailable without clearing them.

Observations older than the current activation are `before_current_activation`,
missing timestamps are `unobserved`, and old/future timestamps remain stale or
clock-skew evidence. Repeated enable on an active policy preserves its activation
time; an actual revoke/re-enable requires a fresh collection before active status.
`ready_to_collect` means prerequisites are present, not that collection or
delivery succeeded. `acknowledgement_recorded` retains the latest local batch
receipt; pending events remain pending, and status never rechecks the server or
proves ClickHouse/Grafana freshness. Keep consent, identity, outbox, cursors and
ACKs intact while diagnosing each phase.

`enrollment_credential_valid` checks only the local file and its token format;
it does not make a network request or prove server acceptance. A reviewed API
client can use `POST /v1/enroll/check` with the owner enrollment credential to
verify the Tailnet and enrollment boundaries without registering a collector or
accessing ClickHouse. This route still enforces authentication and rate limits;
it does not grant authority to read a credential or reset collection state.

## Unrecoverable historical gaps

Some copied histories have no trustworthy ownership boundary. Do not infer one
from rewritten timestamps or weaken collection checks to make the upload pass.
If the owner chooses to resume current collection, preserve the original state,
cursor, outbox, and server records; record the exact unprocessed interval and
new boundary in a private owner ledger. Mark that interval unprocessed, never
successfully uploaded. Changing the boundary needs explicit scope approval;
an existing approval for that exact action remains sufficient.

This is an owner-controlled recovery, not an automatic format migration or a
public cursor-reset command. `worker backfill-history --confirm-rebuild` retries
the current collection path and does not reconstruct all earlier history.

Never expose token files, edit Codex SQLite, delete task data, run
`VACUUM`, or infer completion from an idle task.
