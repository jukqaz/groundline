---
name: evaluate-ai-usage-maturity
description: Use when explicitly assessing Codex workflow efficiency or choosing a Codex model and effort from current task shape.
---

# Evaluate AI Usage Maturity

## Purpose

Choose a simple Codex mode for the current task, or evaluate operating behavior
from artifacts and redacted evidence. When evidence starts in histories, use
`audit-agent-history -> evaluate-ai-usage-maturity`; consume its Codex Evidence
Packet.

## Quick Mode Choice

Resolve the installed GroundLine root from this `SKILL.md` before opening
bundled references or using the installed binary. Never resolve these files
from the user's current repository.

When the user only asks what to use for the current task:

1. Name the primary outcome and whether one hardest question dominates.
2. List independent lanes only when they can return separate outputs without
   concurrent edits to the same surface.
3. Read `$GROUNDLINE_ROOT/references/model-effort-routing.md`.
4. Return exactly one recommendation:
   - `Luna low` for closed work with deterministic verification.
   - `Terra medium` for everyday exploration, debugging, and broad review.
   - `Sol medium` for normal difficult, ambiguous, or important work.
   - `Sol high` for complex logic, security, difficult review, or
     release-critical reasoning.
   - `Sol xhigh` only after a credible lower-effort attempt leaves a bounded
     problem unresolved, or a named risk justifies the deeper pass.
   - `Sol max` for one hardest sequential problem with a stopping condition.
   - `Sol ultra` only for at least two independent lanes when the user permits
     subagent work.
5. Reject Ultra when the active instructions allow subagents only after an
   explicit request and the user has not made that request. Ultra's proactive
   delegation is then unavailable; use the lowest fitting Sol effort instead.
6. Warn before Ultra when the runtime allows eight or more concurrent threads;
   do not change the limit.
7. Do not require a history audit or maturity score for this quick route.
8. Do not edit Codex settings.

When the user asks to compare modes empirically, define the same task, input,
time boundary, verification contract, and protected outcomes for each run.
Return a `GroundLine Codex Benchmark` plan or result and keep Max and Ultra on
their separate specialist tracks.

When the user asks how much a GroundLine workflow could improve, use
`groundline efficiency simulate --audit <audit.json> --json` with one or
more redacted session-audit JSON files. Report conservative, expected, and
optimistic projections separately. Do not convert reported total or cached
tokens into billing.

## Workflow

1. Define person/team, time window, Codex scope, repositories, and artifacts.
2. Declare evidence mode: `artifact-backed maturity`.
3. Gather diffs, tests, docs, validation/release evidence, automation configs,
   issue/PR summaries, explicit native Goals, and an optional Chronicle
   aggregate that follows the bundled contract.
4. State Codex coverage, method, exclusions, confidence, and evidence-to-score mapping.
5. Score scope control, context discipline, verification, safety, reuse, and
   release closure from 0 to 4; add longitudinal comparison only when comparable
   prior evidence exists.
6. For GPT-5.6, Max, context, or usage efficiency, add the Codex efficiency overlay. Read model routing guidance only for a concrete recommendation and verify the current catalog first.
7. For an accepted growth loop, pass only redacted numeric signals and
   protected-outcome counts to the strict comparison command; GroundLine keeps
   no separate mutable growth ledger.
8. Return prioritized development edges with owner and verification. Include
   one bounded behavior experiment when efficiency is in scope. Keep any
   Chronicle A/B experiment independent and do not modify its state or ledger.

## Rules

- Never paste transcripts, credentials, secret-bearing prompts, or private provider state.
- Do not claim full conversation coverage without approved redacted collection.
- Prefer artifacts over self-report. Score workflow quality, not intelligence.
- Tool count is neutral; reward orchestration only when outcomes and boundaries improve.
- Never infer billable tokens from bytes, file sizes, tool calls, or elapsed time.
- Recommend an escalation ladder; do not change Codex model, reasoning effort, Max, Ultra, or service tier.
- Quality and safety constrain efficiency. Do not record an experiment without a healthy completion/verification baseline, low avoidable rework, and zero permission, privacy, safety, or irreversible-effect breach.
- Keep one active experiment. Discuss its evidence-backed result before another; the user may accept, revise, or reject the hypothesis.
- Store no raw evidence in growth state and write state only with explicit approval.
- Explain every weak/capped axis and the behavior that raises it. Cap discernment and verification when polished output was accepted without review or checks.

## Output Contract

Use `GroundLine Codex Mode` for quick routing. Use `GroundLine AI Usage
Maturity` for an evidence-backed assessment. Add `GroundLine Growth Challenge`
only when requested or already active.
