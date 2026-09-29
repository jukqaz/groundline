# Codex 호환성

GroundLine은 Codex의 모델·권한·컨텍스트·플러그인 기능을 사용하며 macOS·Linux의
ARM64·x86-64를 지원합니다. 과거 Windows 관측값은 보존하지만 Windows 실행
환경은 지원하지 않습니다. 모델 추천은 현재 카탈로그를 확인하며 전역 모델이나
추론 강도를 고정하지 않습니다. 공통 모델 계열 분류에서 알 수 없는 이름은
`other`가 되고, 모델별 문맥 횟수는 토큰이나 비용을 뜻하지 않습니다.

## 실행 환경 확인

App 번들 CLI와 PATH CLI의 경로·버전·native plugin 상태를 각각 확인합니다.
명령은 [실행 환경 확인](../codex-compatibility.md#check-the-runtime-you-use)에 있습니다.
버전 출력만으로 플러그인 활성화나 실제 작업 성공을 판단하지 않습니다.
[정확 커밋 설치](../installation.md#update-an-existing-installation)로 갱신하며,
App Refresh만으로 다음 릴리스로 넘어가지는 않습니다.

## 데이터와 증거 경계

가장 높은 번호의 `state_<n>.sqlite`를 읽기 전용으로 열고 필수 열을 검증합니다.
일반·Zstandard JSONL을 같은 경로로 읽으며 최신 turn이 완료된 작업을 주간
표본으로 선택합니다. 재개된 작업은 활동 집계에 포함합니다. 서로 다른 작업의
도구·모델 기록을 섞지 않고 같은 컨텍스트 압축은 한 번만 셉니다.

알려진 다른 제품의 origin은 활동을 읽기 전에 제외하고 알 수 없는 origin은
불완전으로 남깁니다. 공유 이력은 명시된 소유 경계 뒤의 suffix만 집계하고
response ID로 사용량을 중복 제거합니다. 부모의 누적 사용량을 더하지 않습니다.
읽지 않은 prefix는 `PARTIAL`, 소유 경계가 없는 복사본은 제외 상태를 유지합니다.
독립 작업에서도 native·UI 누적 토큰은 기준점이 다를 수 있어 합산하지 않습니다.
구간 내 감소·누락된 기준점·확인되지 않은 응답은 측정값을 만들어 채우지 않습니다.

읽기·메모리·파서 예산을 제한하고 실패를 결과에 남깁니다. 심볼릭 링크 DB·세션
루트는 거부합니다. 안전한 원본이 없으면 `native_activity_unavailable`,
`ready_to_collect: false`로 표시하지만 기존 outbox를 삭제하거나 전송을 막지
않습니다. 읽기 성공도 전체 모집단의 완전한 관측을 보장하지 않습니다.
수치 제한과 usage 우선순위는 [영문 계약](../codex-compatibility.md#known-evidence-limits)에 모았습니다.

새 수집 계약은 **API를 먼저 올리고 collector를 업데이트**합니다. 캐시된 token이
있어도 서버 capability를 확인하며 미호환이면 `api_upgrade_required`로 중단하고
outbox를 보존합니다. 최초 7일과 이후 cursor·고정된 실패 구간·자동 3회 제한은
[수집 확인과 재시도](../insights-operations.md#collector-verification-and-retries)를 따릅니다.
합성 테스트, 설치 파일 검사, 실제 Codex 작업, 새 서버 수신·대시보드 반영은
각각 별도 검증이며 기존 동의·대상 서버·활성화 상태를 바꾸지 않습니다.
