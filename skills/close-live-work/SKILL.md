---
name: close-live-work
description: Use when local checks pass but a live result still needs proof.
---

# Close Live Work

## Purpose

Local checks and CI do not prove live behavior. Use this skill for runtime
closure and native Goal completion.

## Workflow

1. Name the target and expected artifact.
2. Confirm frozen Goal scope, source revision, and checks.
3. Inspect relevant jobs, artifacts, logs, and queues.
4. Probe live version, process, smoke, or user flow.
5. For GroundLine, first run
   `groundline provider-smoke --require-installed --json` and
   follow [the native upgrade contract](../../references/native-upgrade.md). For
   any plugin, prove source, package, published ref, install, and fresh task.
6. Complete the Goal only after every required live proof is `PASS`.

## Minimum Evidence

Use health and version for APIs; asset and browser smoke for web; revision and
processing for workers; build and device or track proof for mobile. Catalog,
dispatch, and acceptance differ. `PASS` requires live artifact and smoke.
Missing live proof is `PARTIAL`; wrong artifact or failed smoke is `FAIL`.

## Output Contract

```text
Status: PASS / PARTIAL / FAIL
Goal status:
Expected artifact:
Evidence:
- pipeline: ...
- runtime: ...
- smoke: ...
Gaps:
- ...
Next action:
- ...
```

Use native Goal completion; never mark unfinished work complete.
