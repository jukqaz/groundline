---
name: reconcile-current-state
description: Reconcile stale state or uncertain scope before a broad or high-impact change. Skip for a clear next step already supported by current evidence.
---

# Reconcile Current State

## Purpose

Prove the relevant current state and task boundary. Prior reports are hints;
reuse current evidence unless the checkout, host, input, or scope changed.

## Decide the next action

For bounded work, confirm the target, authorization, and relevant check, then
act. For uncertain or high-impact work, collect only evidence that can change
the decision; synthesize it once and settle the scope, mutation boundary,
success criteria, and verification before editing. This is a decision aid,
not an additional approval gate or a required report template.

Reuse established facts. Stop collecting when the next authorized action is
clear. Defer unrelated observations; explicit user steering can revise the plan.

## Workflow

1. Identify the current task, target, source, and request. Check the worktree or
   runtime when it affects the action.
2. Read durable context; avoid broad transcript loading.
   For GroundLine installation/application with existing-setting repair, follow
   [installation alignment](../../references/installation-alignment.md) as part
   of this task. Ordinary project work does not trigger a home-wide audit.
3. Inspect the affected state and targeted diff before editing. Check history,
   worktree attachment, or live systems only if relevant to this decision;
   do not require a full repository audit for a bounded change.
4. Continue authorized implementation through verification. Reviews remain
   read-only. Ask only for missing information or authority that changes the
   next action; complete independent authorized work while waiting.
5. Incorporate corrections in the current task and retain applicable approvals.
   Answer side questions briefly, then resume. Reconcile a changed repository
   or authority boundary without treating it as a request for a new task.
   Create, fork, move tasks, or create a native Goal only when requested.
6. Verify the affected boundary. Require live evidence only for a live outcome.
   Repeat passed checks only for relevant changes or unresolved risk. Diagnose
   unchanged failures before retrying; infrastructure failure is not a code defect.
7. Correct stale claims and make unresolved evidence visible without blocking
   unrelated work. Preserve completed work across steering and compaction.

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
