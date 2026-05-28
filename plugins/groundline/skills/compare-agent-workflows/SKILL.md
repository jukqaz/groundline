---
name: compare-agent-workflows
description: Use when comparing agent tools, skills, plugins, MCP servers, hooks, workflow frameworks, or provider features against GroundLine scope and supported runtimes.
---

# Compare Agent Workflows

## Purpose

Use this skill after research has produced candidates. The goal is to decide
which ideas are useful for GroundLine and which are noise.

## Routing Boundary

Use this after research has produced two or more candidates, or one candidate plus a clear GroundLine baseline.
Use `evaluate-agent-capability` first when the user asks whether one artifact is any good.
Use `recommend-groundline-upgrades` when the ranking already exists and the
remaining task is a GroundLine change decision.

## Comparison Axes

Score each candidate in plain text:

- `runtime fit`: Codex, Claude Code, Antigravity, or provider-neutral.
- `job fit`: current-state proof, research, planning, review, handoff,
  verification, safety, setup, or release.
- `context cost`: always loaded, skill-triggered, reference-only, script-only,
  or external tool.
- `setup weight`: none, optional CLI, optional MCP, plugin install, or hook.
- `mutation risk`: read-only, local write, provider-home write, remote write,
  production, access, billing, or secret exposure.
- `overlap`: replaces, complements, or duplicates an existing GroundLine skill.
- `maintenance`: stable primary source, active churn, unclear owner, or stale.

## Comparison Rules

- Favor narrow, triggerable skills over global instructions.
- Favor source-backed references over copied prompt packs.
- Treat hooks as deterministic guardrails, not general workflow memory.
- Treat MCP as optional setup unless the user asks to enable it.
- Reject candidates that require broad provider scope outside GroundLine.

## Output Contract

Use `GroundLine Comparison` from `references/output-contracts.md`.
