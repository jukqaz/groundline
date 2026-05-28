---
name: recover-worktree-branch
description: Use when a worktree path vanished, a branch is missing or detached, a previous agent used a temporary checkout, stale branch names need cleanup, or branch/worktree attachment must be proven before continuing.
---

# Recover Worktree Branch

## Purpose

Use this skill when repository state is uncertain. The goal is to prove the relationship between paths, worktrees, branches, and remotes before recreating or deleting anything.

## Trigger Examples

- "브랜치 어디갔어?"
- "워크트리 되살려."
- "Codex worktree가 없어졌는데 이어서 해."
- "detached HEAD 상태 복구해."
- "old branch랑 new branch가 같은지 보고 정리해."

## Workflow

1. Identify the canonical repository path, expected worktree path, intended branch, and remote name.
2. Check whether the expected worktree path exists before running commands inside it.
3. Inspect the canonical repository:

```bash
git status --short --branch
git worktree list --porcelain
git show-ref --heads --remotes
```

4. If the worktree exists, inspect its branch and diff before switching or recreating.
5. If the worktree path is missing, recreate it from the canonical repository only after confirming the intended base and branch state.
6. If a branch exists only inside a recovered worktree, push from that worktree rather than assuming the canonical checkout has the ref.
7. Before deleting stale branch names, prove no divergence:

```bash
git merge-base old-branch new-branch
git diff old-branch...new-branch --stat
git log --left-right --cherry-pick --oneline old-branch...new-branch
git worktree list --porcelain
```

8. Do not delete or force-reset refs without explicit user approval.

## Recovery Patterns

Missing path:

```bash
test -d "$WORKTREE_PATH"
git -C "$CANONICAL_REPO" worktree list --porcelain
git -C "$CANONICAL_REPO" show-ref --heads --remotes
git -C "$CANONICAL_REPO" worktree add -b "$BRANCH" "$WORKTREE_PATH" "$BASE"
```

Detached worktree:

```bash
git -C "$WORKTREE_PATH" status --short --branch
git -C "$WORKTREE_PATH" branch --show-current
git -C "$WORKTREE_PATH" switch -c "$BRANCH"
```

Branch exists only in worktree:

```bash
git -C "$WORKTREE_PATH" status --short --branch
git -C "$WORKTREE_PATH" push -u origin "$BRANCH"
```

## Common Mistakes

- Running commands inside a deleted worktree path.
- Pushing from the canonical repository when the branch only exists in the worktree.
- Deleting an old branch because names look duplicated without proving identical history.
- Using `git reset --hard` or checkout-based recovery before preserving user changes.

## Output Contract

```text
Conclusion:
- recovered / safe to recreate / unsafe to delete / blocked

Evidence:
- canonical repo: ...
- worktree path: ...
- branch refs: ...
- divergence: ...

Action taken:
- ...

Remaining risk:
- ...
```
