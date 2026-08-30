# Changelog

## 0.20.1

- Enforce explicit owner opt-in for Insights, preserve only the exact deployed
  private-state upgrade contracts, and expose actionable readiness, freshness,
  clock-skew, Tailnet, and delivery states.
- Skip detached workers while disabled, fail closed on malformed local state,
  and make error receipts honest when partial mutation is unknown.
- Add a tested fail-closed owner-profile example and clarify that native plugin
  executables must be resolved from the installed target directory rather than
  assuming a user-shell `PATH` alias.

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
