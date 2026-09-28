# GroundLine Core changes

## 2026.09.29-a (`2026.929.1`)

- 릴리스 이름은 날짜와 당일 순번으로 표시합니다. CLI·태그·manifest·전송 값은 숫자 SemVer를 유지하며, 공용 변환에서 표시명을 만듭니다. [버전 규칙](../../docs/versioning.md)

- 사용 관측 → 비공개 작업 결과 → 동일 작업군 비교로 Core 책임을 정리했습니다. 설치·이력 관측·`optimize-codex-workflow` 세 스킬을 제공하며 Codex가 실행을 소유합니다.
- 고정 가정 시뮬레이션, Chronicle fuse, batch, project-audit, 중복 integration 상태, 개인 스킬 registry, Astra preset과 별도 개인 실험 writer/evaluator를 제거했습니다. 기존 개인 상태는 `personal status/rollback`으로 보존·복구합니다.
- 기간 내 인덱스 표본과 전체 저장소 진단(`audit store`)을 분리했습니다. 선택 표본의 누락·중복은 계속 차단하고, 미확인 전체 분모는 null로 유지하며 미선택 작업에 일반화하지 않습니다.
- `audit review --input`은 저장 관측만 재사용하고 현재 추천을 다시 계산합니다. 과거 PASS 추천을 현재 결정으로 재사용하지 않습니다.
- delivery manifest와 routing evidence/proposal을 schema 2로 정리했습니다. 중복 관찰 JSON 대신 원래 근거를 연결하고, resource entry의 선택적 effective 모델·effort로 child 귀속과 미확인을 구분합니다. 기존 schema-1 receipt는 보존합니다.
- GPT-6 Astra·Sol·Luna의 현재 카탈로그와 직접 결과만 경험적 비교에 사용합니다. 품질·재작업·총자원·중앙값 보호 및 명시적 사용자 선택을 유지하며 14개 기능 사용률 판정을 제거했습니다.
- `efficiency delivery-summary`는 완료·실패·미확정·재작업과 모든 소유 자원을 함께 보여줍니다. 집계와 소스 테스트로 실사용 개선·native 활성화·토큰 절약을 단정하지 않습니다.
- literal Cargo 환경 접두사의 검증 명령을 인식하고, 인용 검색문·동적 실행을 성공 검사로 오인하지 않는 회귀를 보강했습니다.

BREAKING CHANGE: 폐기 명령·스킬·preset에는 별칭을 두지 않습니다. 새 routing evidence/proposal과 delivery manifest는 schema 2이며 이전 입력을 명시 거절합니다. 기존 개인 상태·native 기록·schema-1 delivery receipt는 삭제하지 않습니다. Insights는 호환 API를 먼저 배포한 뒤 수집기를 갱신해야 합니다.

## 0.25.5

- Preserve explicit missing-usage quality reasons while reviewing valid lifecycle data.
- Align packaging with the Insights lifecycle-retention correction.

## 0.25.4

- Preserve independent tool-output signal counts across collection windows.
- Keep report, usage, and privacy validation aligned with Insights.

## 0.25.3

- Use the shared canonical cache-ratio calculation and strict event validator.
- Align packaging with Insights storage integrity and bounded quarantine retention.

## 0.25.2

- Preserve quarantined-event counts and quality reasons when reviewing Insights reports.
- Require coherent usage evidence without inventing missing native token splits.

## 0.25.1

- Prioritize GPT-6/Astra guidance while supporting GPT-5.6 and preserving selected settings.
- Reconcile weekly advice within the current authorized task without extra approval or recurring-review requirements.
- Propose diagnosis from aggregate failure signals, retaining quality/cohort context and strict personal-trial gates.

## 0.25.0

- Separate model-guidance review from Insights-based personal trials; preserve
  selected settings unless configuration setup is explicitly requested.
- Load the detailed personal trial workflow only when that route is requested.
- Scope reconciliation and live verification to the current outcome, reuse
  evidence, and retain the task through steering. Batch advice recommends new
  tasks or forks only for explicit requests and cannot complete a changed scope
  using the previous verification.

## 0.22.2

- Wait for authenticated API and Grafana readiness before the release stack
  checks anonymous dashboard access. Preserve the redirect and semantic assertions.

## 0.22.1

- Recover interrupted personal guidance restores and keep trial history,
  temporary files, candidate selection, and PR regression checks bounded.
- Audit larger native records without expanding unused history bodies, and
  recognize a directly proven fresh native usage baseline.

## 0.22.0

- Add `personal review|evaluate|rollback` and an explicit seventh skill for
  current-model workflow improvement. Keep private trials outside Git and
  require directly evidenced, disjoint, comparable outcomes before retention.
- Preserve native settings and user edits; do not apply changes from incomplete
  reports, short prompts, high effort, or aggregate repetition alone.

## 0.21.4

- Verify record timestamps when selecting native audit activity; metadata-only
  thread updates no longer reintroduce inactive inherited history.

## 0.21.3

- Publish alongside the API image launch-permission correction.

## 0.21.2

- Stream native audit inputs with separate I/O and retained-record budgets,
  preserving compaction and tool-result metrics while dropping unused bodies.
- Keep native and UI usage baselines independent and include active turns with
  stale sidebar recency. Keep unknown history ownership incomplete.
- Align installation, verification, and Korean guidance with the current native
  commands and consolidate obsolete documentation.

## 0.21.1

- Publish alongside the Insights correction for existing collection generations.

## 0.21.0

- Add offline `config-audit` against a supplied native model catalog.
- Add strict personal-skill audit/snapshot contracts and share skill metadata
  validation with packaging, preserving existing local changes and settings.
- Support current native compressed and shared-history inputs, bounded parser
  diagnostics, event-time windows, and explicit incomplete coverage.
- Keep scoped approvals, optional native Goals, user-selected models, and
  explicit delegation boundaries in the six packaged skills.

## 0.20.x

- Establish one canonical Core package, independent from optional Insights.
- Harden read-only state/rollout access and qualify six native targets.
- Resolve native binaries from the installed package without assuming `PATH`.

The [historical package changelog](https://github.com/jukqaz/groundline/blob/v0.21.1/plugins/groundline/CHANGELOG.md)
preserves earlier details. Core remains offline and installs no hooks.
