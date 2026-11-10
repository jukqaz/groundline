# Release checklist

Select one product: `core-vYEAR.MMDD.N` or `insights-vYEAR.MMDD.N`.
Core publishes only Core binaries and advances `core-stable` / `groundline-core`.
Insights publishes its collector and API image and advances `stable` / `groundline`.
Steps involving API compatibility, ClickHouse, Grafana, collection or deployment
apply only to an Insights release. A Core release preserves those deployed systems
and installed state; no server qualification or image job is required for it.

1. Freeze scope and update the workspace plus both plugin manifests to the same
   strict semantic version.
2. Run formatting and targeted CLI/xtask tests while editing. On final source,
   run workspace tests, Clippy, dependency policy, source verification, and
   `git diff --check` once.
   Include window partition, partial-read recovery, prepared-event restart,
   bounded retries/diagnostics, and real loopback capability preflight tests.
   Confirm owner APIs advertise the new ingest contract before collector
   upgrades; public stable promotion alone cannot prove private API readiness.
   Both native PR and artifact jobs must execute
   `improvement_cli`, setup, config-repair and shell installer suites.
   Native PR jobs additionally execute the real Codex independent-installation
   regression on every supported target. Listing tests or applying a test-name filter does
   not qualify these suites.
   Before claiming improved skill behavior, compare
   candidate and current guidance on representative matched work with the same
   inputs and acceptance criteria. Protect completion quality and rework before
   optimizing owned tokens or elapsed time; include child, approval, failure,
   and retry resources. Unknown outcomes do not pass this gate. Deterministic
   tests are not proof of behavioral improvement. Report missing natural follow-up
   separately from source/package qualification.
3. In the isolated qualification ClickHouse, run the explicit mutation-enabled
   integration lane. It must exercise API-owned schema migration, enrollment,
   accepted upload, duplicate retry, weekly reporting, every Grafana query, and
   authenticated collector deletion. Validate the selected four-component
   infrastructure compatibility profile first; never mix a partial candidate
   with the release-tested defaults implicitly.
4. Validate both canonical manifests. Confirm Core has zero hooks and Insights
   has exactly five fail-open hooks using `groundline-insights`. Confirm starter
   prompts contain at most three nonempty entries of at most 128 characters
   each, and neither native CLI emits a manifest warning.
   Confirm public metadata and documentation describe separate Core and Insights
   distributions; selecting both means installing each independently.
   Ship only Core, Insights, and the API; no separate GUI artifact or build job.
   Verify the installed native collector and a real Codex lifecycle hook,
   preserving existing settings and consent. Record an accepted receipt and
   matching storage query.
5. Confirm current source and every Git object reachable from branches, remote
   refs, and tags contain no production endpoint, credential, personal path,
   infrastructure inventory, deployment receipt, Python runtime dependency, or
   duplicate root package. A later deletion does not repair public history.
   Audit public Actions logs, artifacts, release assets, and image attestations
   separately: clean Git history does not prove these surfaces are clean.
   The Docker builder must use `provenance-add-gha=false` to exclude raw GitHub
   event payloads (which may contain private email addresses). Keep maximum
   build provenance, SBOMs, and signed attestations enabled; disable automatic
   Docker build-record uploads. Existing published records are not repaired by
   a workflow change. Inventory and privately back up affected objects before
   requesting owner approval for cleanup or replacement.
6. On the release tag, boot the rendered ClickHouse, API, and Grafana stack once.
   Require API storage readiness, every provisioned dashboard query, and semantic
   fleet/roster/storage frames to pass through Grafana itself. Confirm the
   one-shot Grafana storage initializer succeeds from a `0750` bind directory,
   the dedicated API ingress bridge is separate from Grafana's plugin-download
   egress, and an unauthenticated dashboard request is redirected to login.
   The template itself must contain only dependency placeholders; the selected
   profile fingerprint and pin status belong in the renderer receipt.
