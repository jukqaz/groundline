# Changelog

## 0.25.3

- 캐시 비율을 토큰 수에서 같은 계산식으로 만들고 검증합니다. 입력 토큰이 없는 `0/0`은 `null`로 표시하며 0으로 저장하지 않습니다.
- 원본 JSON과 88개 집계 열의 매핑을 통합하고 ClickHouse 제약으로 불일치를 차단합니다. 같은 수집기·세대에서 겹치는 기간과 근거 없는 기간은 API에서 거절하며 정상 재전송은 허용합니다.
- 분석에서 제외한 수신 기록은 7일 보관 후 TTL로 정리합니다. 정상 기록은 운영자의 보관 기간을 유지하고 리포트와 Grafana가 동일한 만료 기준을 사용합니다.
- 원본 이벤트의 구조·해시·ID를 오프라인으로 검증하는 `insights validate-event`를 추가했습니다. 기존 데이터는 운영자가 백업과 검증을 거쳐 정리해야 하며 자동으로 재작성하지 않습니다.
- ingest contract revision 5를 요구합니다. 기존 데이터의 품질 검토와 보정을 마친 뒤 API를 먼저 배포하고 플러그인을 갱신해야 합니다.

## 0.25.2

- 사용량의 합계·캐시·추론 토큰과 출처 카운터가 모순되면 수집 및 API 검증에서 거절합니다. ClickHouse에도 직접 쓰기 방어 조건을 추가했습니다.
- 분석용 데이터와 격리된 수신 기록을 분리했습니다. 구형 계약, 사용량 미관측, 불완전한 출처·읽기 결과는 통계에서 제외하고 리포트와 Grafana에 격리 건수를 표시합니다.
- 작업 시작 직후 사용량이 아직 없는 정상 구간은 수신 기록을 보존합니다. 수집을 중단하거나 사용량을 만들어 넣지 않으며, 중복 전송 확인과 기존 보관 정책을 유지합니다.
- ingest contract revision 4를 사용합니다. API를 먼저 갱신한 뒤 Core·Insights를 함께 업데이트해야 합니다. Codex 로컬 작업 DB를 변경하거나 초기화하지 않습니다.

## 0.25.1

- GPT-6/Astra를 우선으로 GPT-5.6에도 적용되는 지침을 정리했습니다. 사용자가 선택한 모델·추론 강도·서비스 등급을 유지하고, 기존 작업에서 승인된 범위를 이어갑니다.
- 주간 추천의 불필요한 새 작업·추가 결정·정기 재검토 요구를 제거했습니다. 긴 작업과 높은 추론 강도만으로 실패나 개선 효과를 단정하지 않습니다.
- 반복 호출 비율이 낮아도 실패 신호가 있으면 개인 리뷰가 원인 진단 후보를 제시합니다. 정상 검색 결과·대기 확인·환경 실패를 먼저 구분하며 자동 실험 적용 조건은 유지합니다.
- 개인 리뷰 출력에 데이터 품질 사유, 사용량 누락·대체 집계와 분모, 버전·모델 구성을 보존합니다. 과거 데이터를 삭제하거나 모델별 성과로 오인하지 않습니다.

## 0.25.0

- 별도 GroundLine Desktop 앱과 GUI 빌드·릴리스 경로를 제거했습니다. Core와 Insights는 Codex 플러그인·CLI로 유지하며, 기존 Insights 설정과 수집 상태는 초기화하지 않습니다.
- Astra 공식 가이드에 맞춰 지침 조사와 사용량 기반 실험을 분리하고, 상태 확인·실행 검증에서 불필요한 선행 절차를 줄였습니다. 일반 지침 정비는 모델 설정 프리셋을 적용하지 않습니다.
- 작업 범위·저장소·권한 변경은 현재 작업에서 재검토하도록 수정했습니다. 새 작업과 분기는 명시 요청이 있을 때만 권고하며, 범위 변경 후 이전 검증으로 완료 판정하지 않습니다.
- App·CLI 환경 판정을 공통화하고 Codex 외 기록과 잘못된 환경 메타데이터가 새 집계·등록에 섞이지 않도록 했습니다. 지원하지 않는 저장 상태는 보존한 채 거절합니다.
- 과거 스키마가 섞인 리포트의 완료 작업 커버리지 누락 사유를 보완해 정상 집계가 계약 오류로 거절되는 문제를 수정했습니다.
- ClickHouse 내부 진단 로그에 7일·30일 보관 기간을 지정했습니다. 업무 이벤트 보관 정책은 유지합니다.
- TrueNAS 배포 검증에서 기존 Grafana JWT 인증을 지원합니다. 인증 방식과 헤더를 현재 설정과 대조하고, 자격증명 없이 확인하는 공개 접근 차단 검사와 분리합니다.
- 일반 모델 지침 검토가 개인 실험 절차까지 불러오지 않도록 상세 절차를 참조 문서로 옮겼습니다.

