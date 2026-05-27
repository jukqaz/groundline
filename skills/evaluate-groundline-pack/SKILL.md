---
name: evaluate-groundline-pack
description: Use when reviewing GroundLine itself for repository readiness, skill completeness, trigger clarity, documentation coverage, safety posture, tests, release quality, or public package fitness.
---

# Evaluate GroundLine Pack

## Purpose

Use this skill to evaluate GroundLine as a package before release, public
visibility changes, or larger workflow additions.

## Evaluation Axes

- `repository readiness`: manifests, docs, CI, release notes, license, security
  policy, and public disclosure posture.
- `skill completeness`: each skill has a clear trigger, purpose, workflow,
  safety rules, and output contract where needed.
- `trigger clarity`: frontmatter descriptions describe when to use the skill,
  not the full workflow.
- `context weight`: skills are narrow and references carry longer details.
- `workflow coverage`: research, comparison, recommendation, handoff, state
  proof, side effects, live evidence, setup, and recovery are covered.
- `verification strength`: tests, lint, scenario runs, provider smoke, and
  release gates match the changed surface.
- `security posture`: no secrets, raw transcripts, auth files, provider state,
  broad hooks, or unsafe install behavior.

## Workflow

1. Check current repo state before judging: branch, dirty files, and recent
   changed surface.
2. Read `README.md`, `docs/llm-guide.md`, `docs/human-guide.md`,
   `references/output-contracts.md`, and the relevant `skills/*/SKILL.md`
   files.
3. Run the smallest credible gates for the claimed readiness.
4. Score each axis as `PASS`, `PARTIAL`, or `FAIL`.
5. Prioritize findings that affect public release, safety, context load,
   provider support, or whether an LLM can use the package correctly.

## Output Contract

Use `GroundLine Pack Evaluation` from `references/output-contracts.md`.
