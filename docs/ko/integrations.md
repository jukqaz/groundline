# 연동과 설치 프로필

GroundLine은 하나의 marketplace에서 서로 독립적인 Codex 플러그인 두 개를
제공합니다. 하나를 설치해도 다른 플러그인이 자동으로 설치되거나 활성화되지
않습니다.

## 설치 프로필 선택

Insights 연결 경로는 네이티브 Codex App/CLI의 hook·읽기 전용 활동 데이터에서
비공개 집계 outbox, 운영자 HTTPS Insights API, ClickHouse, Grafana/JSON report로
이어집니다. 추론 프록시, 생성된 모델 카탈로그, custom provider, Core 설치는
필요하지 않습니다. Insights는 Codex `config.toml`이나 추론 인증 정보를 읽거나
고치지 않습니다. 프록시 제거 후 남은 Codex provider 설정은 네이티브 Codex
시작 문제로 별도 처리하고 Insights의 identity·동의·cursor·outbox는 보존합니다.

동일한 네이티브 `CODEX_HOME`을 유지합니다. 다른 홈은 자동 마이그레이션 대상이
아닙니다. `doctor`와 `worker status`는 수집기와 같은 규칙으로 가장 높은 번호의
`state_<n>.sqlite`를 찾습니다. 데이터 원본이 없거나 안전하게 열리지 않으면
`native_activity_unavailable`, `ready_to_collect: false`로 표시합니다. 파일 존재는
스키마·서버 저장·Grafana 검증이 아닙니다. 기존 전송 대기나 운영 조치 사유가 있으면
그 사유가 우선하며, 원본 누락 때문에 pending event를 삭제하거나 전송을 막지 않습니다.

| 프로필 | 설치 대상 | 외부 서비스 | 용도 |
| --- | --- | --- | --- |
| Core만 | `groundline` | 없음 | 로컬 가이드, 감사, 증거 계약 |
| Insights만 | `groundline-insights` | 사용자 소유 Insights 서비스 | Core skill이 필요 없는 collector·운영 노드 |
| Core + Insights | 플러그인 둘 다 | 사용자 소유 Insights 서비스 | 로컬 GroundLine 작업과 비공개 집계 분석을 함께 사용 |

