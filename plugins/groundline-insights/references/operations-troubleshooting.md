# GroundLine Insights Operations Troubleshooting

Diagnose from bounded evidence before reinstalling or changing state.
Core is not an Insights dependency; an absent Core installation is not an
Insights fault. Diagnose only the selected installation profile.

Inference proxies and generated model catalogs are not dependencies either.
Do not reinstall one or edit model/provider settings to repair collection.
Keep native Codex startup problems separate from the Insights owner-service path.

## Read-only sequence

```bash
groundline-insights platform --json
groundline-insights doctor --json
groundline-insights provider-smoke --require-installed --json
groundline-insights tailnet-status --json
groundline-insights worker status
codex plugin list --json
```

Interpret the lanes independently:

- provider smoke proves the installed manifest, native target, hook manifest,
  artifact size, and SHA-256;
- hook list proves effective configuration/trust only when obtained from the
  owning App runtime;
- a lifecycle receipt proves actual dispatch;
- Tailnet status proves only local Tailscale state;
- `tailnet_connected: null` with a bounded probe reason means the local CLI result
  is unverified; it must not be relabeled as disconnected;
- outbox status proves local durability, not server acceptance;
- an accepted upload proves API acknowledgement, not Grafana freshness.

## Common actions

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
- `collection_operator_action_required`: three read attempts failed (including
  interrupted attempts). Automatic hooks stop reading this window. Fix the
  cause, then explicitly run `worker run-once`; this does not reset history.
  Known native shared prefixes are an intentional scope exclusion, not a retry
  failure. Unknown ownership, malformed metrics, and read limits are blockers.
- `api_upgrade_required`: update the owner API first and confirm `/healthz`
  advertises Basic schema 5 and ingest contract revision 3 or newer. Then run
  `worker run-once` explicitly. Cached credentials do not bypass this check;
  pending aggregates remain local and are not silently downgraded.
- `invalid_owner_profile`: install the reviewed owner-local schema-7 input with
  `groundline-insights worker configure --input <profile.json>`; it must contain
  an enrollment credential, and neither the real endpoint nor credential belongs
  in the plugin. Start from `owner-profile.example.json`; its short placeholder
  token is intentionally invalid until replaced in an owner-private copy.
- `runtime_binary_missing`: Refresh the moving public marketplace; do not
  build or download from the hook.
- `invalid_artifact_checksum`: stop using that cache and reinstall from a
  verified release.
- Tailnet disconnected: connect Tailscale, then run an explicitly authorized
  `groundline-insights worker run-once`.
- delayed/overdue outbox: preserve the outbox, restore Tailnet/API availability,
  and retry once. Automatic hooks honor `delivery_next_attempt_utc`; permanent
  rejection sets `delivery_operator_required`. Do not delete evidence.
- `unsupported_local_state`: a consent, policy, or status file does not match
  the current contract. No import, downgrade, or automatic conversion exists.
  Inspect the complete field contract: a former policy can still declare
  schema 1 while lacking `updated_at_utc` and carrying unsupported fields.
  Matching schema numbers alone do not establish compatibility. Inventory each
  runtime partition separately, including its collection cursor and pending data.
  Stop collection, preserve the state and outbox, and obtain explicit approval
  before a fresh setup. Do not delete files or replay old pending events to
  bypass the error. `worker disable` can still explicitly revoke collection.
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

Never expose token files, edit Codex SQLite, delete task data, run
`VACUUM`, or infer completion from an idle task.
