---
name: reconcile-current-state
description: Use when resuming stale work or before broad, ambiguous, current-fact-dependent, or high-impact changes need a selective pre-implementation gate.
---

# Reconcile Current State

## Purpose

Prove checkout, runtime, Goal, and task boundary. Prior reports are hints.

## Selective Pre-implementation Gate

Use a light gate for bounded work: prove state, scope, and verification. Use a
full gate before broad, ambiguous, current-fact-dependent, or high-impact work:

1. `COLLECT` targeted repository, runtime, or official-source evidence.
2. `SYNTHESIZE` facts, risks, unknowns, and viable options once.
3. `FREEZE` scope, non-goals, mutation boundary, success criteria, verification,
   and stop condition.

Do not broaden research when it cannot change the decision. After `FREEZE`,
defer new non-blocking observations.

## Workflow

1. Identify the App task, worktree, branch, target, source, and request.
2. Read durable context; avoid broad transcript loading.
3. Prove repo root, status, targeted diff, worktree attachment, and history.
4. If explicitly requested, view or create the native Goal. Do not infer one
   from a broad prompt.
5. Classify the batch as `COLLECT`, `SYNTHESIZE`, `FREEZE`, `IMPLEMENT`,
   `VERIFY`, or `RELEASE`. Apply only a user-accepted bounded change.
6. Keep the task while outcome, repository, and permission match. Otherwise use
   a side question, fork, packet, or new task.
7. Verify the required live process, endpoint, CI/PR, release, queue, or flow.
8. Mark prior claims `confirmed`, `stale`, `contradicted`, or `unverified`.
   Continue only when the next safe action and mutation boundary are clear.

Use `groundline efficiency batch --input <packet.json> --json` for a
deterministic boundary. It is read-only; Codex owns execution and verification.

## Output Contract

```text
Current conclusion: continue / pause / repair first / blocked
Goal and phase:
Task boundary:
Preflight: light / full and why
Confirmed:
- ...
Drift or contradiction:
- ...
Unverified:
- ...
Next safe action:
- ...
```

Never claim completion solely because another agent did.