BREAKING CHANGE: 별도 Desktop 배포 파일은 제공하지 않습니다. Core·Insights 플러그인과 CLI로 연결·수집을 관리합니다.

## 0.24.6

- Desktop 설정에서 중복 Core 패키지 진단과 관련 명령·권한·직접 의존성을 제거했습니다. Core CLI 진단은 그대로 사용할 수 있습니다.
- 설정은 창 닫기와 테마를 중심으로 정리하고, 문제 알림을 트레이 사용 시 표시하는 작은 선택란으로 합쳤습니다. 저장된 알림 선호와 수집 동의는 보존합니다.
- 진단 파일 내보내기를 서버 연결 상세에 통합하고, 새 서버 구성은 보조 도구로 배치했습니다. 연결 화면의 중복 탐색과 설명을 줄였습니다.

## 0.24.5

- Codex 설정 알림의 공유 순번과 큰 도구 출력 때문에 정상 활동 집계가 중단되는 문제를 수정했습니다. JSON 검증과 전체 읽기·레코드·보관 메모리 한도는 유지합니다.
- 큰 App 기록이 CLI 집계를 막지 않도록 실행 환경을 먼저 구분하고 하나의 읽기 예산으로 처리합니다. 수집 실패를 진행 중으로 표시하던 문구를 수정했습니다.
- 자동 수집과 개인정보 안내를 서버 연결 화면에 통합하고 중복 테마 선택·상태 표시·연결 설명을 제거했습니다. 수집·대시보드 바로가기는 해당 항목으로 이동합니다.
- Grafana 바로가기가 구성된 GroundLine 대시보드를 직접 열도록 수정했습니다. macOS 제목과 본문 테마를 맞추고 시스템 기본값을 유지합니다.

## 0.24.4

- Desktop 개요에 사용자가 요청할 때 읽는 오늘·최근 7일 로컬 사용량과 최근 20개 전송 이력을 추가했습니다. 새 수신, 중복 수신, 새 전송 없음과 실패를 구분합니다.
- 연결 파일 가져오기, API·ClickHouse 준비 상태 진단, 주소·키·원문을 제외한 진단 내보내기를 추가했습니다. Core 진단은 설정으로 이동했습니다.
- 트레이에 수집 상태·대기 건수·마지막 수신 시각을 표시하고 기본 꺼짐인 문제 알림을 추가했습니다. 상태 감시는 별도 수집이나 업로드를 시작하지 않습니다.
- 화면·대상 환경을 바꾸면 진행 중인 연결 파일 읽기 결과를 폐기하며, 서버 준비 상태를 인증·전송 성공으로 표시하지 않습니다.

- Desktop에 Radix Themes 3.3.0을 도입해 버튼·입력창·드롭다운과 색상·간격을 통일했습니다. 설정은 소형 컨트롤을 유지하면서 정렬·섹션 여백을 조정했습니다.

- Desktop 개요를 수신 기록 중심으로 정리하고 개요·서버·설정의 중복 안내와 불필요한 스타일을 제거했습니다. 창 닫기는 한 줄 드롭다운, 테마는 아이콘 선택 버튼으로 축소하고 시스템 기본값을 유지합니다.
- Desktop의 Grafana 바로가기는 HTTPS 외에 localhost·127.0.0.1·[::1]의 HTTP를 허용합니다. 외부 HTTP와 자격증명·경로·쿼리가 포함된 주소는 거절합니다.

- Insights API를 절대경로로 실행하여 custom app의 `PATH` 설정에 영향을 받지
  않도록 한다. 두 Linux 아키텍처의 게시 전 실행 검사도 제한된 `PATH`를 사용한다.
