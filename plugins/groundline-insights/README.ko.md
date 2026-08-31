# GroundLine Insights

GroundLine Insights는 GroundLine Core와 같은 공개 모노레포에서 배포되는 선택형
self-hosted 데이터 플러그인입니다. Core와 독립적으로 설치할 수 있고 Core를
필수 의존성으로 요구하지 않습니다. 네트워크 기능은 Insights에만 있습니다.

- fail-open Codex lifecycle hook 4개
- owner-private identity, consent, checkpoint, credential, outbox
- Tailnet 전용 수집과 owner report
- Rust/Axum API, ClickHouse schema, Grafana dashboard, 범용 배포 도구

GroundLine skill을 중복 설치하거나 global Codex 설정을 바꾸지 않으며 daemon,
scheduler, model router를 만들지 않습니다.

## 설치와 업그레이드

모노레포를 한 번 등록하고 Insights를 설치합니다. 이 명령은 Core를 자동으로
설치하지 않습니다. Core skill과 로컬 감사도 필요할 때만
`groundline@groundline`을 별도로 설치합니다.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline-insights@groundline --json
codex plugin marketplace upgrade groundline --json
```

Core만, Insights만, 둘 다 설치하는 선택 기준은
[연동과 설치 프로필](../../docs/ko/integrations.md)에 정리되어 있습니다.

binary 이름은 macOS/Linux에서 `groundline-insights`, Windows에서
`groundline-insights.exe`입니다. 세 운영체제의 ARM64·x86-64를 지원합니다.
실행 파일은 설치된 plugin의 `bin/<target>`에서 찾습니다. Codex plugin 설치가
사용자 shell의 `PATH` 등록까지 보장하는 것은 아닙니다.

## Owner 설정

설치만으로 수집이 활성화되지 않습니다. `worker configure`는 Tailnet endpoint와
owner-issued `enrollment_token`이 들어간 schema-7 입력을 받아, secret이 제거된
profile과 credential을 `~/.codex/groundline/insights` 아래의 서로 다른 비공개
파일로 저장합니다. secret은 출력하거나 plugin에 복사하지 않습니다.

첫 enrollment에는 Tailnet 연결과 credential이 모두 필요하며, 이후 collector별
token을 사용합니다. 설정 입력은 운영 비밀이므로 Git에 commit하면 안 됩니다.
`references/owner-profile.example.json`은 전체 필드를 제공하지만 `REPLACE_ME`를
의도적으로 짧게 두었으므로 그대로는 활성화되지 않습니다. endpoint와 token을
owner-private 값으로 바꾼 복사본만 `worker configure --input`에 전달합니다.
수집 wire contract는 raw prompt, response, transcript, command, patch, path,
hostname, 저장소명, task/rollout/account/IP 식별자를 거부합니다.

`worker status`는 `collection_state`, `ready_to_collect`, 제한된
`blocking_reason_codes`로 의도적인 비활성, 설정 누락/오류, Tailnet 미확인/끊김,
첫 수집 대기, 7일 이상 수집 정체, 시계 오차, 정상 수집 상태를 구분합니다.
`tailnet_connected: null`은 연결 끊김이 아니라 현재 실행 경계에서 확인하지
못했다는 뜻입니다.

지원 collector runtime은 Codex App과 Codex CLI입니다. worker endpoint는 Tailnet
IPv4 또는 `*.ts.net`만 허용하며, 각 운영자가 자신의 비공개 서비스와 credential을
제공합니다. 공식 서비스 경로는 Rust/Axum API, ClickHouse 저장소, 엄격한
7일·30일·90일 CLI JSON report, provision된 Grafana dashboard입니다. Docker
Compose는 공개 self-hosting preview이고 TrueNAS는 선택형 운영 overlay일 뿐
plugin 필수 조건이 아닙니다. 운영 배포에는 fresh-host, immutable image, 외부 TLS
증거가 추가로 필요합니다.

ClickHouse schema migration의 단일 소유자는 Insights API입니다. 수집 테이블은
`ReplacingMergeTree`를 사용하고 report와 Grafana는 `FINAL`이 적용된
`basic_active` view를 읽어 논리 중복을 제거합니다. API 재전송은 idempotent하게
처리하며, 물리 중복 row가 생기면 storage report에 품질 신호로 드러냅니다.

marketplace refresh, package checksum, hook 4개, lifecycle dispatch, accepted
upload, ClickHouse 반영, Grafana frame, image 게시, 배포, stable 승격은 각각
분리해서 검증합니다. 운영 endpoint·credential·dataset path·receipt는 public
Git과 public CI 밖에 둡니다.

서버 checkout, private Compose render, 실제 ClickHouse·Grafana 검증 순서는
[셀프호스팅 가이드](../../docs/ko/self-hosting.md)를 참조하세요.

License: MIT.
