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
