# Examples

These examples show how GroundLine should activate and what kind of answer it
should produce.

## Resume Prior Agent Work

Prompt:

```text
Claude Code was working on this. Recheck where it stopped and finish safely.
```

Expected flow:

```text
reconcile-current-state -> guard-side-effects when needed -> close-live-work when runtime proof matters
```

Expected answer shape:

```text
GroundLine Assessment:
- current conclusion: partial
- verified state: branch and PR checked
- capability gaps: live endpoint proof missing
- side-effect boundary: read_only
- recommended mode: companion-superpowers
- recommended next skill or tool: close-live-work
- exact next prompt: Verify the deployed endpoint and report PASS/PARTIAL/FAIL.
- verification checklist: endpoint, version, smoke
```

## Audit History For Reusable Patterns

Prompt:

```text
Find repeated failure patterns across my current agent histories and suggest improvements.
```

Expected skill:

```text
audit-agent-history
```

## Guard A Risky Operation

Prompt:

```text
Deploy this to production and update access permissions.
```

Expected skill:

```text
guard-side-effects
```

Expected answer shape:

```text
Boundary:
- classification: external_mutation
- approval needed: yes
- intended side effect: production deploy and access update
- read-only checks: current branch, CI status, target surface
- execution allowed: false
- secret value printed: false
```

## Close Live Work

Prompt:

```text
CI passed. Check whether the dev runtime actually serves this change.
```

Expected skill:

```text
close-live-work
```

## Align Agent Home

Prompt:

```text
Align my current agent homes and tell me what should be shared.
```

Expected skill:

```text
align-agent-home
```

## Recover Worktree And Branch

Prompt:

```text
The previous worktree path is gone. Prove the branch state before recreating it.
```

Expected skill:

```text
recover-worktree-branch
```
