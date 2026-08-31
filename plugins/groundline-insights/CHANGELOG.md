# Changelog

## v0.20.1 - 2026-08-31

- Document Insights-only and combined install profiles, the current integration
  matrix, owner-operated infrastructure boundary, and unsupported sink classes.
- Make a missing owner policy explicitly disabled so installation and lifecycle
  hooks remain inert until reviewed configuration and `worker enable` complete.
- Validate the complete owner-policy contract, require a valid profile and
  enrollment credential before enablement, and stop silently treating malformed
  outbox or status state as empty.
- Add schema-2 worker readiness output with explicit collection state, readiness,
  freshness and clock-skew checks, and bounded blocker codes instead of reporting
  every readable state as `PASS`.
- Skip the detached worker process entirely while collection is disabled and add
  unit plus native CLI regression coverage for the opt-in boundary.
- Import only the exact previously shipped private policy-v1 and status-v3
  records so an upgrade preserves explicit consent and collection watermarks;
  malformed or unknown state still fails closed.
- Standardize worker error receipts and report mutation as `null` when a failed
  local, enrollment, audit, or upload operation may already have persisted
  private state instead of falsely claiming no mutation occurred.
- Ship a complete, privacy-safe owner-profile example with an intentionally
  invalid placeholder token and execute it through configure/enable in the CLI
  contract harness after test-only substitution.

## v0.20.0 - 2026-08-30

- Move Insights into the public GroundLine monorepo as a distinct installable
  plugin with the `groundline-insights` native executable.
- Require an owner enrollment credential separate from Tailnet reachability,
  collector tokens, the admin token, and the trusted-proxy token.
- Store schema-7 configuration as a sanitized profile plus a separate private
  credential file and expose presence booleans only.
- Remove Hermes runtime compatibility and rename the still-required current
  Codex source fallback metric without legacy terminology.
- Build Core and Insights together across six native targets, publish a signed
  Linux amd64/arm64 API image, and advance one atomic `stable` channel.
- Make the owner-run TrueNAS controller preserve a valid enrollment credential
  and inject the required owner-local credential only for installations that do
  not have one; malformed existing values fail before mutation.
- Fail malformed or version-mismatched tags before the release matrix, wait for
  all native artifacts before publishing the API image, and align dependency
  license policy with the actual Rust graph.
- Make the API the single active ClickHouse schema migrator and gate Grafana on
  API readiness. Provision the ClickHouse datasource declaratively with prune
  semantics and the current pinned plugin.
- Add an isolated end-to-end ClickHouse lane for enrollment, accepted upload,
  duplicate retry, weekly reporting, all dashboard queries, and authenticated
  deletion; fix the report-quality contract drift it exposed.

## v0.19.3 - 2026-08-30

- Include the canonical Codex marketplace manifest in the moving `stable`
  distribution so `codex plugin marketplace add ... --ref stable` can discover
  and install GroundLine Insights.
- Publish `stable` as a deterministic, tag-bound minimal distribution containing
  only the marketplace metadata, installable plugin package, and six verified
  native target bundles; source, workflows, services, and infrastructure remain
  on the immutable release tag.
- Restore executable modes only for the four Unix binaries after GitHub artifact
  download, while preserving data-file modes and validating the exact release
  package set before publication.

## v0.19.2 - 2026-08-30

- Assemble each immutable target artifact from its explicit download directory
  before release verification and stable promotion, avoiding download-action
  merge ambiguity.
- Publish the API image as `groundline-insights-api` so the fresh public
  repository does not inherit permissions from the private-era container
  package with the old name.
- Publish GroundLine Insights as public source while keeping all real endpoints,
  credentials, infrastructure inventory, deployment receipts, and private-era
  GitHub history in a separate private operations boundary.
- Remove the repository-scoped TrueNAS runner, self-hosted selectors, paid-runner
  gates, artifact importer, and public production-deployment workflow. Run all
  CI and release jobs on standard GitHub-hosted runners with read-only pull
  request permissions, bounded timeouts, exact artifact binding, and pinned
  external Actions.
- Split the installable private companion from the public GroundLine core under
  the distinct `groundline-insights` plugin and marketplace IDs. Remove public
  guidance skills from the companion package while preserving the collector
  binary name, state directory, and event contracts for continuity.
- Move the real collector endpoint out of the plugin package into a validated,
  owner-local schema-6 profile installed with `groundline worker configure`.
  Derive the transmitted release version from the binary and keep profile,
  endpoint, and private paths out of receipts.
- Parameterize Grafana access URL and TrueNAS app name through protected GitHub
  environment variables, add fail-fast validation, and prevent assembled
  binaries from being re-scanned as source during stable promotion.

- Split moving-source validation into a bounded pull-request fast lane and one
  explicit full qualification before release artifacts. Remove implicit
  GitHub-hosted fallbacks from trusted Linux work, require an explicit dispatch
  opt-in before hosted release orchestration, narrow Insights triggers to its
  actual build surface, and enforce the cost contract in the xtask test suite.
- Complete the Rust-only cutover: remove all 71 Python source, service, hook,
  test, and packaging files; replace the old script matrix with one native CLI,
  one Rust Insights API, and Rust `xtask` release/deployment tooling.
- Add native `doctor`, `project-audit`, and checksum-verifying
  `provider-smoke` commands, and make `verify-source` reject Python workflow
  or documentation commands, stale package copies, and non-native hooks.
- Move TrueNAS deployment to authenticated WSS JSON-RPC 2.0 with bounded job
  handling, API/Grafana health verification, and rollback; execute service
  migrations, owner-report SQL, and every Grafana query against pinned
  ClickHouse in CI.
- Make ClickHouse 25.8 bootstrap and queries executable over HTTP by sending an
  explicit body length, creating the database outside its not-yet-existing
  context, and fixing aggregate aliases plus `ARRAY JOIN` syntax; parse the
  complete provisioned dashboard so all 18 Grafana SQL targets are exercised.
- Preserve Grafana `$__timeFilter` while rendering Compose, support the standard
  macOS `/tmp` symlink through canonical parent targets, cache the container
  Cargo registry, and emit only a fixed reason code when the API cannot start.
- Remove the shared ingest-token collector bypass. Collectors now require their
  enrollment token; collector-free owner reports use the separate admin token.
- Consolidate install, upgrade, operations, Insights, privacy, and release
  documentation around the six native targets and moving Rust `stable`
  channel, removing obsolete compatibility and duplicate script guides.
- Harden the Rust packaging path so one no-follow source handle supplies both
  staged bytes and checksums, with staged-file sync and Unix parent-directory
  sync before completion.
- Add cross-platform bounded file opening, Windows reparse-point and ACL
  checks, process-tree-owned Tailnet probes, privacy-safe reason codes, and
  Linux/Windows ARM64 build coverage.
- Partition optional Insights networking dependencies away from `xtask`, add
  Cargo workspace version enforcement to the release gate, and align the
  source, package, Insights defaults, and release workflow on 0.18.0.
- Reject input symlinks and counter overflow across the Rust CLI and typed
  report contracts, and block stable promotion while tracked Python or any of
  the six packaged target artifacts remains missing.
- Exercise the real CLI through a cross-platform integration harness, and bind
  both Linux architectures to their native `musl-gcc` compiler and linker in
  CI instead of relying on target-toolchain autodetection.
- Reuse the six-platform Rust workflow from tag releases, assemble immutable
  artifacts into one exact package set, and advance `stable` only to a direct
  binary-only child of the verified release tag.
- Add a read-only Git/worktree provenance preflight that rejects marketplace
  caches as development origins, broken linked-worktree metadata, unexpected
  push remotes, and repository mismatches without emitting paths or remote URLs.
