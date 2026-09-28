---
name: align-agent-home
description: Install, apply, or verify GroundLine and repair evidenced Codex configuration mistakes within the requested scope.
---

# Install and verify GroundLine

Use [installation alignment](../../references/installation-alignment.md) for
installation or explicitly requested settings repair. Preserve user settings and
native defaults. An explicit model/effort choice must exist in the active native
catalog; no fixed GroundLine preset or automatic optimization writes settings.
The distribution installer already runs setup; reuse its result.

Resolve the installed executable through [platform commands](../../references/platform-commands.md).
Use `provider-smoke --require-installed` for the package and native artifact.
Use native strict doctor for effective Codex configuration and the actual Insights
`worker status` for collection state. A file's existence is not live-state proof.
Read [configuration review](../../references/codex-configuration.md) only for a
relevant configuration problem. Scope general skill maintenance to the user's
request using native tools; Core does not maintain a second skill registry.

Source validation, package checks, installed executable behavior, and the requested
live outcome are separate evidence. Verify the affected execution path, retain
private backups for settings writes, and preserve collection consent, pending
events, native task data, and user edits. Check App-bundled and PATH CLIs for
settings changes. A package-only installation does not authorize configuration
rewrites. Do not expose private config values or patch provider caches.
