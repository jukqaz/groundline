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
- outbox status proves local durability, not server acceptance;
- an accepted upload proves API acknowledgement, not Grafana freshness.

## Common actions

- `invalid_owner_profile`: install the reviewed owner-local schema-7 input with
  `groundline-insights worker configure --input <profile.json>`; it must contain
  an enrollment credential, and neither the real endpoint nor credential belongs
  in the plugin.
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

Never expose token files, edit Codex SQLite, delete task data, run
`VACUUM`, or infer completion from an idle task.
