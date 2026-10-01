# GroundLine Insights 셀프호스팅

이 문서는 운영자 소유 API·ClickHouse·Grafana를 배포합니다.
[수집기 설치](../installation.md)는 별도입니다. 공개 Compose는 self-hosting
preview이며 Core의 필수 조건이 아닙니다. 운영 준비는 정확한 릴리스를 실제
호스트에서 검증하고 인증된 외부 접근까지 확인해야 판단할 수 있습니다.

## 일반 HTTPS와 선택형 Tailscale

기본값은 `--bind-ip 127.0.0.1`입니다. 호스트의 HTTPS 프록시에서 Insights API 주소를 127.0.0.1:18080, Grafana 주소를 127.0.0.1:13000으로 연결합니다. 두 서비스의 외부 HTTPS 주소는 별도로 지정합니다. ClickHouse에는 외부 포트를 열지 않습니다.

Tailscale을 선택할 때만 `--require-tailnet --bind-ip 100.64.0.1`을 지정하고 실제 서버의 Tailnet IPv4로 바꾸세요. API의 `GROUNDLINE_REQUIRE_TAILNET=true`가 전용 접근을 강제합니다. 기본 일반 HTTPS 모드에서는 이 값이 false이며, 등록키·수집기 토큰·관리자 토큰 인증과 요청 제한은 그대로 적용됩니다. Tailscale 접근이 필요 없는 클라이언트는 로컬 Tailscale 설치나 로그인 상태를 검사하지 않습니다.

API 자체는 TLS를 종료하지 않습니다. 외부에는 유효한 인증서를 가진 HTTPS 프록시를 제공하세요. 공개 인터넷에서 HTTP로 자격증명을 전송하지 마세요. 로컬 개발용 loopback HTTP와 선택형 Tailnet HTTP만 예외로 허용합니다.

## 요구 사항

`GROUNDLINE_REQUIRE_TAILNET` 값이 없는 기존 배포는 이미지 업데이트 후에도
Tailnet 전용 접근을 유지합니다. 일반 HTTPS로 전환하려면 TLS 프록시를 먼저
구성·검증한 뒤 비공개 서버 설정에서 값을 명시적으로 `false`로 바꿉니다.
업데이트 도구는 기존 `true`·`false`를 보존하고 누락 시 `true`를 추가하며,
잘못된 값은 거부합니다.

- Docker Engine 또는 Docker Desktop과 Compose v2
- Docker host의 HTTPS reverse proxy. Tailscale은 선택 사항입니다.
- source checkout 배포 도구를 실행할 Rust stable
- Tailscale Serve 또는 owner reverse proxy가 제공하는 Grafana HTTPS origin
- 운영 배포에 사용할 검토한 GroundLine 소스 태그와 Insights API 이미지 digest
- 인프라 compatibility profile 하나. 저장소의
  `infrastructure/compatibility.json`은 해당 release에서 검증한 기본 조합이며
  영구적인 최대 지원 버전이 아닙니다.

Linux와 macOS Docker host에서 절대 dataset path를 렌더링할 수 있습니다.
Docker Desktop에서는 VM과 공유되는 경로를 사용합니다. collector 플러그인은 두
운영체제의 ARM64·x86-64 binary로 별도 배포됩니다.

## 1. 버전이 지정된 소스 checkout

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

블록을 실행하기 전에 세 placeholder assignment를 모두 실제 값으로 교체합니다.
`RELEASE_TAG`에는 canonical 숫자 버전의 태그를 지정합니다.
예를 들어 `v2026.929.1`의 표시 이름은 `2026.09.29-a`입니다. 같은 release의
binary checksum은 GitHub Release에서 확인하고,
`ghcr.io/jukqaz/groundline-insights-api:$RELEASE_TAG`는 GHCR에서 inspect합니다.
게시된 multi-platform index digest를
`ghcr.io/jukqaz/groundline-insights-api@sha256:...` 형식의 immutable image
reference로 복사합니다. 운영에는 moving image tag를 사용하지 않습니다.

소스 태그에는 설치용 플러그인 실행 파일이 없습니다. 수집기는 검증한 `stable`
배포본으로 별도 설치합니다. 태그·릴리스 이름만으로 GitHub의 변경 잠금이
입증되지는 않으므로 정확한 커밋, 자산 체크섬, 서명된 빌드 출처를 확인합니다.

## 2. Git 밖에 owner-private 경로 준비

Unix shell 예시입니다.

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

owner 디렉터리는 저장소 안에 두지 않습니다. Linux·macOS absolute path에
포함된 공백은 지원하지만 relative path, Windows drive path, UNC share, traversal
segment는 거부합니다.

## 3. 비밀값을 출력하지 않고 Compose 렌더링

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

