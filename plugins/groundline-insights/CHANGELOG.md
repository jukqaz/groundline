# GroundLine Insights changes

## 2026.09.29-a (`2026.929.1`)

- 지원 범위를 macOS·Linux의 ARM64·x86-64 네 가지 대상으로 정리했습니다. Windows 설치기·전용 런타임·빌드·훅을 제거하며 과거 Windows 관측 데이터는 보존합니다.
- 릴리스 이름은 날짜와 당일 순번으로 표시합니다. CLI·태그·manifest·전송 값은 숫자 SemVer를 유지하며, 공용 변환에서 표시명을 만듭니다. [버전 규칙](https://github.com/jukqaz/groundline/blob/main/docs/versioning.md)

- GPT-6 Astra·Sol·Luna의 집계 라벨을 분리하고 ingest contract revision 8을 사용합니다. 업그레이드 전에 owner API가 새 계약을 광고하는지 확인해야 합니다.
- `worker check-server`는 기존 연결의 호환성을 읽기 전용으로 검사합니다. 설치기는 marketplace 갱신 전에 이를 실행하며, 실패 시 수집기·설정·동의 상태를 유지합니다.
- API의 기존 신뢰 판정 정의를 원본 보존·재검증·중단 후 재개 가능한 명시적 마이그레이션으로 이행합니다. 알 수 없는 정의와 SQL식 변조는 거절합니다.
- 공유 계약과 SQL 분석 bucket 제한을 통일해 유효한 81·89·90개 조합을 보존합니다.
- Codex 기본 프롬프트 길이 제한과 독립적인 Core/Insights 설치 계약을 유지합니다. 수집 동의·목적·기존 이벤트 기록을 자동으로 재설정하지 않습니다.

Earlier release notes are maintained in the
[repository changelog](https://github.com/jukqaz/groundline/blob/main/CHANGELOG.md).
