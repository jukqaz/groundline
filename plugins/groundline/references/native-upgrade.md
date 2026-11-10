# Native upgrade boundary

Codex owns marketplace and plugin changes. Core releases use `core-v...`, channel
`core-stable` and marketplace `groundline-core`. Insights uses `insights-v...`,
channel `stable` and marketplace `groundline`. Each distribution catalogs and
ships exactly one product. A Core release does not publish an API image or
refresh Insights. Source tags contain no binaries.

Use the reviewed complete distribution's `install.sh`. It verifies the product,
source parent, exact commit and native artifact before registration changes,
then pins native delivery to that commit. Default installation does not read a
model catalog, apply settings, enroll or enable collection. Existing choices and
records remain intact. The optional `--profile` must match the distribution.

The installer migrates an enabled official `groundline@groundline` to
`groundline@groundline-core` using native commands, retaining the old source pin
for Insights. A disabled legacy Core or enabled historical learning profile
requires review before migration; it is preserved and the installer stops before
writes. An existing disabled Core in the new namespace stays disabled. Insights
updates require any legacy Core to be migrated first.

App Refresh rechecks the pinned commit. A newer release requires a newly reviewed
distribution and installer. Only a newly selected product uses `plugin add`;
re-adding a disabled product would enable it. No cache files are edited manually.
Insights compatibility checks belong to its own installer, not Core maintenance.

Before native writes, the actual config is backed up privately. Caught source or
artifact failures attempt scoped native recovery to the previous product/source
registration and verify installed flags. Recovery never overwrites config with
an old snapshot over concurrent user edits. The receipt reports
`previous_product_registration_restored`; failed recovery requires review.
Native commands are not atomic across power loss or process kill.

Verify source/tag, package bytes, installed bytes and fresh native skill discovery
separately. The isolated regression covers migration, disabled state, independent
Insights upgrades, a channel moving after review and source/add/refresh failure
recovery. Fixture binaries are current even when older metadata is staged.
These checks prove neither natural work improvement nor authenticated delivery.
For manual repair see [installation alignment](installation-alignment.md).
