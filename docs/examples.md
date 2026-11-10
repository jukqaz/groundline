# Core examples

Use `$groundline:optimize-codex-workflow` for a real repeated friction. Native
Codex authors the private brief, reads context and applies the justified diff.
The owner does not maintain forms. The [owning procedure](../plugins/groundline/references/conversation-improvement.md)
defines the small input fields and privacy boundaries.

```console
groundline improve status --state PRIVATE_RECORDS
groundline improve capture --input brief.json --target PROJECT/AGENTS.md --owner-project-root PROJECT --state PRIVATE_RECORDS
# Apply the reviewed diff through native Codex and verify it.
groundline improve record --capture CAPTURE_SHA --target PROJECT/AGENTS.md --owner-project-root PROJECT --state PRIVATE_RECORDS
# Only when later independent real work exists:
groundline improve observe --change CHANGE_SHA --input followup.json --state PRIVATE_RECORDS
```

For requested maintenance use `doctor --json`, `provider-smoke --require-installed
--json`, or `learning status --state PRIVATE_RECORDS`. A specifically requested
aggregate diagnosis can use `audit weekly --days 7 --review --json` once; retain
and reuse its output. No audit, route packet or direct delivery receipt is needed
for ordinary work. See [installation](installation.md) for native setup/update.