- 연결 화면에서 Insights 등록키의 서버 환경변수·생성 파일 이름을 정확히 안내하고
  TrueNAS 관리용 API 키와 구분한다.

## 0.24.3

- 데스크톱 중복 실행은 기존 창을 열고, 창 닫기는 기본적으로 트레이로 숨긴다.
  설정에서 완전 종료를 선택할 수 있으며 명시적 종료는 앱의 작업을 정리한다.
  자동 수집·전송은 기존 Codex 훅이 담당하고 앱 종료와 독립적으로 동작한다.
- 개요와 서버 화면에 최근 서버 수신 확인 건수·시각과 대기 건수를 표시한다.
  확인 기록은 검증된 서버 응답 뒤에만 저장하며 원문·식별자·자격증명을 포함하지 않는다.
- 수집기 프로세스가 종료되어도 운영체제가 잠금을 해제하여 다음 실행을 막지 않는다.

- Windows에서 다른 프로세스가 정책 파일을 읽는 중에도 수집 중지를 저장할 수
  있도록 표준 라이브러리의 원자적 파일 교체를 사용한다. 이전 읽기 핸들,
  소유자 전용 권한, 실패 시 기존 데이터 보존을 회귀 검증한다.
- Markdown 링크 검증에 `pulldown-cmark`와 `percent-encoding`을 적용해 참조형
  링크, 이미지, 한글·공백·이스케이프 경로와 인코딩된 경로 이탈을 처리한다.
- 개인정보 검사기의 제한된 테스트 예외를 `syn`·`proc-macro2`의 실제 Rust 구문
  범위로 판별해 문자열·주석 안의 괄호가 이후 검사 범위를 가리지 못하게 한다.
- Windows 파일 교체와 프로세스 간 중지 검사를 PR 단계에서도 실행한다.
  0.24.2 태그는 Windows x64 검증에서 멈춰 공개 파일·이미지를 게시하지 않았다.

## 0.24.2

- Support literal loopback HTTP endpoints for local Docker Compose verification,
  while retaining HTTPS for public endpoints.
- Read metric groups as object keys to avoid Linux string-pool concatenation
  triggering the strict artifact privacy guard. Keep the guard unchanged and
  verify the event-to-row mapping with a regression test.
- The 0.24.1 tag passed source, Compose, and desktop qualification but published
  no release assets because the Linux API artifact guard stopped publication.

## 0.24.1

- Pin the dependency-checking tool and its checksum so desktop and workspace
  qualification use the same supported cargo-deny options.
- Enforce the qualified macOS Apple Silicon desktop target and document the
  unresolved Linux-only GLib advisory without suppressing it.
- Include the desktop, HTTPS integration, and privacy fixes prepared in 0.24.0.
  The 0.24.0 tag did not publish release assets because qualification failed.

## 0.24.0

- Synchronize collection stop with in-flight reads and requests across desktop,
  CLI, and hooks; preserve unsent events and recheck consent before each request.
- Preserve the network restriction of existing API deployments when their mode
  is unset, while explicitly selecting general HTTPS for new Compose setups.

- Add a macOS Apple Silicon desktop preview with unified server connection and
  Docker Compose setup, independent Core diagnostics, and explicit collection consent.
- Support general HTTPS endpoints with Tailscale as an optional transport. Keep
  enrollment authentication, bounded retries, private credentials, and strict schemas.
- Add a read-only enrollment-key check and distinguish enrollment, proxy, and
  peer authentication failures without returning secret values.
- Share API and Grafana address drafts across setup flows, preserve saved
  connections, and default desktop appearance to the operating system theme.
- Publish Core, Insights, API, and the desktop preview from one versioned source.
  Upgrade the owner API before using the new desktop connection check.

## 0.23.2

- Reuse validated native catalogs for repair audits and binary-search model
  lookups. Collect safe schema-field diagnostics only on decode failure.
- Add property-based configuration and catalog checks with proptest, and
  compare parsing time and allocation counts with Divan benchmarks.
- Preserve quoted TOML keys and leading comments when applying setup values,
  fixing a regression found by generated settings. Qualify generated file
  preservation and catalog checks on all six native platforms.
- Remove ten unused direct dependencies across six workspace crates, share
  catalog validation, and parse the setup policy once per invocation.

## 0.23.1

