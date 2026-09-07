---
name: close-live-work
description: Use when local checks pass but a live result still needs proof.
---

# Close Live Work

## Purpose

Local checks and CI do not prove live behavior. Verify the requested runtime
outcome; this skill does not authorize deployment, installation, or other writes.

## Workflow

1. Name the target and expected artifact.
2. Confirm the requested scope, source revision, and relevant existing checks.
   A native Goal is optional: without an explicitly requested Goal, close the
   ordinary task without creating one or requiring a Goal operation.
3. Inspect relevant jobs, artifacts, logs, and queues.
4. Probe live version, process, smoke, or user flow.
5. For GroundLine, read
   [installed command resolution](../../references/platform-commands.md), run
   `groundline provider-smoke --require-installed --json` and
   follow [the native upgrade contract](../../references/native-upgrade.md). For
   any plugin, prove source, package, published ref, install, and fresh task.
6. Complete an explicitly requested native Goal only after every required proof
   passes. Otherwise report the ordinary task's outcome. Reuse valid checks;
   retry failed probes only after a changed condition or within a bounded
   transient-retry policy. Keep new external mutations behind their approval.

## Minimum Evidence

Use health and version for APIs; asset and browser smoke for web; revision and
processing for workers; build and device or track proof for mobile. Catalog,
dispatch, and acceptance differ. `PASS` requires live artifact and smoke.
Missing live proof is `PARTIAL`; wrong artifact or failed smoke is `FAIL`.

## Output Contract

```text
Status: PASS / PARTIAL / FAIL
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

Include Goal status only when a Goal exists. Omit irrelevant evidence lanes.
Name the exact instruction if a skill causes a pause; never mark unfinished work
complete or treat an unavailable probe as a demonstrated code failure.
