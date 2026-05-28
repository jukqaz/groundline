# External Workflow Interop

This reference supports the `agent-ecosystem-radar` skill set. It helps an
agent research, compare, and recommend external workflow ideas without turning
GroundLine into a heavy setup bundle.

## Research

Prefer these source families:

- Official provider docs for Codex, Claude Code, and Antigravity.
- Primary repositories and release notes.
- Security advisories and red-team references.
- Skill registries only as discovery feeds.
- Curated lists only as candidate indexes.

Known high-signal sources:

- Spec Kit: spec-first phases and agent-neutral command scaffolding.
- Agent OS: project standards discovery and spec shaping.
- BMAD: role-based planning and development workflow structure.
- Superpowers: TDD, worktrees, verification, and development discipline.
- gstack: role pack, planning, review, QA, and shipping patterns.
- grill-me: pressure-test questioning before implementation.
- Arc and flow-next: bundled workflow packaging and plan-first task tracking.
- planning-with-files: persistent markdown planning and handoff state.
- skills.sh and MDSkill: skill discovery and audit signals.
- Context7, Exa, and GitHub MCP: optional research and repo evidence tools.
- Promptfoo: coding-agent red-team cases for prompt injection, terminal output
  injection, and delayed exfiltration.

## Compare

Compare each candidate against GroundLine's current scope:

- Codex, Claude Code, and Antigravity only.
- macOS on Apple Silicon and Linux only.
- Skills and references before hooks or always-on services.
- MCP recommendations stay optional and user-approved.
- Provider home writes require explicit approval.
- Public package contents must not include auth files, sessions, caches, or
  raw transcripts.

## Recommend

Use four labels:

- `adopt`: add directly.
- `adapt`: borrow the pattern but keep a lighter GroundLine shape.
- `watch`: track source changes before adding behavior.
- `reject`: keep out of GroundLine.

Prefer this mapping:

- Repeatable judgment -> skill.
- Long source-backed detail -> reference.
- Deterministic local check -> script.
- Source drift -> radar.
- Local runtime posture -> doctor.
- Human setup or release policy -> docs.
- Deterministic blocking guard -> hook, only after review.

## Adoption Rules

- Do not clone another workflow pack wholesale.
- Do not add provider targets outside the supported three.
- Do not install third-party packages during research.
- Do not treat popularity as proof of quality.
- Require a verification gate for every `adopt` or `adapt` recommendation.
- If a candidate mainly increases context load, classify it as `watch` or
  `reject`.
