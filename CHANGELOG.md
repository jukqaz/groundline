# Changelog

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