7. Run the manual qualification workflow once if the release tag path has not
   already done so.
   To prepare a candidate before merging, dispatch the exact candidate branch
   with `release_product=core|insights` and `build_release_artifacts=true`.
   This builds the selected product's three target packages
   after qualification; release, image publication, and stable promotion still
   require a version tag. Candidate artifacts do not replace tagged release
   assets or their source attestations.
8. Build the selected binary for the three supported macOS/Linux targets from the exact release commit. Verify
   its target set, executable name, manifest, size, and SHA-256. Remap
   GitHub runner workspace and home paths before compiling release binaries.
9. Publish one versioned product GitHub release without replacing an existing release:
   three selected binary archives plus `SHA256SUMS`. Only Insights tags publish
   the API image. For Insights, record the launch smoke for both API
   artifacts after downloading them from Actions:
   the image must restore executable permissions lost during artifact transport,
   and its real entrypoint must reach the API configuration check without secrets.
   Record the multi-platform image index digest, and verify every binary asset with
   `gh attestation verify --repo jukqaz/groundline --source-digest <commit> --source-ref refs/tags/<tag> --signer-workflow jukqaz/groundline/.github/workflows/rust.yml --deny-self-hosted-runners <asset>`.
   Apply the same source/workflow checks to `oci://<image>@sha256:<digest>`.
   Record GitHub's `isImmutable` result separately: the workflow's no-overwrite
   check does not enable [GitHub release locking](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases).
   Confirm the normal renderer rejects a
   moving image tag or unversioned Grafana plugin without the explicit
   qualification-only override. Do not inject
   production deployment credentials into release jobs.
10. Before an owner-run TrueNAS preflight or apply, provide
   `GROUNDLINE_INSIGHTS_ENROLLMENT_TOKEN` and
   `GROUNDLINE_INSIGHTS_GRAFANA_ADMIN_PASSWORD` from owner-private local state
   for a Basic-enabled Grafana deployment. For an existing JWT deployment, pass
   a current owner-authenticated token in `GROUNDLINE_OWNER_GRAFANA_ACCESS_JWT`
   instead. Its dedicated `X-*` or `Cf-Access-Jwt-Assertion` header must match the
   private Compose file and current Grafana configuration. The controller sends
   it only to the bounded Grafana datasource probe, follows no redirects, and
   rejects disabled or mismatched authentication without changing Grafana login.
   Pass the exact owner-rendered private Compose file explicitly, confirm it is
   a bounded regular file private to the current user rather than a symlink, and
   that the authenticated, redacted preflight passes before any mutation.
11. Run `xtask stage-distribution --release-tag <product-tag> --packages <selected-packages>`
   on the clean tagged source. The generated commit adds only the selected nine
   artifact files, selected native catalog and `distribution.json` (eleven paths).
   Advance only that product's channel with a verified tag, source parent,
   manifest/checksum bytes, monotonic product version and `force-with-lease`.
12. Refresh the marketplace in Codex. Verify Core and Insights package fingerprints
   independently, then verify hook dispatch, upload, ClickHouse, Grafana, and any
   production deployment as separate live lanes.
   Version tags contain source; the selected product channel adds its verified binaries.
   If refresh leaves an installed version unchanged, inspect the reviewed source
   pin and cache evidence, then resume the reviewed installer. Preserve disabled
   state; re-adding an existing disabled plugin would enable it.
13. Keep the generic Compose path labeled public preview until the exact release
   passes on a fresh host and an authorized external client verifies TLS reachability,
   unauthenticated rejection, authenticated dashboard access, collector upload,
   ClickHouse visibility, and Grafana frames.
   Use another Tailnet node for the optional Tailnet mode, and preserve the
   network policy of existing deployments when updating their API image.
14. To extend support to newer infrastructure, run the manual workflow with all
   four candidate inputs. After the live mutation and Grafana semantic lanes
   pass, replace moving tags with resolved digests and the installed plugin with
   its exact stable version, rerun without the override, and only then update the
   release-tested profile in a reviewed change.
