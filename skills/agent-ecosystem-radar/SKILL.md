---
name: agent-ecosystem-radar
description: Use when the user wants one pass that researches external agent tools, compares them, and recommends GroundLine upgrades for Codex, Claude Code, or Antigravity.
---

# Agent Ecosystem Radar

## Purpose

Use this as the set entrypoint for agent ecosystem discovery. It chains three
GroundLine skills into one decision loop:

```text
research-agent-ecosystem -> evaluate-agent-capability -> compare-agent-workflows -> recommend-groundline-upgrades
```

If the runtime can load named skills, use them in that order. If not, run the
same phases inline using the contracts in `references/output-contracts.md`.

## Workflow

1. Scope the target: user goal, supported runtime, current GroundLine surface,
   and whether network research is allowed.
2. Run `research-agent-ecosystem` to collect source-backed options.
3. Run `evaluate-agent-capability` for any existing tool, skill, plugin, MCP
   server, hook, agent, or workflow pack that may be adopted.
4. Run `compare-agent-workflows` to score overlap, risk, context cost, setup
   surface, and GroundLine fit.
5. Run `recommend-groundline-upgrades` to classify each candidate as `adopt`,
   `adapt`, `watch`, or `reject`.
6. End with exact next work items and verification gates.

## Source Hints

Use `references/external-workflow-interop.md` for known source families and
`references/source-registry.json` for radar seeds. Prefer official docs and
primary repositories. Treat registry listings and blog posts as discovery
signals until a primary source confirms the claim.

## Safety Rules

- Do not install third-party skills, plugins, hooks, MCP servers, or agents
  during research.
- Do not enable hooks or mutate provider homes while comparing options.
- Do not recommend broad context loading when a narrow skill or reference file
  would do.
- Keep unsupported providers out of the GroundLine target scope.

## Output Contract

Return all three sections in order:

```text
GroundLine Research:
- scope:
- primary sources:
- secondary sources:
- candidate list:
- confirmed facts:
- unverified claims:
- uncertainty:

GroundLine Capability Evaluation:
- target:
- artifact type:
- source evidence:
- unverified claims:
- context cost:
- setup and mutation surface:
- security risk:
- maintenance signal:
- GroundLine fit:
- decision:
- verification needed:

GroundLine Comparison:
- candidates:
- scoring axes:
- strongest matches:
- overlap with GroundLine:
- context and setup cost:
- rejected or deferred:
- comparison gaps:

GroundLine Recommendation:
- adopt:
- adapt:
- watch:
- reject:
- next GroundLine changes:
- side-effect boundary:
- verification checklist:
```