- Run that preflight first in the release gate, keep development-branch
  exceptions exact and explicit, and execute primary and Insights CI on the
  moving `stable` branch as well as `main`.
- Extract App Server schema comparison from the monolithic doctor into a small
  packaged runtime probe with focused redaction and failure-contract tests.
- Preserve distinct runtime-channel, hook-transport, cleanup, lifecycle, and
  weekly-audit evidence so Codex App and PATH CLI drift is not flattened into a
  false pass.
- Add a resource-bounded, repository-scoped TrueNAS runner for trusted Linux
  work, split hot runner I/O onto NVMe and Docker capacity onto HDD, route only
  owner-authored same-repository pull requests to it, and keep forks, bots,
  native macOS/Windows artifacts, and production deployment on GitHub-hosted
  boundaries. Preflight the Docker Buildx and Compose plugins required by
  Insights CI before starting a build, and isolate rendered secrets in a
  job-scoped temporary path so persistent runners cannot reuse stale files.
  Add a separate temporary administrator-runner fallback for release
  orchestration when GitHub-hosted billing is unavailable, without exposing
  production credentials to the TrueNAS build runner. Load pinned workflow
  support before replacing the checkout with an older immutable tag so an
  existing release remains resumable after release-tooling changes. Validate
  protected TrueNAS, health, and conditional Tailscale inputs before image
  builds, share that preflight with direct deployments, and report only the
  missing input name when native deployment configuration is incomplete. Keep
  tagged Compose input immutable while running current reviewed deployment
  control code, accept unencrypted health probes only on Tailscale CGNAT
  addresses, and restore the live Grafana ClickHouse datasource query that the
  Rust cutover had reduced to a process-only health check. Construct deployment
  deadlines inside the Tokio runtime so invalid inputs return bounded errors
  instead of panicking before the TrueNAS connection, and make the isolated
  deployment binary explicitly install its pinned rustls `ring` provider before
  creating HTTPS or WSS clients.
- Separate the deployment controller from source-only `xtask` commands, build
  it once as a checksummed workflow artifact, and prohibit Cargo compilation in
  production-access jobs. Add explicit read-only `preflight` and mutating
  `apply` commands linked by a strict current-config fingerprint, retry and
  verified rollback state transitions, full 19-query Grafana semantic checks,
  access-gate verification, and redacted phase receipts. Allow a fresh release
  workflow to reuse an already-published digest only after verifying the
  release tag resolves to it, so controller-only fixes do not rebuild the image
  or rely on a re-run pinned to the old commit. Derive the public Grafana Host
  from the validated access URL, keep product version migration bound to the
  tagged Compose template rather than the controller version, and budget every
  RPC, health phase, rollback, and job deadline so failure handling cannot run
  indefinitely or lose its receipt window.
- Add a fingerprinted ARM64 Linux controller harness that retains bounded
  Rustup, Cargo, and target volumes across successful runs, initializes cache
  ownership before compilation, explicitly prunes stale fingerprints, and
  distinguishes compiler, volume, Docker daemon, and stream failures. Split
  fast edit-loop checks from the one-shot final source gate and keep macOS,
  ARM64, workflow, and production evidence independent.

## v0.17.9 - 2026-08-25

- Speed up redacted Codex usage audits by avoiding impossible JSON-object
  decodes, classifying command terms in one pass, caching exact repeated-call
  categories, and scanning terminal failure candidates without changing the
  output schema or privacy boundary.

## v0.17.8 - 2026-08-25

- Refresh the official Codex documentation registry and local capability map
  for the 0.149 runtime, including the App Server replacement for deprecated
  `codex mcp-server` integration.
- Add evidence-based root `.worktreeinclude` and checked-in local-environment
  guidance, with metadata-only discovery and inactive nested-file warnings.
- Audit project trust-dependent config, permission-profile versus legacy
  sandbox conflicts, inline versus JSON hooks, and rule-example coverage
  without emitting configuration values or definitions.
- Harden scheduled-task cleanup guidance around manual prompt testing,
  unattended least privilege, archive candidates, pinned worktrees, and
  user-owned state changes, with cross-platform regression coverage.

## v0.17.7 - 2026-08-24

- Make the Insights worker's explicit `status` command resolve the active Codex
  installation before comparing lifecycle hook sources, preventing a source
  checkout from falsely reporting a healthy installed hook as stale.
- Add a regression test that keeps source-checkout diagnostics separate from
  the installed plugin package used by `hooks/list`.

## v0.17.6 - 2026-08-24

- Close Codex `hooks/list` verification on installations without a managed App
  Server control socket by using one same-`CODEX_HOME`, deadline-bounded stdio
  server after the proxy path is unavailable.
- Keep spawned-server configuration evidence distinct from the owning App
  engine, preserve command/hash/path redaction, and require a fresh native
  lifecycle receipt before treating dispatch as adopted.

## v0.17.5 - 2026-08-24

- Make Grafana roster and fleet coverage independent of ClickHouse outer-join
  default values, with identical results under `join_use_nulls=0` and `1`.
- Execute the managed dashboard SQL and an independent fleet reference against
  pinned ClickHouse 25.8 on Insights pull requests, not only release tags.
- Fail a TrueNAS rollout when live Grafana fleet, roster, or storage frames are
  malformed or semantically inconsistent, including epoch timestamps and
  duplicate-count drift.
- Separate Codex log-database file size, allocated pages, in-use pages, free
  pages, exact row count, and estimated payload in the opt-in read-only health
  probe.
- Split stale active and archived rollout counts into root and subagent cohorts,
  without emitting task IDs, paths, titles, or messages.
- Add an explicit cleanup-safety contract: idle is not completion evidence,
  archive only cleans the task picker, deletion is permanent, active worktrees
  and deep-link tasks stay preserved, and direct SQLite edits or `VACUUM` are
  never recommended.

## v0.17.4 - 2026-08-23

- Add one installable, cross-platform operations troubleshooting contract for
  moving-versus-pinned upgrade channels, process-scoped private Git access,
  incomplete four-hook views, Tailnet health, durable outbox recovery, and
  accepted-or-duplicate closure evidence.
- Add a concise Korean companion and route install, update, README, and native
  upgrade guidance to the shared decision table before reinstalling or changing
  local state.
- Align current-version, worker, manual Insights, and next-version wording with
  the shipped manifest, and add regression checks that prevent operational
  version examples and packaged reference counts from drifting again.

## v0.17.3 - 2026-08-21

- Make bounded validation prefetch ordering deterministic across Windows,
  macOS, and Linux by sorting relative POSIX paths instead of platform-native
  `Path` objects.

## v0.17.2 - 2026-08-21

- Recover valid Basic aggregates whose completed-root period is unavailable by
  staging their normalized generation timestamp before upload. This preserves
  the durable acknowledgement boundary instead of misclassifying an empty
  root-time range as a corrupt outbox.

## v0.17.1 - 2026-08-21

- Align the native marketplace-channel verifier with Codex 0.149 Refresh:
  classify local channel identity from source, ref, and sparse paths while
  treating config `last_revision` as registration metadata that may lag the
  active snapshot.
- Keep update currentness strict by comparing the active marketplace snapshot
  with the authenticated remote ref, and add a regression covering the exact
  post-Refresh state observed on macOS.

## v0.17.0 - 2026-08-19

- Re-pin the reviewed Codex runtime contract to stable CLI 0.149.0 and record
  the configurable skill-metadata budget, expanded native doctor checks,
  session-start command-environment capture, active MCP hook runtime, and
  permission-state restoration.
