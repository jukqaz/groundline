# GroundLine Core changes

This package summary covers the current release line. See the repository
[changelog](https://github.com/jukqaz/groundline/blob/main/CHANGELOG.md) for shared release and packaging changes.

## 0.25.1

- Prioritize GPT-6/Astra guidance while supporting GPT-5.6 and preserving selected settings.
- Reconcile weekly advice within the current authorized task without extra approval or recurring-review requirements.
- Propose diagnosis from aggregate failure signals, retaining quality/cohort context and strict personal-trial gates.

## 0.25.0

- Separate model-guidance review from Insights-based personal trials; preserve
  selected settings unless configuration setup is explicitly requested.
- Load the detailed personal trial workflow only when that route is requested.
- Scope reconciliation and live verification to the current outcome, reuse
  evidence, and retain the task through steering. Batch advice recommends new
  tasks or forks only for explicit requests and cannot complete a changed scope
  using the previous verification.

## 0.22.2

- Wait for authenticated API and Grafana readiness before the release stack
  checks anonymous dashboard access. Preserve the redirect and semantic assertions.

## 0.22.1

- Recover interrupted personal guidance restores and keep trial history,
  temporary files, candidate selection, and PR regression checks bounded.
- Audit larger native records without expanding unused history bodies, and
  recognize a directly proven fresh native usage baseline.

## 0.22.0

- Add `personal review|evaluate|rollback` and an explicit seventh skill for
  current-model workflow improvement. Keep private trials outside Git and
  require directly evidenced, disjoint, comparable outcomes before retention.
- Preserve native settings and user edits; do not apply changes from incomplete
  reports, short prompts, high effort, or aggregate repetition alone.

## 0.21.4

- Verify record timestamps when selecting native audit activity; metadata-only
  thread updates no longer reintroduce inactive inherited history.

## 0.21.3

- Publish alongside the API image launch-permission correction.

## 0.21.2

- Stream native audit inputs with separate I/O and retained-record budgets,
  preserving compaction and tool-result metrics while dropping unused bodies.
- Keep native and UI usage baselines independent and include active turns with
  stale sidebar recency. Keep unknown history ownership incomplete.
- Align installation, verification, and Korean guidance with the current native
  commands and consolidate obsolete documentation.

## 0.21.1

- Publish alongside the Insights correction for existing collection generations.

## 0.21.0

- Add offline `config-audit` against a supplied native model catalog.
- Add strict personal-skill audit/snapshot contracts and share skill metadata
  validation with packaging, preserving existing local changes and settings.
- Support current native compressed and shared-history inputs, bounded parser
  diagnostics, event-time windows, and explicit incomplete coverage.
- Keep scoped approvals, optional native Goals, user-selected models, and
  explicit delegation boundaries in the six packaged skills.

## 0.20.x

- Establish one canonical Core package, independent from optional Insights.
- Harden read-only state/rollout access and qualify six native targets.
- Resolve native binaries from the installed package without assuming `PATH`.

The [historical package changelog](https://github.com/jukqaz/groundline/blob/v0.21.1/plugins/groundline/CHANGELOG.md)
preserves earlier details. Core remains offline and installs no hooks.
