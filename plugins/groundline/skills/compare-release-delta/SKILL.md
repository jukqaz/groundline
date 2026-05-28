---
name: compare-release-delta
description: Use when a deployed release must be compared with a previous version, expected changes, runtime evidence, regression signals, install state, rollback notes, or post-deploy checklist items.
---

# Compare Release Delta

## Purpose

Use this skill after a release or deploy is live. It compares the deployed
version with the previous version and turns the difference into a checklist.

## Workflow

1. Identify the previous version and deployed version from tags, commits,
   release notes, package metadata, provider smoke, or runtime output.
2. Build a delta checklist:
   - expected changes: items that should be present after the deploy.
   - unexpected changes: files, behavior, docs, config, manifests, or runtime
     output that changed without an explicit reason.
   - runtime evidence: endpoint, CLI, package, plugin, smoke, scenario, or
     user-flow proof.
   - install evidence: provider target, manifest, skill count, or package
     artifact state.
   - docs evidence: changelog, release notes, install/update notes, and
     public-facing labels.
   - regression evidence: tests, logs, known flows, old issue repros, or
     rollback-sensitive areas.
   - rollback note: what to revert, pin, disable, or re-run if the release
     fails.
3. Classify each checklist item as `pass`, `partial`, `fail`, or
   `not_checked`.
4. Separate expected gaps from release blockers.
5. Report whether the deployed version should be kept, monitored, or rolled
   back.

## Rules

- Do not mutate production, tags, releases, provider homes, or access.
- Do not treat a tag or CI success as runtime evidence.
- If the previous version is unknown, state the comparison gap before scoring.
- Keep post-deploy checks read-only unless the user explicitly approves a
  rollback or fix.
- Use `close-live-work` when runtime proof is missing.
- Use `stabilize-release-cut` if the release decision was never made.

## Output Contract

Use `GroundLine Release Delta` from `references/output-contracts.md`.
