# Weekly audit for Codex optimization

A scheduled review uses usage patterns and current Codex information to improve
GroundLine and the owner's Codex environment, not merely to publish usage totals.
For conversation-driven improvement, follow
[conversation improvement](conversation-improvement.md): inspect the previous
change's natural follow-up, reuse one audit, read a bounded set of relevant
user/assistant exchanges, interpret the correction, and implement a scoped
guidance or skill change within existing authority. Use
[the optimization loop](codex-optimization-loop.md) for relevant official updates
and other workflow changes. The native audit below is its evidence layer, not an automatic source,
configuration, or plugin updater. An audit-only request remains audit-only.

## Connect the scheduled review to actual behavior

Use the installed Core's [conversation route](conversation-improvement.md) and
one existing owner-private learning state. Read `learning status` before proposing
another change; its `conversation_work` and readiness identify the next missing
stage. Native Codex interprets the necessary exchanges, submits
`review-conversation`, records actual scoped changes, then submits distinct
activation and natural observations with `observe-conversation`.
The product owns the record, scope, evidence and follow-up contract. Keep the
scheduler prompt to purpose, authorized targets and reporting; do not duplicate
this workflow or maintain an automation-specific improvement ledger.
Usage aggregates are supporting investigation evidence, not semantic labels or
proof that an instruction improved. Follow the owning product reference for
project/common boundaries, unknown opportunities and effect limits.

The scheduled routine itself must perform the bounded contextual follow-up in
that route before selecting another candidate. Use later-work timestamps and
model metadata to locate evidence, then read the actual requirement, assistant
action/tool result and feedback to assess the trigger and criterion behavior.
Check exact target exposure and comparison conditions separately. Missing
invocation or model comparability leaves effect claims inconclusive but does not
replace this content review. Record unavailable context explicitly rather than
reporting no opportunity from metadata alone.

Local native audit, semantic review, source changes, installed activation and
Insights upload are separate paths. A stale central upload does not invalidate a
working local audit. If installed Core lacks the required product commands,
report the missing version/stage and use the reviewed native update path.
Do not claim a scheduler-owned fallback closed that product gap or enable
collection/manufacture delivery evidence to make an optional bridge pass.

Use `groundline audit weekly --days 7 --review --json` for one audit, one
recommendation, and a deterministic Korean readout. Use `report_ko` and `readout`
for the report, preserving the nested `audit` and `recommendation` evidence.
The execution fields distinguish an audit that ran with partial data from a
recommendation failure. Do not run a second audit or recommendation after this
combined command. `groundline audit weekly --days 7 --json` remains the raw audit
interface for callers that explicitly need separate stages.
For a saved full result from the same observation window, use
`groundline audit review --input <private-result.json> --json`.
This reads the saved evidence once, scans no native history, and calculates one
current offline recommendation from its audit. Saved recommendation text/status
is not reused as a current policy decision. The summary includes the freshly
computed `recommendation`, so its action and limits need no second command.
The distinct summary kind is not a full audit: retain its original saved input
and source hash. Reuse preserves the window and unknowns; it does not establish
that current history is unchanged. The window end must be within 24 hours
(`--max-age-hours` may tighten this). Changed scope or a request for current
history requires fresh collection. Execution counters measure this command,
not surrounding agent work, model tokens, or end-to-end savings.
Check the command exit status, `execution.audit_runs`,
`execution.recommendation_runs`, and the nested audit/recommendation statuses
independently. Retain the audit if recommendation fails. Review counts, coverage,
failure reason codes, and the proposed single workflow change. Keep source
validation, installed runtime validation, and user-visible behavior as separate
evidence lanes.

Use the actual App runtime first and resolve its CLI through
[platform commands](platform-commands.md); display names and historical internal
paths do not establish the current executable. Only fall back to PATH CLI when
the App runtime cannot be verified,
and label that evidence scope. Do not treat a CLI feature flag as active App
behavior or a manual invocation as proof of a scheduled trigger.

Resolve the enabled installed version from the active native Codex CLI's
`plugin list --marketplace groundline --json`, then match that exact cache
manifest and executable. Capture provider output privately and emit only the
required safe fields. Do not sort cache directories to guess the active version
or search the entire Codex home. Read only the relevant GroundLine configuration
tables if needed. Run the installed `provider-smoke --require-installed --json`
once before claiming package integrity; a manifest alone proves no live hook or
server receipt. Resolve one active root and read this reference and the matching
`optimize-codex-workflow` route when present. Do not require retired skill names.
Missing required files or commands are
UNVERIFIED, not grounds to substitute a different cache or unpublished binary.