- Create user-owned Windows configuration fixtures and verify default-owner
  rejection without weakening runtime ownership checks. Run the bounded
  Windows setup matrix for relevant PRs before release qualification.
- Verify installed Windows artifacts through .NET SHA-256 without depending
  on PowerShell module autoloading. Accept one native UTF-8 catalog BOM while
  retaining strict JSON, model, and encoding validation.
- Add explicit cross-platform installation and setup with the declared
  GPT-6 Astra / xhigh / Fast-off baseline, private configuration backups,
  native context restoration, and bounded retired Core hook trust cleanup.
  Preserve unrelated settings and reject unsupported host/layer state.
- Carry install-and-apply requests through existing-setting and active-guidance
  repair. Add reviewed-plan context repair and exercise setup/installer failure
  boundaries in CLI and native-platform CI checks.

The v0.23.0 source tag failed Windows qualification before release publication
or stable promotion. These installation features first ship in v0.23.1.

## 0.22.2

- Wait for authenticated API and Grafana readiness before the release stack
  checks anonymous dashboard access. Preserve the redirect and semantic assertions.

## 0.22.1

- Make personal rollback resumable across durable writes, preserve bounded
  interrupted-write files, and validate archived baselines and file capacity.
  Review current state before selecting eligible guidance, and run personal
  regression tests in PR qualification.
- Accept a proven zero baseline for a resumed native usage stream without
  dropping preceding window usage or weakening ownership/reset checks.
- Read larger native compaction and completion records using borrowed unused
  payloads, separate raw/projection byte limits, and bounded object fields.
  Preserve malformed-record rejection and plain/compressed history parity.

## 0.22.0

- Add offline personal workflow review, one-rule guidance trials, comparable
  outcome evaluation, and rollback that preserves user edits. Require fresh
  native model/catalog and official guidance evidence; preserve selected models,
  effort, permissions, and non-trial instruction surfaces.
- Add an explicit personal improvement skill and English/Korean documentation.
  Partial reports or unavailable direct outcomes remain observation-only; tokens
  and repeated-call aggregates alone cannot authorize an automatic change.
- Correct report fleet counts for unmatched ClickHouse joins. Return an explicit
  report-contract rejection for unsupported stored history instead of a storage
  outage, preserving the records and strict current report contract.

## 0.21.4

- Confirm actual rollout records intersect the requested audit window before
  including a candidate selected by thread metadata clocks. Archiving or resuming
  inactive inherited history no longer blocks unrelated new activity. Preserve
  ownership, syntax, and timestamp checks for records that may affect the window.

## 0.21.3

- Set executable permissions inside the API image even when release artifact
  downloads reset file modes. Launch both downloaded architecture artifacts
  through the image entrypoint before publishing.

## 0.21.2

- Keep native response and UI usage totals on independent baselines, preserving
  reset and trailing-response checks for the selected source. Accept Codex's
  explicit subagent ownership boundary for paginated fork metadata.
- Stream large native histories into bounded audit projections, retain metric
  semantics, and reject other runtimes after metadata without reading their bodies.
- Include active turns whose sidebar recency predates their latest update.
  Preserve explicit incomplete results for unreadable or unattributed history.
- Consolidate package release summaries and language guidance, remove unwired
  scenario files, and align installation, history retry, and release verification
  documentation in English and Korean. Preserve older notes in Git history.

## 0.21.1

- Preserve an existing collector's active history generation by retrieving it
  during authenticated enrollment. Reuse the same identity and token, refresh
  enrollment metadata, and require ingest contract revision 3 before collection.
- Qualify nonzero-generation collection, immutable retries, and API reporting.
- Allow the owner profile's health endpoint through the collector URL guard so
  the real capability preflight can run before enrollment or upload.

## 0.21.0

- Clarify bring-your-own Insights instances, separate operator and collector
  credentials, and explicit enablement. Add regression coverage for independent
  owner configuration and rejection of management credentials in collector profiles.

- Exclude raw GitHub event payloads from Docker builder provenance and disable
  automatic build-record uploads while retaining maximum build provenance,
  SBOMs, and signatures. Add workflow regression guards and independent public
  log, artifact, and image-metadata privacy gates to the release checklist.

