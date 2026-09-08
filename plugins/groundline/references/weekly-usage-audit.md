# Weekly local audit

Use `groundline audit weekly --days 7 --json` for aggregate local evidence.
Review counts, coverage, failure reason codes, and the proposed single workflow
change. Keep source validation, installed runtime validation, and user-visible
behavior as separate evidence lanes.

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

Candidate recency has no upper bound: continuing a task after the audit end
must not remove its earlier events. Record timestamps define the requested
window. Selection uses the newest available update or recency timestamp, so
stale sidebar ordering does not hide active turns. Standalone native thread
totals and UI totals have independent checkpoints; valid native totals take
precedence without adding the streams. Unanchored trailing response usage,
missing cross-source baselines, and selected-source resets inside the window
remain incomplete. Resets before the window do not invalidate later baselines.

Diagnostics keep at most 32 examples with exact total/omitted counts. Parser
budgets are one million records and 4 MiB per record. Windowed metric records
without usable timestamps are incomplete. Known inherited prefixes remain a
scope caveat while complete owned suffixes may be collected; unknown boundaries
are a true collection blocker. No raw diagnostic input is exported.
