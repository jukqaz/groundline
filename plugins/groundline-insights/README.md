# GroundLine Insights

Insights is an optional, independently installable companion to Core. Four
fail-open Codex hooks collect bounded native App/CLI activity into a private
aggregate outbox for an owner-operated HTTPS API, ClickHouse, and Grafana.
Tailnet restriction is optional. Core, inference proxies, model catalogs, and
provider credentials are not dependencies.

Insights installs no skills, daemon, or scheduler and changes no global Codex
settings. [Security](SECURITY.md) and the
[contract](references/insights-contract.md) define private state, consent,
authentication, collection limits, and excluded raw content.

## Install and upgrade

For package installation, connection, consent, and verification together, use the
reviewed distribution's `install.sh --profile insights` (or `both`). For package-only
installation:

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline-insights@groundline --json
```

This does not install Core. Source tags contain no binaries. Follow
[native upgrade](references/native-upgrade.md) for API-first checks, commit
pinning, and recovery; do not re-add a disabled plugin to refresh it.

After package-only installation, `groundline-insights setup` reports remaining
steps. For an explicitly selected connection, use `--endpoint <https-origin>
--enrollment-token-file <private-file> --enable --verify`, or
`--input <private-profile>`. Matching inputs are reused; conflicting connections
are rejected without resetting identity/history. Exit 2 means action or first
activity is pending. Setup grants no hook trust and fabricates no delivery receipt.

A changed hook hash requires Codex review. GroundLine never trusts itself, and
`plugin list` proves neither hook trust nor dispatch. Resolve `groundline-insights`
from the installed `bin/<target>` directory: macOS/Linux ARM64 and x86_64 are
supported, but shell `PATH` registration is not promised.

## Owner configuration

Copy [owner-profile.example.json](references/owner-profile.example.json) outside
the plugin/repository, restrict it to its owner, and replace the endpoint and
intentionally invalid short token. `worker configure` accepts that schema-7 input
and stores the sanitized profile and enrollment credential separately below
`~/.codex/groundline/insights`. It never prints or copies secrets into the plugin.

```console
groundline-insights worker configure --input /owner-private/owner-profile.json
groundline-insights worker enable
groundline-insights worker run-once
groundline-insights worker status
```

Installation is inert; `worker enable` explicitly consents to owner-service upload.
First enrollment requires connectivity and the owner-issued credential; later
requests use a per-collector token. Missing consent is created on explicit enable,
with unconsented pending events quarantined. Valid consent survives re-enable.
Unsupported state is preserved and rejected, not migrated by enable: stop
collection, retain state/outbox, and obtain approval before a fresh setup.

Collection starts with seven days, preserves subsequent cursors, and freezes
incomplete windows. Three failed read attempts require operator retry. For status,
source discovery, gaps, and safe recovery use
[operations troubleshooting](references/operations-troubleshooting.md).

Fleet reporting is a separate administrative operation:

```console
groundline-insights insights fetch-report \
  --admin-token-file /owner-private/admin-report-token \
  --days 7 --json
```

Fleet reports require an explicit private file containing the separate admin
token; a collector token cannot authorize them. Keep it off collector-only hosts,
Git, and logs.

## Evidence lanes

Package integrity, four effective hooks, lifecycle dispatch, accepted upload,
ClickHouse visibility, Grafana query frames, image publication, deployment, and
stable promotion are separate evidence. Unobserved lanes remain `UNVERIFIED`.
Operational endpoints, credentials, dataset paths, and receipts stay outside
public Git and CI.

The [contract](references/insights-contract.md) owns storage/delivery bounds,
retention, deduplication, and 7/30/90-day reports. Docker Compose is the public
self-hosting preview; TrueNAS is an optional overlay. Production still requires
fresh-host, immutable-image, and external TLS evidence. See the
[self-hosting guide](https://github.com/jukqaz/groundline/blob/main/docs/self-hosting.md),
[privacy policy](https://github.com/jukqaz/groundline/blob/main/docs/privacy.md),
and [integration profiles](https://github.com/jukqaz/groundline/blob/main/docs/integrations.md).

License: MIT.
