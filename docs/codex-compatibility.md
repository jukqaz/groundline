# Codex compatibility

GroundLine follows observable Codex contracts rather than maintaining a second
model catalog, permission policy, context manager, or plugin updater. A newer
Codex version does not by itself require a GroundLine release. Supported hosts
are Apple Silicon macOS (ARM64) and Linux on ARM64 and x86-64. Historical Windows observations remain
readable without Windows runtime support.

## Boundaries that change independently

| Surface | GroundLine behavior | Recheck when |
| --- | --- | --- |
| Models and effort | Use the active runtime catalog for advice; preserve dynamic public IDs and opaque private identities in observations | Native metadata, naming grammar, or attribution changes |
| State database | Select the highest numeric `state_<n>.sqlite`, validate columns, open read-only | Required thread columns change |
| Rollout storage | Read plain or Zstandard JSONL in memory with file, decoder-window, and invocation limits | Physical format changes |
| Task lifecycle | Weekly sample requires the latest turn to complete; activity includes resumed work | Lifecycle events change |
| Usage | Prefer cumulative deltas for standalone histories; owned response records for shared suffixes | Provider usage semantics change |
| Plugin upgrade | Codex manages packages at the installer's reviewed exact Git commit | Provider source or upgrade interface changes |
| Permissions and context | Preserve user settings and use supported native features | Native schema or account availability changes |

Model observation identities and effort labels are defined once in
`crates/groundline-contracts/src/model.rs`. Supported public naming shapes retain
their exact version and snapshot IDs; other valid private IDs become stable,
domain-separated `private-<sha256>` keys without exporting their original names.
Historical family/`other` labels remain historical, and unknown/overflow stay
explicit. Observation labels do not establish account availability or eligibility
for the separate empirical comparison policy. Model-context counts are not token
attribution or billing estimates. See the
[current observation contract](../plugins/groundline-insights/references/insights-contract.md#collection-and-transport).

## Check the runtime you use

Check the App-bundled CLI and PATH CLI independently. Record the actual binary
path and version, then inspect native interfaces:

```console
CODEX="/absolute/path/to/codex"
"$CODEX" --version
"$CODEX" plugin list --json
"$CODEX" plugin marketplace list --json
"$CODEX" --strict-config doctor --summary --no-color --ascii
command -v codex
codex --version
```

See [runtime selection](installation.md#start) and
[exact-commit updates](installation.md#update-an-existing-installation).
Version output does not prove plugin activation or a successful task. Use the
current runtime catalog and available tools for model, effort, and feature
support. Experimental context features remain provider-owned and opt-in;
GroundLine does not enable them or pin global model settings.

## Known evidence limits

- The [official TypeScript SDK](https://github.com/openai/codex/blob/main/sdk/typescript/src/exec.ts) launches CLI subprocesses with `codex_sdk_ts`; its roots and delegated children are `codex_cli` data and remain outside `codex_app` windows.
- Codex may retain imported tasks with an explicit `Claude Code`, `Hermes`,
  `Gemini CLI`, or `Antigravity` originator.
  These are outside GroundLine's Codex-only collection scope, even when the
  native database source is `vscode`. The local audit counts them in
  `non_codex_excluded_rollout_count` without reading their activity, including
  their tokens, changing native task data, or blocking Codex collection. The
  count remains local and does not expand the Insights wire schema. Unknown
  originators still make collection incomplete; this is not multi-provider
  support or a fallback to database-source attribution.
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
  reader. A missing or unsafe source yields `native_activity_unavailable` and
  `ready_to_collect: false`; it does not delete or prevent draining an outbox.
  Source presence alone does not prove schema validity or successful delivery.
- Streaming reads retain only fields used by the audit. Source I/O is bounded
  to 1 GiB decoded per rollout and 8 GiB per invocation; retained audit records
  remain capped at 512 MiB. Other runtimes are rejected after their metadata.
  Unreadable inputs remain visible; successful reads do not prove full coverage.

Native `thread_token_usage` and UI `token_count` totals use independent
checkpoints because their baselines can differ. Valid native thread totals take
precedence; the two sources are never added. A decreasing selected-source total
inside the requested window remains incomplete. A reset before that window does
not invalidate its later baseline. Missing cross-source baseline continuity and
uncovered response suffixes still fail closed. A first owned native response
may prove a new zero baseline only when its cumulative total equals its own
usage and there is no earlier in-window usage to discard.
Shared suffixes never use inherited cumulative totals. Collection completeness
is separate from the intentionally partial view of an unread shared prefix.

Parser work is capped at one million projected records and 4 MiB per projection.
The reader accepts raw native records up to 64 MiB, borrowing unused payload
bodies instead of expanding them. Envelope and payload objects each allow at
most 128 fields; retained values share the 4 MiB budget. The scan and retained
history budgets remain independent. Diagnostics retain at most 32 bounded examples plus total and
omitted counts. A plain/compressed file disappearing during open is retried
once; permission, symlink, ownership, and invalid-data failures never trigger
representation fallback. Historical events stay eligible after later task updates.
Candidate selection uses the newest available update or recency timestamp;
sidebar recency alone cannot exclude a long-running turn.

When accepted dimensions change, deploy the API before collectors. They check
advertised capabilities even with cached credentials; incompatibility preserves
the outbox and reports `api_upgrade_required`. See
[collector verification and retries](insights-operations.md#collector-verification-and-retries).

Synthetic native-store/parser tests establish source behavior. Package builds,
installed artifact checks, a real Codex task, and fresh accepted dashboard data
are separate evidence. See [development](development.md) for source verification.
