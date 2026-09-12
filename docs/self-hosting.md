# Self-hosting GroundLine Insights

The Codex plugin and the owner-operated service are separate installations. The
plugin contains the collector binary; the service checkout supplies the Axum
API image, ClickHouse, Grafana, and the secret-safe Compose renderer. Installing
the plugin never connects to a maintainer service.

The generic Compose path is an optional public self-hosting preview. It is not a
hosted GroundLine service and it is not required by Core. Qualify the exact
release on a fresh host and verify the external HTTPS endpoint before treating
an operator deployment as production-ready.


## Standard HTTPS and optional Tailscale

The default is `--bind-ip 127.0.0.1`. Terminate TLS at a reverse proxy on the Docker host: route the API HTTPS origin to 127.0.0.1:18080 and the separate Grafana HTTPS origin to 127.0.0.1:13000. ClickHouse has no host port.

Only for Tailscale, pass `--require-tailnet --bind-ip 100.64.0.1` with the actual server Tailnet IPv4. This sets `GROUNDLINE_REQUIRE_TAILNET=true`. Standard HTTPS uses false and preserves enrollment, collector and admin token authentication, rate limits and bounded requests. Clients skip Tailscale probing for ordinary HTTPS endpoints. The API does not terminate TLS itself; expose a valid TLS proxy, never a public plaintext API. Loopback HTTP is supported for local development; Tailnet HTTP is optional.

## Choose the Insights enrollment credential

Use the API container's `GROUNDLINE_ENROLLMENT_TOKEN` for the collector enrollment
key. The Compose generator names the same value `ENROLLMENT_TOKEN` in its private
`secrets.json`. A TrueNAS management API key is for NAS administration, and a
Grafana administrator password is for dashboard sign-in. Give collectors the
Insights enrollment key.

If collection returns `api_upgrade_required`, update the server to an Insights
API distribution that advertises the collector's current ingest contract. A TrueNAS custom
app's `1.0.0` or up-to-date label does not identify the API product version. Verify
server startup, enrollment credentials, and accepted stored uploads separately.

## Requirements

Existing deployments without `GROUNDLINE_REQUIRE_TAILNET` retain Tailnet-only
API access after an image upgrade. To deliberately switch to general HTTPS,
first configure and verify the TLS proxy, then set the variable to `false` in
the private server configuration. The update controller preserves explicit
`true`/`false`, adds `true` when absent, and rejects ambiguous values.

- Docker Engine or Docker Desktop with Compose v2;
- an HTTPS reverse proxy on the Docker host; Tailscale is optional;
- Rust stable for the source-checkout deployment tools;
- an HTTPS access origin for Grafana, normally supplied by Tailscale Serve or an
  owner-managed reverse proxy;
- a reviewed GroundLine source release tag and Insights API image digest for
  production;
- one infrastructure compatibility profile. The checked-in
  `infrastructure/compatibility.json` is the release-tested default, not a
  permanently supported maximum version.

Linux, macOS, and Windows Docker hosts can render absolute dataset roots. Use a
path shared with the Docker VM on Docker Desktop. The collector plugin itself is
released separately for ARM64 and x86-64 on all three operating systems.

## 1. Check out one source release

```console
RELEASE_TAG="vMAJOR.MINOR.PATCH"
INSIGHTS_IMAGE_DIGEST="ghcr.io/jukqaz/groundline-insights-api@sha256:REPLACE_WITH_64_HEX_DIGEST"
INSIGHTS_ACCESS_URL="https://grafana.example.com"
COMPATIBILITY_PROFILE="infrastructure/compatibility.json"
git clone https://github.com/jukqaz/groundline.git
cd groundline
git fetch --tags
git switch --detach "$RELEASE_TAG"
```

Replace all three placeholder assignments before running the block. Set
`RELEASE_TAG` to the reviewed `vMAJOR.MINOR.PATCH` release. Obtain the
matching binary checksums from GitHub Releases and inspect
`ghcr.io/jukqaz/groundline-insights-api:$RELEASE_TAG` in GHCR. Copy the published
multi-platform index digest into an immutable image reference such as
`ghcr.io/jukqaz/groundline-insights-api@sha256:...`. Do not deploy a moving image
tag in production.

The source tag does not contain installed plugin binaries. Collector installation
uses the verified `stable` distribution separately. A tag or release name alone
does not prove GitHub release locking; verify the exact commit, asset checksums,
and signed provenance before deployment.

PowerShell uses the same versioned source and digest-pinned image:

