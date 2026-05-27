---
name: package-agent-task
description: Use when a broad, ambiguous, multi-agent, resumed, or high-context request needs an LLM-ready task packet with goal, context, constraints, non-goals, mutation boundary, success criteria, verification, and handoff.
---

# Package Agent Task

## Purpose

Use this skill to turn a loose request into a task packet that another agent or
future turn can execute without guessing the goal, boundaries, or proof needed.

## Workflow

1. State the current conclusion in one sentence.
2. Extract the real goal, not only the latest wording.
3. List context that matters now and discard stale or unrelated detail.
4. Define constraints, non-goals, mutation boundary, and approval needs.
5. Name the expected artifacts and success criteria.
6. Select the smallest credible verification for the task.
7. Produce a handoff that another agent can continue from.

## Rules

- Keep the packet short enough to fit in the next agent's working context.
- Separate facts verified in the current worktree from assumptions.
- Put "do not do" items in non-goals when the user narrowed scope.
- Do not include raw transcripts, credentials, or secret values.
- If the task is too broad, split it into ordered packets instead of one vague
  packet.

## Output Contract

Use `GroundLine Task Packet` from `references/output-contracts.md`.
