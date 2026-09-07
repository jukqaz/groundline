---
name: align-agent-home
description: Use when explicitly aligning Codex guidance, worktree readiness, plugins, skills, MCP, rules, hooks, model posture, or runtime state.
---

# Align Agent Home

Audit the requested Codex project/home surfaces and apply authorized changes,
preserving official components and user-owned settings.

## Select the relevant checks

Resolve the installed platform binary using
[platform commands](../../references/platform-commands.md). Source, installed
package, App-bundled CLI, and PATH CLI are separate evidence. Do not patch
provider caches or substitute an unrelated binary when one is missing.

- Project structure: `groundline project-audit --repo . --json` inventories
  guidance, config, skills, agents, rules, hooks, and worktree surfaces.
- Installed package: `groundline provider-smoke --require-installed --json`
  verifies the package and native artifact.
- Slow or inconsistent local state: `groundline doctor --json` checks only
  presence and installation structure; it does not read stored content.
- Configuration, model/effort, permissions, or worktree readiness: read
  [configuration review](../../references/codex-configuration.md). Use its
  offline `config-audit` and native strict doctor only when relevant.
- Personal/imported skill inventory, source refresh, deduplication, or regression
  checks: read [skill maintenance](../../references/skill-maintenance.md).
  GroundLine owns profile/baseline checks; Codex performs reviewed changes.

Do not load every reference or run every command for a narrow request. Before
adopting current Codex features, consult official OpenAI documentation and the
actual runtime. Keep native diagnostics separate from GroundLine checks.

## Apply the requested alignment

Read-only reviews do not authorize edits. An implementation request covers its
routine in-scope steps; ask only for a material missing choice or new authority.
Keep user steering attached to the same task while its outcome remains aligned.

Keep global guidance minimal and repository behavior local. Remove custom
duplicates only after establishing ownership. Preserve model, effort, service
tier, permissions, experiments, Chronicle, personality, and UI unless the user
requests a change. Native Codex owns planning, delegation, context management,
and plugin upgrades; no parallel orchestration layer is needed.

Never copy private configuration, personal skills, credentials, transcripts,
databases, or runtime caches into the public plugin. Emit structural counts and
minimal relevant non-secret excerpts, not whole private files. Core has no
lifecycle hooks; optional Insights has a separate consented checkpoint contract.
Hook trust alone does not prove installation, dispatch, or collection consent.

Verify affected behavior once, then broaden only for new changes or unresolved
risk. Report the result, useful evidence, and remaining gaps without empty
templates. Distinguish source changes, package validation, installed runtime,
and live behavior. Name and link the exact skill instruction if it causes a
pause or changes the requested direction.
