# Native upgrade boundary

Codex owns marketplace refresh, plugin installation, and plugin upgrade. A
GroundLine release publishes immutable target artifacts and advances the moving
`stable` branch only after qualification.

Core and Insights share that marketplace channel but remain independent plugin
installations. Refreshing Core never installs or activates Insights, and an
Insights-only installation does not require Core.

After refresh or upgrade, verify four distinct lanes:

1. source revision and tag;
2. packaged plugin manifest and file fingerprint;
3. installed plugin manifest and native artifact checksum;
4. a new-task runtime smoke result.

A result from one lane does not prove the others.

Use the provider's current upgrade command or App action. Codex CLI 0.153.0
added remote-marketplace operations and merged-configuration Git marketplace
upgrades; a remote marketplace and GroundLine's Git `stable` channel remain
different sources. Inspect the actual installed source before troubleshooting.
GroundLine does not maintain a parallel updater or rewrite Codex plugin state.

For an Insights release that expands a validated event dimension, update the
owner API before enabling updated collectors. A previous API can reject a new
family label even though Core itself remains usable offline.