Before execution, choose how the full redacted result will survive tool return:
retain it in a persistent runner or, within an authorized analysis artifact scope,
save it once to a new owner-private file outside the repository. Capture the
result before printing a bounded projection. An exit code or top-level key list
cannot replace the audit's status, coverage and missing-data reasons.

When using the separate-stage interface, run the weekly audit once. Retain its
redacted JSON and pass those same bytes to `groundline efficiency recommend --audit - --json` through
standard input (maximum 2 MiB). Check both commands' exit status independently.
Do not use `/dev/stdin`, write outside the authorized artifact scope, or repeat
the expensive audit merely because a projection discarded its evidence. If recommendation fails,
report the captured audit and the recommendation failure separately. A regular
JSON file remains supported when saving it is authorized.

`groundline efficiency recommend --audit - --json` proposes one
advisory review within the current task and existing scoped authority. Boundary
signals trigger in-place reconciliation, not a new task or mandatory planning
cycle. Compatible user steering preserves completed work. A one-off review does
not request another approval or schedule recurring work.

Long turns, compactions, and high effort do not establish failure. Repeated-call
and nonzero-exit ratios select diagnosis candidates, not proven waste: classify
expected empty search results, normal polling, infrastructure failures, and
actual code failures before changing behavior. Preserve selected model, effort,
and service tier. Compare matched outcomes before recommending a setting change;
an aggregate-only recommendation cannot establish high confidence in benefit.

The command is read-only, performs no network request, and does not emit raw
task content or private paths. The surrounding research is native Codex work,
not functionality hidden inside this command. Reuse the same in-memory audit
for that research; extract its nested `audit` value for raw-audit consumers
without running collection again. A saved result follows the same authorized
private-artifact scope; it is not a new collector, ledger or uploaded transcript.

For the final weekly report, preserve `report_ko` task samples, completed turns,
usage source, observed/selected rollouts, and unresolved reasons; cross-check
`readout`. Add the previous-change disposition, bounded semantic findings, relevant
verified update, and selected improvement's scope, actual application/activation
state, follow-up opportunities, recurrence/unknown counts, acceptance, rollback
and remaining UNVERIFIED lanes. Distinguish GroundLine product defects, shared
neutral principles and project-local requirements.
Do not claim savings from a proposed change, a simulation, or a successful install.

Codex's latest numeric `state_<n>.sqlite` is selected read-only and its thread
columns are checked before use. Plain `.jsonl` and compressed `.jsonl.zst`
representations share one logical identity; audit never materializes or rewrites
them. Streaming projection retains only audit fields. Decoded input is limited
to 1 GiB per rollout and 8 GiB per invocation, with at most 512 MiB of retained
audit records. Other runtimes are excluded after reading their metadata.
These are read budgets, not estimates of disk occupancy or model tokens.
Native user content projection is streamed and bounded to 1,024 blocks and the
per-record byte budget. Excess fails explicitly without truncating or rewriting
the native message; images and unsupported bodies are never emitted.

A weekly sample requires the latest lifecycle event to complete the turn.
Previous completed turns do not make a resumed or interrupted task complete.
An incoming native user message also resumes a thread before `task_started` is
written. `scope.root_selection` and the typed `readout.root_selection` distinguish
indexed candidate rows, readable window candidates and roots excluded because the
latest turn is incomplete. A readable candidate can have unresolved timestamps;
it does not prove a record belongs in the window. These are sequential filters with different units,
not an additive population partition or a full-history denominator. Missing
diagnostics in a saved observation stay unavailable, rather than becoming zero.
For authorized conversation-pattern improvement, also inspect the relevant
owner-selected ongoing exchanges through native history tools when needed;
their feedback can be useful without qualifying as a completed audit sample.
Reuse bounded relevant evidence instead of expanding collection just to reach
the statistical minimum. A small sample limits the aggregate recommendation;
it does not by itself rule out a directly evidenced product defect or correction.
Activity audits include ongoing work, with `completed_root_coverage=false` on
export. Unreadable or unclassified inputs make the result `PARTIAL` and remain
visible as aggregate counts. Weekly analysis reads only the relevant indexed
window, validates native identities, and excludes every ambiguous duplicate in
that selected sample. `coverage.recommendation_evidence_complete` describes
that sample; missing or false values cannot qualify a change recommendation.

