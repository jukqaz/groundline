# Bounded delegation and task continuity

Use this reference when a requested workflow improvement needs scope decisions,
lane context, long-task recovery, verification reuse, or large-output handling.
It is not a mandatory plan, ledger, review cohort, or new execution layer. Keep
native Codex in charge and preserve the user's settings and scoped authority.

## Resolve shared and project guidance

Keep project-specific behavior, technical constraints, and acceptance criteria in
that project's local guidance. Shared guidance must not turn one project's
choices into defaults for other projects.

Use the instruction chain supplied by native Codex. Inspect relevant scoped
guidance when needed; preserve its owning layer instead of collecting or merging
every instruction file. Product features, architecture, compatibility, test
commands, release policy, and presentation conventions belong to the project.
GroundLine command formats and installation rules apply to GroundLine operations.

Read a matching skill's entry before following it, then load only the reference
needed for the selected operation. A link makes detail available; it does not
require loading every linked document or invoking every available skill.

## Keep the accepted change bounded

Before changing user-visible behavior, establish the current behavior, intended
behavior, and behavior to preserve from available requirements and evidence.
Do not expand the request into extra features, settings, or technical uniformity.
A clear small change needs no separate plan or redundant confirmation. A
dependency on another project does not authorize editing that project.

Judge new findings by reproduction, user impact, and relation to the accepted
outcome. Distinguish defects from improvements and whether they block acceptance.
Finish required authorized fixes and checks, including every in-scope item in an
explicit fix-all request. Defer only optional work that was not requested and
does not block acceptance; do not defer required work for convenience.

Base cleanup and dependency replacement on actual use, current supported inputs,
public contracts, data retention, the concrete problem, and verification evidence.
Names such as legacy, fallback, or compatibility do not establish dead code.
Preserve protections still needed by current users; retire support only when the
evidence establishes that it is outside the current contract.

## Give a lane the context it needs

A short prompt can still fork the full conversation. Inspect the active spawn
interface before choosing context controls. Where it exposes `fork_turns`, use
`none` for an independent task whose necessary context fits in the brief, or
`all` when the work needs inherited history and those values are supported.
Other clients may expose different controls or none; do not invent a field,
unsupported history option or unobserved context reduction.

Include the goal, worktree and base revision or dirty fingerprint, owned files
or output, constraints, acceptance check, and necessary interfaces, decisions
and evidence paths. Narrowing history must not
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

Choose checks from the project's requirements, changed behavior and actual risk.
Use its existing verification path when suitable. A command proves only the
outcome it checks; add runtime or user acceptance evidence when that outcome is
required. Shared guidance does not prescribe a framework, test-first policy,
full-suite run or fixed report shape for every change.

Keep the result tied to the checked revision or artifact, relevant inputs and
runtime/environment. Identify the actual binary/artifact checked; a PASS from a
different binary cannot qualify changed source. Reuse a passing check while
those remain applicable.
Compare current content with the checked bytes before treating verification as
pending. Uncommitted changes can already be verified; a commit or clean working
tree does not by itself prove verification. Keep explicitly requested Git work
as a separate completion criterion.
When conditions affect the outcome, keep them with the result. Normal-use and
stress scenarios are distinct evidence; a pass in one does not erase a failure
in the other. Changing timing or inputs does not by itself establish a fix.
Rerun the affected check when its inputs change or a concrete unresolved risk
invalidates it; a new message alone does not invalidate evidence. For a correction,
review the changed diff and affected findings before repeating broader review.
Do not cap necessary implementation or verification to meet a savings target.
Once required acceptance checks pass, close the accepted scope; optional new
findings do not restart completed reviews.

When a check fails, read the actual error and distinguish invocation, environment
and product failures. Trace the relevant cause and test a focused hypothesis
with the smallest credible check. Retry after a changed condition or under a
bounded transient-retry policy; preserve unresolved evidence instead of layering
speculative fixes or weakening acceptance checks.

## Keep large output inspectable

First narrow the search/read range or aggregate with native tools. When a large
batch returns through one orchestration response, budget the combined output,
not just each nested tool's limit. Select fields or bound excerpts before
printing; a collection of individually bounded results can still overflow its
shared response. Inspect every result, including rejected calls and nonzero
exits, before producing the compact readout. Keep an explicit count and scope
for omitted detail; do not present a clipped batch as a complete inspection.

A line limit is not an output-size limit. A JSON document, minified source or
log record can occupy one very long line. Check file sizes before previewing
unfamiliar data. Use a bounded byte preview for an unknown format, then parse
structured data locally and print only the required fields or aggregates.
Do not use `cat`, a line-range preview, or a display truncation option as the
size guard for a large structured document. Parsing an entire local file is
different from returning that entire file through a tool result.

When a failed or truncated result needs later inspection, return status/exit code,
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

For multi-step completion, connect each delivered result to what its evidence
proves and the remaining required proof or next action and owner. Keep evidence
for distinct required outcomes separate; a passed check does not establish an
unobserved outcome. Claim only the requested outcomes actually established;
small tasks need no extra document, template, or empty fields.
Keep completion reports within the accepted target. Include separately delegated
project work when the user requests it or when it is an actual dependency of the
accepted outcome; unrelated work does not make the requested project incomplete.

Keep discovery, enabled/trusted state, actual invocation and requested outcome
separate under [capability routing](capability-routing.md). Output bytes saved
are not whole-task tokens, price estimates or subscription quota. For a requested
comparison, use [usage assessment](usage-assessment.md), include retrieval,
retries and all owned child/integration cost, and preserve unknowns. An instruction
or synthetic check does not prove live quality, activation or savings.
