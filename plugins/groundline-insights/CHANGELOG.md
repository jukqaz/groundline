# GroundLine Insights changes

## 2026.10.07-c (`2026.1007.3`)

- 5개 fail-open native 훅에서 원문 없이 작은 비공개 작업 경계를 보존합니다.
- 학습 opt-in과 수집 동의를 분리하고 worker에서 제한된 오프라인 소비를 실행합니다.
- 중앙 스키마·수집기 identity·대기 전송·기존 데이터를 유지합니다.

## 2026.10.07-b (`2026.1007.2`)

- Core와 후보 버전을 맞춥니다. Insights의 hook·수집·인증·저장소·ingest revision은 변경하지 않습니다.

## 2026.10.07-a (`2026.1007.1`)

- 외부에서 revoked로 표시된 수집기는 owner 등록 키로도 다시 활성화하거나 토큰·generation을 변경할 수 없도록 등록 경계를 보완합니다. 정상 등록·재등록과 삭제된 ID의 거부는 유지합니다.
- 설치 문서의 현재 최소 ingest contract revision을 10으로 정정합니다. 저장소 계약·기존 수집 데이터와 동의는 유지합니다.

## 2026.10.06-b (`2026.1006.2`)

- root/child의 정확한 모델·effort·coverage 및 기간별 모델 패턴 조회를 추가했습니다. 과거 모델 계열과 미관측 ID를 보존합니다.
- ClickHouse·Grafana 모델 패턴 질의와 overflow·coverage 검증을 강화하며 기존 수집·전송 계약을 유지합니다.

## 2026.10.06-a (`2026.1006.1`)

- Core와 릴리스 버전을 맞추고 ingest revision 9를 유지합니다. API 호환성을 먼저 확인하며 기존 동의·연결·수집 상태를 보존합니다. 설치 무결성과 실제 훅·worker·서버 수신의 증거는 별도로 확인합니다.

## 2026.10.03-a (`2026.1003.1`)

- schema 5를 유지하며 6.1 Sol 집계와 ingest revision 9를 지원합니다. 실제 revision 8 fingerprint에서 원본·기존 상태를 보존하는 저장소 전환을 검증합니다. API를 먼저 갱신해야 하며 revision 9 데이터 수용 뒤에는 forward repair가 기본입니다.
- worker 상태에 로컬 수집 범위, capture·worker 처리·수집·전송의 독립 근거를 표시합니다. 과거 ACK나 플러그인 활성만으로 현재 실행·cloud·계정 전체 수집을 주장하지 않습니다.
- 활성 정책의 반복 enable은 activation 시각을 보존하며, 실제 재활성화 전 성공 기록을 새 수집 성공으로 표시하지 않습니다. 네 fail-open hook과 기존 capture·ACK 전달 방식은 유지합니다.

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
