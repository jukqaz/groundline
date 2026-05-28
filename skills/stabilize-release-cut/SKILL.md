---
name: stabilize-release-cut
description: Use when a growing change set, plugin package, skill set, or release candidate needs scope lock, change budget, must-fix triage, dogfood evidence, release gates, regression checks, or a ship decision.
---

# Stabilize Release Cut

## Purpose

Use this skill when improvement work needs to stop expanding and start proving.
It turns a growing change set into a bounded release candidate.

Use when the active question is ship, hold, or continue.

## Workflow

1. Check current branch, dirty files, version, and changed surface.
2. Declare a scope lock: what is in, what is out, and what may still change.
3. Classify new ideas before accepting scope: `must fix`, `adapt`, `watch`,
   `defer`, or `reject`.
4. Set a change budget for `must fix` and narrow `adapt` items only.
5. Classify remaining requests as `watch`, `defer`, or `reject`.
6. Run release gates: validation, lint, tests, scenario runs, provider smoke,
   privacy review, and docs review as appropriate.
7. Collect dogfood evidence from supported providers or explain the missing
   evidence.
8. Make a ship decision: `ship`, `hold`, or `continue`.

## Rules

- Do not add new capability during stabilization unless it resolves a must-fix
  release blocker.
- Classify new ideas before accepting scope.
- must fix requires release-blocking evidence.
- watch is the default for promising but unproven ideas.
- Do not add a new skill during stabilization unless repeated failure,
  dogfood evidence, or a clear output-contract boundary proves the need.
- Treat passing tests as necessary but not enough for public release.
- Keep dogfood evidence separate from scripted smoke output.
- If evidence is missing, say what gate is missing instead of smoothing over it.
- Do not mutate remotes, publish, or change access without explicit approval.

## Output Contract

Use `GroundLine Release Cut` from `references/output-contracts.md`.
