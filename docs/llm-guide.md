# LLM Guide

This guide is for LLM agents reading GroundLine as operating context.

## Role

Use GroundLine as a control plane for agent work. It helps decide what evidence
is needed before continuing, mutating, releasing, or handing off work.

Do not treat GroundLine as permission to mutate user files, provider homes,
remotes, production systems, billing, access, or secrets.

## Primary Flow

1. Read the user's latest request.
2. Check current repository, branch, runtime, and dirty state.
3. Select the smallest relevant GroundLine skill.
4. Keep mutation boundaries explicit.
5. Run the smallest credible verification.
6. Report PASS, PARTIAL, or FAIL with concrete evidence.

## Skill Selection

- `reconcile-current-state`: stale handoff, previous agent claim, current worktree proof
- `audit-agent-history`: derive reusable improvements from agent history
- `guard-side-effects`: any action touching files, remotes, money, access, secrets, or production
- `close-live-work`: CI or local tests passed but live runtime proof is still needed
- `align-agent-home`: provider home, config, hooks, rules, skill, or plugin boundary review
- `recover-worktree-branch`: missing worktree, detached branch, cleanup, or recovery

## Safety Rules

- Do not print secret values.
- Do not copy provider auth files, sessions, logs, caches, shell snapshots, or
  local databases into source control.
- Treat `mutation_performed=false` as evidence only for that command, not for
  the whole task.
- Ask for explicit approval before external mutation or destructive git work.
- Use `--home` with a temporary directory when testing install plans.

## Output Shape

Prefer concise evidence:

```text
status: PASS|PARTIAL|FAIL
scope: files, runtime, branch, or provider checked
evidence: command output summary or file path
mutation_performed: true|false
next_action: only if needed
```

For public or user-shared output, redact default home paths as `~` and avoid
long transcript excerpts.

Before recommending a repository visibility change, check
`docs/git-history-privacy.md` and separate current-tree findings from git
history findings.
