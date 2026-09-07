# Changelog

## Unreleased

## 0.21.0

- Add `config-audit` for private, offline model/effort posture checks against a
  supplied native catalog. Keep effective settings and native schema separate.
- Share skill metadata validation with packaging and shorten alignment guidance
  through on-demand configuration and Astra references.

- Add native guidance audit/snapshot commands and an align-agent-home
  maintenance workflow for imported skills, source provenance, local safeguards,
  and scoped regressions. Separate host-local profiles from path-free baselines;
  discover additions/removals on every audit without a legacy registry adapter.
  Keep private receipts separate from content backups
  and preserve Codex ownership of execution, discovery, and upgrades.

- Align skills with current-model instruction following: preserve scoped
  approval, make native Goals optional, bound retries and report size, and
  separate reasoning effort from delegation authority.
- Resolve installed commands consistently on all six target platforms; clarify
  Core versus optional Insights hooks and safe guidance-review output.
- Validate skill metadata, invocation policy, index parity, and local links in
  the source gate; keep behavioral evaluation a separate evidence lane.

- Preserve historical event windows when a thread resumes, reconcile native
  cumulative checkpoints, and mark unanchored mixed usage incomplete.
- Bound diagnostics and parser work; retry only transient representation
  disappearance once. Extend opt-in release benchmarks with three workloads.
- Borrow unused rollout payloads instead of allocating full JSON trees, reuse
  metadata classification, and remove repeated statistics sorting and copies.
  Add an opt-in synthetic benchmark with deterministic aggregate fingerprints.
- Read Codex's plain and Zstandard-compressed rollouts through a bounded shared
  reader, deduplicate representation paths, and select the latest numeric state
  store without falling back to stale or rejected copies.
- Correct resumed-task selection, cross-task call/model accounting, paired
  compaction counts, nullable effort metadata, and incomplete audit status.
- Centralize privacy-safe model labels with Astra support; preserve Codex-owned
  model selection, permissions, context management, and plugin upgrades.
- Attribute native shared-history suffixes using ordinals and deduplicated,
  matching-thread response usage. Keep unread prefixes and legacy histories
  without ownership boundaries partial instead of counting parent totals.

## 0.20.2

- Harden local Codex state and rollout reads against symlink, ownership,
  traversal, oversized metadata, and unbounded row-allocation attacks while
  retaining read-only audit behavior on supported desktop platforms.

## 0.20.1

- Make the Core-only install path explicit and document that Core never installs
  or activates the independently packaged Insights plugin.
- Clarify that agents must resolve the packaged native executable from the
  installed plugin root instead of assuming Codex adds it to the user shell's
  `PATH`.

## 0.20.0

- Make `plugins/groundline` the only canonical Core package in the monorepo.
- Keep Core offline and zero-hook while sharing versioned Rust contracts and a
  single marketplace/release channel with the optional Insights plugin.
- Remove root package synchronization and verify the canonical package directly.

## 0.19.0

- Establish a clean public, local-first GroundLine core with no lifecycle hook,
  network client, background worker, remote destination, or collector identity.
- Keep bounded local Codex audits, project configuration inventory, deterministic
  efficiency contracts, and six-target native packaging.
- Add a zero-hook provider smoke contract and a public-readiness gate that rejects
  private infrastructure markers, personal paths, and package drift.
- Keep GitHub Actions cost-bounded: pull requests run fast checks, while full
  qualification and release artifacts require explicit manual dispatch.
