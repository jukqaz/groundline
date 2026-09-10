# GroundLine Insights

GroundLine Insights는 GroundLine Core와 같은 공개 모노레포에서 배포되는 선택형
self-hosted 데이터 플러그인입니다. Core와 독립적으로 설치할 수 있고 Core를
필수 의존성으로 요구하지 않습니다. 네트워크 기능은 Insights에만 있습니다.

- fail-open Codex lifecycle hook 4개
- owner-private identity, consent, checkpoint, credential, outbox
- 일반 HTTPS 수집과 owner report, 선택형 Tailscale 제한
- Rust/Axum API, ClickHouse schema, Grafana dashboard, 범용 배포 도구

GroundLine skill을 중복 설치하거나 global Codex 설정을 바꾸지 않으며 daemon,
scheduler, model router를 만들지 않습니다.

수집 원본은 네이티브 Codex App/CLI입니다. 추론 프록시, 생성된 모델 카탈로그,
custom provider 설정, Core 설치가 필요하지 않습니다. 네이티브 활동을 비공개
집계 outbox에 저장하고 HTTPS 또는 선택형 Tailnet Insights API로 직접 전송하며, ClickHouse와
Grafana는 이 API에 연결됩니다. 모델 설정이나 추론 인증 정보는 읽지 않습니다.

## 설치와 업그레이드

모노레포를 한 번 등록하고 Insights를 설치합니다. 이 명령은 Core를 자동으로
설치하지 않습니다. Core skill과 로컬 감사도 필요할 때만
`groundline@groundline`을 별도로 설치합니다.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline-insights@groundline --json
```

갱신은 `codex plugin marketplace upgrade groundline --json`으로 실행한 뒤
`codex plugin list --json`으로 설치 버전을 확인합니다. 필요하면 같은 Insights ID를
다시 설치합니다. 소스 태그에는 실행 파일이 없으므로
[네이티브 업그레이드](references/native-upgrade.md)의 배포본·API 우선 절차를 따릅니다.

업그레이드로 `hooks/hooks.json` hash가 바뀌면 Codex에서 새 hook을 다시 검토하고
신뢰해야 합니다. GroundLine은 자신의 trust를 승인하지 않습니다.
`codex plugin list`는 설치·활성화 증거일 뿐 새 hook의 검토·실행 증거가 아니므로,
새 task의 lifecycle receipt를 별도로 확인합니다.

Core만, Insights만, 둘 다 설치하는 선택 기준은
[연동과 설치 프로필](https://github.com/jukqaz/groundline/blob/main/docs/ko/integrations.md)에 정리되어 있습니다.

binary 이름은 macOS/Linux에서 `groundline-insights`, Windows에서
`groundline-insights.exe`입니다. 세 운영체제의 ARM64·x86-64를 지원합니다.
실행 파일은 설치된 plugin의 `bin/<target>`에서 찾습니다. Codex plugin 설치가
사용자 shell의 `PATH` 등록까지 보장하는 것은 아닙니다.

## Owner 설정

설치만으로 수집이 활성화되지 않습니다. `worker configure`는 HTTPS endpoint 또는 선택형 Tailnet endpoint와
owner-issued `enrollment_token`이 들어간 schema-7 입력을 받아, secret이 제거된
profile과 credential을 `~/.codex/groundline/insights` 아래의 서로 다른 비공개
파일로 저장합니다. secret은 출력하거나 plugin에 복사하지 않습니다.

첫 enrollment에는 서버 연결과 credential이 모두 필요하며, 이후 collector별
token을 사용합니다. 설정 입력은 운영 비밀이므로 Git에 commit하면 안 됩니다.
`references/owner-profile.example.json`은 전체 필드를 제공하지만 `REPLACE_ME`를
의도적으로 짧게 두었으므로 그대로는 활성화되지 않습니다. endpoint와 token을
owner-private 값으로 바꾼 복사본만 `worker configure --input`에 전달합니다.
복사본은 플러그인·저장소 밖에 두고 소유자만 읽을 수 있게 제한합니다.

```console
groundline-insights worker configure --input /owner-private/owner-profile.json
groundline-insights worker enable
groundline-insights worker run-once
groundline-insights worker status
```

수집 wire contract는 raw prompt, response, transcript, command, patch, path,
hostname, 저장소명, task/rollout/account/IP 식별자를 거부합니다.

fleet 전체 CLI report는 관리 작업입니다. collector token은 사용할 수 없고,
별도의 admin token만 들어 있는 owner-private 파일을 명시해야 합니다.

```console
groundline-insights insights fetch-report \
  --admin-token-file /owner-private/admin-report-token \
  --days 7 --json
