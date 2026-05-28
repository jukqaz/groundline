---
name: curate-groundline-skills
description: Use when deciding whether GroundLine should create, adapt, merge, split, deprecate, or reject skills, references, scripts, agents, hooks, MCP recommendations, or docs.
---

# Curate GroundLine Skills

## Purpose

Use this skill to manage GroundLine as a readable skill portfolio instead of a
pile of loosely related prompts.

## Workflow

1. Read `references/skill-index.json` for the current LLM-readable catalog.
2. Read `docs/skill-portfolio.md` for the human-readable inventory.
3. Classify the new or existing capability with `references/skill-lifecycle.md`.
4. Decide whether to create, adapt, merge, split, deprecate, or reject.
5. Update the smallest necessary surfaces:
   - `skills/<name>/SKILL.md`
   - `skills/<name>/agents/openai.yaml`
   - `references/skill-index.json`
   - `docs/skill-portfolio.md`
   - related tests and validation scripts
6. Run the relevant validation gate before claiming the portfolio is current.

## New Skill Gate

Create a new skill only when at least one condition is true:

- the same failure appears in repeated work or dogfood
- adding the workflow to an existing skill would make that skill too broad
- the workflow needs its own output contract, risk boundary, or lifecycle entry

Otherwise classify the idea as a source-registry item, reference note, example,
or `watch` item.

## Decision Rules

- Create a skill when a repeatable judgment workflow needs triggerable
  instructions.
- Move long source-backed detail into a reference file.
- Move deterministic checks into scripts.
- Merge skills with the same trigger, stage, and output contract.
- Split a skill that mixes unrelated stages or risk levels.
- Deprecate skills that are no longer useful but still explain prior releases.
- Reject candidates that add broad context load, unsafe setup, or unsupported
  provider scope.

## Output Contract

```text
GroundLine Skill Curation:
- current conclusion:
- candidate:
- classification:
- decision: create|adapt|merge|split|deprecate|reject
- affected surfaces:
- human-readable update:
- LLM-readable update:
- verification checklist:
```