- Add an explicit, redacted native context-budget probe that compares the
  current prompt surface with a bounded candidate while retaining only counts,
  ratios, and skill-locator deltas; never change `skills.max_context_tokens`
  automatically.
- Keep GroundLine's four durable checkpoints as synchronous command hooks and
  require `hooks/list` to report that handler type and execution mode before
  readiness can pass; expose only bounded handler/mode status to the worker.
- Replace the obsolete close-or-toggle-every-hook upgrade procedure with the
  Codex 0.148-or-newer refresh boundary: loaded hooks refresh in place, changed
  hashes remain user-trusted, and a fresh task is still required for the
  skill/tool catalog. On 0.149, command hooks retain the environment captured
  when the task started, so environment changes also require a fresh task.
- Extend the read-only weekly contract with bounded native-doctor integrity and
  rollout-inventory evidence without automatic SQLite repair or stale-row
  deletion.
- Deduplicate repeated thread-inventory references to the same rollout file
  while preserving distinct forked rollouts, and expose only a bounded excluded
  count in the local audit.
- Classify durable outbox backlog as recent, delayed, or overdue without
  exposing event timestamps, and make delayed or overdue delivery visible in
  local collection health.
- Preserve concurrent bounded collection issue codes in status so a primary
  hook action cannot hide an overdue upload, Tailnet, history, or local-state
  problem.
- Allow a completed-root sample truncated only by the ten-root cap to produce
  one explicitly sample-scoped, low-confidence optimization candidate while
  prohibiting generalization to unselected roots.
- Add a backward-compatible `/v3/reports/weekly` quality contract that separates
  the 24-hour initial-report grace window, overdue never-reported installations,
  and observed installations stale for seven days; retain `/v1` and `/v2` for
  installed older clients.
- Reconcile ClickHouse stored rows with deduplicated active-generation events
  and report physical duplicate excess, six-hour delivery delay, 24-hour
  overdue delivery, and five-minute clock skew without automatic optimization
  or deletion. Preserve the actual worker attempt and retry reason across later
  `not_due` checks, and return all applicable bounded next actions in priority
  order.
- Parallelize bounded package-validation hydration and provider-package equality
  checks, and reuse one isolated test package template so file-provider-backed
  worktrees do not repeatedly read the same source payloads; retain size,
  UTF-8, link/reparse, and concurrent-change safety checks.
- Let stable-channel planning verify an already-local remote commit directly
  and fetch only when the referenced object is absent, avoiding unnecessary
  private-SSH transfers without weakening tag, version, or lease checks.

## v0.16.5 - 2026-08-13

- Resolve installed package caches from the active `CODEX_HOME`, including the
  dedicated Codex homes used by Hermes-originated Codex sessions, without
  emitting the configured path.
- Verify noninteractive private Git access immediately after the initial
  marketplace clone, before the first Refresh creates its revision snapshot;
  distinguish that snapshot-initialization step from authentication failure.
- Keep SSH as the owner-default private-repository transport, retain HTTPS
  credential helpers as a process-scoped alternative, and never place PATs or
  other credentials in repository URLs, commands, config, or diagnostics.

## v0.16.4 - 2026-08-13

- Align the LLM-readable remote install/update proof packet with the v0.16.3
  private-owner contract: authenticated SSH source, moving `stable` ref, and
  JSON results for marketplace add, upgrade, plugin install, and rollback.
- Add regression coverage so source diagnostics cannot silently reintroduce the
  legacy HTTPS shorthand or non-JSON update commands.

## v0.16.3 - 2026-08-13

- Separate a locally configured moving marketplace channel from private Git
  authentication, an available remote revision, and an observed Codex App
  Refresh. Add a bounded opt-in noninteractive Git probe that emits no source,
  ref, revision, path, process output, or credential.
- Make SSH the documented owner path for the private repository, keep tokens
  out of commands and URLs, use the official Refresh terminology, and require
  before/after revision, installed-version, and package-fingerprint evidence.
- State in the manifest and current documentation that GroundLine supports
  Codex only. Treat `hermes` solely as a Codex rollout-origin cohort backed by a
  dedicated Codex `CODEX_HOME`, never as a Hermes plugin target.

## v0.16.2 - 2026-08-13

- Apply the same 4 KiB strict advisory-cache boundary to lifecycle notification
  processing and status, so oversized or malformed local state cannot suppress
  an update notification or persist unvalidated fields.
- Treat an incomplete Codex App hook list as unavailable evidence, distinguish
  CLI trust persistence from live Desktop hook-engine reload, and require a
  fresh-task current-version receipt without using trust bypasses.
- Accept all Codex-native Goal states without mutating provider-owned blocked
  or quota-limited states, and require all four discovered lifecycle hooks to
  expose current hashes before the provider readiness probe can pass.

## v0.16.1 - 2026-08-12

- Ship an installed, LLM-readable native upgrade contract that keeps the moving
  `stable` channel, immutable release tags, custom refs, mutation approval, task
  boundaries, hook trust, and post-upgrade proof as separate decisions.
- Diagnose the local marketplace channel from bounded config and active
  marketplace metadata without echoing private source values, then classify it
  as moving stable, immutable release tag, other ref, or unknown before any
  update action.
- Suppress stale, missing, or malformed advisory-cache values from bounded
  status so old server truth cannot select or justify a marketplace upgrade.
- Reject dead or package-escaping relative Markdown links during package
  validation and require the installed native upgrade contract to ship with the
  package.

## v0.16.0 - 2026-08-12

- Add enrollment schema 2 with exactly four bounded fleet fields—OS family,
  runtime family, execution mode, and GroundLine version—and migrate legacy
  local token metadata once with the same collector token. Preserve the
  server-side collector generation and original creation time; reject unknown,
  malformed, or identity-mismatched current metadata before network work.
- Add an installation-weighted fleet roster and schema-2 owner report alongside
  the existing event-weighted cohorts. Keep enrollment, metadata-known,
  observed, reporting, recent, never-reported, current package claim, and
  accepted current-version execution as separate evidence.
- Extend the private Grafana dashboard and ClickHouse migration with additive
  fleet metadata, unknown/legacy visibility, and current-version observation
  panels without exposing hostnames, IP addresses, paths, collector IDs,
  tokens, endpoints, prompts, responses, or commands.
- Add an opt-in redacted Codex doctor that evaluates the primary Desktop-bundled
  CLI and a distinct PATH CLI separately, while retaining conservative
  aggregate status and never emitting executable paths or raw diagnostic
  output.
- Keep the six-skill, two-implicit-skill context surface below its enforced
  metadata budget with at least ten-percent headroom. Preserve Codex-native
  hooks, goals, agents, plugin upgrade, and doctor ownership instead of adding
  a GroundLine daemon, MCP lifecycle, automatic trust, or automatic updater.
- Treat a trusted but stale lifecycle-hook source as not ready, require a
  successful enabled TrueNAS deployment before promoting the native `stable`
  upgrade channel, and retain the existing fail-open four-hook contract across
  macOS, Windows, and Linux.

## v0.15.3 - 2026-08-11

- Add a verified moving `stable` marketplace channel so Codex App's native
  Upgrade action can discover a newer GroundLine release without replacing an
  immutable tag pin by hand.
- Promote `stable` only after the tagged release gates, pinned ClickHouse
  contract, image build, and enabled TrueNAS deployment succeed. Reject
  downgrade, same-version replacement, and concurrent promotion races with a
  monotonic semantic-version check and an exact force-with-lease.
- Make `stable` an explicit owner opt-in to Codex-owned startup refresh while
  retaining exact release tags as reproducible rollback pins. GroundLine adds
  no updater, timer, hook-side installer, or marketplace mutation.
