# Changelog

## 0.21.3

- Set executable permissions inside the API image even when release artifact
  downloads reset file modes. Launch both downloaded architecture artifacts
  through the image entrypoint before publishing.

## 0.21.2

- Keep native response and UI usage totals on independent baselines, preserving
  reset and trailing-response checks for the selected source. Accept Codex's
  explicit subagent ownership boundary for paginated fork metadata.
- Stream large native histories into bounded audit projections, retain metric
  semantics, and reject other runtimes after metadata without reading their bodies.
- Include active turns whose sidebar recency predates their latest update.
  Preserve explicit incomplete results for unreadable or unattributed history.
- Consolidate package release summaries and language guidance, remove unwired
  scenario files, and align installation, history retry, and release verification
  documentation in English and Korean. Preserve older notes in Git history.

## 0.21.1

- Preserve an existing collector's active history generation by retrieving it
  during authenticated enrollment. Reuse the same identity and token, refresh
  enrollment metadata, and require ingest contract revision 3 before collection.
- Qualify nonzero-generation collection, immutable retries, and API reporting.
- Allow the owner profile's health endpoint through the collector URL guard so
  the real capability preflight can run before enrollment or upload.

## 0.21.0

- Clarify bring-your-own Insights instances, separate operator and collector
  credentials, and explicit enablement. Add regression coverage for independent
  owner configuration and rejection of management credentials in collector profiles.

- Exclude raw GitHub event payloads from Docker builder provenance and disable
  automatic build-record uploads while retaining maximum build provenance,
  SBOMs, and signatures. Add workflow regression guards and independent public
  log, artifact, and image-metadata privacy gates to the release checklist.

- Qualify Insights' direct native-Codex collection path without inference proxy,
  generated catalog, or Core dependencies. Unify source discovery and readiness,
  reject blocking FIFO inputs, and add isolated native-state/outbox regression tests.

- Add offline native-catalog configuration checks without pinning models or
  rewriting settings. Share skill frontmatter validation between Core and
  packaging, and simplify the alignment skill with focused Astra guidance.

- Remove the old Insights consent, private policy, and private status import
  paths. Reject unsupported local state before enablement writes; keep current
  consent, bounded retries, and pending-data protections without auto-migration.

- Integrate personal skill maintenance into Core with strict host profiles,
  portable baselines, fresh inventory, upstream comparison, and new private
  receipts. Remove the unreleased personal-registry adapter and init command. Reuse
  align-agent-home for reviewed updates and scoped behavior tests without
  overwriting personal skills, adding hooks, or uploading private state.

- Harden current-model guidance, optional Goal handling, permission boundaries,
  and bounded verification. Add structured skill metadata and reference checks;
  see [guidance validation](docs/guidance-validation.md) for behavior acceptance.

- Preserve event-time windows after later task updates; initialize collection
  with an explicit seven-day lookback rather than thread modification times.
- Persist one frozen collection window and exact prepared event, publish only
  complete owned-scope aggregates, and stop automatic reads after three failed
  attempts. Keep partial windows, outbox durability, and delivery state distinct.
- Reconcile native and legacy cumulative checkpoints without summing duplicate
  sources; surface unanchored mixed usage and counter resets as incomplete.
- Bound diagnostic examples, record count, and record bytes; retry only a
  missing plain/compressed representation once and normalize consent timestamps.
- Check API ingest capabilities before cached-token enrollment or upload;
  preserve pending data and report `api_upgrade_required` on incompatible APIs.
- Reduce audit parsing and statistics allocations without changing aggregate
  output; add a private-data-free, opt-in repeatable performance benchmark.
- Follow the active Codex model catalog without pinning model or effort;
  centralize bounded Astra-aware telemetry dimensions and usage provenance.
- Support bounded Zstandard rollouts and native shared-history suffix usage,
  numeric state-store discovery, and explicit incomplete-read coverage.
- Correct resumed-task selection, activity export, cross-task attribution, and
  paired compaction counts, with regression and isolated Insights query tests.
- Document Codex-owned permissions, context management, and API-first rollout.

## 0.20.2

- Restrict weekly reports to the owner admin credential and require the Insights
  CLI to read that credential from an explicit private, no-follow token file.
- Isolate authenticated request budgets by role and collector, cache bounded
  readiness probes, and cap concurrent storage work and active rate scopes.
- Reject contradictory, overflowing, and high-cardinality event metrics before
  ClickHouse insertion; enforce per-collector and global storage watermarks with
  a bounded retention TTL while preserving idempotent retries.
- Harden local Codex audit reads against symlink, ownership, traversal, oversized
  metadata, and unbounded row allocation across macOS, Linux, and Windows.
- Build the API image from separately verified stable-Rust musl binaries using a
  digest-pinned final image, bounded Docker context, checksums, OCI provenance,
  SBOMs, and registry attestations instead of a mutable in-Docker toolchain.
- Separate collection cadence from bounded delivery retries, cap the private
  outbox at 256 events and 16 MiB, drain 16-event batches with durable backoff,
  and capture every hook trigger before starting a detached worker.
