---
name: package-agent-task
description: Use when explicitly packaging broad or high-context work for handoff, compaction, or bounded delegation.
---

# Package Agent Task

## Purpose

Use this skill to turn a loose request into a task packet that another agent or
future turn can execute without guessing the goal, boundaries, or proof needed.

## Workflow

1. State the current conclusion in one sentence.
2. Extract the native Goal objective and current GroundLine phase. If no Goal
   was explicitly requested, record `Goal: none` rather than creating one.
3. Separate approved scope from deferred observations. Incorporate explicit
   user steering when it revises the same outcome; do not restart for unsolicited
   non-blocking ideas.
4. List context that matters now and discard stale or unrelated detail.
5. Define constraints, non-goals, mutation boundary, and approval needs.
6. Name the expected artifacts, success criteria, and smallest credible checks.
7. Keep the same task unless a boundary change requires a handoff. Describe a
   fork or new task only when useful; do not create one without a user request.
8. Assign a qualitative context budget: `lean`, `standard`, or `expanded`.
9. Name what to load first, what to defer or omit, and when to stop loading.
10. Set a delegation budget: `single` or `bounded-parallel`, with independent
   lanes and a stop-spawning condition.
11. Produce a handoff that another agent can continue from.

## Rules

- Keep the packet short enough to fit in the next agent's working context.
- Separate facts verified in the current worktree from assumptions.
- Put "do not do" items in non-goals when the user narrowed scope.
- Do not include raw transcripts, credentials, or secret values.
- If the task is too broad, split it into ordered packets instead of one vague
  packet.
- Use `lean` unless multiple repositories, providers, or evidence surfaces are
  required. A larger context budget is not a substitute for a clear goal.
- Do not invent a token count when the provider does not expose one.
- Default to `single`. Use `bounded-parallel` only when explicitly authorized
  by the user and useful for independent, read-heavy
  lanes whose results can return as compact evidence rather than raw logs.

## Output Contract

Return the outcome, verified state/artifact, authorization and non-goals,
remaining proof, and next action. Include Goal, phase, deferred candidates,
context/delegation budgets, or load order only when they affect continuation.
Do not emit empty fields or an exhaustive checklist for a small handoff.
