# Native upgrade boundary

Codex owns marketplace refresh, plugin installation, and plugin upgrade. A
GroundLine release publishes checksummed, attested target artifacts and advances the moving
`stable` branch only after qualification.

Version tags contain source; the generated `stable` commit adds both native
binary trees. A source tag alone is not an installable binary distribution.

Core and Insights share that marketplace channel. A fresh Core installation does
not add Insights, and an Insights-only installation does not require Core.
However, refreshing the shared marketplace can update both products when they
are already installed. `--profile core` selects Core's explicit install/setup
steps; it does not pin an installed Insights collector to its previous version.

After refresh or upgrade, verify four distinct lanes:

1. source revision and tag;
2. packaged plugin manifest and file fingerprint;
3. installed plugin manifest and native artifact checksum;
4. a new-task runtime smoke result.

A result from one lane does not prove the others.

Use Codex App Refresh or `codex plugin marketplace upgrade groundline --json`.
Inspect `codex plugin list --json`; if Core remains on the old version, install
the same ID again with `codex plugin add groundline@groundline --json` and verify
its checksum. A remote marketplace and GroundLine's Git `stable` channel are
different sources. Inspect the actual installed source before troubleshooting.
GroundLine does not maintain a parallel updater or rewrite Codex plugin state.

When the request includes applying GroundLine or repairing the existing setup,
continue with [installation alignment](installation-alignment.md) after package
verification. Complete evidenced configuration/guidance repairs in the same
authorized task; package refresh alone does not prove the old setup was fixed.
The reviewed stable distribution's `install.sh`/`install.ps1` joins those native
commands to artifact verification, preserving setup, and strict
doctor. It does not independently resolve versions or modify cached packages.
Use the current complete `stable` distribution when updating, then rerun the same
installer. The canonical numeric version identifies manifests and native cache
directories; the date-and-letter release title is only a display name. A repeated
unchanged version rechecks artifacts and setup without resetting owner state.
For existing Insights state, the installer verifies the candidate collector and
uses its read-only `worker check-server` before marketplace add/upgrade. Missing
profiles cause no request; malformed profiles and incompatible or unreachable
APIs block native writes. Direct Codex refresh commands do not run this installer
preflight.

For an Insights release that expands a validated event dimension, update the
owner API before enabling updated collectors. A previous API can reject a new
family label even though Core itself remains usable offline. Validate API
compatibility before advancing or refreshing their shared `stable` channel when
Insights is already enabled. Core-only installation is not an isolation measure.

The native isolated regression stages an older package manifest, advances local
Git `stable`, and checks cache versions, same-version repeat, model settings and
disabled collection consent preservation. Both fixture versions contain the
current binary: this proves native metadata transition, not an old runtime's
behavior, authenticated task execution, or delivery to an owner API.
