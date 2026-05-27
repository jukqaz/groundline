---
name: guard-side-effects
description: Use when an action may mutate external services, production, money, billing, access, secrets, remotes, destructive git state, or user data; or when read-only checks must be clearly separated from actual execution.
---

# Guard Side Effects

## Purpose

Use this skill to keep action boundaries explicit. The main failure to prevent is reporting a read-only check as if it changed money, production, access, secrets, or user data.

## Trigger Examples

- "배포해."
- "구매/충전 자동화 돌려."
- "DNS/Cloudflare/access 권한 바꿔."
- "secret 잘 들어갔는지 확인해."
- "브랜치 삭제하거나 force push 해."
- "production DB에서 정리해."

## Classification

Classify the requested action before executing it:

- `read_only`: inspection, status, logs, dry-run output, metadata inventory
- `local_mutation`: local files, local test data, local generated artifacts
- `repo_mutation`: commits, branch changes, PR updates
- `external_mutation`: deploy, cloud resource, DNS, billing, access, production data
- `money_moving`: purchase, top-up, payment, paid API usage with material cost
- `secret_sensitive`: any command or file path that may expose credentials
- `destructive`: deletion, reset, force push, irreversible cleanup

Use the highest applicable class. For example, a production deploy using a secret is both `external_mutation` and `secret_sensitive`.

## Workflow

1. State the intended side effect in plain language.
2. Check whether approval is required by policy, sandbox, or user instruction.
3. Prefer dry-run or read-only probes before mutation.
4. For secrets, validate only item names, file presence, parser success, or tool availability. Do not print secret values.
5. Execute only the approved action. Do not expand scope while running.
6. Report explicit booleans after execution.

## Approval Boundaries

Require explicit approval before:

- production deploys
- DNS, billing, access, or cloud permission changes
- purchases, top-ups, payments, or money-moving actions
- destructive file or git operations
- printing, copying, or moving secrets
- changing persistent global settings that affect every repository

Do not use a hook, script, or shortcut to bypass the approval boundary.

## Common Mistakes

- Reporting `purchase_executed=true` when only a cart or dry-run was checked.
- Printing secret values while trying to prove a config path works.
- Letting a deploy command expand into DNS, database, or permission changes.
- Treating local generated files and external service mutation as the same risk class.

## Output Contract

```text
Boundary:
- classification: read_only / external_mutation / money_moving / ...
- approval needed: yes/no
- intended side effect: ...

Result:
- read_only_checked=true/false
- deploy_executed=true/false
- top_up_executed=true/false
- purchase_executed=true/false
- secret_value_printed=false

Evidence:
- ...
```

If only a read-only check ran, say that directly.
