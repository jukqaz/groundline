---
name: evaluate-ai-usage-maturity
description: Use when assessing a person or team's AI usage maturity, AI fluency, agent workflow habits, verification discipline, tool orchestration, reuse, automation, safety, and next improvement steps.
---

# Evaluate AI Usage Maturity

## Purpose

Use this skill when the user wants to understand how effectively a person or
team uses AI agents. It evaluates operating behavior from evidence, not from
raw prompt dumps or secret-bearing history.

When the evidence starts in local provider histories, use
`audit-agent-history -> evaluate-ai-usage-maturity`. The first step prepares a
redacted Provider Evidence Packet; this skill then scores the workflow without
requiring raw transcript review.

## Workflow

1. Define scope: individual or team, time window, agent providers, repositories,
   and artifacts to inspect.
2. Declare provider coverage and evidence mode:
   - `artifact-backed maturity` for durable artifacts and redacted summaries.
   - `collector-backed fluency` only when a collector has run with explicit
     approval.
3. Collect privacy-preserving evidence: diffs, docs, tests, validation logs,
   release notes, automation configs, issue or PR summaries, and explicit user
   goals.
4. When provider-specific history evidence is needed, request a Provider
   Evidence Packet from `references/multi-provider-fluency-boundary.md` instead
   of raw transcripts.
5. State the evaluation method: evidence types, scoring axes, confidence, and
   what was excluded for privacy or uncertainty.
6. Score with `references/ai-usage-maturity.md`.
7. Add the fluency overlay when the user asks about AI fluency, periodic
   progress, or a self-assessment.
8. Build an evidence-to-score map that explains why each axis received its
   score.
9. Include a longitudinal comparison only when prior assessment evidence exists.
10. Diagnose the main problems behind weak or capped axes.
11. Return development edges and an improvement plan with priority order, owner,
   and verification.

## Evaluation Rules

- Do not paste raw transcripts, credentials, prompts containing secrets, or
  private provider state.
- Do not imply full-provider conversation analysis unless a collector actually
  produced redacted evidence.
- Do not ask for a full raw message scan as the normal path; use
  artifact-backed evidence first, then approved redacted packets when needed.
- Prefer artifact-backed evidence over self-reporting.
- Score the workflow, not the person's intelligence.
- Penalize impressive-looking usage that lacks verification, reuse, or safety
  boundaries.
- Treat tool count as neutral; tool orchestration only scores when it improves
  outcomes with clear boundaries.
- For every weak or capped axis, explain the problem and the concrete behavior
  that would raise the score.
- Watch for artifact passivity: polished AI output accepted without review,
  tests, revision, or fact checks should cap discernment and verification.

## Output Contract

Use `GroundLine AI Usage Maturity` from `references/output-contracts.md`.
