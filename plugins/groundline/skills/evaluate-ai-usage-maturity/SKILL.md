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

Resolve references from this installed skill's plugin root, not the user's
repository. If CLI evidence is needed, read
[installed command resolution](../../references/platform-commands.md).

When the user only asks what to use for the current task:

1. Name the primary outcome and whether one hardest question dominates.
2. List independent lanes only when they can return separate outputs without
   concurrent edits to the same surface.
3. Read `$GROUNDLINE_ROOT/references/model-effort-routing.md`.
4. Check the current session's available models and supported efforts. Use the
   primary runtime's `codex debug models` when the session lacks that evidence;
   distinguish its refreshed catalog from `--bundled` and from account access.
   Return one task-scoped recommendation using the criteria in the reference.
   Keep the provider recommendation when no change would improve this task.
5. Evaluate supported Max/Ultra effort by reasoning difficulty independently of
   delegation. The user's explicit request, not an effort label, authorizes
   subagents in this workflow.
6. Use the lowest supported effort that fits the task. Increase it for a
   concrete unresolved problem, not just because a newer model is available.
7. Do not require a history audit or maturity score for this quick route.
8. Do not edit Codex settings.

When the user asks to compare modes empirically, define the same task, input,
time boundary, verification contract, and protected outcomes for each run.
Return a `GroundLine Codex Benchmark` plan or result. Hold model, tools, and
delegation policy constant when comparing effort; vary one factor at a time.

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
6. For model, context, or usage efficiency, add the Codex efficiency overlay.
   Use current catalog evidence for recommendations; model families in audit
   output are aggregate labels, not an availability or pricing catalog.
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
- For a tracked efficiency experiment, establish a healthy verification
  baseline and no permission, privacy, or safety breach before claiming gains.
  Missing baseline evidence limits the experiment, not authorized repairs or
  ordinary implementation. Keep one tracked experiment active unless the user
  explicitly requests independent comparisons; discuss evidence before the next.
- Store no raw evidence in growth state and write state only with explicit approval.
- Explain every weak/capped axis and the behavior that raises it. Cap discernment and verification when polished output was accepted without review or checks.

## Output Contract

Use `GroundLine Codex Mode` for quick routing. Use `GroundLine AI Usage
Maturity` for an evidence-backed assessment. Add `GroundLine Growth Challenge`
only when requested or already active.