Whole-store metadata reconciliation is explicit: `groundline audit store --json`.
It reports unindexed files, stale paths, duplicate identities, unknown archive
states and traversal limits without changing data. It does not decide the
requested window's eligibility or certify an atomic population snapshot.
Weekly `eligible_root_count` and `selection_coverage` remain null because the
full-history denominator was not measured. A known unrelated old missing row
does not invalidate current observations; a missing selected row still does.
Neither sample evidence nor storage diagnostics authorize history resets.

The readout separates parsed-root `collection_issue_count` from store-level
unreadable root/delegated/Guardian counts and unclassified origins. A zero parser
issue count does not mean files excluded before parsing were readable. Do not
sum counts with different denominators or turn missing scope counters into zero.

Standalone histories prefer cumulative window deltas, then matching-thread
response records, then last-usage events. These sources never add on top of
one another. Native paginated shared histories use explicit ordinal boundaries
and unique response IDs to count only locally owned suffix usage, never parent
cumulative totals. The unread parent prefix remains `PARTIAL`; legacy copied
histories without a known ownership boundary remain excluded. Do not claim full
fork or subagent coverage. Response records count as fallback rollouts and have
an explicit bounded provenance label, separate from last-usage-only evidence.

Model contexts use bounded family and effort labels. New GPT-6 observations
use gpt-6.1-sol, gpt-6-astra, gpt-6-sol, and gpt-6-luna; the two Sol versions
remain separate. Historical unversioned labels remain descriptive and outside
optimization scope. Do not assign previously collapsed Sol labels to 6.1 or pool
observations across changed normalization rules as evidence of an improvement. They do
not attribute token totals to individual models or estimate billing.

Report completed root tasks and completed turns separately. `task_latency` counts
completed turns; Guardian `review_count` is a completed review-turn proxy, not a
count of approvals. Show usage coverage as observed rollouts / selected rollouts
and retain the source label. Input includes cached input; reasoning output is a
subset of output. Do not add these subsets again or treat token totals as bills.
Zero observed prompt text leaves prompt-shape ratios unavailable, not proof of
zero user instructions. Guardian outcome, risk, effort, and workspace attribution
remain unavailable unless their explicit availability fields establish otherwise.

`activity.user_message_events` retains the native event count.
Native `response_item` user messages are also inspected; source counts and
representation duplicates are separate from unique input and extracted requests.
Exact cross-source copies within a rollout-local input burst count once. Repeated
requests from one source and later turns stay separate; this is representation
matching, not authenticated user identity or semantic deduplication.
`user_messages_with_text` and `prompt_shape` measure only locally extracted
conversation text: literal requests, annotation comments plus their own request,
question answers, and the own-request section of file mentions. Recognized
automation and context envelopes are counted separately, not as user requests.
Question wording, selected source quotes, and host instructions are excluded.
`prompt_shape.input_classification` records revision 3, bounded category and source
counts, and `complete` for observed native inputs; an empty input sample is not
complete. Malformed recognized envelopes, unsupported blocks and unavailable text remain
explicitly unobserved. This is envelope classification, not authenticated author
identity. Scope terms are lexical hints, not semantic scope or rework evidence.
`coverage_observation_complete` separately reports known extraction holes; their
typed `coverage_issue_counts` mark the component PARTIAL without inventing parser
errors. A context-only or empty native sample has no such hole but still has no
usable observed request text, so `complete` remains false.
The recommendation reports prompt classification as complete, incomplete, or
unknown and withholds short-message ratios unless classification is complete.
Do not pool older whole-envelope statistics with normalized text as improvement
evidence, or revision-1 event-only counts with message coverage. Revision 3
recognizes the current tagged ambient-browser block between response annotations
and their own-request section; browser state remains excluded. Revision-2 saved
observations remain unchanged and may retain a hole for that composition.
Saved reports and native history remain unchanged. Read
`readout.native_user_input_sources` alongside selection diagnostics: input zero
can reflect envelope exclusions, missing supported text or a completed-only
sample, and does not mean the owner gave no instructions.

