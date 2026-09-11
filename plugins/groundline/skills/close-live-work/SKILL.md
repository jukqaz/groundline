---
name: close-live-work
description: Verify a requested live outcome after local checks. Use only the runtime evidence needed for that outcome.
---

# Close Live Work

## Purpose

Local checks and CI do not prove live behavior. Verify the requested runtime
outcome; this skill does not authorize deployment, installation, or other writes.

## Workflow

1. Name the target and expected artifact.
2. Reuse the relevant source revision and passing checks. Select the missing
   runtime observation; do not reopen a completed source audit.
3. Inspect the relevant artifact and probe its version, process, or user flow.
   A local uninstall needs absence and process checks, not a release pipeline.
4. For an installed GroundLine plugin check, resolve its executable through
   [platform commands](../../references/platform-commands.md) and use that
   product's `provider-smoke --require-installed --json`. For a requested
   upgrade, follow [native upgrade](../../references/native-upgrade.md) and prove
   source, package, published ref, install, and a fresh task. An ordinary runtime
   check does not require publishing or installing anything.
5. Complete an explicitly requested native Goal only after every required proof
   passes. Otherwise report the ordinary task's outcome. Reuse valid checks;
   retry failed probes only after a changed condition or within a bounded
   transient-retry policy. Keep new external mutations behind their approval.

## Minimum Evidence

Use health and version for APIs; asset and browser smoke for web; revision and
processing for workers; build and device or track proof for mobile. Catalog,
dispatch, and acceptance differ. `PASS` requires live artifact and smoke.
Missing live proof is `PARTIAL`; wrong artifact or failed smoke is `FAIL`.

## Output Contract

State the outcome, decisive runtime evidence, and remaining gap or next action.
Separate source, package, install, and live claims only where relevant. Include
Goal status only when a Goal was explicitly requested; do not create one here.
Name the exact instruction if a skill causes a pause; never mark unfinished work
complete or treat an unavailable probe as a demonstrated code failure.
