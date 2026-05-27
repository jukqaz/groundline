---
name: reconcile-current-state
description: Use when continuing prior Claude Code, Codex, Antigravity, or subagent work; when handoff notes may be stale; when a previous agent claimed progress; or when current repository, runtime, CI, PR, or endpoint state must be rechecked before acting.
---

# Reconcile Current State

## Purpose

Use this skill as the first pass before acting on previous agent claims. Treat memory, handoff notes, chat summaries, Discord bot messages, and subagent reports as hints until the active checkout and runtime state prove them current.

## Trigger Examples

- "Claude가 하던 거 이어서 마무리해."
- "이전 Codex가 어디까지 했는지 보고 계속해."
- "handoff 보고 남은 일 처리해."
- "PR은 만들어졌다는데 지금 상태 다시 봐."
- "Discord/Hermes agent가 했다는 내용 검증하고 이어가."

## Workflow

1. Identify the target lane: repository path, worktree path, branch, PR, issue, deployment environment, previous agent/source, and the user's latest request.
2. Collect only durable prior context first: memory summaries, handoff files, PR descriptions, task notes, or recent terminal evidence. Avoid loading raw transcripts wholesale.
3. Prove current local state:
   - `pwd`
   - `git status --short --branch`
   - `git rev-parse --show-toplevel`
   - `git worktree list --porcelain` when worktrees are involved
   - `git diff --stat` and targeted `git diff` for touched files
   - recent commits only when branch history matters
4. Prove current runtime or external state when the task depends on it: local process, port listener, health endpoint, CI check, PR status, version endpoint, systemd unit, queue worker, or browser smoke.
5. Compare prior claims against current evidence. Mark each important claim as confirmed, stale, contradicted, or unverified.
6. Continue only after the next safe action is clear.

## Evidence Checklist

Use the smallest credible checks for the lane:

- Repository: `pwd`, repo root, branch, upstream, dirty diff, worktree attachment.
- Branch continuity: local refs, remote refs, recent commits, PR head/base.
- Runtime: process, port, logs, health endpoint, version endpoint.
- CI/release: latest relevant run, failed job name, artifact/tag/image/version.
- User-facing flow: browser, mobile, CLI, smoke command, screenshot, or API probe.
- Safety boundary: whether the next action mutates local files, remotes, money, secrets, or production.

## With Superpowers

- Use this before `superpowers:systematic-debugging` when the bug report comes from a previous agent or stale handoff.
- Use this before `superpowers:executing-plans` when the plan was written in another session.
- Use `superpowers:verification-before-completion` after implementation, not as a replacement for this start-of-work reconciliation.

## Common Mistakes

- Trusting a prior agent summary without checking the active branch.
- Reading broad raw transcripts before checking the repository state.
- Treating a successful CI run as proof that a dev or production endpoint changed.
- Ignoring the user's newest request because an older handoff had a different goal.

## Output Contract

Lead with the current conclusion, then attach proof.

```text
Current conclusion: continue / pause / repair first / blocked
Confirmed:
- ...
Drift or contradiction:
- ...
Unverified:
- ...
Next safe action:
- ...
```

Do not claim prior work is complete because another agent said so. Verify the relevant state yourself.