`provider_reported_usage.token_field_availability` distinguishes source-observed
components from partial numeric accumulators. A missing/null/invalid source field
stays unavailable through selected-rollout sums and both endpoints of a window
delta. Numeric zero alone does not establish an observed split; absent metadata
is not backfilled.

Verification outcomes use native exit/status metadata. Test names or stdout words
such as `timeout` and `rejected` are not failures. Running, missing, and unrecognized
results stay unresolved; orchestrator completion does not prove nested command
success. Verification classification inspects supported literal command positions,
not test-command words quoted inside a search or inspection. Unsupported dynamic
or compound shell/JavaScript shapes remain `other_command`; preserve
`tools.unclassified_command_call_count` and `tools.verification_classification_scope`
as classification coverage limits, not proof that no verification occurred.
`tools.command_diagnostics_revision: 1` adds bounded `other_command_reasons`
without changing category membership or native verification outcomes:

| Reason | Meaning |
| --- | --- |
| `parsed_exec_without_verification` | A supported literal exec wrapper contains no recognized verification; it may contain recognized inspection or poll calls |
| `unclassified_nested_command` | A supported exec wrapper contains an unsupported nested command and no recognized verification |
| `unsupported_exec_shape` | The exec wrapper cannot be mapped by the bounded literal AST parser |
| `other_direct_command` | A direct call remains an unclassified command |

Count one reason per top-level `other_command` call, not per nested command.
The reason counts sum to `tools.unclassified_command_call_count`; do not add
them to that total. Korean review and `readout` retain these diagnostics only
when the revision, keys, integer counts and sum agree. Older saved audits lack
this observation and display unknown diagnostics, never invented zeroes.
Unsafe keys or nonnumeric values are rejected before a saved audit is retained
in output; numeric inconsistencies remain unknown in the readout.
Diagnostics do not prove a check ran or succeeded, recover unsupported dynamic
execution, or measure accepted deliveries. No raw command, path, handle or source
text is exported. Compare outcomes only under the same classifier and collection
conditions; this diagnostic revision alone is not a quality improvement.

Report outcome coverage before interpreting success ratios. Separate
calls to poll a process are connected only with a matching explicit handle in the
same rollout and observation window. Literal awaited native polls can be recognized
in straight-line exec wrappers, including multiple top-level text emissions.
Each result must match its emission slot; unobserved calls and unmatched shapes
cannot establish success. Dynamic, interactive, conditional, or ambiguous polls
stay uncorrelated. Inspect `verification_unresolved_reasons`; its sum equals
the unresolved count. `verification_recovered_by_poll_count` identifies recovered
terminal results without counting polls as new verifications. Missing source
usage and unsupported ownership boundaries remain separate usage reason counts.
Central aggregates
from collectors before 0.25.6 used textual heuristics: do not compare their failure
or verification ratios directly with current metadata-based results. Local audits
recompute their requested window with the installed parser; they do not repair or
delete historical server events.

Candidate recency has no upper bound: continuing a task after the audit end
must not remove its earlier events. Record timestamps define the requested
window. Selection uses the newest available update or recency timestamp, so
stale sidebar ordering does not hide active turns. Standalone native thread
totals and UI totals have independent checkpoints; valid native totals take
precedence without adding the streams. Unanchored trailing response usage,
missing cross-source baselines, and selected-source resets inside the window
remain incomplete. Resets before the window do not invalidate later baselines.
A first owned native response can anchor a new zero counter only when its total
equals its own usage and no earlier in-window usage would be discarded.

Diagnostics keep at most 32 examples with exact total/omitted counts. Parser
budgets are one million records and 4 MiB per projected record. Raw native
records allow 64 MiB, while unused nested bodies stay borrowed and each envelope
or payload object is bounded to 128 fields. Windowed metric records
without usable timestamps are incomplete. Known inherited prefixes remain a
scope caveat while complete owned suffixes may be collected; unknown boundaries
are a true collection blocker. No raw diagnostic input is exported.
Use `scope_exclusion_count` for known excluded prefixes and
`collection_issue_count` for actual unknown/incomplete evidence. Neither is proof
of database corruption. Keep conservative recommendation gating for partial data;
do not delete history or reset collection state to manufacture a complete sample.