- Qualify Insights' direct native-Codex collection path without inference proxy,
  generated catalog, or Core dependencies. Unify source discovery and readiness,
  reject blocking FIFO inputs, and add isolated native-state/outbox regression tests.

- Add offline native-catalog configuration checks without pinning models or
  rewriting settings. Share skill frontmatter validation between Core and
  packaging, and simplify the alignment skill with focused Astra guidance.

- Remove the old Insights consent, private policy, and private status import
  paths. Reject unsupported local state before enablement writes; keep current
  consent, bounded retries, and pending-data protections without auto-migration.

- Integrate personal skill maintenance into Core with strict host profiles,
  portable baselines, fresh inventory, upstream comparison, and new private
  receipts. Remove the unreleased personal-registry adapter and init command. Reuse
  align-agent-home for reviewed updates and scoped behavior tests without
  overwriting personal skills, adding hooks, or uploading private state.

- Harden current-model guidance, optional Goal handling, permission boundaries,
  and bounded verification. Add structured skill metadata and reference checks;
  see [guidance validation](docs/guidance-validation.md) for behavior acceptance.

- Preserve event-time windows after later task updates; initialize collection
  with an explicit seven-day lookback rather than thread modification times.
- Persist one frozen collection window and exact prepared event, publish only
  complete owned-scope aggregates, and stop automatic reads after three failed
  attempts. Keep partial windows, outbox durability, and delivery state distinct.
- Reconcile native and legacy cumulative checkpoints without summing duplicate
  sources; surface unanchored mixed usage and counter resets as incomplete.
- Bound diagnostic examples, record count, and record bytes; retry only a
  missing plain/compressed representation once and normalize consent timestamps.
- Check API ingest capabilities before cached-token enrollment or upload;
  preserve pending data and report `api_upgrade_required` on incompatible APIs.
- Reduce audit parsing and statistics allocations without changing aggregate
  output; add a private-data-free, opt-in repeatable performance benchmark.
- Follow the active Codex model catalog without pinning model or effort;
  centralize bounded Astra-aware telemetry dimensions and usage provenance.
- Support bounded Zstandard rollouts and native shared-history suffix usage,
  numeric state-store discovery, and explicit incomplete-read coverage.
- Correct resumed-task selection, activity export, cross-task attribution, and
  paired compaction counts, with regression and isolated Insights query tests.
- Document Codex-owned permissions, context management, and API-first rollout.

## 0.20.2

- Restrict weekly reports to the owner admin credential and require the Insights
  CLI to read that credential from an explicit private, no-follow token file.
- Isolate authenticated request budgets by role and collector, cache bounded
  readiness probes, and cap concurrent storage work and active rate scopes.
- Reject contradictory, overflowing, and high-cardinality event metrics before
  ClickHouse insertion; enforce per-collector and global storage watermarks with
  a bounded retention TTL while preserving idempotent retries.
- Harden local Codex audit reads against symlink, ownership, traversal, oversized
  metadata, and unbounded row allocation across macOS, Linux, and Windows.
- Build the API image from separately verified stable-Rust musl binaries using a
  digest-pinned final image, bounded Docker context, checksums, OCI provenance,
  SBOMs, and registry attestations instead of a mutable in-Docker toolchain.
- Separate collection cadence from bounded delivery retries, cap the private
  outbox at 256 events and 16 MiB, drain 16-event batches with durable backoff,
  and capture every hook trigger before starting a detached worker.
- Require explicit re-consent before replacing a legacy no-network receipt,
  issue a new owner-service receipt, and preserve incompatible pending events
  in a private quarantine instead of uploading or deleting them.
- Apply Tailnet-peer and global pre-authentication budgets before collector
  body reads and lookup, bound body-read time and concurrent requests, shed
  saturated collector work without queued waiters, reserve operator storage
  capacity, and keep readiness probes single-flight outside the cache lock.
- Preserve permanent-rejection stops across automatic cycles, classify 4xx
  before parsing its body, checkpoint accepted delivery before deleting outbox
  files, and claim hook markers so a concurrent later capture cannot be lost.
- Attest every binary release asset, surface eventual ClickHouse TTL cleanup in
  reports and Grafana, and require private no-follow TrueNAS runtime inputs.

## 0.20.1