```powershell
$ReleaseTag = "vMAJOR.MINOR.PATCH"
$InsightsImageDigest = "ghcr.io/jukqaz/groundline-insights-api@sha256:REPLACE_WITH_64_HEX_DIGEST"
$InsightsAccessUrl = "https://grafana.example.com"
$CompatibilityProfile = "infrastructure/compatibility.json"
git clone https://github.com/jukqaz/groundline.git
Set-Location groundline
git fetch --tags
git switch --detach $ReleaseTag
```

## 2. Prepare private owner paths

The following Unix shell example keeps generated configuration outside Git:

```console
REPOSITORY_ROOT="$(pwd)"
DEPLOY_ROOT="$(dirname "$REPOSITORY_ROOT")/groundline-insights-owner"
DATASET_ROOT="$DEPLOY_ROOT/data"
COMPOSE_FILE="$DEPLOY_ROOT/compose.yaml"
SECRETS_FILE="$DEPLOY_ROOT/secrets.json"
BIND_IP="127.0.0.1"
mkdir -p "$DATASET_ROOT/clickhouse" "$DATASET_ROOT/grafana"
chmod 0750 "$DATASET_ROOT/clickhouse" "$DATASET_ROOT/grafana"
```

On Windows PowerShell, use resolved absolute paths under a Docker-shared folder:

```powershell
$DeployRoot = Join-Path $env:LOCALAPPDATA "GroundLine\Insights"
$DatasetRoot = Join-Path $DeployRoot "data"
$ComposeFile = Join-Path $DeployRoot "compose.yaml"
$SecretsFile = Join-Path $DeployRoot "secrets.json"
$BindIp = "127.0.0.1"
New-Item -ItemType Directory -Force "$DatasetRoot\clickhouse", "$DatasetRoot\grafana"
```

Do not place the owner directory inside the repository. Spaces in absolute Unix,
macOS volume, and Windows drive paths are supported; relative paths, UNC shares,
and traversal segments are rejected.

## 3. Render without printing secrets

```console
cargo run --locked -p xtask -- render-compose \
  --output "$COMPOSE_FILE" \
  --secrets-file "$SECRETS_FILE" \
  --dataset-root "$DATASET_ROOT" \
  --bind-ip "$BIND_IP" \
  --dashboard-port 13000 \
  --ingest-port 18080 \
  --image "$INSIGHTS_IMAGE_DIGEST" \
  --compatibility-profile "$COMPATIBILITY_PROFILE" \
  --access-url "$INSIGHTS_ACCESS_URL" \
  --json
docker compose -f "$COMPOSE_FILE" config --quiet
```

PowerShell equivalent:

```powershell
cargo run --locked -p xtask -- render-compose `
  --output $ComposeFile `
  --secrets-file $SecretsFile `
  --dataset-root $DatasetRoot `
  --bind-ip $BindIp `
  --dashboard-port 13000 `
  --ingest-port 18080 `
  --image $InsightsImageDigest `
  --compatibility-profile $CompatibilityProfile `
  --access-url $InsightsAccessUrl `
  --json
docker compose -f $ComposeFile config --quiet
```

`INSIGHTS_IMAGE_DIGEST` must be an immutable registry digest. The compatibility
profile supplies the ClickHouse, Nginx, and Grafana image references plus the
Grafana ClickHouse plugin reference. Normal rendering requires every image to
contain `@sha256` and the plugin to contain an exact stable semantic version.
`INSIGHTS_ACCESS_URL` must be an HTTPS origin without a path, query, fragment, or
embedded credentials. The renderer creates six random service credentials in a
separate private file, writes the rendered Compose file with private
permissions, rejects public bind addresses, and refuses overwrite unless
explicitly requested.

Both `SECRETS_FILE` and the rendered `COMPOSE_FILE` contain live service
credentials and are secret-bearing owner files. Protect, back up, rotate, and
delete them under the same policy; the Compose file is not a public derivative.
For `insights fetch-report`, provision a separate private file containing only
the `GROUNDLINE_ADMIN_TOKEN` value and pass it with `--admin-token-file`. Do not
reuse or copy a collector token: collector credentials are intentionally denied
access to the owner-wide report.

The checked-in service defaults retain events for 365 days, cap each collector
at 4,096 retained events and 256 MiB of logical payload, and reserve the last
10% of the two-million-row or 64 GiB dataset ceiling for administrative work.
Override `GROUNDLINE_RETENTION_DAYS`, `GROUNDLINE_COLLECTOR_MAX_EVENTS`,
`GROUNDLINE_COLLECTOR_MAX_PAYLOAD_BYTES`, `GROUNDLINE_DATASET_MAX_ROWS`, or
`GROUNDLINE_DATASET_MAX_BYTES` only in an owner-private rendered Compose file;
the API rejects values outside its documented safety bounds.

