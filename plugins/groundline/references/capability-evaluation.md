# Capability Evaluation

Use this rubric when evaluating existing tools, existing skills, plugins, MCP
servers, hooks, agents, prompt packs, CLIs, and workflow frameworks before
GroundLine adopts or recommends them.

## Evidence Inputs

- Primary source: official docs, source repository, release notes, tests, and
  security advisories.
- Secondary source: registries, curated lists, technical posts, and research
  papers.
- Weak source: social posts, generated summaries, unsourced catalogs, and
  screenshots without reproducible links.

## Rubric

Score each axis as `strong`, `acceptable`, `weak`, or `unknown`.

- `source evidence`: Is the claim backed by a primary source?
- `artifact clarity`: Is it clear whether this is a skill, plugin, MCP server,
  hook, agent, prompt pack, CLI, or documentation?
- `trigger clarity`: Can an LLM tell when to use it without loading everything?
- `context cost`: Does it load only when useful, or does it bloat every run?
- `setup weight`: Does it require no setup, optional setup, provider-home
  writes, hooks, MCP servers, or external services?
- `security risk`: Could it expose secrets, mutate providers, broaden access,
  or create prompt/context poisoning paths?
- `maintenance signal`: Are there recent releases, tests, issue activity, or a
  clear owner?
- `GroundLine fit`: Does it improve Codex, Claude Code, or Antigravity while
  keeping GroundLine lightweight?

## Decision Labels

- `adopt`: strong evidence, low setup risk, low context cost, and direct
  GroundLine fit.
- `adapt`: useful pattern, but copy only the smaller idea or safer workflow.
- `watch`: promising but unstable, under-documented, or changing quickly.
- `reject`: high risk, weak evidence, duplicated behavior, unsupported
  provider scope, or too much context/setup weight.

## Required Output Notes

Every evaluation should say:

- what was evaluated
- source evidence used
- unverified claims
- context cost
- setup and mutation surface
- security risk
- maintenance signal
- GroundLine fit
- final decision
- verification needed before adoption
