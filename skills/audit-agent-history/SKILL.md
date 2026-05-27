---
name: audit-agent-history
description: Use when analyzing local Codex, Claude Code, or Antigravity history stores; when comparing current provider storage; when improving retention; when recovering prior context; or when deriving reusable skills from past agent work.
---

# Audit Agent History

## Purpose

Use this skill to understand local agent history as a storage and continuity surface, not as a prompt dump. Prefer compact inventory, metadata, and narrow keyword searches before opening any transcript content.

## Trigger Examples

- "Claude/Codex 대화내역 기반으로 만들 만한 스킬 찾아봐."
- "히스토리 구조 개선할 수 있는지 봐."
- "예전 agent가 뭘 했는지 로그에서 찾아줘."
- "Codex, Claude Code, Antigravity 기준으로 반복 패턴 뽑아줘."
- "세션/아카이브가 너무 커졌는데 정리 방향만 제안해."

## Safety Rules

- Default to read-only inspection.
- Do not delete, move, archive, compress, or quarantine history unless the user explicitly asks.
- Do not print secrets, OAuth tokens, raw credentials, or long transcript excerpts.
- Treat generated summaries as hints that must be rechecked against current files or live systems before execution.

## Workflow

1. Identify provider homes and likely storage roots for Codex, Claude Code, and Antigravity, plus project-local state, memory indexes, and archived session folders.
2. Build a metadata inventory first:
   - file counts
   - total sizes
   - newest and oldest modification times
   - session/archive/index layout
   - notable config and state files
3. Search by path and keyword instead of loading broad histories. Use `rg`, `find`, `jq`, or provider-native indexes when available.
4. Open only the minimum number of matching files needed to answer the user's question.
5. Synthesize the result into actionable buckets:
   - reusable workflow patterns
   - stale or duplicated history surfaces
   - retention or cleanup candidates
   - candidate skills, agents, hooks, or docs
   - facts that require live revalidation

## Search Strategy

Prefer narrow commands and stable metadata:

- Use `find` for path, size, and modified-time inventory.
- Use `rg` for task keywords, repo names, branch names, PR numbers, and exact error text.
- Use `jq` only after confirming the file is JSON or JSONL.
- Open one or two matching summaries before raw logs.
- Stop reading when you have enough evidence to answer the current question.

## Candidate Extraction

When looking for reusable skills or plugin features, score each pattern:

- `repeatability`: Has the user asked for this more than once?
- `risk`: Does the pattern prevent a costly false success, data leak, bad deploy, or destructive command?
- `portability`: Can it apply across repositories or runtimes?
- `non-obviousness`: Would a fresh agent likely miss it without guidance?
- `automation-fit`: Is it better as a skill, script, hook, agent, or plain documentation?

## Common Mistakes

- Loading entire chat logs into context before building an inventory.
- Treating memory-derived facts as current proof.
- Mixing provider runtime state with source-controlled configuration.
- Recommending deletion or archive actions without explicit user intent.

## Output Contract

```text
Provider inventory:
- Codex: ...
- Claude Code: ...
- Antigravity: ...
- Hermes/other: ...

High-signal patterns:
- ...

Candidates:
- skill/plugin/doc/hook/agent/script ...

Retention or cleanup notes:
- read-only recommendation ...

Revalidation needed:
- ...
```