```

이 파일은 collector 전용 호스트에 복사하거나 Git에 commit·출력하면 안 됩니다.

`worker enable`은 owner-service upload에 대한 명시적 동의 경계입니다. 동의서,
정책, 상태 파일은 현재 형식만 지원합니다. 구형·알 수 없는 형식은 변환하거나
삭제하지 않고 `unsupported_local_state`로 거부하며, `worker enable`도 이를
마이그레이션하지 않습니다. 수집을 중지하고 원본 상태와 pending event를 보존한 뒤,
명시적으로 승인받아 새로 설정해야 합니다. 동의서가 없는 경우에만 enable이 새
receipt를 만들고 미동의 pending event를 quarantine으로 격리합니다. 유효한
동의서는 다시 활성화해도 유지합니다.

최초 수집은 최근 7일입니다. 이후에는 커서를 보존하며, 불완전한 구간을 고정한 채
자동 읽기를 최대 3회 시도합니다. 과거 누락 구간의 시작 경계를 바꾸려면 원본과
미처리 기록을 보존하고 운영자가 명시적으로 결정해야 합니다.
[문제 해결](references/operations-troubleshooting.md)을 참고하세요.

`worker status`는 `collection_state`, `ready_to_collect`, 제한된
`blocking_reason_codes`로 의도적인 비활성, 설정 누락/오류, Tailnet 미확인/끊김,
첫 수집 대기, 7일 이상 수집 정체, 시계 오차, 정상 수집 상태를 구분합니다.
`tailnet_connected: null`은 연결 끊김이 아니라 현재 실행 경계에서 확인하지
못했다는 뜻입니다.

`doctor`와 수집기는 가장 높은 번호의 `state_<n>.sqlite`를 같은 규칙으로 찾습니다.
`codex_state_store_present`는 파일 존재 검사이며 스키마·전송 성공 증거가 아닙니다.
원본이 없거나 안전하게 열리지 않으면 `native_activity_unavailable`과 준비 미완료를
표시합니다. 기존 전송 대기·운영 조치 사유는 우선 표시하며 pending event는 보존합니다.
추론 wrapper를 제거해도 기존 Codex 홈, identity, 동의와 outbox를 유지해야 합니다.

지원 collector runtime은 Codex App과 Codex CLI입니다. worker endpoint는 일반 HTTPS
origin 또는 선택형 Tailnet 주소를 허용하며, 각 운영자가 자신의 비공개 서비스와 credential을
제공합니다. 공식 서비스 경로는 Rust/Axum API, ClickHouse 저장소, 엄격한
7일·30일·90일 CLI JSON report, provision된 Grafana dashboard입니다. Docker
Compose는 공개 self-hosting preview이고 TrueNAS는 선택형 운영 overlay일 뿐
plugin 필수 조건이 아닙니다. 운영 배포에는 fresh-host, immutable image, 외부 TLS
증거가 추가로 필요합니다.

ClickHouse schema migration의 단일 소유자는 Insights API입니다. 수집 테이블은
`ReplacingMergeTree`를 사용하고 report와 Grafana는 `FINAL`이 적용된
`basic_active` view를 읽어 논리 중복을 제거합니다. API 재전송은 idempotent하게
처리하며, 물리 중복 row가 생기면 storage report에 품질 신호로 드러냅니다.
기본 서비스 계약은 365일을 보존하고 collector별 retained event 4,096개와 논리
payload 256 MiB를 제한합니다. dataset row·byte ceiling의 90%에 도달하면 관리
작업용 여유 공간을 남기기 위해 추가 ingest를 중단합니다.
로컬 outbox도 event 256개·16 MiB·전송 batch 16개로 제한합니다. 재시도 가능한
장애는 영속적인 capped backoff를 사용하고 영구 remote 거절은 operator 조치를
요구합니다. Grafana TTL panel은 보존 기한이 지났지만 ClickHouse background
merge를 기다리는 row를 표시하며 `OPTIMIZE`나 삭제를 자동 실행하지 않습니다.

marketplace refresh, package checksum, hook 4개, lifecycle dispatch, accepted
upload, ClickHouse 반영, Grafana frame, image 게시, 배포, stable 승격은 각각
분리해서 검증합니다. 운영 endpoint·credential·dataset path·receipt는 public
Git과 public CI 밖에 둡니다.

서버 checkout, private Compose render, 실제 ClickHouse·Grafana 검증 순서는
[셀프호스팅 가이드](https://github.com/jukqaz/groundline/blob/main/docs/ko/self-hosting.md)를 참조하세요.

License: MIT.
