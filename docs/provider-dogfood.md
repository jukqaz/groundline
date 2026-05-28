# Provider Dogfood

Provider dogfood has two evidence levels. The staged contract harness proves
that GroundLine can be staged for Codex, Claude Code, and Antigravity, and that
the shared scenario contracts are present. It does not prove live LLM skill selection.
Record real provider invocation proof separately with the sanitized shape below.

Run the reproducible staged contract harness:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

Expected safety fields:

- `suite=provider-dogfood`
- `status=PASS`
- `scenario_count=6`
- `mutation_performed=false`
- `real_home_touched=false`
- each provider reports `runtime_probe.probed=true`
- each provider reports `install.target_exists=true`
- each provider reports `install.target_skill_count_matches_source=true`
- each provider reports six `scenario_results`

Use `--home` and `--bin-dir` when a deterministic test home or fake runtime bin
directory is needed:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --home /tmp/groundline-home --bin-dir /tmp/groundline-bin --stage-package --probe-runtimes --json
```

The harness may create staged package files inside the explicit or temporary
home used for the test. It rejects staged package writes to the real home
directory.

## Sanitized Invocation Proof

Use this proof shape when recording real provider session evidence. Store only
the fields below; do not commit raw transcripts, full home paths, auth material,
provider cache dumps, or provider runtime state.

```text
provider: Codex | Claude Code | Antigravity
prompt_family: handoff | side-effect-guard | release-cut | ecosystem-evaluation | ai-usage-maturity
selected_skill: <groundline skill name>
output_contract: <contract name>
evidence: short sanitized summary
mutation_performed: true | false
raw_transcript_stored: false
provider_home_dumped: false
result: PASS | PARTIAL | FAIL
```

Evidence should be short enough to audit during release review. Prefer selected
skill, selected output contract, verification command, and stop condition over
conversation excerpts.

Use `docs/provider-activation-matrix.md` to track live provider rows for the
five prompt families. Keep this document focused on staged contract harness
evidence and the sanitized proof schema.

## Scenario Suite

The staged harness checks the six release-critical contract paths below.

| Scenario | Expected skill | Expected contract |
| --- | --- | --- |
| `long-context-handoff` | `package-agent-task` | `GroundLine Task Packet` |
| `completion-proof` | `close-live-work` | `Status: PASS / PARTIAL / FAIL` |
| `side-effect-guard` | `guard-side-effects` | `Boundary` |
| `release-cut` | `stabilize-release-cut` | `GroundLine Release Cut` |
| `ecosystem-evaluation` | `evaluate-agent-capability` | `GroundLine Capability Evaluation` |
| `ai-usage-maturity` | `evaluate-ai-usage-maturity` | `GroundLine AI Usage Maturity` |

## Prompt Family Coverage

| Prompt family | Primary skill | Evidence source |
| --- | --- | --- |
| `handoff` | `package-agent-task` | staged harness plus live matrix |
| `side-effect-guard` | `guard-side-effects` | staged harness plus live matrix |
| `release-cut` | `stabilize-release-cut` | staged harness plus live matrix |
| `ecosystem-evaluation` | `evaluate-agent-capability` | staged harness plus live matrix |
| `ai-usage-maturity` | `evaluate-ai-usage-maturity` | staged harness plus live matrix |

## Result Rules

- `PASS`: package staging, runtime probe when requested, target skill count, and
  all scenario contracts pass for all supported providers.
- `PARTIAL`: at least one provider runtime, staged target, skill count, or
  scenario contract is missing.
- `FAIL`: the harness cannot parse inputs or required package files are absent.