`INSIGHTS_IMAGE_DIGEST`는 immutable registry digest여야 합니다. compatibility
profile은 ClickHouse·Nginx·Grafana image reference와 Grafana ClickHouse plugin
reference를 제공합니다. 일반 렌더링은 모든 image에 `@sha256`이 있고 plugin에는
stable semantic version이 정확히 지정된 경우만 허용합니다.
`INSIGHTS_ACCESS_URL`은 path·query·fragment·내장 credential이 없는 HTTPS
origin이어야 합니다. renderer는 서비스 credential 6개를 별도 private file에
생성하고, Compose를 private permission으로 쓰며, public bind address와 암묵적
overwrite를 거부합니다.

`SECRETS_FILE`과 rendered `COMPOSE_FILE`에는 모두 실제 service credential이
들어 있으므로 같은 owner-secret 정책으로 보호·백업·회전·삭제해야 합니다.
Compose 파일도 공개 가능한 파생물이 아닙니다.

저장소 초기화 서비스는 Grafana 디렉터리를 UID 472, mode `0750`으로 맞춥니다.
소유권 오류를 `0777`로 우회하지 않습니다. Grafana는 첫 시작에 고정된 plugin을
내려받습니다. API·ClickHouse·ingress의 private network, ingress의 포트 게시용
bridge, Grafana의 download bridge는 outbound firewall을 대신하지 않습니다.
필요한 환경에는 호스트 egress 정책을 적용합니다. Grafana 사용 보고와 자동
버전·plugin 업데이트는 꺼져 있습니다. 보존·용량·로그 정책은
[운영 안내](../insights-operations.md#diagnostic-logging)에 모았습니다.

## 더 최신 dependency 조합 검증

기본 profile은 검증한 조합이며 최대 지원 버전이 아닙니다. 변경 전
[의존성 검증과 마이그레이션](../insights-operations.md#dependency-upgrades-and-recovery)을
따릅니다. API 이미지만 바꾸면 ClickHouse·Grafana·Nginx·datasource plugin은
바뀌지 않습니다. 데이터와 설정의 일관된 백업을 보존하고 격리 복제본에서 검증합니다.
이미지만 되돌리는 것은 DB 복원이 아닙니다.

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

verifier는 API storage readiness와 Grafana 자체 상태를 확인한 뒤 provision된 모든
dashboard query를 ClickHouse datasource를 통해 실행하고 fleet·roster·storage
quality frame 의미까지 검증합니다. 외부 HTTPS/Tailscale access gate는 이 명령의
검증 대상이 아니므로 collector 설정 전에 허용된 클라이언트에서 별도로 확인합니다.
Tailnet 모드에서는 다른 Tailnet node를 사용합니다.

그 클라이언트에서 TLS reachability와 미인증 dashboard 요청이 login으로
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

새 수집기보다 API를 먼저 업데이트합니다. 현재 요구 사항은 Basic schema 5와
ingest contract revision 9 이상입니다. `worker check-server --json`은 등록·동의
변경·수집 없이 이 호환성을 확인합니다. TrueNAS 앱의 버전 표기만으로 API 제품
버전이나 실제 저장 상태를 판단하지 않습니다.

등록키는 API의 `GROUNDLINE_ENROLLMENT_TOKEN`(생성한 secrets의
`ENROLLMENT_TOKEN`)이며 NAS 관리 키·Grafana 비밀번호와 다릅니다. 기존 identity·
token과 지원하지 않는 상태는 검토를 위해 보존하고 등록을 강제하려고 삭제하지
않습니다. [자격증명 역할](integrations.md#개인-운영-경계)을 확인하세요.

설치된 플러그인의 `references/owner-profile.example.json`을 플러그인·저장소
밖으로 복사하고 소유자만 읽을 수 있게 `0600`으로 제한합니다.
endpoint와 enrollment placeholder를 채운 뒤
그 비공개 디렉터리에서 실행합니다.

```console
groundline-insights worker configure --input owner-profile.json
groundline-insights worker check-server --json
groundline-insights worker enable
groundline-insights worker run-once
groundline-insights worker status
```

Codex가 shell `PATH`를 만들지 않았다면 설치된 plugin의 `bin/<target>`에서
`groundline-insights`를 실행합니다. accepted upload, ClickHouse 반영, Grafana
frame은 각각 따로 확인합니다.

최초 수집·제한된 재시도·운영자 보고서는
[운영 안내](../insights-operations.md#collector-verification-and-retries)를 따릅니다.
플러그인 설치, Compose 문법, API health, 인증된 Grafana query, 새 업로드와 DB
반영은 별도 증거입니다. 생성한 설정·데이터·검증 영수증은 공개 Git 밖에 둡니다.