- Keep updates at a Codex task boundary: an existing task retains its frozen
  plugin catalog and hook engine, while a new task loads the promoted package
  and still requires normal hook trust for a changed command hash.

## v0.15.2 - 2026-08-11

- Make all four Codex lifecycle commands fail open without output when their
  frozen launcher is missing or fails, preventing a deleted old plugin cache
  from turning a `Stop` hook error into an unbounded continuation loop on
  POSIX and Windows. This protects tasks started with v0.15.2 during later
  updates; it cannot retroactively change commands frozen by v0.15.1.
- Document the Codex 0.147 active-task update boundary: finish tasks before a
  Terminal update, or disable and later re-enable the four hooks in the same
  Desktop App so its live hook engines are reloaded. A new task is still
  required to adopt the updated plugin catalog.
- Clarify that IDE integrations and external Codex companion brokers can keep
  tasks and old hook engines alive after the Desktop App exits, so every client
  that owns an affected task must release it before an update.

## v0.15.1 - 2026-08-10

- Record a privacy-bounded, current-version native lifecycle receipt so actual
  automatic dispatch remains visible when the optional Codex app-server control
  socket is unavailable. Keep hook configuration readiness separate, preserve
  the receipt across manual cycles, and reject stale, mismatched, or malformed
  receipt evidence.
- Match an active marketplace checkout to the installed cache by its manifest
  version and package-content fingerprint instead of a private absolute path,
  while retaining stale-package detection and emitting no source path or hook
  hash.
- Align owner status, weekly audit, provider-smoke, and update documentation with
  the two independent evidence lanes: hook configuration and observed lifecycle
  dispatch.
- Refresh the pinned Codex source contract against the official 0.147.0 stable
  tag; changed evidence hashes retain the same bounded skill, model, hook,
  thread, and multi-agent semantic claims.
- Keep the optional Linux/arm64 Docker release proof bounded at ten minutes so
  the expanded full suite can complete under local architecture emulation.

## v0.15.0 - 2026-08-09

- Add a private, fixed-window Insights report API and foreground client so a
  weekly or monthly optimization review can read one redacted packet from the
  active-generation ClickHouse view without scraping Grafana or receiving SQL,
  collector identifiers, task identifiers, paths, commands, or raw content.
- Upgrade Basic aggregates to schema 5 with explicit measurement-capability
  flags, completed-root selection coverage, latency denominators, per-root
  boundary counts, component-specific usage provenance and fallback counts,
  Guardian workspace-attribution coverage, and incomplete-review exclusions.
  Schemas 1 through 4 remain ingest-compatible and cannot be mistaken for
  measured zero.
- Report eligible, selected, and truncated completed-root counts plus bounded
  UTC recency coverage; prefilter normal weekly audits by time before reading
  rollout metadata, and mark truncated evidence as partial.
- Let the scheduled weekly review read one existing foreground seven-day
  Insights report, keep activity windows separate from completed-root samples,
  avoid replaying unchanged release history with a durable verified tag/target
  pair, and distinguish live App hook evidence from static declarations and
  persisted local trust.
- Resolve yielded verification commands through same-rollout `wait` or
  `write_stdin` terminal results using transient, fail-closed correlation so
  completed checks are not left permanently unresolved and correlation IDs are
  never emitted.
- Classify command tools from the executable payload only, keep workdir and
  wrapper metadata out of category inference, and attribute each Guardian
  review to its own rollout workspace with an explicit coverage denominator.
- Make lifecycle status depend on a real running App Server `hooks/list`
  response, the exact launcher and four events, enabled state, and user trust.
  Expected-package source matching remains separate, no second App Server is
  spawned, and an unverified probe never blocks collection or produces a false
  healthy status.
- Surface privacy-bounded Tailnet connectivity locally while retaining pending
  aggregates and watermarks offline. Central stale-install labels remain
  delivery-recency signals rather than claims about Tailnet state.
- Treat every locally confirmed non-connected Tailnet state as a short retry,
  retain unknown probe states as fail-open, and bypass ambient HTTP(S) proxies
  for bearer-authenticated owner traffic.
- Stage the newest outbox boundary before upload so a process exit between
  server acceptance and local success recording cannot regenerate an
  overlapping activity window with a different event ID.
- Execute the fixed report SQL against pinned ClickHouse 25.8 in the tag gate,
  fix aggregate-alias and RFC3339 parameter handling, require release tags to
  resolve to `main`, and run the complete portable eight-gate preflight before
  image publication.
- Validate every managed Grafana panel query during deployment, use an explicit
  report-presence join flag, and label the mean of window p90 values accurately
  instead of presenting it as a merged percentile.
- Keep a single-window owner report descriptive and comparison-ineligible until
  an explicit baseline/candidate comparison proves matching cohorts and the
  personal or cross-install sample threshold.

## v0.14.1 - 2026-08-06

- Align install, runtime-matrix, and next-version guidance with the four-event
  lifecycle-checkpoint manifest instead of the retired SessionEnd-only path.
- Add a native Windows GitHub Actions gate that verifies `py -3`, package
  validation, staged Codex smoke, native-host scenario evidence, and the full
  unit suite on `windows-latest`.
- Mark portable scenario results separately from native-host proof so a Mac or
  Linux contract run cannot be reported as Windows installation evidence.
- Require the local release gate's macOS scenario to prove the current host is
  actually macOS while retaining non-native platform checks as explicit
  portable contracts.
- Normalize package fingerprints, staged install paths, project-audit paths,
  and validation diagnostics to stable forward-slash output across operating
  systems. Use Windows-native executable fixtures and keep POSIX mode and Linux
  Docker fixture assertions on the platforms where those semantics exist.
- Keep actual Codex App installation, user-owned hook trust and dispatch, local
  file permissions, and an accepted upload as per-install live proof rather
  than inferring them from CI.

## v0.14.0 - 2026-08-06

- Replace SessionEnd-only automatic collection with one activity-window path
  shared by `SessionStart`, root-turn `Stop`, `PostCompact`, and root-only
  `SessionEnd`. Stop events coalesce for fifteen minutes; resume, compaction, and
  close boundaries flush immediately without cron, timers, MCP, or a daemon.
- Include open, long-lived, resumed, compacted, and interrupted roots in strict
  redacted aggregates. Provider cumulative totals are converted to window
  deltas so checkpoints cannot count the same tokens twice.
- Add schema-4 `observed_root_count` and a fixed `collection_trigger` enum,
  preserve root-turn completion metrics as a separate signal, and update
  ClickHouse and Grafana to use observed-root denominators. Schemas 1 through 3
  remain ingest-only support for deployed history and durable pre-upgrade
  outboxes.
- Evaluate evidence sufficiency after the dashboard period is aggregated so
  sparse lifecycle checkpoints contribute to weekly comparisons instead of
  being discarded individually.
- Replace the old SessionEnd policy fields and launcher filename with one
  checkpoint contract while preserving the collector identity, consent,
  token, watermark, outbox, and server history.

## v0.13.2 - 2026-08-06

- Preserve schema-3 verification success, failure, and unresolved tool-result
  counts when the session audit is compacted into the weekly audit used by
  Insights.
- Add an end-to-end weekly regression so missing outcome fields cannot silently
  degrade every verification result to unresolved again.

## v0.13.1 - 2026-08-06

- Restore a selective pre-implementation gate in the implicit current-state
  skill. Clear bounded work uses a light state, scope, and verification check;
  broad, ambiguous, current-fact-dependent, or high-impact work collects
  targeted evidence, synthesizes once, and freezes one implementation boundary
  before writing.
