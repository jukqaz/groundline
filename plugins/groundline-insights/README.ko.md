# GroundLine Insights

Insights는 Core와 독립적으로 설치하는 선택형 플러그인입니다. fail-open Codex
hook 4개가 네이티브 App/CLI 활동을 제한된 집계로 읽어 비공개 outbox에 저장하고,
운영자의 HTTPS API·ClickHouse·Grafana로 전송합니다. Tailnet 제한은 선택 사항이며,
Core·추론 프록시·모델 카탈로그·추론 인증 정보는 필요하지 않습니다.

스킬·daemon·scheduler를 설치하거나 global Codex 설정을 바꾸지 않습니다.
[보안](SECURITY.md)과 [계약](references/insights-contract.md)에 비공개 상태,
동의, 인증, 수집 한도와 원문 제외 범위를 정의합니다.

## 설치와 업그레이드

설치·연결·동의·검증을 함께 수행하려면 검토한 배포본의
`install.sh --profile insights` 또는 `both`를 사용합니다. 패키지만 설치하려면:

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline-insights@groundline --json
```

Core는 함께 설치되지 않으며 소스 태그에는 실행 파일이 없습니다. API 우선 검사,
커밋 고정과 복구는 [업그레이드 절차](references/native-upgrade.md)를 따릅니다.
갱신하려고 비활성 플러그인을 다시 `plugin add`하지 마세요.

패키지 설치 후 `groundline-insights setup`으로 남은 단계를 확인합니다. 명시적으로
선택한 연결에는 `--endpoint <URL> --enrollment-token-file <비공개 파일>
--enable --verify` 또는 `--input <비공개 프로필>`을 사용합니다. 일치하는 입력은
재사용하고 다른 연결은 identity·이력을 초기화하지 않고 거절합니다. 종료 코드 2는
추가 조치 또는 첫 활동 대기이며 setup은 hook trust를 부여하거나 전송 성공을 꾸며내지 않습니다.

변경된 hook hash는 Codex에서 다시 검토해야 합니다. GroundLine은 자신을 신뢰
처리하지 않으며 `plugin list`도 trust·실행 증거가 아닙니다. macOS/Linux의
ARM64·x86_64를 지원합니다. 실행 파일은 설치된 `bin/<target>/groundline-insights`에서
찾으며 shell `PATH` 등록은 보장하지 않습니다.

## Owner 설정

[owner-profile.example.json](references/owner-profile.example.json)을 플러그인·저장소
밖에 복사하고 소유자만 읽도록 제한한 뒤 endpoint와 의도적으로 짧은 토큰을
교체합니다. `worker configure`는 schema-7 입력을 받아 비밀을 제거한 프로필과
enrollment credential을 `~/.codex/groundline/insights`에 분리 저장합니다.
비밀값은 출력하거나 플러그인에 복사하지 않습니다.

```console
groundline-insights worker configure --input /owner-private/owner-profile.json
groundline-insights worker enable
groundline-insights worker run-once
groundline-insights worker status
```

설치만으로 수집되지 않습니다. `worker enable`이 운영자 서비스 업로드의 명시적
동의 경계입니다. 최초 enrollment에는 서버 연결과 운영자가 발급한 credential이
필요하며 이후에는 collector별 토큰을 사용합니다. 동의서가 없으면 enable이 만들고
미동의 pending event를 격리합니다. 유효한 동의서는 재활성화해도 유지합니다.
지원하지 않는 상태는 변환·삭제하지 않고 거절합니다. 수집을 중지하고 상태·outbox를
보존한 뒤 명시적인 승인을 받아 새로 설정해야 합니다.

최초 수집은 최근 7일이며 이후 커서를 보존합니다. 불완전한 구간은 고정하고
읽기 3회 실패 후에는 운영자의 재시도가 필요합니다. 상태 해석, 원본 발견과
누락 구간 복구는 [문제 해결](references/operations-troubleshooting.md)을 따릅니다.

fleet 보고서는 별도 관리 작업입니다. collector 토큰으로 조회할 수 없으며,
admin 토큰만 든 비공개 파일을 명시해야 합니다.

```console
groundline-insights insights fetch-report \
  --admin-token-file /owner-private/admin-report-token \
  --days 7 --json
```

이 파일을 collector 전용 호스트에 복사하거나 Git·로그에 남기지 마세요.

## 운영 증거

패키지 무결성, 활성 hook 4개, lifecycle 실행, 업로드 승인, ClickHouse 반영,
Grafana frame, 이미지 게시, 배포와 stable 승격은 각각 검증합니다. 관측하지 못한
단계는 `UNVERIFIED`로 남깁니다. 운영 endpoint·credential·dataset 경로·receipt는
공개 Git과 CI 밖에 둡니다.

저장·전송 한도, 보존, 중복 제거와 7일·30일·90일 보고서는
[계약](references/insights-contract.md)을 따릅니다. Docker Compose는 공개 셀프호스팅
preview이며 TrueNAS는 선택형 overlay입니다. 운영에는 fresh-host, immutable image,
외부 TLS 증거가 필요합니다. [셀프호스팅](https://github.com/jukqaz/groundline/blob/main/docs/ko/self-hosting.md)과
[연동 프로필](https://github.com/jukqaz/groundline/blob/main/docs/ko/integrations.md)을 참고하세요.

License: MIT.
