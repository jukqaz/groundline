---
name: hold-the-line
description: Use when a request, task, research thread, plugin package, or active change set starts expanding with new ideas, extra tools, broad cleanup, or unclear finish criteria before the current work is closed.
---

# Hold The Line

## Purpose

Use this skill to counter expansion pressure before it turns useful work into
an unbounded queue. It forces a scope decision before accepting another idea.

Use first when expansion pressure appears before the current work is closed.

## Workflow

1. State the current work and the nearest finish point.
2. Capture the expansion trigger in one sentence.
3. Classify the trigger: defect, blocker, adjacent improvement, research
   curiosity, future option, or distraction.
4. Pick one decision:
   - `finish_current`: close the current work before adding anything.
   - `accept_with_budget`: allow one small change with a time, file, and
     verification budget.
   - `defer`: park the idea for a later task.
   - `watch`: track the idea until repeated failure or dogfood evidence exists.
   - `reject`: drop it because it does not improve the current goal.
5. Set one next action and one verification command or artifact.
6. Return parked ideas separately from the active scope.

## Rules

- finish current is the default when the current scope is unverified.
- watch is the default for interesting external tools, new skills, and broad
  workflow upgrades without failure evidence.
- accept with budget requires a named benefit, a narrow edit boundary, and a
  concrete verification.
- do not start research just because an idea is interesting.
- new skill requires repeated failure, dogfood evidence, or a distinct output
  contract that existing skills cannot cover.
- Keep follow-up suggestions to three or fewer.
- Use `package-agent-task` if the scope remains broad.
- Use `stabilize-release-cut` when the active question is release readiness.

## Output Contract

Use `GroundLine Scope Hold` from `references/output-contracts.md`.
