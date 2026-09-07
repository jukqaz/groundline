---
name: reconcile-current-state
description: Use when resuming stale work or before broad, ambiguous, current-fact-dependent, or high-impact changes need a selective pre-implementation gate.
---

# Reconcile Current State

## Purpose

Prove the relevant current state and task boundary. Prior reports are hints;
reuse current evidence unless the checkout, host, input, or scope changed.

## Selective Pre-implementation Gate

Use a light gate for bounded work: prove state, scope, and verification. Use a
full gate before broad, ambiguous, current-fact-dependent, or high-impact work:

1. `COLLECT` targeted repository, runtime, or official-source evidence.
2. `SYNTHESIZE` facts, risks, unknowns, and viable options once.
3. `FREEZE` scope, non-goals, mutation boundary, success criteria, verification,
   and stop condition.

Do not broaden research when it cannot change the decision. After `FREEZE`,
defer unsolicited non-blocking observations. Explicit user steering may revise
the same scope without creating a new task.

## Workflow

1. Identify the App task, worktree, branch, target, source, and request.
2. Read durable context; avoid broad transcript loading.
3. Inspect the affected state and targeted diff before editing. Check history,
   worktree attachment, or live systems only if relevant to this decision;
   do not require a full repository audit for a bounded change.
4. If explicitly requested, view or create the native Goal. Do not infer one
   from a broad prompt. Without a requested Goal, continue the ordinary task.
5. Classify the batch as `COLLECT`, `SYNTHESIZE`, `FREEZE`, `IMPLEMENT`,
   `VERIFY`, or `RELEASE` when phase tracking helps. Reviews and diagnoses are
   read-only unless a fix is requested. Continue routine implementation within
   existing approval; ask only for a material missing choice or new authority.
6. Keep the task while outcome, repository, and permission match. Otherwise use
   propose a handoff. Do not create, fork, or move tasks without a user request.
7. Verify the affected boundary. Require live evidence only for a live outcome.
   Repeat passed checks only for relevant changes or unresolved risk. Diagnose
   unchanged failures before retrying; infrastructure failure is not a code defect.
8. Mark prior claims `confirmed`, `stale`, `contradicted`, or `unverified`.
   Continue only when the next safe action and mutation boundary are clear.

When a structured batch decision is useful, read
[installed command resolution](../../references/platform-commands.md) and run
`groundline efficiency batch --input <packet.json> --json`. This optional,
read-only helper does not own execution or authorize mutations.

## Output Contract

Give the current conclusion, decisive evidence or contradiction, any actionable
gap, and next safe action. Include phase, preflight depth, or Goal status only
when relevant. Skill guidelines do not override user intent or higher-priority
permissions; identify the exact instruction if it causes a pause.

Never claim completion solely because another agent did.
