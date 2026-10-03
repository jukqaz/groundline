# Bounded delegation and task continuity

Use this reference when a requested workflow improvement needs lane context,
long-task recovery, verification reuse, or large-output handling. It is not a
mandatory plan, ledger, review cohort, or new execution layer. Keep native Codex
in charge and preserve the user's settings and scoped authority.

## Give a lane the context it needs

A short prompt can still fork the full conversation. Inspect the active spawn
interface before choosing context controls. Where it exposes `fork_turns`, use
`none` for an independent task whose necessary context fits in the brief, a
supported recent-turn selection for shared recent decisions, or full history
when the work needs it. Other clients may expose different controls or none;
do not invent a field or claim an unobserved context reduction.

Include the goal, owned files or output, constraints, acceptance check, and
necessary interfaces, decisions and evidence paths. Narrowing history must not
remove the task contract. Reuse an existing agent when its retained context is
relevant; batch similar small tasks when this avoids repeated setup. Otherwise
keep sequential work local. Account for root integration and every child's
work; parallelism can improve time or quality while increasing total tokens.

Return the result, changed revision/files, verification evidence, unresolved
acceptance work and relevant concerns. Keep raw exploration logs out of the
parent context; retain a local reference when a concrete finding needs inspection.
The parent owns requirement checks, conflicting results and final integration.

## Resume long work without restarting it

For several turns, compaction, interruption or external waiting, retain a small
handoff in the task's existing work area when native state alone is insufficient.
The parent updates it; children report evidence instead of concurrently rewriting
shared state. Record the task and accepted changes, current status, completed
evidence, next action/blockers, worktree/HEAD and any pending native handle.
Use artifact references instead of copying the conversation or whole plan.

On resume, reconcile the record with Git state, changed inputs and the actual
operation handle. Retrieve its terminal result and requested output before
starting it again. If the handle or result expired, state that limit and inspect
Git, artifacts and current execution state for duplicate work; rerun only the
still-needed action within the existing authority. A process start, old
acknowledgement or partial artifact is not completion. Create Goals, new chats or recurrence only when explicitly
requested; a handoff does not replace native state or extend authority.

## Reuse valid verification

Keep the result tied to the checked revision or artifact, relevant inputs and
runtime/environment. Reuse a passing check while those remain applicable.
Rerun the affected check when its inputs change or a concrete unresolved risk
invalidates it; a new message alone does not invalidate evidence. For a correction,
review the changed diff and affected findings before repeating broader review.
Do not cap necessary implementation or verification to meet a savings target.

## Keep large output inspectable

First narrow the search/read range or aggregate with native tools. When a large
failed or truncated result needs later inspection, return status/exit code,
essential errors, exact scope and a local original-output reference when available.
Keep that reference retrievable within the task's bounded retention scope; read
the existing result before rerunning an expensive command. If the original is
unavailable, state that limit rather than presenting the summary as complete.

Do not hide failed checks, omitted rows, changed command semantics or debug/ready
signals to shrink output. Preserve the original command's meaning and count
baseline, not a larger output introduced by the summarizer. Do not add a global
shell wrapper, proxy, hook or indexer for this pattern. Keep raw commands/output
and private paths out of Insights and public reports; authorized local inspection
does not authorize transcript upload.

## State what the evidence proves

Keep discovery, enabled/trusted state, actual invocation and requested outcome
separate under [capability routing](capability-routing.md). Output bytes saved
are not whole-task tokens, price estimates or subscription quota. For a requested
comparison, use [usage assessment](usage-assessment.md), include retrieval,
retries and all owned child/integration cost, and preserve unknowns. An instruction
or synthetic check does not prove live quality, activation or savings.
