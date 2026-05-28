# AI Usage Maturity

Use this rubric to assess how effectively a person or team uses AI agents in
real work. The goal is better operating loops, not vanity scoring.

## Evidence Inputs

- Strong evidence: repository diffs, tests, CI logs, validation commands, issue
  or PR summaries, release notes, written operating rules, and installed skill
  or automation surfaces.
- Acceptable evidence: recent task summaries, tool usage summaries, handoff
  packets, and user-stated goals that match artifacts.
- Weak evidence: raw transcripts, screenshots, vibes, tool counts without
  outcomes, and unverifiable claims.

Do not use raw transcripts unless the user explicitly provides a short excerpt
and approves its use. Do not record secret values.

When provider-specific history evidence is needed, request a Provider Evidence
Packet instead of broad transcript access. The packet should contain provider
coverage, time window, task types, tools used, verification evidence, handoff
evidence, repeated failure patterns, approved excerpts, and privacy boundary.

## Evidence Modes

Every assessment must state provider coverage and evidence mode.

- `artifact-backed maturity`: default mode. Use durable artifacts and redacted
  summaries from Codex, Claude Code, Antigravity, repositories, CI, docs, and
  automation configs. This mode can assess multi-agent operating maturity but
  must not claim CEFR scoring.
- `collector-backed fluency`: optional future mode. Use only after explicit
  approval to inspect provider histories, normalize them into a redacted
  evidence packet, run behavior extraction, and score CEFR-style fluency.

Use `references/multi-provider-fluency-boundary.md` when the user asks whether
the assessment includes all providers, all agents, or whole conversation
history.

## Score Axes

Score each axis from 0 to 10.

- `task framing`: goals, constraints, success criteria, and stopping points are
  stated clearly enough for an agent to act.
- `context packaging`: relevant files, prior decisions, and boundaries are
  supplied without flooding the agent.
- `tool orchestration`: local tools, providers, MCP servers, browser checks,
  tests, and scripts are selected for evidence rather than novelty.
- `agent delegation`: subagents, skills, handoffs, or provider-specific agents
  are used with separate scopes and clear outputs.
- `verification discipline`: claims are checked with tests, lint, builds,
  smoke probes, screenshots, runtime checks, or live evidence.
- `reuse`: repeated patterns become skills, docs, scripts, templates, rules, or
  checklists.
- `automation leverage`: repeated checks, research loops, update reviews, and
  release gates are automated only where useful.
- `safety`: secrets, permissions, provider homes, user data, git history,
  public release risk, and side effects are handled explicitly.
- `impact`: the workflow produces shipped code, useful docs, fewer defects,
  faster triage, or clearer decisions.

## Evaluation Method

Every assessment must explain its evaluation method before giving advice:

- evidence types used and excluded
- provider coverage
- evidence mode
- scoring axes and score range
- confidence level
- privacy boundary
- unverified claims

## Evidence-To-Score Map

For each axis, include an evidence-to-score map:

- score
- evidence that supports the score
- why the score is capped
- missing evidence that could change the score

## Fluency Overlay

When the user asks for AI fluency, periodic self-assessment, or a quarterly
review, add a fluency overlay without replacing the GroundLine score axes.
Use the overlay to explain behavior in user-friendly terms:

- `Delegation`: what work is handed to agents, what remains human-owned, and
  whether autonomy is calibrated per task.
- `Description`: how clearly the user supplies goals, constraints, examples,
  formats, and stopping criteria.
- `Discernment`: how actively the user questions, edits, tests, or rejects AI
  outputs.
- `Diligence`: how consistently the user protects sensitive data, verifies
  claims, and keeps side effects bounded.

The overlay maps to GroundLine axes:

- Delegation -> task framing, agent delegation, automation leverage.
- Description -> task framing, context packaging.
- Discernment -> verification discipline, problem diagnosis.
- Diligence -> safety, verification discipline.

Track artifact passivity under Discernment and verification discipline: polished
AI artifacts that are accepted without a revision, test, review, or fact check
cap the score even when the output looks useful.

## Longitudinal Comparison

If a previous assessment exists, include a longitudinal comparison. Compare only
durable fields: overall score, level, axis scores, fluency overlay ratings,
automation count, skill or workflow reuse, verification evidence, and open
development edges. If previous evidence is missing or self-reported, mark the
comparison as low confidence instead of inventing a trend.

Use a quarterly cadence for recurring reviews unless the user asks for a
different interval. Do not create reminders or background jobs unless the user
explicitly asks.

## Development Edges

Development edges are falsifiable improvement targets carried between
assessments. Each edge should include:

- title
- target behavior
- why it matters
- status: open, in-progress, resolved, or dropped
- proof needed at the next review

Create one to three edges only. A good edge changes observable behavior, such
as "run one explicit verification command before accepting generated code" or
"package long-context work before handing it to another agent."

## Problem Diagnosis

Problem diagnosis should identify the operating issue behind weak or capped
axes. Prefer specific workflow problems over generic advice.

Examples:

- unclear stopping criteria
- too much context before the agent has a task boundary
- tool use without a verification target
- skills created without enough dogfooding
- automation added before the manual loop is stable

## Improvement Plan

The improvement plan should list actions in priority order. Each action should
include the target axis, expected behavior change, proof command or artifact,
and review timing.

## Levels

- 0-20: casual user
- 21-40: assisted user
- 41-60: workflow user
- 61-80: AI power user
- 81-100: agent-native operator

## Required Output Notes

Every assessment should say:

- scope and evidence used
- provider coverage
- evidence mode
- provider evidence packet status
- unverified inputs
- evaluation method
- overall score and level
- axis scores
- fluency overlay when requested or useful
- longitudinal comparison when prior assessment evidence exists
- evidence-to-score map
- top strengths
- highest-leverage gaps
- problem diagnosis
- development edges
- improvement plan
- next upgrades
- safety notes
- verification needed for a stronger assessment
