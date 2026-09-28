# GroundLine Core changes

## 0.29.0

- GPT-6 Astra·Sol·Luna의 모델·effort를 작업 단계와 현재 카탈로그로 선택합니다. max와 유용한 독립 위임을 지원하며 명시적 사용자 선택을 보존합니다.
- 고정 작업 유형별 추천을 제거하고 직접 완료 근거가 있는 경우만 경험적 대체 추천을 허용합니다. 일반 작업 판단은 네이티브 Codex가 계속 수행합니다.
- `efficiency record-delivery`와 `route --deliveries`에서 산출물·관측 선택·완료·재작업·전체 자원 근거를 검증합니다. 중복, 미확인 실행과 자원 부족을 추천 근거로 사용하지 않습니다.
- 직접 결과와 선택적 감사/ClickHouse 집계를 분리하고, 감사의 `coverage.rollout_count`를 분모 검증에 사용합니다.
- 지침을 간결한 작업 흐름과 점진적으로 읽는 참조로 정리했습니다. 실전 개선이나 모델 우위를 합성 검사와 혼동하지 않습니다.
- Cargo 환경 접두사 검증 명령 인식, native App CLI 경로 및 Codex 기본 프롬프트 호환성을 개선했습니다.
- 시뮬레이션 schema 2는 낮은·중간·높은 감소 가정을 표시하며 실측 절감으로 표현하지 않습니다.

This package summary covers the current release line. See the repository
[changelog](https://github.com/jukqaz/groundline/blob/main/CHANGELOG.md) for shared release and packaging changes.

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
