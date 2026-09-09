# Native upgrade boundary

Codex owns marketplace refresh, plugin installation, and plugin upgrade. A
GroundLine release publishes checksummed, attested target artifacts and advances the moving
`stable` branch only after qualification.

Version tags contain source; the generated `stable` commit adds both native
binary trees. A source tag alone is not an installable binary distribution.

Core and Insights share that marketplace channel but remain independent plugin
installations. Refreshing Core never installs or activates Insights, and an
Insights-only installation does not require Core.

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
commands to artifact verification, the declared `setup` baseline, and strict
doctor. It does not independently resolve versions or modify cached packages.

For an Insights release that expands a validated event dimension, update the
owner API before enabling updated collectors. A previous API can reject a new
family label even though Core itself remains usable offline.
