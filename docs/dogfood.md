# Dogfood

This document records self-use validation for GroundLine. It separates scripted
smoke checks from real provider sessions.

## Dogfood Matrix

Date: 2026-05-28

Real home mutation: false

| Provider | Scenario | Expected skill | Evidence | Result |
| --- | --- | --- | --- | --- |
| Codex | Current thread used GroundLine to stop expansion and define the next proof loop. | `stabilize-release-cut`, `package-agent-task` | `codex --version` returned `codex-cli 0.134.0`; current thread produced scope-lock guidance and dogfood plan. | PARTIAL: workflow shape works, plugin discovery not proven. |
| Claude Code | Preflight only. Confirm runtime exists and package is ready for manual install. | `package-agent-task`, `close-live-work` | `claude --version` returned `2.1.152 (Claude Code)`; provider smoke reports manifest present and target not installed. | PARTIAL: runtime exists, real session not run. |
| Antigravity | Preflight only. Confirm runtime exists and package is ready for manual install. | `package-agent-task`, `stabilize-release-cut` | `agy --version` returned `1.0.2`; provider smoke reports manifest present and target not installed. | PARTIAL: runtime exists, real session not run. |

## Scripted Evidence

- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json`
  returned `status=PASS`.
- Source package reported `skill_count=19`.
- Source package reported `skill_index_consistent=true`.
- Provider targets were not installed in the real home.
- `mutation_performed=false`.
- `real_home_touched=false`.

## Staged Install Evidence

The staged install used a temporary home under `/private/tmp` and copied only the
provider manifests plus `skills/` into provider target paths. It did not write to
the real provider homes.

- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --home
  /private/tmp/groundline-provider-installed-IfgJlC/home --json` returned
  `status=PASS`.
- `fake_home_used=true`.
- Each provider target reported `target_exists=true`.
- Each provider target reported `target_manifest_present=true`.
- Each provider target reported `target_skills_present=true`.
- Each provider target reported `target_skill_count=19`.
- Each provider target reported `target_skill_count_matches_source=true`.
- `mutation_performed=false`.
- `real_home_touched=false`.

## Scenario Prompts

Use the same prompts across providers after installing or staging GroundLine:

1. Long-context handoff:

   ```text
   This thread is getting long. Package the current goal so another agent can continue without guessing.
   ```

2. Completion proof:

   ```text
   Tests passed. Decide whether this work is actually complete and what evidence is still missing.
   ```

3. Expansion control:

   ```text
   I keep adding ideas. Lock the release cut and classify what is must fix, defer, or reject.
   ```

## Release Discipline

new skills derived from dogfood results should not enter release scope until
there is dogfood or repeated failure evidence. A promising idea without that
evidence stays in `watch` or `defer`.

## Must Fix

Must fix items block a release cut until they have evidence or an explicit
defer decision.

- Real provider sessions have not yet proved skill discovery or automatic
  selection.
- Manual install or staged provider-home install still needs explicit approval
  before mutation.

## Defer

- More scenarios beyond the three prompts above.
- Provider-specific hook or MCP setup.
- New skills derived from dogfood results until a failure is reproduced.

## Next Review

Run `stabilize-release-cut` after one real session per provider. Only promote a
finding to `must fix` when the same issue appears in a provider session or a
scripted gate.
