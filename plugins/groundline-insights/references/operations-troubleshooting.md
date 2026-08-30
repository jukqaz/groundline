# GroundLine Insights Operations Troubleshooting

Diagnose from bounded evidence before reinstalling or changing state.

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
  and retry once. Do not delete evidence.
- changed hook hash: review and trust it in Codex. GroundLine never approves
  itself.

Read `worker status` from the operational fields. `status: PASS` with
`collection_state: disabled` is an intentional inert installation. `status: WARN`
identifies a collection blocker through `blocking_reason_codes`; it is not a
package-integrity failure. Enabling without a valid owner profile and enrollment
credential is rejected before identity, consent, or policy state is created.
Seven days without a successful collection becomes `collection_state: stale`;
a success timestamp more than five minutes in the future becomes `clock_skew`.

Never expose token files, edit Codex SQLite, delete task data, run
`VACUUM`, or infer completion from an idle task.
