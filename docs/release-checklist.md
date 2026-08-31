# Release checklist

1. Freeze scope and update the workspace plus both plugin manifests to the same
   strict semantic version.
2. Run formatting and targeted CLI/xtask tests while editing. On final source,
   run workspace tests, Clippy, dependency policy, source verification, and
   `git diff --check` once.
3. In the isolated qualification ClickHouse, run the explicit mutation-enabled
   integration lane. It must exercise API-owned schema migration, enrollment,
   accepted upload, duplicate retry, weekly reporting, every Grafana query, and
   authenticated collector deletion.
4. Validate both canonical manifests. Confirm Core has zero hooks and Insights
   has exactly four fail-open hooks using `groundline-insights`. Confirm public
   metadata and documentation describe Core-only, Insights-only, and combined
   installation without implying an automatic sibling dependency.
5. Confirm source contains no production endpoint, credential, personal path,
   infrastructure inventory, deployment receipt, Python runtime dependency, or
   duplicate root package.
6. Run the manual qualification workflow once if the release tag path has not
   already done so.
7. Build both binaries for all six targets from the exact release commit. Verify
   each product's target set, executable name, manifest, size, and SHA-256.
8. Publish one immutable GitHub release and the Insights API image. Do not inject
   production deployment credentials into release jobs.
9. Before an owner-run TrueNAS preflight or apply, provide
   `GROUNDLINE_INSIGHTS_ENROLLMENT_TOKEN` from owner-private local state. Confirm
   that the redacted preflight passes before any mutation.
10. Assemble both plugin binary trees in one generated commit and advance `stable`
   only with a verified tag, monotonic version, exact artifact diff, and
   `force-with-lease`.
11. Refresh the marketplace in Codex. Verify Core and Insights package fingerprints
   independently, then verify hook dispatch, upload, ClickHouse, Grafana, and any
   production deployment as separate live lanes.
