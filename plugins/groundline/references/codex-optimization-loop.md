# Codex optimization loop

GroundLine uses usage patterns and current Codex knowledge to improve its own
guidance/software and the owner's Codex environment. The objective is better
verified deliveries for the owner's quality, token, and time preferences, not
the fewest tool calls or the most enabled features. Native Codex remains the
executor. Core is an offline evidence and trial engine; it does not research
the web, activate instructions, or create a second scheduler/model router.

Optimization applies only to GPT-6 Astra, Sol, and Luna. Use
[model/effort and adaptive delegation](model-effort-routing.md) for task-level
selection. Earlier or unversioned cohorts remain historical evidence, not tuning
targets. Mixed/unknown root generations receive no aggregate optimization
candidate; collect appropriate future evidence without rewriting prior data.

## Review before collecting again

Start with the last candidate or trial and its authorized scope. Report retained,
reverted, inconclusive, not applied, or awaiting evidence only when supported by
the relevant records. No prior trial is a valid first-run state. Do not invent
an outcome because a week elapsed. If a trial is active, evaluate it before
proposing another confounded intervention.

For a weekly run, follow [the weekly audit contract](weekly-usage-audit.md) once
and retain its full redacted result before emitting a smaller projection. Reuse that result for recommendations,
research, and supported counterfactuals. Reuse available Insights reports when
their scope is adequate; fetch only missing authorized windows needed for the
question. Never run another audit because research revealed a new idea or an
earlier output was lost. Keep missing history, source ownership, and collector
health separate. A partial audit can expose a reproducible source defect even
when it cannot qualify a personal trial.

For authorized remote analysis, identify the blocked layer: edge/SSO login,
application/API authorization, or database access. A sign-in page establishes
pending authentication, not a rejected database credential. Complete the normal
existing sign-in flow within the user's requested destination; stop for an
actual denial, unavailable credential or required user challenge. Do not create
credentials, broaden access or bypass a gate. Confirm a successful data query
before reporting access restored. Browser dashboard observations remain distinct
from a strict owner report; preserve filters, timestamps and completeness.

Classify patterns using evidence appropriate to the claim: repeated reading
with unchanged inputs, oversized irrelevant results, premature handoffs,
avoidable serial waits, missed acceptance checks, or skill-selection conflicts.
Aggregate ratios identify candidates; they do not prove any of these causes.
Long tasks, high effort, compactions, and valid approvals are not waste by
themselves. Do not upload raw conversations for diagnosis.

Before proposing a model change, distinguish aggregate root/delegated token
shares from per-delivery owned resources. A high aggregate root share flags root
context and integration for investigation; only matched delivery receipts can
establish a task-level bottleneck or savings. High cached input is not proof of
waste or permission to reset context. Low repeated-call or failure-signal ratios
do not establish that retry suppression is the main saving, either; inspect
their definitions, denominator and actual calls before prioritizing that change.
Keep collector purpose (`production`/`verification`/`unclassified`) separate from
task kind and phase. It is an explicit declaration for future collection windows,
not an inferred classification of conversations. Never relabel it from task text.
Derive only relevant task/phase and acceptance notes from authorized local
conversations. Missing outcome evidence limits measured comparisons; it does not
require the user to manually label every prompt or prevent useful native task
judgment. Report a concrete behavior change,
its acceptance check, and whether it was applied, merely prepared, or measured.

## Research only relevant changes

Compare actual client/plugin versions and previously checked sources, then
fetch current official Codex documentation for changes relevant to observed
work. Verify the affected pages, not search snippets; record their URL, checked
date, and applicable version privately. An unavailable page stays UNVERIFIED.
Recheck affected guidance after a model/client change, without rereading every
installed skill or researching unrelated features every week. Research/model
calls also consume resources; avoid duplicate lookups and unrequested paid evals.

Keep these capability claims separate:

1. Documented by the provider.
2. Present in the actual App-bundled CLI or host tool inventory.
3. Available to this account and task, with required authentication.
4. Authorized for this operation.
5. Observed running correctly on the intended surface.

A feature flag or stored hook approval proves neither tool availability nor
activation. CLI projections do not prove the active App prompt. API parameters
are not Codex configuration. Do not enable experiments, broaden permissions,
change hook trust, or force a model merely to increase feature use.

## Choose the useful Codex surface

