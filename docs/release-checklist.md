# Release checklist

1. Freeze scope and update the workspace plus both plugin manifests to the same
   strict semantic version.
2. Run formatting and targeted CLI/xtask tests while editing. On final source,
   run workspace tests, Clippy, dependency policy, source verification, and
   `git diff --check` once.
   Include window partition, partial-read recovery, prepared-event restart,
   bounded retries/diagnostics, and real loopback capability preflight tests.
   Confirm owner APIs advertise the new ingest contract before collector
   upgrades; public stable promotion alone cannot prove private API readiness.
3. In the isolated qualification ClickHouse, run the explicit mutation-enabled
   integration lane. It must exercise API-owned schema migration, enrollment,
   accepted upload, duplicate retry, weekly reporting, every Grafana query, and
   authenticated collector deletion. Validate the selected four-component
   infrastructure compatibility profile first; never mix a partial candidate
   with the release-tested defaults implicitly.
4. Validate both canonical manifests. Confirm Core has zero hooks and Insights
   has exactly four fail-open hooks using `groundline-insights`. Confirm public
   metadata and documentation describe Core-only, Insights-only, and combined
   installation without implying an automatic sibling dependency.
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
8. Build both binaries for all six targets from the exact release commit. Verify
   each product's target set, executable name, manifest, size, and SHA-256. Remap
   GitHub runner workspace and home paths before compiling release binaries.
9. Publish one versioned GitHub release without replacing an existing release,
   and publish the Insights API image. Record the launch smoke for both API
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
   `GROUNDLINE_INSIGHTS_GRAFANA_ADMIN_PASSWORD` from owner-private local state.
   Pass the exact owner-rendered private Compose file explicitly, confirm it is
   a bounded regular file private to the current user rather than a symlink, and
   that the authenticated, redacted preflight passes before any mutation.
11. Assemble both plugin binary trees in one generated commit and advance `stable`
   only with a verified tag, monotonic version, exact artifact diff, and
   `force-with-lease`.
12. Refresh the marketplace in Codex. Verify Core and Insights package fingerprints
   independently, then verify hook dispatch, upload, ClickHouse, Grafana, and any
   production deployment as separate live lanes.
   Version tags contain source; `stable` adds the verified native binaries.
   If refresh leaves an installed version unchanged, reinstall only that same
   plugin ID from the refreshed distribution and verify its checksum.
13. Keep the generic Compose path labeled public preview until the exact release
   passes on a fresh host and another Tailnet node verifies TLS reachability,
   unauthenticated rejection, authenticated dashboard access, collector upload,
   ClickHouse visibility, and Grafana frames.
14. To extend support to newer infrastructure, run the manual workflow with all
   four candidate inputs. After the live mutation and Grafana semantic lanes
   pass, replace moving tags with resolved digests and the installed plugin with
   its exact stable version, rerun without the override, and only then update the
   release-tested profile in a reviewed change.