- Let a user-accepted weekly Insights recommendation update the operating
  contract while keeping daily events collection-only. New non-blocking ideas
  are deferred after freeze instead of repeatedly restarting implementation.
- Add a regression contract for the gate and retain the six-skill, two-implicit,
  3,000-word total, and 600-word implicit context ceilings with at least ten
  percent headroom.
- Upgrade Basic aggregates to schema 3 with redacted verification tool-result
  success, failure, and unresolved counts while accepting complete schema 1 and
  schema 2 events.
- Add a deterministic cohort comparison gate with `READY`, `INSUFFICIENT`, and
  `COHORT_MISMATCH` results for personal longitudinal or cross-install review.
- Align dashboard weeks to UTC Monday and add normalized workflow ratios, token
  components, model/effort runtime contexts, comparison readiness, verification
  outcome coverage, and explicit sample missingness. Wall-turn latency remains
  labeled as a proxy rather than model inference latency.

## v0.13.0 - 2026-08-04

- Add a strict server-to-collector update advisory containing only current,
  latest, and minimum-supported semantic versions plus a fixed status enum.
  Clients recompute the status and reject inconsistent responses; the server
  cannot supply notification copy, commands, or URLs.
- Notify stale macOS and Windows Codex App installations after root
  `SessionEnd`, once per target release with a seven-day reminder. Notification
  failure is advisory-only, retries no sooner than 24 hours, and never changes
  collection success. CLI, Hermes, Linux, and headless collectors remain
  dashboard-only.
- Publish the deployed release policy to ClickHouse and add Grafana panels for
  stale collector count and per-install version status using short random
  collector codes without hostnames, device names, auto-update, cron, MCP, or
  a resident process.

## v0.12.1 - 2026-08-03

- Remove the unsupported top-level `hooks` field from the Codex plugin
  manifest and rely on native `hooks/hooks.json` discovery. The reviewed
  root-only `SessionEnd` hook remains packaged and unchanged.
- Make package validation and provider smoke reject unsupported manifest fields
  so source/install fingerprint equality cannot hide an App ingestion failure.
- Preserve the intended six-skill surface: two implicit task-boundary skills
  and four explicit skills.

## v0.12.0 - 2026-08-03

- Automatically start or resume one strict Basic history sync on the first
  trusted root `SessionEnd`, atomically activate the completed generation, and
  immediately catch up work completed after the fixed initial cutoff. Existing
  generation-1 collectors migrate to `ready` without replay.
- Persist a content-free history-sync contract and expose only
  `initializing_history`, `ready`, `retrying`, or `disabled` in normal status;
  internal generation numbers remain an administrator implementation detail.
- Add schema-2 numeric counts for unclassified runtime-originator exclusions
  and legacy fallback coverage. The server and ClickHouse migration remain
  backward-compatible with complete schema-1 events, and Grafana surfaces the
  aggregate coverage signal without originator values.
- Keep revoke, failure backoff, durable outbox, and generation activation
  fail-closed. A central collector deletion never causes silent identity reset
  or historical re-upload; explicit `backfill-history` remains recovery-only.

## v0.11.1 - 2026-08-03

- Partition automatic incremental collection and historical backfill by the
  rollout's low-cardinality runtime originator, so Codex App, CLI, and Hermes
  collectors sharing a state database cannot count the same root or Guardian
  twice.
- Exclude present but unrecognized originators from automatic cohorts instead
  of relabeling unrelated imported tasks as Codex usage. Legacy records without
  originator metadata retain a conservative Codex source fallback.

## v0.11.0 - 2026-08-03

- Add an explicit, resumable `backfill-history` command that reconstructs every
  locally available completed root, delegated-agent, and Guardian aggregate in
  non-overlapping seven-day windows without exposing task identifiers or raw
  content.
- Introduce per-collector collection generations. Historical events stage in
  the immediate next generation, and the server activates it only after the
  expected event count is present; interrupted uploads keep the previous
  dashboard generation visible and resume idempotently.
- Preserve legacy schema-1 events as generation 0, widen count columns to
  UInt32, and make Grafana read the active-generation view so a rebuild cannot
  double totals.
- Remove the 365-day ClickHouse TTL so older backfilled periods are not deleted
  immediately. Basic history now remains inside the operator's 1 TiB dataset
  quota and snapshot boundary until explicit collector deletion.
- Raise the bounded private ingest throughput to 600 requests per minute so a
  multi-year weekly rebuild can complete without imposing a total event-count
  ceiling.

## v0.10.0 - 2026-08-03

- Collect after every trusted root `SessionEnd` instead of waiting for a daily
  cadence. Each cycle covers the complete interval after the last acknowledged
  cutoff, so a healthy installation normally contributes one aggregate per
  completed root and the next cycle catches eligible completions missed during
  downtime.
- Remove total-count ceilings for selected roots, delegated-agent and Guardian
  rollouts, catch-up windows, and automatic or recovery uploads. Failed and
  unattempted outbox events remain durable; the fixed 64 KiB event schema,
  strict allowlist, per-event retry bound, and 365-day retention remain safety
  boundaries rather than collection-count limits.
- Accept completed, readable task rollouts even when the Codex task is not
  archived, while still excluding in-progress tasks. Preserve aggregate-only
  privacy: no prompt, response, command, path, thread, task identifier, secret,
  or free-form label is collected.
- Migrate legacy MCP/daily owner policies to a schema-3 all-completed
  SessionEnd profile without resetting the pseudonymous collector, consent,
  token, outbox, watermark, or existing server history.

## v0.9.0 - 2026-08-03

- Keep automatic Insights collection-only. A separate read-only weekly command
  proposes one optimization candidate for user review, refuses poisoned or
  insufficient evidence, and never changes workflow, model, effort, settings,
  Chronicle, or experiments.
- Add only four low-cardinality workflow counters needed for weekly ratios:
  text-message count, tool-call count, short-message count, and broad-scope
  message count. The server accepts complete legacy events, adds ClickHouse
  columns idempotently, and exposes cohort ratios without message content.
- Collect one strict Basic aggregate per due day while retaining weekly Grafana
  reporting. Automatic windows advance from the last acknowledged cutoff and
  never fall back to older completed roots, preventing overlapping token sums.
  A returning installation catches up at most seven daily windows per cycle so
  long idle gaps stay day-addressable without an unbounded SessionEnd worker.
- Replace the undocumented MCP lifecycle trigger with one reviewed, root-only
  Codex `SessionEnd` hook. The one-second fail-open launcher ignores event
  input, emits nothing, performs no network work, and detaches a one-shot
  worker.
- Remove the bundled MCP server and keep the prompt tool catalog unchanged.
  Concurrent SessionEnd launches coalesce through an OS-released advisory
  lock with no stale-file deletion race; retries retain the durable outbox and
  wait for a later SessionEnd.
- Pass only a strict location/runtime environment allowlist to the worker so
  ambient credentials cannot cross the hook process boundary.
- Report the last lifecycle check, collected-through watermark, next due time,
  current due state, policy/consent state, pending outbox count, runtime cohort,
  and one bounded next action so a healthy idle collector no longer looks
  broken. Distinguish invalid outbox state from audit failure without attempting
  enrollment or upload. Add idempotent pre-first-run disable and explicit
  re-enable commands.
- Migrate an existing active v0.8 MCP owner policy in place to the v0.9
  SessionEnd contract while preserving its pseudonymous identity, consent, and
  collection watermark.
- Treat daily sample sufficiency separately from weekly dashboard confidence
  and reduce the stale-collector threshold from eight days to 36 hours.
- Pin the reviewed Codex source contract to `003ec63b` and verify root-only
  dispatch, transcript flush order, one/three-second timeout limits, advisory
  output, and hash-based plugin hook trust.
