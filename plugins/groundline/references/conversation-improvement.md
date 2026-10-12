# Improve guidance from actual conversation sequences

Use this route when the owner requests conversation-pattern improvement or an
existing weekly automation authorizes analysis, scoped changes, and follow-up.
Native Codex interprets the conversation. GroundLine owns the private records,
scope and evidence validation, actual-change checks, and follow-up readout. Its
offline audit provides supporting observations. Core does not call another
model, schedule itself, or infer semantic failures from aggregate counters.

An assessment-only request stays read-only. A workflow improvement request covers
routine reversible edits inside its stated boundary after the concrete diff and
appropriate verification. It does not authorize a new schedule, collection,
hook, credential change, deployment, publication, or broader permissions.

## Read one relevant sequence

Reuse the current audit or saved result before collecting again. Read only the
selected native chat units needed to answer the question, including the earlier
user requirement, the assistant's response/action and relevant tool result, the
user's correction or acceptance, and the assistant's recovery when available.
Use the same task's criterion, delivery, and completion evidence if present.
Record the observation window, sample selection, source references, and missing
context. Include successful work or a counterexample when it can test the
candidate; sampling only short corrections cannot describe overall behavior.

Separate actual authored requests and annotation comments from quoted source,
question wording, host instructions, and automation envelopes. Treat uncertain
origin or incomplete context as unknown. A short instruction, repeated message,
"again", or request for more improvement is not by itself failure or rework.
Aggregate audit signals only help locate a sequence worth reading.

For each relevant sequence, state a contextual interpretation and its evidence:

| Interpretation | Evidence needed |
| --- | --- |
| Assistant error | The preceding action violated an existing requirement or made a demonstrably unsupported claim |
| Requirement change | The user introduced or changed a completion criterion after the earlier work |
| Direction change | The user chose another approach or outcome within the ongoing task |
| Exploratory iteration | The exchange intentionally tried or refined alternatives without evidence of a violated criterion |
| Unknown | The applicable criterion, action, cause, or outcome is missing or ambiguous |

Distinguish what was observed from an inferred cause and what remains unknown.
An exchange can contain more than one reason; keep distinct reasons attached to
their evidence rather than forcing one label for the entire chat. Do not turn
requirements changes or exploratory work into assistant-error counts. Verified
completion still needs actual outcome evidence, not the label or a terminal
event. Existing learning correction kinds are a narrower record contract; do
not silently map exploratory iteration into acceptance or assistant error.

Keep the private analysis local. Store minimal derived observations and source
references through the installed Core's existing learning record store outside
public Git. Do not create an automation-specific ledger or copy this workflow
into a scheduler prompt.
Read native history without rewriting it. Do not persist or upload raw
transcripts, prompts, secrets, or selected source quotations as a reusable
profile. Local analysis works even when Insights delivery is unavailable; upload
consent remains independent. Native memory changes need the owner's request.

## Choose the target before writing the rule

Choose one bounded candidate addressing an evidenced friction. Identify the
failure mechanism, the smallest useful behavior change, its trigger, exclusions,
and an observable acceptance check. Check the relevant current guidance or skill
for an existing owner of that rule; amend it instead of adding a duplicate layer.
Do not invent a permanent rule for a one-off missing fact or requirement change.

| Scope | Appropriate target |
| --- | --- |
| Project-specific | The closest applicable project `AGENTS.md`, project skill, or project reference |
| Common ordinary-work default | A project-neutral rule in the applicable native `AGENTS.md` layer |
| Common task-specific procedure | A general-purpose skill whose existing domain matches the pattern |
| GroundLine product | Its source, parser, command, or workflow reference when the defect is in the product |

A common candidate needs an explicit transferability rationale: explain which
decision remains useful after removing project names, stack/API choices,
endpoints, fixed device or OS matrices, timings, release rules, and local
permission assumptions. Test a materially different project or counterexample.
Repeated observations in several similar projects alone do not establish a
global policy. If the rationale fails or evidence is insufficient, retain the
candidate in the original project scope or defer it. Preserve project parameters
and examples in their local target; do not erase them or carry them into common
guidance. User preferences and authority still remain scoped to what was said.

