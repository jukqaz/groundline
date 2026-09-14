# Weekly local audit

Use `groundline audit weekly --days 7 --json` for aggregate local evidence.
Review counts, coverage, failure reason codes, and the proposed single workflow
change. Keep source validation, installed runtime validation, and user-visible
behavior as separate evidence lanes.

Resolve the enabled installed version from the active native Codex CLI's
`plugin list --marketplace groundline --json`, then match that exact cache
manifest and executable. Capture provider output privately and emit only the
required safe fields. Do not sort cache directories to guess the active version
or search the entire Codex home. Read only the relevant GroundLine configuration
tables if needed. Run the installed `provider-smoke --require-installed --json`
once before claiming package integrity; a manifest alone proves no live hook.

Run the weekly audit once. Retain its redacted JSON in process memory and pass
those same bytes to `groundline efficiency recommend --audit - --json` through
standard input (maximum 2 MiB). Check both commands' exit status independently.
Do not use `/dev/stdin`, write an unapproved temporary report, or repeat the
expensive audit because its output was discarded. If recommendation fails,
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
task content or private paths.

Codex's latest numeric `state_<n>.sqlite` is selected read-only and its thread
columns are checked before use. Plain `.jsonl` and compressed `.jsonl.zst`
representations share one logical identity; audit never materializes or rewrites
them. Streaming projection retains only audit fields. Decoded input is limited
to 1 GiB per rollout and 8 GiB per invocation, with at most 512 MiB of retained
audit records. Other runtimes are excluded after reading their metadata.
These are read budgets, not estimates of disk occupancy or model tokens.

A weekly sample requires the latest lifecycle event to complete the turn.
Previous completed turns do not make a resumed or interrupted task complete.
Activity audits include ongoing work, with `completed_root_coverage=false` on
export. Unreadable or unclassified inputs make the result `PARTIAL` and remain
visible as aggregate counts. `selection_coverage` describes selection among
known eligible roots; it does not mean every stored task was readable.

Standalone histories prefer cumulative window deltas, then matching-thread
response records, then last-usage events. These sources never add on top of
one another. Native paginated shared histories use explicit ordinal boundaries
and unique response IDs to count only locally owned suffix usage, never parent
cumulative totals. The unread parent prefix remains `PARTIAL`; legacy copied
histories without a known ownership boundary remain excluded. Do not claim full
fork or subagent coverage. Response records count as fallback rollouts and have
an explicit bounded provenance label, separate from last-usage-only evidence.

Model contexts use bounded family and effort labels, including Astra. They do
not attribute token totals to individual models or estimate billing.

Report completed root tasks and completed turns separately. `task_latency` counts
completed turns; Guardian `review_count` is a completed review-turn proxy, not a
count of approvals. Show usage coverage as observed rollouts / selected rollouts
and retain the source label. Input includes cached input; reasoning output is a
subset of output. Do not add these subsets again or treat token totals as bills.
Zero observed prompt text leaves prompt-shape ratios unavailable, not proof of
zero user instructions. Guardian outcome, risk, effort, and workspace attribution
remain unavailable unless their explicit availability fields establish otherwise.

Verification outcomes use native exit/status metadata. Test names or stdout words
such as `timeout` and `rejected` are not failures. Running, missing, and unrecognized
results stay unresolved; orchestrator completion does not prove nested command
success. Report outcome coverage before interpreting success ratios. Separate
calls to poll a process may remain uncorrelated and unresolved. Central aggregates
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