- Keep the ClickHouse Grafana account read-only while allowing its official
  datasource to apply the bounded `max_execution_time` query setting, fixing
  dashboards that passed datasource health but failed every panel query.

## v0.8.5 - 2026-07-31

- Restrict the read-only Nginx ingress to its loopback probe and dynamic Docker
  host gateway, resolve the upstream over IPv4, and attest the Tailnet bind
  address so dual-stack lookup and bridge source rewriting cannot break
  collector enrollment or open enrollment to peer containers.
- Send ClickHouse 25.8 its explicit best-effort DateTime input setting so the
  schema's UTC ISO-8601 timestamps work for collector and Basic event inserts.
- Retain low-cardinality Basic weekly aggregates for 365 days and enforce the
  TTL idempotently when the API starts, including on existing installations.
- Add dashboard cards for data freshness and active pseudonymous collectors,
  with an eight-day stale-data threshold and a 365-day default view.
- Document the owner deployment's 1 TiB dataset quota and recursive hourly and
  daily ZFS snapshot policy.

## v0.8.4 - 2026-07-31

- Preserve the read-only Nginx ingress root filesystem on TrueNAS 25.10 by
  generating its configuration inside the existing `/tmp` tmpfs instead of
  using an inline Compose config that TrueNAS rejects for read-only services.
- Route every Nginx temporary-file class to `/tmp`, preventing unused FastCGI,
  uwsgi, or SCGI defaults from attempting writes under `/var/cache/nginx`.

## v0.8.3 - 2026-07-31

- Keep the Insights API on the internal-only Docker network and publish the
  Tailnet ingest port through a digest-pinned, non-root Nginx sidecar with no
  database, admin, ingest, or collector credentials. The sidecar receives only
  a scoped proxy-hop token.
- Authenticate the private proxy hop with a separately generated token before
  trusting its single-IP `X-Forwarded-For` value. Existing private secret files
  gain only this new token and retain every prior credential.

## v0.8.2 - 2026-07-31

- Pin the TrueNAS stack to ClickHouse `25.8.28.1-alpine`, the current 25.8
  LTS build that runs on the owner's non-AVX Celeron host. ClickHouse 26.7.1
  exits with `SIGILL` on that CPU before configuration is loaded.
- Move every workflow to current Node 24 actions, suppress Git's checkout
  default-branch hint at the source, cancel stale CI runs, bound job runtimes,
  reduce artifact retention, and remove duplicate Insights tests and release
  image builds.

## v0.8.1 - 2026-07-31

- Keep the private GHCR image release green by skipping GitHub build
  attestations only where the platform does not support them: user-owned
  private repositories. The image still builds with BuildKit provenance and
  SBOM metadata and remains digest-addressable.

## v0.8.0 - 2026-07-31

- Make the private repository an owner-only activation boundary: installation
  declares one optional zero-tool MCP worker, and a fresh Codex task performs
  an idempotent Basic weekly due check without a hook, cron entry, timer, OS
  daemon, external scheduler, or agent-visible tool.
- Add Tailnet-scoped first-contact enrollment and random per-install collector
  tokens. The server stores only token digests; no shared ingest secret is
  committed to the private repository.
- Add a separately consented, foreground-only Basic uploader that accepts only
  Tailnet endpoints as a manual recovery path, retries a bounded set of
  transport failures, and removes only acknowledged outbox events.
- Add a private schema-1 ingest and collector-deletion API with strict payload
  validation, bounded responses, idempotency, rate and size limits, and no
  request-content logging.
- Add a TrueNAS Custom App Compose stack with ClickHouse 90-day retention,
  separate ingest and read-only Grafana users, fixed resource ceilings, private
  networking, and a small-sample descriptive dashboard.
- Put the single public Grafana hostname behind Cloudflare Access while keeping
  ingest, ClickHouse, and TrueNAS administration on Tailscale-only surfaces.
- Add release-gated GitHub Actions that build and attest an immutable private
  GHCR image, connect through an ephemeral Tailscale runner, change only the
  API image in the current TrueNAS configuration, verify all health gates, and
  restore the previous configuration on failure.
- Remove the optional release advisory hook. Versioned plugin-cache roots can
  outlive an updated long-running Codex task and turn an advisory into a
  tool-blocking missing-file failure; release boundaries now stay in skills,
  checks, and GitHub Actions.
- Keep Diagnostic data, timer-driven scheduling, always-on telemetry,
  automatic experiments, Codex settings, and Chronicle state outside
  GroundLine.

## v0.7.0 - 2026-07-31

- Add an opt-in, local-only GroundLine Insights foundation that converts the
  existing redacted weekly audit through a strict Basic allowlist.
- Separate Mac, Windows, Linux, Codex App, Codex CLI, and Hermes cohorts with a
  resettable random per-install identifier that is never derived from a user,
  host, account, repository, IP address, or path.
- Add preview-before-consent, Basic-only consent receipts, atomic private
  outbox events, deterministic idempotency, manual export, revoke, and explicit
  pending-event deletion.
- Keep production ingest, databases, dashboards, Diagnostic per-task data,
  background jobs, hooks, and automatic upload out of this release.
- Define schema, retention, confidence, quality guardrails, threat model,
  private-infrastructure boundaries, and staged follow-up in one packaged
  contract.
- Update release and weekly-audit guidance for the authenticated private
  GitHub repository and preserve Chronicle automation, ON/OFF state, and
  experiment-ledger ownership.

## v0.6.1 - 2026-07-27

- Add one installed weekly-audit contract so scheduled tasks resolve the active
  Git marketplace source, package, installed cache, frozen App catalog, and hook
  state without carrying a duplicated long prompt.
- Add a bounded read-only weekly aggregator that selects completed roots once
  and separates root, related delegated-agent, and canonical Guardian usage
  without emitting prompts, thread IDs, rollout paths, or private paths.
- Make Guardian usage match the general session contract: prefer the final
  cumulative `total_token_usage` snapshot per rollout and sum
  `last_token_usage` only as a fallback.
- Keep Chronicle automation, ON/OFF state, memory, and experiment ledgers
  outside GroundLine ownership while allowing the existing optional redacted
  five-count fusion.

## v0.6.0 - 2026-07-27

- Make Codex App and its bundled CLI the primary runtime and keep PATH CLI as
  the packaging, automation, and secondary-channel evidence surface.
- Add a native Goal-aware operating loop that collects and synthesizes
  observations before freezing one implementation batch.
- Add deterministic task-boundary decisions for stay, side question, fork,
  compact packet, and new task without creating or editing provider state.
- Add `groundline_codex_efficiency.py` for Goal batch assessment, transparent
  conservative/expected/optimistic usage simulation, and fusion of exact Codex
  counters with strict redacted Chronicle behavior counts.
- Preserve Chronicle ownership: GroundLine changes no Chronicle setting,
  automation, ON/OFF state, recording, memory, or experiment ledger.
- Bundle one trust-gated `PreToolUse` release advisory hook. It reads only the
  current Bash input, never logs commands, and never blocks, rewrites, approves,
  or executes a tool call.
- Declare the official OpenAI documentation MCP as an explicit dependency of
  the Codex surface-alignment skill instead of bundling a GroundLine MCP.
- Keep the six-skill/two-implicit surface, provider-owned model settings,
  explicit-only subagents, and source/package/install/fresh-App-task proof.

## v0.5.2 - 2026-07-24

- Separate database integrity, thread-inventory consistency, storage pressure,
  and observed process pressure so accumulated storage alone no longer produces
  a critical runtime-health verdict.