Choose the exposure path with the target: identify which applicable instruction
layer ordinary work reads, or which matching request invokes the skill and its
reference. Transferability does not load a rule into future work. For an
ordinary-work default, amend the applicable `AGENTS.md` within the authorized
scope and verify native loading in a new task. Do not broaden a specialist
skill's trigger to every implementation or rely on installing its reference as
automatic exposure. Keep procedural detail in its owning skill/reference;
default guidance needs only the useful neutral principle. Verify exact target
loading separately from the later assistant's behavior.

For a skill, keep its entrypoint focused on the real trigger and essential
constraints, with detailed procedures in its owning reference. Check nearby
instructions for conflict and remove superseded duplication within the approved
scope. Consult current official Codex guidance when capability or instruction
semantics are in question; a dated external workflow is inspiration, not proof
that its commands or permission assumptions fit this runtime.

## Apply a concrete candidate and record what ran

Use the product-owned route even for retrospective exchanges without a direct
delivery. Resolve the installed executable with
[platform commands](platform-commands.md), then reuse one owner-private learning
state directory for all stages. If absent, create that authorized owner-private
directory with mode `0700` before reading status. These explicit operations do
not enable a background profile, hooks or Insights collection.

```console
groundline learning status --state learning-state
groundline learning review-conversation --input review.json --state learning-state --criterion-evidence criterion.json --context-evidence context.json --assistant-evidence assistant-action.json --correction-evidence user-correction.json
groundline learning capture-conversation-target --input capture.json --state learning-state --target owning-guidance.md --before-output before.md --owner-project-root project-root
# Apply the reviewed target diff only after the capture succeeds.
groundline learning record-conversation-change --input change.json --state learning-state --target owning-guidance.md --before before.md --verification-evidence verification.json --comparison-context-evidence comparison-context.json --owner-project-root project-root
groundline learning observe-conversation --input activation-or-followup.json --state learning-state --evidence observation.json --comparison-context-evidence comparison-context.json
```

Native Codex authors the small input and derived evidence files in the task's
authorized private artifact location. The owner does not fill forms or JSON.
Core checks the exact evidence bytes before immutable publication. Native work
and feedback references are bounded digests, not copied conversations.

- `groundline-learning-conversation-review-input` binds `pattern_id`, stable
  `feedback_ref_sha256` and `source_work_sha256`, contextual
  `interpretation`, `observed_scope`, `project_dependent`, criterion digest/version,
  original assistant-action and own-correction digests, context evidence and
  observed time. An error can lead to a change; ordinary iteration and new
  requirements remain `no_change`, and unknown interpretation needs more context.
- `groundline-learning-conversation-capture-input` contains `schema: 1`, the
  returned `review_sha256`, `scope`, `target_id` and `target_kind` (`skill` or
  `agents_block`). Run it against the actual owning target **before editing**.
  Core creates the immutable snapshot and private before artifact from that
  file; it derives the before digest, target/root binding and capture time.
  Caller-authored snapshots or arbitrary before files are not actual-change
  proof. Keep the returned `snapshot_sha256`, `before_sha256` and
  `target_binding_sha256` for the change input.
- `groundline-learning-conversation-change-input` binds the returned review and
  mandatory `snapshot_sha256` and `target_binding_sha256`, exact target
  identity/kind/scope and captured before/current after bytes, verification evidence,
  application time and comparison context. Project scope uses the SHA-256 of the
  normalized absolute owning project root; the actual target must remain within
  that root. Common changes require a positive transferability review and
  `--scope-review-evidence`; project-dependent requirements cannot be promoted.
- `groundline-learning-conversation-activation-input` binds the change digest,
  exact after revision, comparison context, evidence and actual activation time.
  A skill needs `skill_invoked`; an instruction block needs `instruction_read`.
- `groundline-learning-conversation-observation-input` binds the change and actual
  natural work/revision, same criterion/version/context, after-target revision,
  nullable opportunity and recurrence, evidence and observation time. Also pass
  `--criterion-evidence` and `--source-revision-evidence`; a recurrence needs
  `--recurrence-evidence`. Different work conditions stay inconclusive. Unknown
  relevant conditions require an unassessed observation, not a clean result.
  `source_revision_sha256` is provenance only; comparison binds the target,
  criterion/version and comparison context. Include a relevant source revision
  in the comparison context when it affects the acceptance conditions.

Use `learning status`'s `conversation_work` and readiness readout to select the
next required action. Repeating the same input retains its immutable record;
reusing one feedback or work reference for conflicting observations is rejected.
Applying bytes, activating instructions, and observing later work are separate
records. A capture is mandatory before mutation, and a change must reference the
same review, scope, target, binding and captured bytes in capture-to-apply order.
These local bindings do not authenticate semantic claims or provider
activation. None requires a fabricated receipt, acceptance, task start or cost.

