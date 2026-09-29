# GroundLine Core changes

## 2026.09.29-a (`2026.929.1`)

- 지원 범위를 macOS·Linux의 ARM64·x86-64 네 가지 대상으로 정리했습니다. Windows 설치기·전용 런타임·빌드·훅을 제거하며 과거 Windows 관측 데이터는 보존합니다.
- 릴리스 이름은 날짜와 당일 순번으로 표시합니다. CLI·태그·manifest·전송 값은 숫자 SemVer를 유지하며, 공용 변환에서 표시명을 만듭니다. [버전 규칙](https://github.com/jukqaz/groundline/blob/main/docs/versioning.md)

- 사용 관측 → 비공개 작업 결과 → 동일 작업군 비교로 Core 책임을 정리했습니다. 설치·이력 관측·`optimize-codex-workflow` 세 스킬을 제공하며 Codex가 실행을 소유합니다.
- 고정 가정 시뮬레이션, Chronicle fuse, batch, project-audit, 중복 integration 상태, 개인 스킬 registry, Astra preset과 별도 개인 실험 writer/evaluator를 제거했습니다. 기존 개인 상태는 `personal status/rollback`으로 보존·복구합니다.
- 기간 내 인덱스 표본과 전체 저장소 진단(`audit store`)을 분리했습니다. 선택 표본의 누락·중복은 계속 차단하고, 미확인 전체 분모는 null로 유지하며 미선택 작업에 일반화하지 않습니다.
- `audit review --input`은 저장 관측만 재사용하고 현재 추천을 다시 계산합니다. 과거 PASS 추천을 현재 결정으로 재사용하지 않습니다.
- delivery manifest와 routing evidence/proposal을 schema 2로 정리했습니다. 중복 관찰 JSON 대신 원래 근거를 연결하고, resource entry의 선택적 effective 모델·effort로 child 귀속과 미확인을 구분합니다. 기존 schema-1 receipt는 보존합니다.
- GPT-6 Astra·Sol·Luna의 현재 카탈로그와 직접 결과만 경험적 비교에 사용합니다. 품질·재작업·총자원·중앙값 보호 및 명시적 사용자 선택을 유지하며 14개 기능 사용률 판정을 제거했습니다.
- `efficiency delivery-summary`는 완료·실패·미확정·재작업과 모든 소유 자원을 함께 보여줍니다. 집계와 소스 테스트로 실사용 개선·native 활성화·토큰 절약을 단정하지 않습니다.
- literal Cargo 환경 접두사의 검증 명령을 인식하고, 인용 검색문·동적 실행을 성공 검사로 오인하지 않는 회귀를 보강했습니다.

BREAKING CHANGE: 폐기 명령·스킬·preset에는 별칭을 두지 않습니다. 새 routing evidence/proposal과 delivery manifest는 schema 2이며 이전 입력을 명시 거절합니다. 기존 개인 상태·native 기록·schema-1 delivery receipt는 삭제하지 않습니다. Insights는 호환 API를 먼저 배포한 뒤 수집기를 갱신해야 합니다.

Earlier release notes are maintained in the
[repository changelog](https://github.com/jukqaz/groundline/blob/main/CHANGELOG.md).
