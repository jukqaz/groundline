# GroundLine Insights 셀프호스팅

Codex 플러그인 설치와 owner가 운영하는 서비스 배포는 서로 다른 작업입니다.
플러그인에는 collector binary가 들어 있고, 서비스 소스 checkout에는 Axum API
image, ClickHouse, Grafana, 비밀값을 출력하지 않는 Compose renderer가 있습니다.
플러그인만 설치해도 maintainer 서비스에 연결되지는 않습니다.

범용 Compose 경로는 선택형 공개 self-hosting preview입니다. hosted GroundLine
서비스가 아니며 Core의 필수 조건도 아닙니다. 운영 준비 완료로 판단하기 전에 정확한
release를 fresh host에서 검증하고 외부 TLS/Tailnet gate를 별도로 확인합니다.

## 요구 사항

- Docker Engine 또는 Docker Desktop과 Compose v2
- 고정 Tailnet IPv4를 가진 Tailscale 연결 host
- source checkout 배포 도구를 실행할 Rust stable
- Tailscale Serve 또는 owner reverse proxy가 제공하는 Grafana HTTPS origin
- 운영 배포에 사용할 immutable GroundLine release tag와 Insights API image digest
- 인프라 compatibility profile 하나. 저장소의
  `infrastructure/compatibility.json`은 해당 release에서 검증한 기본 조합이며
  영구적인 최대 지원 버전이 아닙니다.

Linux, macOS, Windows Docker host에서 절대 dataset path를 렌더링할 수 있습니다.
Docker Desktop에서는 VM과 공유되는 경로를 사용합니다. collector 플러그인은 세
운영체제의 ARM64·x86-64 binary로 별도 배포됩니다.

## 1. immutable release checkout

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

블록을 실행하기 전에 세 placeholder assignment를 모두 실제 값으로 교체합니다.
`RELEASE_TAG`에는 검토한 `vMAJOR.MINOR.PATCH` release를 지정합니다. 같은 release의
binary checksum은 GitHub Release에서 확인하고,
`ghcr.io/jukqaz/groundline-insights-api:$RELEASE_TAG`는 GHCR에서 inspect합니다.
게시된 multi-platform index digest를
`ghcr.io/jukqaz/groundline-insights-api@sha256:...` 형식의 immutable image
reference로 복사합니다. 운영에는 moving image tag를 사용하지 않습니다.

PowerShell도 같은 immutable 입력을 사용합니다.

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

## 2. Git 밖에 owner-private 경로 준비

Unix shell 예시입니다.

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

Windows PowerShell에서는 Docker와 공유되는 absolute path를 사용합니다.

```powershell
$DeployRoot = Join-Path $env:LOCALAPPDATA "GroundLine\Insights"
$DatasetRoot = Join-Path $DeployRoot "data"
$ComposeFile = Join-Path $DeployRoot "compose.yaml"
$SecretsFile = Join-Path $DeployRoot "secrets.json"
$BindIp = (tailscale ip -4 | Select-Object -First 1).Trim()
New-Item -ItemType Directory -Force "$DatasetRoot\clickhouse", "$DatasetRoot\grafana"
```

owner 디렉터리는 저장소 안에 두지 않습니다. Unix·macOS volume·Windows drive의
absolute path에 포함된 공백은 지원하지만 relative path, UNC share, traversal
segment는 거부합니다.

## 3. 비밀값을 출력하지 않고 Compose 렌더링

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

PowerShell에서는 다음과 같습니다.

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

`INSIGHTS_IMAGE_DIGEST`는 immutable registry digest여야 합니다. compatibility
profile은 ClickHouse·Nginx·Grafana image reference와 Grafana ClickHouse plugin
reference를 제공합니다. 일반 렌더링은 모든 image에 `@sha256`이 있고 plugin에는
stable semantic version이 정확히 지정된 경우만 허용합니다.
`INSIGHTS_ACCESS_URL`은 path·query·fragment·내장 credential이 없는 HTTPS
origin이어야 합니다. renderer는 서비스 credential 6개를 별도 private file에
생성하고, Compose를 private permission으로 쓰며, public bind address와 암묵적
overwrite를 거부합니다.

일회성 `grafana-storage-init` service가 Grafana bind directory를 UID 472, mode
`0750`으로 맞춥니다. ownership 오류를 `0777`로 우회하지 않습니다. Grafana는 첫
시작에 고정된 ClickHouse datasource plugin을 내려받기 위한 outbound access가
필요합니다. API·ClickHouse·ingress는 private data network를 공유합니다.
ingress에는 Docker host gateway와 Tailnet source만 식별·허용하기 위한
ingress는 Docker published Tailnet port에 필요한 전용 bridge도 사용하며 Nginx는
해당 bridge gateway와 Tailnet source만 허용하고 목적지를 API로 고정합니다.
Grafana는 별도 plugin-download egress network를 사용합니다. 이 bridge 분리는
service path 격리이지 application-layer outbound firewall은 아니므로 그 경계가
필요한 운영 환경은 host egress policy를 추가해야 합니다. Grafana usage
reporting, 버전 확인,
플러그인 업데이트 확인, preinstalled plugin 자동 업데이트는 비활성화되며
template은 compatibility profile이 선택한 dependency만 설치합니다.

