---
name: align-agent-home
description: Use when the user asks about Codex, Claude Code, or Antigravity settings, official documentation alignment, guidance files, skills, custom agents, rules, hooks, provider boundaries, private config sync, or runtime doctor output.
---

# Align Agent Home

## Purpose

Use this skill for current agent home hygiene across Codex, Claude Code, and Antigravity. Keep global configuration minimal, separate desired capability from runtime state, and prove the result with local validation.

## Trigger Examples

- "3사 agent 설정 공식 문서 기준으로 정리해."
- "AGENTS.md/global guidance 맞는지 봐."
- "agents/rules/hooks 겹치는 거 정리해."
- "private repo에 어떤 agent config만 넣을지 정하자."
- "GroundLine doctor 기준으로 마무리해."

## Workflow

1. Inventory the active surface before editing:
   - Codex guidance and rule surfaces
   - Claude Code guidance and plugin surfaces
   - Antigravity plugin and skill surfaces
   - hooks files or inline hooks
   - selected custom skills and plugin manifests
2. Separate source-of-truth config from runtime state. Do not sync or print `auth.json`, sessions, archived sessions, logs, OAuth material, shell snapshots, plugin caches, or local databases unless the user explicitly asks for a read-only inventory.
3. For current runtime behavior that may drift, verify with the local CLI or official documentation before making claims.
4. Keep global config to intentional choices. Move repo-specific policy into repo-local guidance or rules.
5. Validate after changes:

```bash
python3 scripts/groundline_doctor.py --json --offline
```

## Source Boundaries

Treat these as source-controlled candidates only after review:

- `config.toml` with intentional non-defaults
- `AGENTS.md` and repo-local guidance
- selected `agents/*.toml`
- selected `rules/*.rules`
- reviewed hook documentation or repo-local hooks
- selected custom skills or plugin manifests

Keep these out of source control and chat output:

- `auth.json`
- sessions and archived sessions
- logs, caches, shell snapshots, OAuth material
- plugin cache directories
- local databases and SQLite files
- raw secret values

## Common Mistakes

- Copying runtime state into a config repo.
- Repeating defaults in global config until drift is hard to see.
- Editing guidance based on memory without checking official docs or local CLI behavior.
- Enabling global hooks before proving they are deterministic and project-neutral.

## Output Contract

```text
Conclusion:
- aligned / partially aligned / blocked

Changed:
- ...

Preserved runtime/private state:
- ...

Verification:
- command: python3 scripts/groundline_doctor.py --json --offline
- result: ...

Unverified:
- ...
```
