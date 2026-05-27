---
name: recommend-groundline-upgrades
description: Use when turning research and comparison findings into GroundLine upgrade candidates, adoption decisions, source registry entries, skills, references, scripts, docs, hooks, or MCP recommendations.
---

# Recommend GroundLine Upgrades

## Purpose

Use this skill to convert research and comparison output into actionable
GroundLine changes.

## Decision Labels

- `adopt`: add the pattern directly because it fits GroundLine with low risk
  and has current release evidence.
- `adapt`: borrow the pattern but keep GroundLine lighter or safer; requires a
  current-scope behavior gap and a verification path.
- `watch`: track source changes before adding behavior. watch is the default
  for promising but unproven ideas without failure evidence.
- `reject`: keep it out because it adds risk, context load, provider sprawl, or
  duplicates existing GroundLine behavior.

## Placement Rules

- Use a `skill` for repeatable judgment workflows.
- Use a `reference` for source-backed facts, matrices, or longer guidance.
- Use a `script` for deterministic checks that should not be rewritten.
- Use `doctor` for local posture detection.
- Use `radar` for source drift and ecosystem monitoring.
- Use `docs` for human setup and release guidance.
- Use `MCP recommendation` only when the tool is optional and user-approved.
- Use `hook` only for deterministic safety checks with reviewed behavior.

## Required Recommendation Shape

Each recommendation must include:

- candidate
- decision label
- reason
- GroundLine target file or surface
- side-effect boundary
- verification gate
- open question, if any

## Output Contract

Use `GroundLine Recommendation` from `references/output-contracts.md`.
