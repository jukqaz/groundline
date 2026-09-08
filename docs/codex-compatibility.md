# Codex compatibility

GroundLine follows observable Codex contracts rather than maintaining a second
model catalog, permission policy, context manager, or plugin updater. A newer
Codex version does not by itself require a GroundLine release.

## Boundaries that change independently

| Surface | GroundLine behavior | Recheck when |
| --- | --- | --- |
| Models and effort | Use the active runtime catalog for advice; fixed aggregate labels for telemetry | New family labels would improve analysis |
| State database | Select the highest numeric `state_<n>.sqlite`, validate columns, open read-only | Required thread columns change |
| Rollout storage | Read plain or Zstandard JSONL in memory with file, decoder-window, and invocation limits | Physical format changes |
| Task lifecycle | Weekly sample requires the latest turn to complete; activity includes resumed work | Lifecycle events change |
| Usage | Prefer cumulative deltas for standalone histories; owned response records for shared suffixes | Provider usage semantics change |
| Plugin upgrade | Codex refreshes the registered Git `stable` marketplace | Provider source or upgrade interface changes |
| Permissions and context | Preserve user settings and use supported native features | Native schema or account availability changes |

Model family and effort labels are defined once in
`crates/groundline-contracts/src/model.rs`. Audit normalization, event ingestion,
weekly reports, and comparison validation share them. New or custom IDs use a
bounded `other` label; arbitrary model names cannot become exported dimensions.
Model-context counts are not token attribution or billing estimates.

## September 2026 refresh

[Official model guidance](https://learn.chatgpt.com/docs/models) includes
`gpt-6-astra`; consult the current session or refreshed catalog for access and
supported efforts. No global model or effort is pinned by this change.
[Codex release notes](https://learn.chatgpt.com/docs/changelog) through CLI
0.153.4 describe the bundled Astra picker/default fix, availability-qualified
async questions, compressed shared histories, and marketplace improvements.
The App-bundled CLI and PATH CLI must be checked independently.

Experimental context management remains provider-owned and opt-in, subject to
account and runtime support. GroundLine neither enables it nor duplicates its
notes or context windows. Asynchronous questions are usable only when the tool
is present; user steering retains the original task outcome and constraints.

## Known evidence limits

- Forked/shared-history prefixes are not reconstructed. Native paginated
  histories use explicit ordinal boundaries to count only the local suffix.
  Missing prefixes remain `PARTIAL`; legacy copied histories without a known
  ownership boundary remain excluded. Full history projection is not claimed.
- Shared suffix usage requires matching-thread `token_usage_record` entries,
  deduplicated by response ID within the audit window. Inherited cumulative
  snapshots are excluded. For standalone histories, cumulative deltas take
  precedence, then response records, then last-usage events; sources never add
  on top of one another. Raw response-completion events are not a second bill.
- Response-only totals use `codex-response-usage-records`; heterogeneous totals
  use `codex-mixed-usage-sources`. `fallback_rollout_count` includes response
  records as well as last-usage fallbacks. Source labels are a shared allowlist.
- Symlinked state databases and session roots remain rejected by the local
  reader. Codex supporting a symlinked layout does not establish GroundLine
  support for that layout.
- Streaming reads retain only fields used by the audit. Source I/O is bounded
  to 1 GiB decoded per rollout and 8 GiB per invocation; retained audit records
  remain capped at 512 MiB. Other runtimes are rejected after their metadata.
  Unreadable inputs remain visible; successful reads do not prove full coverage.

Native `thread_token_usage` and UI `token_count` totals use independent
checkpoints because their baselines can differ. Valid native thread totals take
precedence; the two sources are never added. A decreasing selected-source total
inside the requested window remains incomplete. A reset before that window does
not invalidate its later baseline. Missing cross-source baseline continuity and
uncovered response suffixes still fail closed.
Shared suffixes never use inherited cumulative totals. Collection completeness
is separate from the intentionally partial view of an unread shared prefix.

Parser work is capped at one million records and 4 MiB per record, in addition
to reader limits. Diagnostics retain at most 32 bounded examples plus total and
omitted counts. A plain/compressed file disappearing during open is retried
once; permission, symlink, ownership, and invalid-data failures never trigger
representation fallback. Historical events stay eligible after later task updates.
Candidate selection uses the newest available update or recency timestamp;
sidebar recency alone cannot exclude a long-running turn.

When a release adds accepted labels or increases a bounded dimension, upgrade
the Insights API before its collectors. Existing strict APIs can reject the new
payload, and permanent rejection requires operator action. Core stays offline.
Collectors verify advertised ingest capabilities before using even a cached
token. Unknown capabilities report `api_upgrade_required` without dropping the
outbox. Initial collection uses a fixed seven-day lookback; a durable incomplete
window retries at most three times automatically and never advances the cursor.
No consent scope, destination, or collection enablement is changed by a refactor.

Verification uses synthetic plain/compressed and resumed-task fixtures, schema
and event round trips, and an isolated ClickHouse report test with Astra
contexts. Cross-platform package builds and production dashboards still need
their own release evidence.

The audit reader borrows JSON payload slices until a metric needs their content.
Unused records still contribute to coverage and JSON syntax validation, without
allocating their full object trees. Time coverage keeps only its endpoints;
latency and prompt-length percentiles share one sort per sample vector. Runtime
classification reuses the parsed originator, and repeated calls use fixed-size
digest keys. The optional `audit_benchmark` example checks aggregate determinism;
timing is profile- and workload-specific, not a provider cost estimate. Run
`cargo run --release -p groundline-contracts --example audit_benchmark -- metrics`
with `metrics`, `unused-payloads`, or `malformed`. Keep it opt-in, outside routine
CI timing gates; use an OS profiler separately for peak memory.
