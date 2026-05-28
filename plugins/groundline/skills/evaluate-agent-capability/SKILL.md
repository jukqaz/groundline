---
name: evaluate-agent-capability
description: Use when evaluating existing agent tools, skills, plugins, MCP servers, hooks, agents, prompt packs, or workflow frameworks for quality, security, context cost, maintenance, and GroundLine fit.
---

# Evaluate Agent Capability

## Purpose

Use this skill during research before comparing or recommending external
capabilities. It evaluates one existing tool or skill-like artifact on its own
merits.

## Routing Boundary

Use this for one candidate at a time. If there are two or more candidates to rank, use `compare-agent-workflows`.
If the user asks for source discovery first, use `research-agent-ecosystem`.
If the user asks for research, comparison, and recommendation together, use
`agent-ecosystem-radar`.

## Workflow

1. Identify the artifact type: skill, plugin, MCP server, hook, agent, prompt
   pack, workflow framework, CLI, or documentation.
2. Gather source evidence from official docs, primary repositories, release
   notes, tests, and security notes. Use registries only as discovery hints.
3. Score the artifact with `references/capability-evaluation.md`.
4. Separate confirmed behavior from claims that still need live verification.
5. Decide whether GroundLine should `adopt`, `adapt`, `watch`, or `reject` it.

## Evaluation Rules

- Favor primary source evidence over popularity.
- Penalize artifacts that require always-on context, broad hooks, unsafe
  permissions, secret exposure, or provider sprawl.
- Treat MCP servers and hooks as optional until the user approves setup.
- Prefer a reference or script over a new skill when the behavior is factual or
  deterministic.
- Do not install or enable the candidate while evaluating it.

## Output Contract

Use `GroundLine Capability Evaluation` from `references/output-contracts.md`.