| Evidenced opportunity | Candidate GroundLine customization | Acceptance evidence |
| --- | --- | --- |
| Eager, irrelevant context or conflicting instructions | Narrow skill triggers and conditional references; task-local context selection | Required evidence still loaded; verified outcomes and owned tokens compared |
| Repeated valid investigation or verification | Reuse evidence tied to unchanged inputs, environment, scope, and risk | Reuse remains valid; changed inputs trigger affected revalidation |
| Independent reads performed serially | Bounded native concurrent reads; GPT-6 agents under an active delegation request or standing policy | Correct per-call results; elapsed time compared without losing coverage |
| Long native operations repeatedly polled without need | Supported async execution/wait mechanism with explicit result correlation | Completion captured and failures handled; no invented terminal success |
| Missing relevant tool/skill or repeated setup | Task-scoped discovery, authenticated MCP, or native worktree setup where justified | Actual invocation/setup succeeds; context/setup cost included |
| Delayed review or follow-up | Appropriate native review or user-requested automation | Requested result verified; scheduled triggers distinguished from manual calls |

These are review surfaces, not auto-applied recipes or newly implemented CLI
controls. Research availability locally; respect the owner's delegation policy.
Count feature opportunities only when eligibility was directly observed. A
missing denominator is unknown, not zero utilization.

For a bounded, feature-aware review, use the conditional
[capability routing matrix](capability-routing.md). Classify only the requested
conversation window and relevant work items; do not inventory every Codex
surface on every turn. Missing or unreadable history means UNKNOWN, not UNUSED.
Use the live host/account catalog and task tools for availability; stored flags,
provider documentation, and aggregate call counts are not proof of activation.

The offline `groundline efficiency route --input ... --catalog ...
[--audit ...] [--report ...] --json` interface joins conversation-local semantic
eligibility classifications with current capability evidence, audit quality and
coverage, model/effort token attribution, and matched per-delivery outcomes. It
returns matched-outcome replacement candidates and verification needs; without
matched outcomes, native task judgment selects the model and effort. Aggregates
are optional descriptive context; it does not
invent eligibility from aggregate counts, enable features, edit settings, or
change the running root. Native Codex executes an authorized task or lane choice
through controls actually exposed by the host. Follow the
[private evidence contract](evidence-routing.md) for inputs and acceptance checks.

## One actionable improvement

Prefer one high-value, evidenced change rather than filling a recommendation
quota. Preserve the deterministic audit candidate and explain if stronger
source/runtime evidence leads to a different final candidate. "Keep current
workflow; gather the missing evidence" is valid when nothing qualifies.

For the selected candidate give: observed problem and coverage; proposed change
and mechanism; product versus personal ownership; exact mutation boundary;
protected outcomes and primary metric; activation check; acceptance and rollback
conditions. Keep the user-facing report short and redact private identifiers.

- **Product defect or reusable improvement:** patch GroundLine source, skills,
  or tests only within authorized implementation scope. Use the established
  release route; source checks do not update the installed plugin. Report
  source, package, installation, and live behavior independently.
- **Personal preference/configuration:** preview the specific affected surface,
  preserve unrelated choices, and use existing authorized native editing and
  backup/validation paths. No global model pin or blanket permission allow.
- **Measured personal guidance trial:** use the strict
  [personal improvement contract](personal-improvement.md). Failed trial gates
  do not block independently authorized repairs to a demonstrated product defect.

An instruction to review weekly is not ongoing authority to modify source,
settings, plugins, databases, collection, or personal guidance. Apply only within
separately established scope; native activation must be verified after a change.

## Measure the outcome, not the activity

Use a stable requested delivery and its acceptance criteria as the outcome unit.
Include eligible failed and unknown outcomes, not just successful easy tasks.
Compare task kind, scope, model/effort/runtime, tools, permissions, service tier,
and unchanged non-trial instructions. Keep unknown usage null; never attribute a
parent counter to a child or equate mixed counters with billing.

For token/time trials, compare all attributable resources per verified delivery,
alongside verified-outcome rate, rework, and user intervention. Elapsed duration
includes waits; report finer timing only when observed. No verified deliveries
means no per-verified-delivery ratio. Sample gates are not significance tests,
and an observational improvement is not a causal savings guarantee. Audit
simulations are explicit fixed-assumption scenarios, not evidence of benefit.

The current personal evaluator conservatively rejects protected-outcome
regressions. A desired speed/token tradeoff needs an explicitly agreed future
comparison contract; do not silently relax gates. Retain, revise, or restore
the authorized change from actual evidence and carry unresolved evidence into
the next scheduled review without repeating completed work.

Official guidance reviewed 2026-09-28:
- [Astra skills and prompts](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra): concise selection descriptions, conditional references, and task-appropriate persistence.
- [Evaluation design](https://developers.openai.com/api/docs/guides/evaluation-best-practices): use task-specific outcome checks; a hypothetical scenario is not a measured result.
