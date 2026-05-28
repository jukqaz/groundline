# Workflow Cookbook

This cookbook is for people and LLM agents who need to choose a GroundLine
workflow without reading the full skill index first. Each row maps a natural
request to the selected skill, output contract, verification evidence, and stop
condition.

Use these as short prompts. Keep raw transcripts, secrets, provider caches, and
full home dumps out of repository artifacts.

## Cookbook

| Workflow | Prompt | Selected skill | Output contract | Verification evidence | Stop condition |
| --- | --- | --- | --- | --- | --- |
| handoff recovery | "This thread is long. Package the current goal so another agent can continue without guessing." | `package-agent-task` | `GroundLine Task Packet` | current branch, dirty diff, active goal, constraints, non-goals, and verification checklist | The packet has enough context for a new agent and names what not to change. |
| risky operation | "This may push, publish, delete, spend, or expose data. Classify side effects before acting." | `guard-side-effects` | `Boundary` | intended side effect, read-only checks, approval needed, and `secret value printed: false` | The next command is allowed, needs approval, or is rejected before mutation. |
| release cut | "I keep adding ideas. Lock the release cut and classify must fix, defer, and reject." | `stabilize-release-cut` | `GroundLine Release Cut` | scope lock, change budget, release gates, dogfood evidence, and regression check | Ship, hold, or continue is explicit and the next action is singular. |
| ecosystem radar | "Research current agent workflow tools, compare them against GroundLine, and recommend upgrades." | `agent-ecosystem-radar` | `GroundLine Research`, `GroundLine Capability Evaluation`, `GroundLine Comparison`, `GroundLine Recommendation` | primary sources, secondary sources, confirmed facts, unverified claims, comparison gaps, and side-effect boundary | Adopt, adapt, watch, and reject lists are separated without adding release scope. |
| AI usage maturity | "Assess my AI usage maturity from artifacts and give strengths, gaps, and next upgrades." | `audit-agent-history -> evaluate-ai-usage-maturity` | `Provider Evidence Packet`, then `GroundLine AI Usage Maturity` | redacted evidence packet, provider coverage, evaluation method, axis scores, and safety notes | The assessment names evidence used, unverified inputs, improvement plan, and verification needed. |

## Provider Proof Notes

Scripted staged dogfood proves that the package contains the selected skills
and output contract strings. It does not prove live LLM selection. Real
provider activation proof should record only:

- provider
- prompt family
- selected skill
- output contract
- mutation status
- short sanitized evidence

Do not store raw conversations or provider runtime state as proof.

## When To Stop

Stop expanding the task when the selected workflow reaches its stop condition.
If a new idea appears, use `hold-the-line` before adding it to the current
release or task.