- Require explicit re-consent before replacing a legacy no-network receipt,
  issue a new owner-service receipt, and preserve incompatible pending events
  in a private quarantine instead of uploading or deleting them.
- Apply Tailnet-peer and global pre-authentication budgets before collector
  body reads and lookup, bound body-read time and concurrent requests, shed
  saturated collector work without queued waiters, reserve operator storage
  capacity, and keep readiness probes single-flight outside the cache lock.
- Preserve permanent-rejection stops across automatic cycles, classify 4xx
  before parsing its body, checkpoint accepted delivery before deleting outbox
  files, and claim hook markers so a concurrent later capture cannot be lost.
- Attest every binary release asset, surface eventual ClickHouse TTL cleanup in
  reports and Grafana, and require private no-follow TrueNAS runtime inputs.

## 0.20.1

- Remove baked-in ClickHouse, Nginx, Grafana, and datasource-plugin versions
  from the public Compose template. Select them through a strict compatibility
  profile, accept a complete newer candidate set in manual qualification, and
  run that candidate through the real mutation and authenticated Grafana query
  lanes without silently changing the release-tested default or production.
- Add a reachable Git-object privacy gate so deleting a leaked source file no
  longer makes release qualification pass while the old blob remains public.
  Scan binary markers too, distinguish exact public GitHub runner roots, and
  remap runner workspace and home paths from future release binaries. Normalize
  only the scanner's marker declaration instead of excluding its whole source
  blob, so a separate leak in that file still fails qualification. Inventory
  historical file names independently so a deleted forbidden secret-file name
  cannot be hidden by blob reuse under another path.
- Add a release-only rendered stack gate that boots ClickHouse, the Axum API,
  and Grafana, then executes every dashboard query through the provisioned
  datasource and validates semantic frames.
- Generalize private Compose dataset roots for Linux, macOS, and Windows Docker
  hosts, move the canonical template out of the TrueNAS-specific path, and add
  end-to-end self-hosting instructions.
- Document independent Core-only, Insights-only, and combined installation
  profiles plus the supported Codex, Tailnet, API, ClickHouse, Grafana, Docker
  Compose, and TrueNAS integration boundary.
- Enforce explicit owner opt-in for Insights, preserve only the exact deployed
  private-state upgrade contracts, and expose actionable readiness, freshness,
  clock-skew, Tailnet, and delivery states.
- Skip detached workers while disabled, fail closed on malformed local state,
  and make error receipts honest when partial mutation is unknown.
- Add a tested fail-closed owner-profile example and clarify that native plugin
  executables must be resolved from the installed target directory rather than
  assuming a user-shell `PATH` alias.
- Require immutable API image digests for normal self-hosted renders, make the
  mutable CI/development exception explicit and machine-auditable, and reject
  unauthenticated Grafana access in the release-only live stack gate.
- Disable Grafana anonymous access, initialize its bind directory with a
  one-shot least-privilege service instead of world-writable permissions, and
  separate the dedicated published-port ingress bridge from Grafana's
  plugin-download egress.
- Authenticate both generic-stack and optional TrueNAS controller Grafana
  semantics checks with owner-local credentials; no secret enters public CI or
  verification receipts. Require the TrueNAS controller's owner-rendered
  Compose input explicitly so it cannot mistake the public placeholder template
  for deployable configuration.
- Reject aliased template, rendered Compose, and secret-store paths; require
  every deployment placeholder before generating credentials and fail closed if
  either generated file is not private to the current user.

## 0.20.0

- Publish one public monorepo with two canonical, independently installable
  plugins: offline zero-hook Core and opt-in self-hosted Insights.
- Remove duplicate root package surfaces and the obsolete separate Insights
  marketplace/repository contract.
- Require a distinct owner-issued enrollment credential in addition to Tailnet
  reachability, and keep it outside the sanitized owner profile.
- Build both binaries for six targets in one cost-bounded workflow, publish a
  multi-architecture API image, and promote both plugin packages atomically.
- Reject malformed or version-mismatched tags before the expensive matrix and
  publish the API image only after every native artifact succeeds.
- Preserve an existing TrueNAS enrollment credential or inject one from an
  owner-local deployment input during migration, without exposing it in Git,
  CI, or deployment receipts.
- Complete RustSec, license, source-privacy, native package, ClickHouse schema,
  and Grafana-query qualification for the public source.
- Make the Insights API the single active ClickHouse schema migrator, reconcile
  weekly report quality reasons with the strict schema-3 contract, and qualify
  enrollment, idempotent retry, reporting, every Grafana query, and deletion
  against a real isolated ClickHouse.
- Reconcile Grafana provisioning, expand private-artifact rejection, and replace
  the quadratic source marker scan with one multi-pattern pass.

## 0.19.0

- Establish a clean public, local-first GroundLine core with no lifecycle hook,
  network client, background worker, remote destination, or collector identity.
- Keep bounded local Codex audits, project configuration inventory, deterministic
  efficiency contracts, and six-target native packaging.
- Add a zero-hook provider smoke contract and a public-readiness gate that rejects
  private infrastructure markers, personal paths, and package drift.
- Keep GitHub Actions cost-bounded: pull requests run fast checks, while full
  qualification and release artifacts require explicit manual dispatch.