Core moves completed chains to the existing private archive when the active
window needs cooling. It retains the latest 32 completed review groups and all
unresolved or shared-reference chains, restores original references for retries
and later observations, and preserves their first record clock. The archive
readout reports active and referenced archive counts with historical coverage
incomplete; it is not a lifetime population total. The existing hard limit still
applies to an oversized unresolved closure. Do not delete state, discard pending
work, or create a scheduler-owned ledger to make that limit disappear.

Prepare an actual target diff and record its baseline/version, scope rationale,
supporting sequence references, acceptance and falsification criteria, rollback
boundary, and the next natural follow-up opportunity. Separate criteria for the
edit being valid from criteria for user-visible behavior improving. Native Codex
authors this record; the owner need not maintain JSON or a rating form.

Use the requested target and current permissions. Routine in-scope reversible
guidance or skill edits need no new permission gate; a new authority or writable
boundary uses the native approval path. Preserve user WIP. Existing managed
`learning propose` and `environment plan/apply` operations retain their explicit
trial authorization and recovery contracts; this route does not bypass them.
The direct delivery comparison remains an optional separate route when genuine
outcome and resource evidence exists. The conversation route itself does not
depend on that bridge. An older installation missing these commands needs the
reviewed Core update; report the missing product stage rather than claiming an
automation-owned fallback completed it. Declared
scope metadata cannot authenticate the conversation, prove common text is
project-neutral, or establish ownership of the actual target file.

Verify the changed instruction's boundaries and relevant source/skill checks.
Keep structural validation distinct from behavior tests. Record the actual
applied time, target digest or version, exact diff, verification, and where it is
active. A source edit is separate from packaging, installation, and activation;
a discovered or installed skill is separate from invocation. Keep unobserved
activation explicit. Do not claim that a planned candidate was applied.

## Evaluate natural follow-up work

On the next authorized review, revisit outstanding candidates before proposing
another change. Reuse recorded decisions and skip unchanged analysis. Use later
work metadata to select bounded source units after the recorded application,
then read the actual relevant requirement, assistant action/tool result, and
feedback or recovery. Metadata alone cannot establish an opportunity, observed
behavior, or absence of recurrence. Perform this contextual review even when
exact target exposure or model comparability is unknown.

From that context, distinguish a present, absent, or unknown trigger and whether
the recorded criterion was met, violated, or remains unknown. Separately check
whether the applicable exact target was read/invoked and whether the criterion,
model, runtime, tools and permissions permit comparison. Missing exposure or
different conditions limit the effect claim; they do not erase an observed
trigger or justify skipping content review. Keep these derived findings and
bounded source references in the existing observation evidence, without copied
transcripts or a parallel ledger.

Use the existing nullable observation fields according to that evidence: an
unknown trigger stays `opportunity: null`; a contextually absent trigger is
`false`; a present trigger is `true`. Unknown recurrence stays `recurred: null`.
A recurrence claim still needs the contract's actual own-correction evidence;
observed criterion behavior and recorded recurrence are distinct. Preserve the
actual comparison conditions rather than copying an earlier context merely to
make the record comparable. Include counterexamples, regressions, and owned
analysis costs when observed; unknown costs remain unknown. Do not replay
completed tasks or deliberately create user corrections to manufacture evidence.

For each candidate, record relevant opportunities separately from observed
recurrences, user corrections, and acceptance/quality evidence. An absent or
unknown trigger, uncertain activation, or unmatched conditions leave the effect
pending or inconclusive; unknown evidence does not count as a success.
Zero recurrences without a known opportunity count does not establish success.
Changed requirements, runtime, tools, permissions, model, or evaluation criteria
limit comparability; do not silently pool those results. A clean later sample is
bounded evidence of no observed regression, not proof of causation or savings.

Retain, revise, hold, or roll back within existing authority according to the
recorded evidence. Report the concrete behavior changed and the later outcome,
alongside unresolved lanes. Retain the conversation chain in the product's
existing learning state. Use [the learning loop](learning-loop.md)'s direct
comparisons and managed decisions only when those extra evidence contracts apply.
The weekly readout links observation, exact applied change, and follow-up status
even when the effect is still pending. Usage totals alone do not close this loop.