- Honor explicit `--codex-home` and `CODEX_HOME` locations without printing
  configured private paths; preserve `--home` as the deterministic fake-home
  boundary.
- Add state-database quick-check plus aggregate active and archived stale
  rollout counts without emitting rollout paths or stored content.
- Replace duplicated Codex source constants with one schema-v2 runtime
  contract containing reviewed evidence-file hashes, and add a read-only local
  checkout drift audit.
- Align usage guidance with the audit implementation: use one final cumulative
  `total_token_usage` snapshot per rollout and sum `last_token_usage` only as a
  fallback.
- Remove stale version-specific maturity and roadmap claims, and correct the
  obsolete statement that GroundLine still packaged a read-only MCP.

## v0.5.1 - 2026-07-24

- Remove the `PreCompact` hook because current Codex accepts only universal
  output fields for that event and does not accept injected additional context.
  Rely on native compaction and the explicit handoff skill instead.
- Add a pinned Codex source-contract snapshot for hook output, skill metadata
  budget, model effort, multi-agent mode, and history-mode behavior.
- Report both GroundLine house ceilings and the Codex-native two-percent skill
  metadata budget, including implicit catalog pressure and cache-stability
  markers.
- Extend the read-only doctor with desktop-bundled versus PATH CLI channels,
  legacy versus paginated history counts, and aggregate active rollout size
  bands without emitting task paths or content.
- Warn when Ultra's proactive delegation conflicts with active instructions or
  an unnecessarily high concurrency limit.
- Replace source-checkout-only update guidance with explicit GitHub marketplace
  release-pin and development-channel workflows.
- Refresh maturity and next-version documentation after installed v0.5.0 proof.

## v0.5.0 - 2026-07-24

- Reduce the Codex-native package to six focused skills and one trust-gated,
  content-free `PreCompact` hook.
- Remove the redundant GroundLine MCP, custom agents, project rules, project
  setup writer, eight overlapping skills, and their runtime/package payloads.
- Cut the pre-release suite from sixteen overlapping gates to eight bounded
  source, test, safety, privacy, staged-smoke, and platform checks; keep live
  installed-runtime proof separate.
- Add a redacted general Codex session audit that replaces repeated transcript
  queries with one compact usage, latency, model/effort, tool, retry, and task
  boundary evidence packet.
- Add token-efficient release gate output with redacted progress events,
  aggregate gate counts, and compact JSON that drops PASS logs while retaining
  failure diagnostics.
- Report the Codex task-start catalog boundary after plugin refresh: verify in
  a new task first and restart only if that fresh task remains stale.
- Add a metadata-only inventory for all 115 unique pages in the official Codex
  manual, a compact Codex-native guidance map, and a drift audit in the release
  gate without copying official page bodies into the prompt surface.
- Keep one metadata-only official Codex manual registry and remove the second
  overlapping content registry.
- Align the fast model and effort path with the official lower-effort-first
  posture: Sol medium for normal difficult work, Sol high for named complexity
  or risk, and xhigh only after a credible lower-effort attempt or equivalent
  risk justification.
- Aggregate Codex token evidence from one final cumulative snapshot per rollout
  and use summed per-turn usage only as an explicit fallback for older logs.
- Scope design-document routing to the nearest applicable `AGENTS.md` chain so
  unrelated subtrees cannot satisfy each other's guidance requirements.
- Reduce the installable package to 34 runtime files: six skills, one hook, six
  referenced guides, six invoked scripts, assets, manifest, and public metadata.
  Keep release, validation, documentation, scenarios, and CI tools source-only.
- Remove the unused prompt context probe, plan-update adapter, packaged-doc
  counter, and duplicate packaged-validator CI invocation.
- Distinguish the `--require-installed` policy from whether installation action
  is actually required, and include plugin assets in content fingerprints.
- Make provider-package sync a true no-op when source and packaged files already
  match, including release validation from a read-only Linux checkout.

## v0.4.1 - 2026-07-23

- Resolve shared references, scripts, docs, and skill paths from each
  `SKILL.md` directory so explicit skills can load their packaged resources in
  a real Codex invocation.
- Make all three plugin-card starter prompts explicitly select their intended
  namespaced GroundLine skill without increasing the prompt count.
- Add a static regression contract that rejects ambiguous plugin-root paths and
  missing shared resources before release.

## v0.4.0 - 2026-07-23

- Make Codex App and Codex-native reasoning the default worker; GroundLine
  now acts only as a compact evidence, continuity, and maintenance layer.
- Reduce the installed skill surface from 19 to 14 by merging the four-stage
  ecosystem chain into `agent-ecosystem-radar` and retiring `hold-the-line`.
- Allow implicit Codex invocation only for `reconcile-current-state` and
  `close-live-work`; history, config, release, ecosystem, and maintenance
  workflows are explicit-only.
- Remove the previous companion profile and active package guidance in favor of
  provider-native workflow boundaries.
- Add a GPT-5.6-aware Codex professional profile without adding skills, hooks,
  MCP servers, model routing, prompt logging, or always-on telemetry.
- Extend the explicit history-to-maturity flow with redacted usage evidence,
  Codex efficiency scoring, an effort escalation ladder, and one bounded
  behavior experiment.
- Add a source-backed, lazy-loaded GPT-5.6 model-by-effort matrix that separates
  Sol, Terra, Luna, Low through Extra High, Max, Ultra, and Fast service tiers
  without creating an implicit router or pinning provider settings.
- Add a redacted Codex Auto-review audit that separates reviewer usage from the
  main agent and detects temporary or external workspace-boundary churn without
  emitting commands, patches, transcript content, or paths.
- Add an opt-in Codex long-session health probe that aggregates app-server,
  MCP-process, task-metadata, and SQLite pressure without emitting commands or
  stored content and without performing cleanup or changing provider settings.
- Add a deterministic Socratic growth loop that turns redacted numeric signals
  into one challenge question, keeps one active experiment, and stores only
  derived opt-in state outside provider homes.
- Add a protected baseline and autonomous correction decisions so efficiency
  experiments halt on unapproved goal changes, quality, verification, rework,
  safety, permission, privacy, or irreversible-side-effect regression without
  self-modifying.
- Add qualitative context and delegation budgets to task packets and static
  `context_surface` regression metrics to package validation.
- Tighten the static surface to 5,800 total skill words, 700 implicit-skill
  words, 2,200 catalog characters, and at least 10% prose headroom; compress the
  two implicit skills and the largest explicit guidance without changing their
  output contracts.
- Add a redacted Codex prompt-input counter, a Codex-only release scope, local
  placeholder-file preflight, and pruned single-pass project scanning.
- Turn `align-agent-home` into a provider-native project and home auditor that
  preserves official plugins, routes design documents on demand, and reports
  exact skill-name collisions without printing configuration values.
- Add `groundline_project_audit.py` for read-only Codex guidance, plugin, skill,
  MCP, rule, hook, and runtime inventory.
- Update Codex staging and remote-install proof to the current full plugin
  package contract.
- Document Codex context-surface inspection and the current native component
  boundaries while keeping GroundLine skills-only by default.

## v0.3.5 - 2026-05-29

- Define v0.3.5 as the remote install and update proof patch, carrying forward
  the local v0.3.4 proof-quality work while adding a fake-home update harness
  before any public tag or provider-home mutation.
- Add `groundline_remote_install_probe.py` to prove fresh install, stale
  previous-version detection, and post-update refresh across Codex, Claude Code,
  and Antigravity without touching the real provider home.
- Add the remote install/update proof to the local release gate and install,
  update, and release checklist docs so update confidence is checked before
  claiming a public package is current.
