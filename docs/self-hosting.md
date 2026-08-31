# Self-hosting GroundLine Insights

The Codex plugin and the owner-operated service are separate installations. The
plugin contains the collector binary; the service checkout supplies the Axum
API image, ClickHouse, Grafana, and the secret-safe Compose renderer. Installing
the plugin never connects to a maintainer service.

The generic Compose path is an optional public self-hosting preview. It is not a
hosted GroundLine service and it is not required by Core. Qualify the exact
release on a fresh host and verify the external TLS/Tailnet gate before treating
an operator deployment as production-ready.

## Requirements

- Docker Engine or Docker Desktop with Compose v2;
- a Tailscale-connected host with a stable Tailnet IPv4 address;
- Rust stable for the source-checkout deployment tools;
- an HTTPS access origin for Grafana, normally supplied by Tailscale Serve or an
  owner-managed reverse proxy;
- an immutable GroundLine release tag and Insights API image digest for
  production;
- one infrastructure compatibility profile. The checked-in
  `infrastructure/compatibility.json` is the release-tested default, not a
  permanently supported maximum version.

Linux, macOS, and Windows Docker hosts can render absolute dataset roots. Use a
path shared with the Docker VM on Docker Desktop. The collector plugin itself is
released separately for ARM64 and x86-64 on all three operating systems.

## 1. Check out one immutable release

```console
RELEASE_TAG="vMAJOR.MINOR.PATCH"
INSIGHTS_IMAGE_DIGEST="ghcr.io/jukqaz/groundline-insights-api@sha256:REPLACE_WITH_64_HEX_DIGEST"
INSIGHTS_ACCESS_URL="https://insights.REPLACE_WITH_YOUR_TAILNET.ts.net"
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

PowerShell uses the same immutable inputs:

```powershell
$ReleaseTag = "vMAJOR.MINOR.PATCH"
$InsightsImageDigest = "ghcr.io/jukqaz/groundline-insights-api@sha256:REPLACE_WITH_64_HEX_DIGEST"
$InsightsAccessUrl = "https://insights.REPLACE_WITH_YOUR_TAILNET.ts.net"
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
BIND_IP="$(tailscale ip -4 | head -n 1)"
mkdir -p "$DATASET_ROOT/clickhouse" "$DATASET_ROOT/grafana"
chmod 0750 "$DATASET_ROOT/clickhouse" "$DATASET_ROOT/grafana"
```

On Windows PowerShell, use resolved absolute paths under a Docker-shared folder:

```powershell
$DeployRoot = Join-Path $env:LOCALAPPDATA "GroundLine\Insights"
$DatasetRoot = Join-Path $DeployRoot "data"
$ComposeFile = Join-Path $DeployRoot "compose.yaml"
$SecretsFile = Join-Path $DeployRoot "secrets.json"
$BindIp = (tailscale ip -4 | Select-Object -First 1).Trim()
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
  --tailscale-bind-ip "$BIND_IP" \
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
  --tailscale-bind-ip $BindIp `
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

The one-shot `grafana-storage-init` service narrows the Grafana bind directory
to UID 472 with mode `0750`; do not work around ownership failures with `0777`.
Grafana needs outbound access on first start to download the pinned ClickHouse
datasource plugin. The API, ClickHouse, and ingress remain on the private data
network. Ingress also uses a dedicated bridge required for Docker's published
Tailnet port; Nginx allows only the bridge gateway plus Tailnet sources and
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
production. After the complete stack and all 19 provisioned queries pass, copy
the resolved image digests and installed plugin version into a pinned candidate
profile, rerender without the override, and repeat the verification.

The manual GitHub workflow exposes four candidate inputs. Supply all four or
none. It starts the selected ClickHouse image, runs the mutation integration
lane, renders the selected full stack, and executes authenticated Grafana
semantic checks. A successful candidate run is compatibility evidence; it does
not rewrite the release-tested profile, publish an image, promote `stable`, or
change an owner deployment.

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
the external HTTPS/Tailscale access gate; verify that separately from another
Tailnet node before configuring collectors.

From that other Tailnet node, first confirm TLS reachability and that an
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

## 5. Configure each collector

Copy the installed plugin's `references/owner-profile.example.json` outside the
plugin, replace the endpoint and enrollment placeholder with owner-private
values, then run:

```console
groundline-insights worker configure --input owner-profile.json
groundline-insights worker enable
groundline-insights worker run-once
groundline-insights worker status
```

Resolve `groundline-insights` from the installed plugin's `bin/<target>` folder
if Codex did not add it to the shell `PATH`. Confirm accepted upload, ClickHouse
visibility, and Grafana frames independently.

## Failure boundaries

- A successful plugin installation does not prove that the service exists.
- A healthy API does not prove Grafana provisioning or external HTTPS access.
- `docker compose config` validates syntax only; `verify-stack` is the runtime
  and semantic gate.
- A Tailnet address provides reachability, not enrollment authorization.
- Generated Compose, secrets, dataset contents, and verification receipts must
  remain outside public Git.