The one-shot `grafana-storage-init` service narrows the Grafana bind directory
to UID 472 with mode `0750`; do not work around ownership failures with `0777`.
Grafana needs outbound access on first start to download the pinned ClickHouse
datasource plugin. The API, ClickHouse, and ingress remain on the private data
network. Ingress also uses a dedicated bridge required for Docker's published
loopback or Tailnet port; Nginx allows loopback, the bridge gateway, and Tailnet sources and
keeps its destination fixed to the API. Grafana uses a different
plugin-download egress network. These bridges isolate service paths but are not
an application-layer outbound firewall; production operators should add host
egress policy when that boundary is required. Grafana usage reporting, version
checks, plugin update checks, and
automatic updates of preinstalled plugins are disabled; only the dependency
selected by the compatibility profile is installed.

## Qualify newer dependencies

GroundLine does not encode a maximum supported ClickHouse, Nginx, Grafana, or
datasource-plugin version in the Compose template. To test a newer combination,
create another four-component compatibility JSON file outside Git and pass it
with `--compatibility-profile`. Use exact registry digests and an exact stable
plugin version for a reproducible candidate. `verify-compatibility-profile`
rejects partial, malformed, prerelease, or unknown dependency fields before
Docker starts.

For discovery only, all three image references may use explicit moving tags and
the plugin may be exactly `grafana-clickhouse-datasource`; add
`--allow-unpinned-dependencies` to both verification and rendering. Grafana then
selects the current plugin version. Never retain that rendered file for
production. After the complete stack and all 20 provisioned queries pass, copy
the resolved image digests and installed plugin version into a pinned candidate
profile, rerender without the override, and repeat the verification.

The manual GitHub workflow exposes four candidate inputs. Supply all four or
none. It starts the selected ClickHouse image, runs the mutation integration
lane, renders the selected full stack, and executes authenticated Grafana
semantic checks. A successful candidate run is compatibility evidence; it does
not rewrite the release-tested profile, publish an image, promote `stable`, or
change an owner deployment.

### Migrate an existing installation

Pin the complete qualified profile before changing an owner deployment. Record
the current images, plugin version, configuration fingerprint, and application
table counts. Updating the API image alone does not update ClickHouse, Grafana,
Nginx, or the datasource plugin.

