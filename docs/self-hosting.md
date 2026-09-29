# Self-hosting GroundLine Insights

This guide deploys the owner-operated API, ClickHouse, and Grafana. Collector
[installation](installation.md) is separate. The public Compose path is a
self-hosting preview; Core does not require it. Production evidence requires the
exact release on the actual host plus authenticated external access checks.

## Standard HTTPS and optional Tailscale

The default is `--bind-ip 127.0.0.1`. Terminate TLS at a reverse proxy on the Docker host: route the API HTTPS origin to 127.0.0.1:18080 and the separate Grafana HTTPS origin to 127.0.0.1:13000. ClickHouse has no host port.

Only for Tailscale, pass `--require-tailnet --bind-ip 100.64.0.1` with the actual server Tailnet IPv4. This sets `GROUNDLINE_REQUIRE_TAILNET=true`. Standard HTTPS uses false and preserves enrollment, collector and admin token authentication, rate limits and bounded requests. Clients skip Tailscale probing for ordinary HTTPS endpoints. The API does not terminate TLS itself; expose a valid TLS proxy, never a public plaintext API. Loopback HTTP is supported for local development; Tailnet HTTP is optional.

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

Linux and macOS Docker hosts can render absolute dataset roots. Use a
path shared with the Docker VM on Docker Desktop. The collector plugin itself is
released separately for ARM64 and x86-64 on both operating systems.

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

Replace the tag, image digest, and access URL placeholders before running.
`RELEASE_TAG` uses the canonical numeric release version, for example
`v2026.929.1` (display name `2026.09.29-a`). Obtain the
matching binary checksums from GitHub Releases and inspect
`ghcr.io/jukqaz/groundline-insights-api:$RELEASE_TAG` in GHCR. Copy the published
multi-platform index digest into an immutable image reference such as
`ghcr.io/jukqaz/groundline-insights-api@sha256:...`. Do not deploy a moving image
tag in production.

The source tag does not contain installed plugin binaries. Collector installation
uses the verified `stable` distribution separately. A tag or release name alone
does not prove GitHub release locking; verify the exact commit, asset checksums,
and signed provenance before deployment.

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

Do not place the owner directory inside the repository. Spaces in absolute Linux and
macOS paths are supported; relative paths, Windows drive paths, UNC shares,
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

The storage initializer sets Grafana's directory to UID 472 and `0750`; do not
use `0777` to bypass ownership errors. Grafana downloads its pinned ClickHouse
plugin on first start. API, ClickHouse, and ingress use a private data network;
ingress has a dedicated published-port bridge and Grafana a separate download
bridge. These are not an outbound firewall. Add host egress policy if required.
Grafana usage reporting and automatic version/plugin updates are disabled.
Storage limits and diagnostic policy are in [operations](insights-operations.md#diagnostic-logging).

## Qualify newer dependencies

The checked-in profile is a tested baseline, not a maximum supported version.
Use [dependency qualification and migration](insights-operations.md#dependency-upgrades-and-recovery)
before changing ClickHouse, Nginx, Grafana, or the datasource plugin. API-only
updates do not update those dependencies. Preserve matching data/configuration
backups; an image rollback does not roll back a database.

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

## 5. Configure each collector

Upgrade the API before collectors. The current collector requires Basic schema
5 and ingest contract revision 8 or newer. `worker check-server --json` verifies
that contract without enrollment, consent changes, or collection. A TrueNAS app
label is not API product-version or storage evidence.

Use the API's `GROUNDLINE_ENROLLMENT_TOKEN` (named `ENROLLMENT_TOKEN` in
`SECRETS_FILE`), not a NAS management key or Grafana password. Keep the existing
identity, tokens, and unsupported state for explicit review; never delete state
to force enrollment. Credential roles are in [integrations](integrations.md#private-owner-deployment-boundary).

Copy the installed plugin's `references/owner-profile.example.json` outside the
plugin and repository, restrict it to the owner (`0600`), and replace its endpoint and enrollment placeholder.
From that private directory, run:

```console
groundline-insights worker configure --input owner-profile.json
groundline-insights worker check-server --json
groundline-insights worker enable
groundline-insights worker run-once
groundline-insights worker status
```

Resolve `groundline-insights` from the installed plugin's `bin/<target>` folder
if Codex did not add it to the shell `PATH`. Confirm accepted upload, ClickHouse
visibility, and Grafana frames independently.

See [collector verification and retries](insights-operations.md#collector-verification-and-retries)
for first collection, bounded retries, and owner reports. Plugin installation,
Compose syntax, API health, authenticated Grafana queries, and fresh stored
uploads each prove different steps. Keep rendered configuration, datasets, and
verification receipts outside public Git.
