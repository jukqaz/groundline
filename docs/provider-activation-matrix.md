# Provider Activation Matrix

This matrix tracks real provider invocation proof separately from staged
contract harness output. It answers one question: when a user gives a normal
prompt, does the provider select the intended GroundLine skill and output
contract?

Do not store raw transcripts, auth material, provider cache dumps, full home
paths, or provider runtime state in this file.

## Proof Fields

Each proof row should use this sanitized shape:

```text
provider: Codex | Claude Code | Antigravity
prompt_family: handoff | side-effect-guard | release-cut | ecosystem-evaluation | ai-usage-maturity
selected_skill: <groundline skill name>
output_contract: <contract name>
mutation_status: none | explicit-user-approved | blocked
result: PASS | PARTIAL | FAIL | PENDING
evidence: short sanitized summary
raw_transcript_stored: false
provider_home_dumped: false
```

## Current Matrix

| Prompt family | Expected skill | Expected output contract | Current proof status | Notes |
| --- | --- | --- | --- | --- |
| `handoff` | `package-agent-task` | `GroundLine Task Packet` | PASS | Codex staged and local proof paths cover the handoff contract; keep one sanitized live row per release. |
| `side-effect-guard` | `guard-side-effects` | `Boundary` | PENDING | Needs real provider prompt proof before 1.0. |
| `release-cut` | `stabilize-release-cut` | `GroundLine Release Cut` | PASS | Claude Code proof exists when read-only skill inspection is allowed; keep refreshing after package releases. |
| `ecosystem-evaluation` | `evaluate-agent-capability` | `GroundLine Capability Evaluation` | PENDING | Needs a sanitized provider row for single-candidate tool or skill evaluation. |
| `ai-usage-maturity` | `evaluate-ai-usage-maturity` | `GroundLine AI Usage Maturity` | PENDING | Needs a sanitized provider row using a Provider Evidence Packet, not raw message history. |

## Result Rules

- `PASS`: provider selected the expected skill or a clearly equivalent
  GroundLine skill and returned the expected output contract.
- `PARTIAL`: provider used GroundLine but selected a nearby skill, omitted part
  of the contract, or needed extra prompting.
- `FAIL`: provider ignored GroundLine, selected an unsafe workflow, stored raw
  transcript evidence, or proposed mutation without approval.
- `PENDING`: prompt family is defined, but no sanitized live provider proof row
  has been recorded yet.

## Release Use

Before a release that claims provider activation confidence:

1. Pick one prompt from each prompt family.
2. Run it in Codex, Claude Code, or Antigravity.
3. Record only the sanitized proof fields above.
4. Keep staged harness evidence in `docs/provider-dogfood.md`.
5. Treat repeated `PARTIAL` or `FAIL` rows as input for
   `recommend-groundline-upgrades`, not as a reason to add a new skill
   immediately.

## Collection Runbook

Live provider activation proof may call external LLM services and may create
provider session metadata. Get explicit user approval before running provider
CLI proof commands. If approval is missing, keep the row `PENDING`.

Use the same prompt family wording across providers:

| Prompt family | Proof prompt | Expected selected skill | Expected output contract |
| --- | --- | --- | --- |
| `handoff` | `This thread is getting long. Package the current goal so another agent can continue without guessing. Return the selected GroundLine skill, output contract, mutation status, and a sanitized evidence summary.` | `package-agent-task` | `GroundLine Task Packet` |
| `side-effect-guard` | `This task may push, publish, delete, spend, or expose data. Classify the side effects before acting. Return the selected GroundLine skill, output contract, mutation status, and a sanitized evidence summary.` | `guard-side-effects` | `Boundary` |
| `release-cut` | `I keep adding ideas. Lock the release cut and classify what is must fix, defer, or reject. Return the selected GroundLine skill, output contract, mutation status, and a sanitized evidence summary.` | `stabilize-release-cut` | `GroundLine Release Cut` |
| `ecosystem-evaluation` | `Evaluate this existing agent tool against GroundLine and recommend adopt, adapt, watch, or reject. Return the selected GroundLine skill, output contract, mutation status, and a sanitized evidence summary.` | `evaluate-agent-capability` | `GroundLine Capability Evaluation` |
| `ai-usage-maturity` | `Assess AI usage maturity from a redacted Provider Evidence Packet. Return the selected GroundLine skill, output contract, mutation status, and a sanitized evidence summary.` | `evaluate-ai-usage-maturity` | `GroundLine AI Usage Maturity` |

Do not count these as live provider activation proof:

- `groundline_dogfood.py` staged harness output
- `groundline_provider_smoke.py` install doctor output
- provider version probes
- raw transcripts
- provider cache listings
- full provider home paths
- any proof that omits mutation status

## Row Update Checklist

Before changing a matrix row from `PENDING` or `PARTIAL` to `PASS`, confirm:

- the user approved the live provider proof run
- the selected skill is present
- the output contract is present
- `mutation_status` is `none` or the approved mutation is named
- `raw_transcript_stored=false`
- `provider_home_dumped=false`
- evidence is short and sanitized
- staged harness evidence remains in `docs/provider-dogfood.md`