- Remove baked-in ClickHouse, Nginx, Grafana, and datasource-plugin versions
  from the public Compose template. Select them through a strict compatibility
  profile, accept a complete newer candidate set in manual qualification, and
  run that candidate through the real mutation and authenticated Grafana query
  lanes without silently changing the release-tested default or production.
- Add a reachable Git-object privacy gate so deleting a leaked source file no
  longer makes release qualification pass while the old blob remains public.
  Scan binary markers too, distinguish exact public GitHub runner roots, and
  remap runner workspace and home paths from future release binaries. Normalize
  only the scanner's marker declaration instead of excluding its whole source
  blob, so a separate leak in that file still fails qualification. Inventory
  historical file names independently so a deleted forbidden secret-file name
  cannot be hidden by blob reuse under another path.
- Add a release-only rendered stack gate that boots ClickHouse, the Axum API,
  and Grafana, then executes every dashboard query through the provisioned
  datasource and validates semantic frames.
- Generalize private Compose dataset roots for Linux, macOS, and Windows Docker
  hosts, move the canonical template out of the TrueNAS-specific path, and add
  end-to-end self-hosting instructions.
- Document independent Core-only, Insights-only, and combined installation
  profiles plus the supported Codex, Tailnet, API, ClickHouse, Grafana, Docker
  Compose, and TrueNAS integration boundary.
- Enforce explicit owner opt-in for Insights, preserve only the exact deployed
  private-state upgrade contracts, and expose actionable readiness, freshness,
  clock-skew, Tailnet, and delivery states.
- Skip detached workers while disabled, fail closed on malformed local state,
  and make error receipts honest when partial mutation is unknown.
- Add a tested fail-closed owner-profile example and clarify that native plugin
  executables must be resolved from the installed target directory rather than
  assuming a user-shell `PATH` alias.
- Require immutable API image digests for normal self-hosted renders, make the
  mutable CI/development exception explicit and machine-auditable, and reject
  unauthenticated Grafana access in the release-only live stack gate.
- Disable Grafana anonymous access, initialize its bind directory with a
  one-shot least-privilege service instead of world-writable permissions, and
  separate the dedicated published-port ingress bridge from Grafana's
  plugin-download egress.
- Authenticate both generic-stack and optional TrueNAS controller Grafana
  semantics checks with owner-local credentials; no secret enters public CI or
  verification receipts. Require the TrueNAS controller's owner-rendered
  Compose input explicitly so it cannot mistake the public placeholder template
  for deployable configuration.
- Reject aliased template, rendered Compose, and secret-store paths; require
  every deployment placeholder before generating credentials and fail closed if
  either generated file is not private to the current user.

## 0.20.0

- Publish one public monorepo with two canonical, independently installable
  plugins: offline zero-hook Core and opt-in self-hosted Insights.
- Remove duplicate root package surfaces and the obsolete separate Insights
  marketplace/repository contract.
- Require a distinct owner-issued enrollment credential in addition to Tailnet
  reachability, and keep it outside the sanitized owner profile.
- Build both binaries for six targets in one cost-bounded workflow, publish a
  multi-architecture API image, and promote both plugin packages atomically.
- Reject malformed or version-mismatched tags before the expensive matrix and
  publish the API image only after every native artifact succeeds.
- Preserve an existing TrueNAS enrollment credential or inject one from an
  owner-local deployment input during migration, without exposing it in Git,
  CI, or deployment receipts.
- Complete RustSec, license, source-privacy, native package, ClickHouse schema,
  and Grafana-query qualification for the public source.
- Make the Insights API the single active ClickHouse schema migrator, reconcile
  weekly report quality reasons with the strict schema-3 contract, and qualify
  enrollment, idempotent retry, reporting, every Grafana query, and deletion
  against a real isolated ClickHouse.
- Reconcile Grafana provisioning, expand private-artifact rejection, and replace
  the quadratic source marker scan with one multi-pattern pass.

## 0.19.0

- Establish a clean public, local-first GroundLine core with no lifecycle hook,
  network client, background worker, remote destination, or collector identity.
- Keep bounded local Codex audits, project configuration inventory, deterministic
  efficiency contracts, and six-target native packaging.
- Add a zero-hook provider smoke contract and a public-readiness gate that rejects
  private infrastructure markers, personal paths, and package drift.
- Keep GitHub Actions cost-bounded: pull requests run fast checks, while full
  qualification and release artifacts require explicit manual dispatch.
