# Dogfood

This document records self-use validation for GroundLine. It separates scripted
smoke checks, staged provider dogfood, and accepted defers.

## Dogfood Matrix

Date: 2026-05-28

Real home mutation: false

| Provider | Scenario | Expected skill | Evidence | Result |
| --- | --- | --- | --- | --- |
| Codex | Staged package plus shared scenario suite. | `package-agent-task`, `close-live-work`, `stabilize-release-cut` | `codex --version` returned `codex-cli 0.134.0`; `groundline_dogfood.py` reported runtime and all scenario contracts present. | PASS |
| Claude Code | Staged package plus shared scenario suite. | `package-agent-task`, `close-live-work`, `stabilize-release-cut` | `claude --version` returned `2.1.152 (Claude Code)`; `groundline_dogfood.py` reported runtime and all scenario contracts present. | PASS |
| Antigravity | Staged package plus shared scenario suite. | `package-agent-task`, `close-live-work`, `stabilize-release-cut` | `agy --version` returned `1.0.2`; `groundline_dogfood.py` reported runtime and all scenario contracts present. | PASS |

## Provider Invocation Evidence

Date: 2026-05-28

Raw transcript stored: false

Provider home dumped: false

| Provider | Prompt family | Selected skill | Output contract | Result | Evidence |
| --- | --- | --- | --- | --- | --- |
| Codex | handoff | `package-agent-task` | `GroundLine Task Packet` | PASS | Read-only `codex exec --ephemeral` loaded the GroundLine skill and returned the canonical contract with `mutation_performed=false`. |
| Claude Code | release-closeout | `groundline:polish-release-candidate` | `release_polish_report` | PARTIAL | `claude -p` selected an installed GroundLine skill, but did not return the canonical `GroundLine Release Polish` contract name. |
| Antigravity | expansion-control | not captured | not captured | PARTIAL | `agy --print` could not complete a constrained no-tool proof in this environment and timed out after tool-driven exploration; no repository mutation was observed. |

## Provider Invocation Follow-up

Date: 2026-05-28

Raw transcript stored: false

Provider home dumped: false

| Provider | Prompt family | Selected skill | Output contract | Result | Evidence |
| --- | --- | --- | --- | --- | --- |
| Claude Code | release-closeout | `stabilize-release-cut` | `GroundLine Release Cut` | PASS | `claude -p` returned the canonical skill and contract after the proof prompt allowed read-only skill doc inspection with `mutation_performed=false`. |
| Antigravity | expansion-control | not captured | not captured | PARTIAL | `agy --print` still entered tool exploration and hit Antigravity CLI app-data write constraints before returning a sanitized proof; package validation and install remain PASS. |

## Superpowers Companion Dogfood

Date: 2026-05-28

Raw transcript stored: false

Provider home dumped: false

Real home mutation: false

Result: PASS for the local operating loop, PARTIAL for live/private evidence.

Fresh local checks:

- `groundline_doctor.py --json --offline` returned `status=PASS`.
- Doctor selected `recommended_mode=companion-superpowers`.
- Doctor reported `superpowers.present=true`.
- Doctor reported all three supported runtimes present.
- Doctor reported `github` available and `context7` plus `exa` as optional gaps.
- Staged provider dogfood returned `status=PASS`, `scenario_count=3`, and
  `real_home_touched=false`.
- The staged run observed `codex-cli 0.134.0`, `Claude Code 2.1.152`, and
  `Antigravity 1.0.3`.

Coverage judgment:

| Work type | Primary owner | GroundLine role | Superpowers role | Result |
| --- | --- | --- | --- | --- |
| Resume or handoff | GroundLine | `reconcile-current-state`, `package-agent-task` | execution plan after state proof | PASS |
| Planning or creative design | Superpowers | scope and side-effect boundary | brainstorming and plan writing | PASS |
| Implementation | Superpowers | guard risky mutations and handoff context | TDD, debugging, executing plans | PASS |
| Parallel agent work | Superpowers | task packet and state reconciliation | dispatch and subagent workflow | PASS |
| Release cut | GroundLine | `stabilize-release-cut`, `polish-release-candidate` | verification before completion | PASS |
| Live runtime proof | GroundLine | `close-live-work` | final evidence discipline | PASS when evidence source is reachable |
| External or private live data | Provider MCP or native tools | decide when optional MCP is needed | use results inside the workflow | PARTIAL by design |
| Provider-specific automation | Provider runtime | keep hooks, rules, MCP, and agents opt-in | avoid replacing provider controls | PARTIAL by design |

Conclusion: Superpowers plus GroundLine covers the general agent operating loop.
It does not try to cover provider-owned live data access, private MCP setup,
global hooks, provider rules, or catalog-specific plugin behavior. Those remain
optional provider integrations.

## Scripted Evidence

- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json`
  returned `status=PASS`.
- Source package reported `skill_count=19`.
- Source package reported `skill_index_consistent=true`.
- Provider targets were not installed in the real home.
- `mutation_performed=false`.
- `real_home_touched=false`.

## Harness Evidence

- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py
  --stage-package --probe-runtimes --json` returned `status=PASS`.
- `suite=provider-dogfood`.
- `scenario_count=3`.
- `fake_home_used=true`.
- `temp_state_created=true`.
- `mutation_performed=false`.
- `real_home_touched=false`.
- A staged run against `--home ~` is rejected before writing package files.
- Each provider reported `runtime_probe.probed=true`.
- Each provider reported `target_exists=true`.
- Each provider reported `target_manifest_present=true`.
- Each provider reported `target_skills_present=true`.
- Each provider reported `target_skill_count=19`.
- Each provider reported `target_skill_count_matches_source=true`.
- Each provider reported three `scenario_results` with `status=PASS`.

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

## Completed

- Provider runtime probes completed for Codex, Claude Code, and Antigravity.
- Staged provider package targets matched source package skill count.
- Shared scenario prompts mapped to expected skills and output contracts.
- v0.2.0 `PARTIAL` dogfood items are closed by the v0.2.1 harness evidence.
- v0.3.0 shipped with sanitized provider invocation evidence and offline safety
  eval coverage.
- The v0.3.1 follow-up reduced the Claude Code contract naming partial with a
  read-only skill doc proof.

## Accepted Defer

- Interactive provider UI sessions are outside the current patch scope because
  provider-native plugin discovery is owned by each runtime.
- Provider-specific hook or MCP setup stays out of this patch.
- New skills derived from dogfood results stay out until a failure is
  reproduced.
- Antigravity print-mode proof remains partial until the CLI can run the
  constrained proof without tool exploration or app-data write failures.

## Next Review

Run `compare-release-delta` after the next release to compare it with v0.3.0
and confirm the remaining provider invocation partial was reduced or still
accepted intentionally.
