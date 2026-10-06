---
name: align-agent-home
description: Use for requested GroundLine installation, Codex environment alignment, or evidenced settings repair. Skip unrelated task setup.
---

# Install and verify GroundLine

Use [installation alignment](../../references/installation-alignment.md) for
installation or explicitly requested settings repair. Preserve user settings and
native defaults. An explicit model/effort choice must exist in the active native
catalog; no fixed GroundLine preset or automatic optimization writes settings.
The distribution installer already runs setup; reuse its result.

Choose only the affected verification route:

- **Requested personal environment alignment:** use
  [adaptive environment](../../references/adaptive-environment.md) to establish
  common intent, host exceptions, scoped changes and actual App/PATH loading.
  Use existing native repair paths; persistent synchronization is not implemented.

- **Package installation or verification:** resolve the installed executable through
  [platform commands](../../references/platform-commands.md) and use
  `provider-smoke --require-installed` for the package and native artifact.
  Reuse the distribution installer's setup result instead of repeating it.
- **Requested Codex settings repair:** read
  [configuration review](../../references/codex-configuration.md), inspect relevant
  active layers, and verify affected settings with scoped native diagnostics such
  as `config/read` or `skills/list`, where supported and relevant. Check App-bundled
  and PATH CLIs after settings changes. Use strict doctor only when full installation
  or runtime health diagnosis is needed.
- **Requested Insights connection or collection verification:** use the actual
  Insights `worker status` and the affected live path. Package installation or
  Codex settings repair alone does not require an Insights check or activation.

A file's existence is not live-state proof. Scope general skill maintenance to
the user's request using native tools; Core does not maintain a second registry.

Source validation, package checks, installed executable behavior, and the requested
live outcome are separate evidence. Verify the affected execution path, retain
private backups for settings writes, and preserve collection consent, pending
events, native task data, and user edits. A package-only installation does not
authorize configuration rewrites. Do not expose private config values or patch
provider caches.
