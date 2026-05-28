---
name: polish-release-candidate
description: Use when a release candidate needs final cleanup, docs polish, duplicate or stale surface review, privacy checks, validation gate ordering, commit split planning, or public readiness review before a ship decision.
---

# Polish Release Candidate

## Purpose

Use this skill after scope is locked but before the final ship decision. It
turns repeated pre-release cleanup into a small, evidence-backed pass.

## Workflow

1. Confirm scope is locked. If new capability appears, run `hold-the-line`.
2. Define the polish scope: docs, tests, scripts, manifests, examples,
   privacy, public release notes, or commit plan.
3. Run a cleanup pass:
   - docs polish: names, examples, install/update notes, release checklist.
   - duplicate cleanup: repeated wording, overlapping skill triggers, stale
     table rows, and stale references.
   - privacy sweep: personal names, emails, paths, secret-like output, provider
     state, history notes.
   - identity sweep: author fields, package names, public-facing labels.
   - gate order: validation, lint, tests, smoke, scenario, and diff checks.
   - commit split: one logical intent per commit with evidence in the message.
4. Classify findings as `fix_now`, `defer`, `watch`, or `reject`.
5. Apply only `fix_now` cleanup that stays inside the polish scope.
6. Rerun the smallest credible gates and report release readiness.

## Rules

- no new capability during polishing.
- Do not rename, restructure, or add a feature unless it fixes a release-facing
  defect.
- Do not run broad history rewrites, publish, push, or change access without
  explicit approval.
- Keep cleanup findings separate from release blockers.
- If a finding needs research, park it under `watch` unless it blocks release.
- Passing gates are required before handing off to `stabilize-release-cut`.

## Output Contract

Use `GroundLine Release Polish` from `references/output-contracts.md`.