- Refresh post-release planning and maturity docs so v0.3.4 starts from the
  published v0.3.3 baseline and names provider install refresh as the expected
  follow-up when source content changes after release.
- Define the v0.3.4 release cut as a proof-quality patch: provider install
  refresh, live activation proof rows, release delta evidence, and validation
  closeout without new skills, runtimes, hooks, MCP setup, or lifecycle
  promotion.
- Record local v0.3.4 provider refresh and release delta evidence: Codex and
  Claude Code direct provider targets match the packaged payload, provider smoke
  passes, and the full local release gate passes including Linux Docker
  execution.

## v0.3.3 - 2026-05-28

- Version-aware provider smoke now reports installed version, source version,
  install source, cache candidates, payload presence, skill count drift,
  same-version content drift, `install_doctor_status`, and
  `secret_value_printed=false` for Codex, Claude Code, and Antigravity, with
  provider-level `recommended_actions` and top-level `next_actions`.
- Add `--require-installed` to provider smoke and use it from the release gate
  so missing provider targets are only accepted during package/path validation,
  not during post-install release proof.
- Add single-source version control so validation compares provider manifests
  against canonical `plugin.json` instead of a hard-coded patch version.
- Add a provider activation matrix and expand staged dogfood to six prompt
  families while keeping live provider proof separate from staged contract
  checks.
- Align the AI usage maturity activation matrix row with the canonical
  `GroundLine AI Usage Maturity` output contract.
- Add a skill graduation plan with machine-readable decisions for all 12
  experimental skills. No lifecycle values are promoted in this patch.
- Add a workflow cookbook that maps five common prompts to selected skills,
  output contracts, verification evidence, and stop conditions.
- Add an artifact lifecycle map for research packet, comparison report,
  upgrade decision, implementation task, dogfood evidence, release cut, and
  release delta handoffs.
- Add a release gate runner that prints or executes the local release gate
  sequence while excluding approval-required tag, push, and GitHub Release
  commands, and preserves compact JSON summaries plus top-level next actions
  for partial gates.
- Add an optional `--release-version` release gate preflight so actual release
  cuts fail when source or packaged manifests still point at the wrong version,
  or when the requested version is not plain `X.Y.Z` semver.
- Add staged provider smoke so a fake refreshed install can be proven with
  `--stage-package --require-installed` before touching real provider homes.
- Preserve staged provider smoke summary fields in release gate output and
  refresh maturity evidence against the current remote CI run.
- Add a provider-native validation gate for read-only Claude Code and
  Antigravity package validation during local release closeout.
- Redact local home paths from release gate and Docker scenario evidence
  outputs before they are copied into release review.
- Separate approval-required publishing commands from read-only release
  evidence in the public release checklist.
- Document the exact version bump sequence for source manifests, package sync,
  validation, changelog movement, and `v`-prefixed release tags.
- Add the deterministic offline safety eval harness to the default CI release
  gate and manual release evidence checklist.
- Add a deterministic privacy scan gate for local home paths, generic
  secret-like values, dynamically checked stale test proof counts, stale remote
  CI run claims, and overstated release claims.
- Align README and update validation docs with source, packaged, safety,
  privacy, smoke, dogfood, and scenario release gates.
- Keep package validation strict for conflict-copy payloads while ignoring
  empty conflict-copy directories that contain no files.
- Prevent installable provider package copies from running package sync and
  creating nested `plugins/groundline-insights` payloads.

## v0.3.2 - 2026-05-28

- Clarify routing boundaries for ecosystem research, single-candidate
  evaluation, candidate comparison, and GroundLine upgrade recommendation.
- Separate staged dogfood contract checks from real provider invocation proof.
- Link provider-history inventory to AI usage maturity assessment through a
  redacted Provider Evidence Packet.
- Make release triage priority explicit across scope hold, pre-ship polish, and
  final ship decision skills.

## v0.3.1 - 2026-05-28

- Record the Claude Code follow-up proof that reduces the v0.3.0 release
  closeout partial when read-only skill document inspection is allowed.
- Keep the Antigravity constrained print-mode proof as an explicit accepted
  defer while package validation and install validation remain passing.
- Add optional provider guardrail and MCP recipe docs for Codex, Claude Code,
  and Antigravity without enabling hooks, rules, MCP servers, commands, or
  provider-level agents by default.
- Record local companion-workflow dogfood showing GroundLine as the state,
  side-effect, live-proof, and release-control layer while the provider owns
  planning, testing, debugging, review, and final verification discipline.

## v0.3.0 - 2026-05-28

- Add provider marketplace packaging for Codex and Claude Code, plus an
  Antigravity install surface and provider packaging guide.
- Add Korean companion docs for human-facing setup, workflow selection, skill
  overview, privacy, release, and next-version planning while keeping English
  as the default and canonical documentation language.
- Add a language policy and validation coverage for bilingual human docs.
- Add sanitized provider invocation proof schema and dogfood evidence for Codex,
  Claude Code, and Antigravity.
- Add an offline safety evaluation harness with synthetic cases for secret-like
  output, destructive command pressure, false completion claims, and unsafe
  provider-home writes.
- Keep Claude Code contract naming and Antigravity constrained print mode as
  explicit accepted partials for the next patch instead of masking them as
  passing.

## v0.2.2 - 2026-05-28

- Add the next work backlog for provider invocation dogfood, safety evaluation,
  representative workflows, artifact lifecycle, ecosystem refresh, and install
  UX.
- Link the backlog and next version plan from README and make package
  validation require both.

## v0.2.1 - 2026-05-28

- Add a provider dogfood harness for staged package, runtime probe, and shared
  scenario contract validation.
- Add provider dogfood runbook and release checklist gate.
- Record v0.2.1 dogfood evidence and accepted defers.

## v0.2.0 - 2026-05-28

- Add public release, privacy, contribution, and security documentation.
- Add separate human and LLM guides plus GitHub issue and pull request templates.
- Add git history privacy guidance for public repository preparation.
- Replace personal author metadata with GroundLine contributor branding.
- Redact default home paths in doctor and provider smoke output.
- Align upgrade packet secret-like input detection with doctor and radar checks.
- Verify the pinned actionlint archive checksum in CI.
- Add an agent ecosystem radar skill set for research, comparison, and upgrade recommendations.
- Add a GroundLine pack evaluation skill for skill completeness and release readiness review.
- Add human-readable skill portfolio docs and an LLM-readable skill index.
- Add skill lifecycle taxonomy and curation guidance.
- Add read-only provider smoke runtime probes for staged install targets.
- Add an existing capability evaluation skill and rubric for tools, skills, plugins, MCP servers, hooks, and agents.
- Add an AI usage maturity assessment skill and rubric for evidence-backed workflow improvement.
- Add task packet and release stabilization skills for context packaging, scope lock, dogfood evidence, and ship decisions.
- Add a release delta comparison skill for post-deploy checklists against the previous version.

## v0.1.1

- Force GitHub JavaScript actions onto Node 24 in validation and radar
  workflows.
- Pin GitHub Actions to current release tags and install pinned actionlint from
  its prebuilt Linux binary.

## v0.1.0

- Add GroundLine manifests for Codex, Claude Code, and Antigravity.
- Add six workflow skills for state reconciliation, history audit, side-effect
  boundaries, live evidence, provider home alignment, and worktree recovery.
- Add stdlib-only doctor, radar, upgrade packet, provider smoke, package
  validation, and scenario scripts.
- Add opt-in external tool probes and command sources with secret-like output
  redaction.
- Add GitHub Actions validation and scheduled radar workflows.
- Add macOS local and Linux Docker scenario gates.
- Add install, update, provider smoke, runtime support, examples, and release
  documentation.
