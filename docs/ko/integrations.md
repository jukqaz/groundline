# 연동과 설치 프로필

GroundLine은 macOS·Linux의 ARM64·x86-64에서 독립적인 Codex 플러그인 두 개를
제공합니다. [설치 안내](../installation.md)에 설치·업데이트·복구 명령을 모았습니다.
별도 GroundLine Desktop 앱은 없습니다.

## 설치 프로필 선택

| 프로필 | 설치 대상 | 외부 서비스 | 용도 |
| --- | --- | --- | --- |
| Core만 | `groundline` | 없음 | 로컬 가이드, 감사, 증거 계약 |
| Insights만 | `groundline-insights` | 사용자 소유 Insights 서비스 | Core skill이 필요 없는 collector·운영 노드 |
| Core + Insights | 플러그인 둘 다 | 사용자 소유 Insights 서비스 | 로컬 GroundLine 작업과 비공개 집계 분석을 함께 사용 |

하나를 설치해도 다른 플러그인이 자동 설치되거나 활성화되지 않습니다. 다만 공유
marketplace 업데이트는 **이미 설치된 두 플러그인을 함께 갱신**할 수 있습니다.
설치기는 기존 활성·비활성 상태를 보존하고 Insights API 호환성을 먼저 확인합니다.

## 현재 Insights 연동

| 대상 | 상태 | 계약 |
| --- | --- | --- |
| Codex App | 내장 | 명시적 활성화 후 fail-open lifecycle checkpoint 5개 |
| Codex CLI | 내장 | desktop, local headless, remote headless 메타데이터 |
| HTTPS | 기본 전송 경로 | 운영자 HTTPS origin, 인증서 검증 및 리다이렉트 거부 |
| Tailscale/Tailnet | 선택형 전송 경로 | Tailnet IPv4 또는 `*.ts.net`; 해당 주소만 로컬 Tailnet 상태 확인 |
| GroundLine Insights API | 내장 | Rust/Axum enrollment, upload, report, 관리 API |
| ClickHouse | 필수 저장소 | API 소유 schema migration, idempotent ingest, 고정 report view |
| CLI JSON report | 내장 | 별도 admin-token 파일이 필요한 엄격한 7일·30일·90일 owner report, collector token은 거부 |
| Grafana | 기본 dashboard | provision된 ClickHouse datasource와 고정 dashboard query |
| Docker Compose | 공개 self-hosting preview | placeholder만 포함한 범용 topology, 인증 필수 Grafana, 비공개 secret 렌더링 |
| TrueNAS | 선택형 운영 overlay | 범용 배포 계약 위에서 owner가 실행하는 preflight/apply controller, private inventory는 미포함 |

[셀프호스팅](self-hosting.md)은 서버 배포·인증·실제 stack 검증을 다룹니다.
플러그인 설치가 Docker 서비스를 만들거나 maintainer 서버에 연결하지 않습니다.
운영 증거에는 선택한 image digest와 실제 호스트·저장소·대시보드·외부 접근
검증이 필요합니다.

## 런타임과 데이터 원본

Codex App/CLI의 신뢰한 hook과 읽기 전용 활동 데이터가 비공개 집계 outbox,
운영자 API, ClickHouse, Grafana/JSON 보고서로 이어집니다. Core·추론 프록시·
custom provider는 필요하지 않습니다. 훅이 없는 동안의 주기적 전송은 없으며,
컴퓨터가 깨어 있고 서버에 연결되어야 합니다.

원래 `CODEX_HOME`을 사용합니다. 다른 홈은 다른 원본이며 자동 이전하지 않습니다.
같은 홈의 App·CLI는 연결 프로필을 공유하지만 수집 상태와 동의는 분리됩니다.
훅은 원본을 전달하고 수동 작업은 명시적으로 선택합니다.

```console
GROUNDLINE_RUNTIME_FAMILY=codex_app GROUNDLINE_EXECUTION_MODE=desktop groundline-insights worker status
GROUNDLINE_RUNTIME_FAMILY=codex_cli GROUNDLINE_EXECUTION_MODE=local_headless groundline-insights worker status
```

`remote_headless`도 지원합니다. 알 수 없는 환경·origin 지정은 상태 쓰기 전에
거부합니다. 한 원본을 꺼도 다른 원본의 동의는 바뀌지 않습니다.
[네이티브 데이터 호환성](codex-compatibility.md)을 확인하세요. 프록시나 예전 앱을
제거할 때도 identity·동의·cursor·outbox를 초기화하지 않습니다.

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

설치·설정은 수집 동의가 아닙니다. [Insights setup](../installation.md#add-insights-in-the-same-flow)으로
명시적으로 활성화하세요. 플러그인 업데이트가 개인 서버를 업데이트하지 않습니다.
수집 구간·재시도·운영자 보고서는 [운영 안내](../insights-operations.md#collector-verification-and-retries)에 있습니다.

## 현재 지원하지 않는 연동

- Claude, Hermes, Antigravity 또는 범용 provider collector
- 범용 webhook, Slack, OpenTelemetry, Prometheus export
- PostgreSQL, SQLite, S3 또는 교체 가능한 저장 backend
- Grafana Cloud 계정 provisioning 또는 hosted GroundLine SaaS
- raw prompt, response, transcript, command, patch, path, 저장소, task, account 전송
