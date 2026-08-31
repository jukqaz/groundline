# 연동과 설치 프로필

GroundLine은 하나의 marketplace에서 서로 독립적인 Codex 플러그인 두 개를
제공합니다. 하나를 설치해도 다른 플러그인이 자동으로 설치되거나 활성화되지
않습니다.

## 설치 프로필 선택

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
| Tailscale/Tailnet | 필수 전송 경로 | Tailnet IPv4 또는 `*.ts.net` HTTPS endpoint만 허용 |
| GroundLine Insights API | 내장 | Rust/Axum enrollment, upload, report, 관리 API |
| ClickHouse | 필수 저장소 | API 소유 schema migration, idempotent ingest, 고정 report view |
| CLI JSON report | 내장 | 엄격한 7일·30일·90일 owner report |
| Grafana | 기본 dashboard | provision된 ClickHouse datasource와 고정 dashboard query |
| Docker Compose | 공개 self-hosting preview | placeholder만 포함한 범용 topology, 인증 필수 Grafana, 비공개 secret 렌더링 |
| TrueNAS | 선택형 운영 overlay | 범용 배포 계약 위에서 owner가 실행하는 preflight/apply controller, private inventory는 미포함 |

immutable checkout, private render, 실제 stack, Grafana semantic 검증,
collector enrollment 순서는 [GroundLine Insights 셀프호스팅](self-hosting.md)을
참조합니다. 서버 배포는 source checkout에서 실행하며 Codex 플러그인만 설치해도
Docker service가 설치되지는 않습니다.

collector와 API 계약은 공개 Compose preview와 분리해 release qualification합니다.
운영 준비 완료를 주장하려면 정확한 release image digest, fresh-host stack 검증,
해당 운영 배포의 외부 TLS/Tailnet 인증 검증이 추가로 필요합니다.

각 사용자는 자신의 Tailnet endpoint, enrollment credential, 저장소, retention,
접근 제어를 제공합니다. 공개 플러그인을 설치해도 maintainer의 ClickHouse,
Grafana, Tailnet에 연결되지 않습니다.

## 사용자가 선택할 수 있는 것

Insights 설치 여부, enable/disable 시점, 개인 Tailnet endpoint, 최초 backfill
실행 여부, CLI JSON report와 Grafana dashboard 사용 여부를 선택할 수 있습니다.
report 기간은 7일, 30일, 90일입니다.

집계 데이터만 수집, native hook checkpoint, 최소 900초 간격, diagnostics 비활성,
ambient proxy와 redirect 금지, ClickHouse 저장은 현재 개인정보·보안 불변식입니다.
일반 설정 옵션으로 열지 않습니다.

## 현재 지원하지 않는 연동

- Claude, Hermes, Antigravity 또는 범용 provider collector
- 임의 인터넷 endpoint, webhook, Slack, OpenTelemetry, Prometheus export
- PostgreSQL, SQLite, S3 또는 교체 가능한 저장 backend
- Grafana Cloud 계정 provisioning 또는 hosted GroundLine SaaS
- raw prompt, response, transcript, command, patch, path, 저장소, task, account 전송

새 연동은 versioned contract, 기본 비활성, 사용자 소유 credential, 제한된 payload,
source·package·runtime·storage·dashboard별 검증을 갖춘 명시적 adapter로 추가해야
합니다.