## 더 최신 dependency 조합 검증

Compose template에는 ClickHouse·Nginx·Grafana·datasource plugin의 최대 지원
버전을 박아두지 않습니다. 더 최신 조합을 시험하려면 Git 밖에 네 구성요소를 모두
담은 compatibility JSON을 만들고 `--compatibility-profile`로 넘깁니다. 재현 가능한
후보는 image마다 정확한 registry digest를, plugin에는 정확한 stable 버전을
사용합니다. `verify-compatibility-profile`은 Docker 실행 전에 일부 입력만 있는 조합,
잘못된 형식, prerelease, 알 수 없는 field를 거부합니다.

최신 버전 탐색 단계에 한해서 image 세 개에 명시적 moving tag를 쓰고 plugin을
`grafana-clickhouse-datasource`로만 지정할 수 있습니다. 검증과 렌더링 모두에
`--allow-unpinned-dependencies`를 추가합니다. 이 Compose는 운영에 보존하지 않습니다.
전체 stack과 provision된 query 19개가 통과하면 실제 image digest와 설치된 plugin
버전을 pinned 후보 profile에 기록하고, override 없이 다시 렌더링·검증합니다.

GitHub 수동 workflow에도 후보 입력 네 개가 있습니다. 네 개를 모두 주거나 하나도
주지 않아야 합니다. 선택한 ClickHouse에서 mutation integration lane을 실행하고,
선택한 전체 stack을 렌더링해 인증된 Grafana semantic check까지 수행합니다. 후보
run 성공은 호환성 증거일 뿐 기본 profile 수정, image 게시, `stable` 승격, owner
배포를 자동 수행하지 않습니다.

## 4. 실제 stack 시작과 semantic 검증

```console
docker compose -f "$COMPOSE_FILE" up --detach --wait --wait-timeout 240
cargo run --locked -p xtask --bin groundline-deploy -- verify-stack \
  --api-url "http://$BIND_IP:18080/healthz" \
  --grafana-url "http://$BIND_IP:13000/api/health" \
  --access-url "$INSIGHTS_ACCESS_URL" \
  --secrets-file "$SECRETS_FILE" \
  --json
```

PowerShell에서는 다음과 같습니다.

```powershell
docker compose -f $ComposeFile up --detach --wait --wait-timeout 240
cargo run --locked -p xtask --bin groundline-deploy -- verify-stack `
  --api-url "http://${BindIp}:18080/healthz" `
  --grafana-url "http://${BindIp}:13000/api/health" `
  --access-url $InsightsAccessUrl `
  --secrets-file $SecretsFile `
  --json
```

verifier는 API storage readiness와 Grafana 자체 상태를 확인한 뒤 provision된 모든
dashboard query를 ClickHouse datasource를 통해 실행하고 fleet·roster·storage
quality frame 의미까지 검증합니다. 외부 HTTPS/Tailscale access gate는 이 명령의
검증 대상이 아니므로 collector 설정 전에 다른 Tailnet node에서 별도로 확인합니다.

다른 Tailnet node에서 TLS reachability와 미인증 dashboard 요청이 login으로
redirect되거나 거부되는지 먼저 확인합니다.

```console
curl --fail --silent --show-error --noproxy '*' --connect-timeout 5 --max-time 10 \
  "$INSIGHTS_ACCESS_URL/api/health"
http_status="$(curl --silent --noproxy '*' --connect-timeout 5 --max-time 10 \
  --output /dev/null --write-out '%{http_code}' \
  "$INSIGHTS_ACCESS_URL/d/groundline-insights/groundline-insights")"
case "$http_status" in 302|401) ;; *) echo "unexpected unauthenticated status: $http_status" >&2; exit 1 ;; esac
```

그 다음 `SECRETS_FILE`의 owner-private `GRAFANA_ADMIN_PASSWORD`를 사용해
`groundline-admin`으로 로그인하고 GroundLine Insights dashboard가 열리는지
확인합니다. 이 password를 Git, CI, issue, 공유 shell transcript에 붙여 넣지
않습니다.

## 5. collector별 설정

설치된 플러그인의 `references/owner-profile.example.json`을 플러그인 밖으로
복사하고 endpoint와 enrollment placeholder를 owner-private 값으로 바꾼 뒤
실행합니다.

```console
groundline-insights worker configure --input owner-profile.json
groundline-insights worker enable
groundline-insights worker run-once
groundline-insights worker status
```

Codex가 shell `PATH`를 만들지 않았다면 설치된 plugin의 `bin/<target>`에서
`groundline-insights`를 실행합니다. accepted upload, ClickHouse 반영, Grafana
frame은 각각 따로 확인합니다.

## 실패 경계

- plugin 설치 성공은 service 존재를 증명하지 않습니다.
- API health는 Grafana provisioning이나 외부 HTTPS access를 증명하지 않습니다.
- `docker compose config`는 문법만 검사합니다. 실제 runtime·semantic gate는
  `verify-stack`입니다.
- Tailnet 주소는 reachability만 제공하며 enrollment 권한은 별도입니다.
- 생성된 Compose, secrets, dataset, 검증 receipt는 public Git 밖에 둡니다.
