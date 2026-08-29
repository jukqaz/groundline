---
name: align-agent-home
description: Use when explicitly aligning Codex guidance, worktree readiness, plugins, skills, MCP, rules, hooks, model posture, or runtime state.
---

# Align Agent Home

## Purpose

Audit Codex project and home surfaces while preserving official plugins,
private state, and user-owned settings.

## Workflow

1. Resolve the installed platform binary from this file's plugin root. Never
   assume the current repository is GroundLine.

2. Start read-only:

```bash
groundline project-audit --repo . --json
groundline provider-smoke --require-installed --json
```

If Codex slows after long use, add the explicit read-only health probe:

```bash
groundline doctor --json
```

It reports only plugin, platform, and local state-store presence; it neither
prints stored content nor mutates Codex state.

Run Codex's strict native doctor separately when current provider diagnostics
are needed; keep its evidence distinct from GroundLine's read-only checks.

3. Map `AGENTS.md`, `.codex/config.toml`, skills, agents, rules, one hook form
   per layer, plugins, MCP, root `.worktreeinclude`, and
   `.codex/environments/*.toml`. Emit structural counts,
   never values or bodies. Leave trust-dependent activation `UNVERIFIED`
   absent provider evidence.
4. Load design guidance on demand, outside always-on context.
5. Keep auth, sessions, logs, OAuth material, shell snapshots, caches,
   databases, MCP headers, and environment values out of source and output.
6. Verify drift-prone claims locally. The App-bundled CLI is primary; PATH CLI
   is a secondary automation surface:

```bash
/Applications/ChatGPT.app/Contents/Resources/codex --version
codex --version
codex features list
codex debug models
codex debug prompt-input
codex --strict-config doctor --summary --no-color --ascii
```

Keep catalog, runtime, persisted settings, defaults, token metadata, quota,
and billing distinct.
7. Check current official OpenAI documentation before adopting plugin surfaces.
8. For App-managed local worktrees, prefer checked-in local environments for
   setup and actions. Create root `.worktreeinclude` only when failed setup or
   project evidence proves an ignored file is required and the user requested
   changes. Use minimal patterns, exclude tracked files and broad directories,
   never emit values, verify targets are ignored, and smoke a fresh worktree
   when practical. It does not affect Remote or CLI Git worktrees.
9. Keep official components. Remove custom duplicates only after identifying
   ownership; keep repository capabilities local and prefer Codex-native
   planning, delegation, debugging, review, and verification. Do not import
   orchestration packs wholesale; add GroundLine only for a repeated gap.
10. Report bundled and PATH CLIs separately; never infer App support from PATH.
11. Treat models, effort, quotas, and compaction as Codex-owned. Preserve
    App-authored settings, plugins, connectors, experiments, Chronicle,
    personality, and UI. Explain Fast/priority tradeoffs; change
    `service_tier` only on request.
12. The public GroundLine package must have no lifecycle hook. Treat any
    packaged owner hook as a failed installation contract and do not trust it.
13. Preserve requested permissions and approvals. Never mix or silently
    migrate beta profiles with legacy
    `sandbox_mode`. Prefer `:workspace`, retain
    `approvals_reviewer = "auto_review"`, and never map safe Auto to
    `:danger-full-access`. Report review cost and latency.
14. Validate App-bundled runtime first, then PATH CLI audit and smoke.

## Source Boundaries

Source control may contain reviewed guidance, intentional config, selected
agents, rules, hooks, and skills. Keep private runtime files and secrets out.

## Output Contract

```text
Conclusion: aligned / partially aligned / blocked
Changed:
- ...
Preserved runtime/private state:
- ...
Verification:
- command and result
Unverified:
- ...
```
