---
name: research-agent-ecosystem
description: Use when gathering current sources about agent tools, skills, plugins, MCP servers, hooks, provider features, or workflow frameworks before comparison or upgrade planning.
---

# Research Agent Ecosystem

## Purpose

Use this skill to gather current, source-backed information before comparing or
adopting external agent workflow ideas.

## Workflow

1. Restate the research scope and supported GroundLine runtimes.
2. Start with primary sources: official docs, source repositories, release
   notes, and security advisories.
3. Use registries and curated lists only to discover candidates.
4. Record each candidate with:
   - source URL or local source path
   - source type
   - last checked date when available
   - claimed capability
   - evidence strength
   - GroundLine relevance
5. Separate confirmed facts from claims that still need verification.

## Evidence Strength

- `primary`: official docs, source repository, release notes, standards docs.
- `secondary`: curated lists, registries, technical posts, conference papers.
- `weak`: social posts, generated summaries, unsourced directory entries.

## GroundLine Fit Questions

- Does this improve Codex, Claude Code, or Antigravity work directly?
- Is it useful without always-on services?
- Can it stay read-only until the user approves setup?
- Does it reduce context waste or false completion?
- Does it belong as a skill, reference, script, doctor probe, radar source, or
  human documentation?

## Output Contract

Use `GroundLine Research` from `references/output-contracts.md`.