marketplace는 한 번만 등록하고 선택한 플러그인 ID만 설치합니다.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
codex plugin add groundline-insights@groundline --json
```

두 `plugin add` 명령은 결합 프로필을 선택한 경우에만 모두 실행합니다. 공유
marketplace를 갱신해도 설치하지 않은 형제 플러그인이 자동 설치되지는 않습니다.

## 현재 Insights 연동

| 대상 | 상태 | 계약 |
| --- | --- | --- |
| Codex App | 내장 | 명시적 활성화 후 fail-open lifecycle checkpoint 4개 |
| Codex CLI | 내장 | desktop, local headless, remote headless 메타데이터 |
| HTTPS | 기본 전송 경로 | 운영자 HTTPS origin, 인증서 검증 및 리다이렉트 거부 |
| Tailscale/Tailnet | 선택형 전송 경로 | Tailnet IPv4 또는 `*.ts.net`; 해당 주소만 로컬 Tailnet 상태 확인 |
| GroundLine Insights API | 내장 | Rust/Axum enrollment, upload, report, 관리 API |
| ClickHouse | 필수 저장소 | API 소유 schema migration, idempotent ingest, 고정 report view |
| CLI JSON report | 내장 | 별도 admin-token 파일이 필요한 엄격한 7일·30일·90일 owner report, collector token은 거부 |
| Grafana | 기본 dashboard | provision된 ClickHouse datasource와 고정 dashboard query |
| Docker Compose | 공개 self-hosting preview | placeholder만 포함한 범용 topology, 인증 필수 Grafana, 비공개 secret 렌더링 |
| TrueNAS | 선택형 운영 overlay | 범용 배포 계약 위에서 owner가 실행하는 preflight/apply controller, private inventory는 미포함 |

버전이 지정된 소스 checkout, 비공개 설정 렌더링, 실제 stack, Grafana semantic 검증,
collector enrollment 순서는 [GroundLine Insights 셀프호스팅](self-hosting.md)을
참조합니다. 서버 배포는 source checkout에서 실행하며 Codex 플러그인만 설치해도
Docker service가 설치되지는 않습니다.

collector와 API 계약은 공개 Compose preview와 분리해 release qualification합니다.
운영 준비 완료를 주장하려면 정확한 release image digest, fresh-host stack 검증,
해당 운영 배포의 외부 TLS/Tailnet 인증 검증이 추가로 필요합니다.

각 사용자는 자신의 HTTPS 또는 선택형 Tailnet endpoint, enrollment credential, 저장소, retention,
접근 제어를 제공합니다. 공개 플러그인을 설치해도 maintainer의 ClickHouse,
Grafana, Tailnet에 연결되지 않습니다.

## 개인 운영 경계

지원 구조는 공용 서비스 가입이 아니라 자기 서비스 연결입니다. 서로 다른 운영자는
각자 별도의 Insights 인스턴스, 저장소, credential을 사용하고, 수집하려는 자신의
Codex 홈만 설정합니다. collector UUID는 한 운영자의 설치본을 구분하는 값이지
다중 사용자 계정이나 tenant 격리 경계가 아닙니다. 공개 회원가입, 서비스 자동 검색,
maintainer 기본 endpoint는 없습니다. 개인 배포 설정과 데이터는 공개 저장소와
배포 package 밖에 둡니다.

| 자격 증명 | 보관 위치 | 용도 |
| --- | --- | --- |
| TrueNAS 관리 API 키, 사용하는 경우 | 비공개 운영자 자격 증명 저장소 | NAS 앱 조회·배포. 수집 전용 호스트에는 설치하지 않음 |
| Insights enrollment credential | 비공개 서버 설정과 승인된 수집기 설정 | 선택한 운영자 서비스에 수집기 등록 |
| collector별 token | 해당 수집기의 비공개 로컬 상태 | 해당 수집기 범위의 전송·작업 인증 |
| Insights admin token과 Grafana 로그인 | 비공개 운영 도구와 dashboard 접근 | 운영자 전체 report와 dashboard 관리. 수집기 등록에는 사용하지 않음 |

사람과 LLM 모두 문서의 `worker configure`, `enable`, `run-once`, `status`를
사용합니다. 서비스 주소와 enrollment credential은 운영자가 제공해야 하며, 설치나
설정만으로 수집에 동의한 것이 아닙니다. 수집 범위를 확인하고 명시적으로 활성화합니다.
서버 배포는 별도 운영 단계이며 공개 플러그인 업그레이드가 개인 서비스를 자동으로
업그레이드하거나 재설정하지 않습니다.

## 사용자가 선택할 수 있는 것

Insights 설치 여부, enable/disable 시점, 개인 HTTPS 또는 선택형 Tailnet endpoint, 수집 재시도
시점, CLI JSON report와 Grafana dashboard 사용 여부를 선택할 수 있습니다.
report 기간은 7일, 30일, 90일입니다.

최초 수집은 최근 7일이며 이후에는 저장된 커서를 사용합니다.
`worker backfill-history --confirm-rebuild`는 같은 수집 경로를 재시도합니다.
커서를 되돌리거나 전체 과거 이력·기존 서버 집계를 다시 만드는 명령이 아닙니다.
복구할 수 없는 과거 구간은 원본과 누락 구간 기록을 보존하고 운영자의 명시적
결정이 있을 때만 수집 시작 경계를 바꿉니다.

집계 데이터만 수집, native hook checkpoint, 최소 900초 간격, diagnostics 비활성,
ambient proxy와 redirect 금지, ClickHouse 저장은 현재 개인정보·보안 불변식입니다.
일반 설정 옵션으로 열지 않습니다.

## 현재 지원하지 않는 연동

- Claude, Hermes, Antigravity 또는 범용 provider collector
- 범용 webhook, Slack, OpenTelemetry, Prometheus export
- PostgreSQL, SQLite, S3 또는 교체 가능한 저장 backend
- Grafana Cloud 계정 provisioning 또는 hosted GroundLine SaaS
- raw prompt, response, transcript, command, patch, path, 저장소, task, account 전송

새 연동은 versioned contract, 기본 비활성, 사용자 소유 credential, 제한된 payload,
source·package·runtime·storage·dashboard별 검증을 갖춘 명시적 adapter로 추가해야
합니다.