Check the selected image on the actual host before mounting database storage:
`docker run --rm --network none --entrypoint clickhouse <image> --version`.
The [official ClickHouse 26.6+ default amd64 build](https://hub.docker.com/_/clickhouse)
requires x86-64-v3, including AVX2. A passing CI run on a different CPU does not
establish host compatibility.
For an unsupported CPU, explicitly qualify a supported LTS profile for that
host or move to compatible hardware; never silently substitute an image.

Pause ingestion and Grafana before taking a consistent database backup. Keep
ClickHouse data, Grafana's database and plugins, and the private deployment
configuration together on owner-controlled storage. Test the ClickHouse upgrade
against an isolated copy and compare every application table before and after.
Never mount the production data directory into a rehearsal container.

Review stored events with the target collector's `insights validate-event`
command before enabling a stricter API contract or quarantine TTL. Preserve a
typed inventory and a recoverable original backup; invalid historical envelopes
must not be rewritten into fabricated current-contract measurements. Removing
history requires the owner's approval of the concrete scope. Check that no new
receipts arrived between inventory and replacement.

A complete lifecycle read can precede provider usage, especially at
`SessionStart`. Preserve these events for the normal retention period and keep
their observed start/completion counters in reports. An `unavailable` usage
source remains explicitly unmeasured; zero storage counters must not be read as
a measured zero-cost task. Reports retain the `usage_missing` quality reason.
Incomplete source reads and incoherent usage provenance remain quarantined
with the short quarantine TTL. Updating this classification does not rewrite
stored event IDs, payloads, counters, or collection periods.

After migration, verify API storage readiness, all provisioned Grafana queries,
the installed datasource version, and a fresh collector receipt matched to a
database row. Preserve enrollment credentials, collection consent, network
policy, and retention settings. If a database upgrade fails, restore its matching
backup before starting the previous image; an image-only downgrade is not a
database rollback.

## 4. Start and verify the real stack

```console
docker compose -f "$COMPOSE_FILE" up --detach --wait --wait-timeout 240
cargo run --locked -p xtask --bin groundline-deploy -- verify-stack \
  --api-url "http://$BIND_IP:18080/healthz" \
  --grafana-url "http://$BIND_IP:13000/api/health" \
  --access-url "$INSIGHTS_ACCESS_URL" \
  --secrets-file "$SECRETS_FILE" \
  --json
```

PowerShell equivalent:

```powershell
docker compose -f $ComposeFile up --detach --wait --wait-timeout 240
cargo run --locked -p xtask --bin groundline-deploy -- verify-stack `
  --api-url "http://${BindIp}:18080/healthz" `
  --grafana-url "http://${BindIp}:13000/api/health" `
  --access-url $InsightsAccessUrl `
  --secrets-file $SecretsFile `
  --json
```

The verifier waits for API storage readiness, checks Grafana itself, executes
every provisioned dashboard query through the ClickHouse datasource, and
validates fleet, roster, and storage-quality frame semantics. It does not prove
the external HTTPS/Tailscale access gate; verify that separately from an
authorized client before configuring collectors (a Tailnet node in Tailnet mode).

From that authorized client, first confirm TLS reachability and that an
unauthenticated dashboard request is redirected to login or rejected:

```console
curl --fail --silent --show-error --noproxy '*' --connect-timeout 5 --max-time 10 \
  "$INSIGHTS_ACCESS_URL/api/health"
http_status="$(curl --silent --noproxy '*' --connect-timeout 5 --max-time 10 \
  --output /dev/null --write-out '%{http_code}' \
  "$INSIGHTS_ACCESS_URL/d/groundline-insights/groundline-insights")"
case "$http_status" in 302|401) ;; *) echo "unexpected unauthenticated status: $http_status" >&2; exit 1 ;; esac
```

Sign in as `groundline-admin` with the owner-private
`GRAFANA_ADMIN_PASSWORD` stored in `SECRETS_FILE`, then confirm the GroundLine
Insights dashboard loads. Never paste that password into Git, CI, an issue, or a
shared shell transcript.

Keep `GF_AUTH_BASIC_ENABLED=true` for the stack verifier and
`GF_AUTH_DISABLE_LOGIN_FORM=false` for interactive owner login. A healthy
`/api/health` response does not prove either authenticated access or datasource
queries. Diagnose deployment overrides before resetting a stored password.
Operator deployments with JWT-only authentication must be checked through their
configured authenticated access path; Basic or password-login failures alone do
not establish an outage. Preserve that access boundary instead of enabling an
alternate login method merely to make a verifier pass.

The Compose template bounds diagnostic logs: `trace_log` and
`processors_profile_log` keep seven days; `text_log` keeps Trace, Debug, and
Information entries for seven days and the remaining severities for thirty days.
This does not change the separate GroundLine event retention policy. Existing
deployments need explicit configuration application and TTL verification; a
source-template change alone does not clean an already running database.
See [ClickHouse TTL](https://clickhouse.com/docs/concepts/features/operations/delete/ttl).

## 5. Configure each collector

Upgrade the API before installing or enabling updated collectors. Its `/healthz`
must advertise Basic schema 5 and ingest contract revision 6 or newer; enrollment
returns the active collection generation. Existing identities and tokens are
reused. Keep unsupported local state for explicit owner review instead of
deleting it to force a fresh enrollment.

Copy the installed plugin's `references/owner-profile.example.json` outside the
plugin and repository, restrict it to the owner (`0600` on Unix or an equivalent
private ACL on Windows), and replace its endpoint and enrollment placeholder.
From that private directory, run:

```console
groundline-insights worker configure --input owner-profile.json
groundline-insights worker enable
groundline-insights worker run-once
groundline-insights worker status
```

Resolve `groundline-insights` from the installed plugin's `bin/<target>` folder
if Codex did not add it to the shell `PATH`. Confirm accepted upload, ClickHouse
visibility, and Grafana frames independently.

First collection covers seven days; later runs resume from the saved cursor.
Unknown ownership, unreadable records, or inconsistent counters preserve the
incomplete window and stop automatic reads after three attempts. See
[operations troubleshooting](../plugins/groundline-insights/references/operations-troubleshooting.md)
before changing state or retrying a historical gap.

## Failure boundaries

- A successful plugin installation does not prove that the service exists.
- A healthy API does not prove Grafana provisioning or external HTTPS access.
- `docker compose config` validates syntax only; `verify-stack` is the runtime
  and semantic gate.
- A Tailnet address provides reachability, not enrollment authorization.
- Generated Compose, secrets, dataset contents, and verification receipts must
  remain outside public Git.
