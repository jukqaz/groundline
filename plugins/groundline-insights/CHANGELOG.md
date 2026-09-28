# GroundLine Insights changes

## 2026.09.29-a (`2026.929.1`)

- 릴리스 이름은 날짜와 당일 순번으로 표시합니다. CLI·태그·manifest·전송 값은 숫자 SemVer를 유지하며, 공용 변환에서 표시명을 만듭니다. [버전 규칙](../../docs/versioning.md)

- GPT-6 Astra·Sol·Luna의 집계 라벨을 분리하고 ingest contract revision 8을 사용합니다. 업그레이드 전에 owner API가 새 계약을 광고하는지 확인해야 합니다.
- `worker check-server`는 기존 연결의 호환성을 읽기 전용으로 검사합니다. 설치기는 marketplace 갱신 전에 이를 실행하며, 실패 시 수집기·설정·동의 상태를 유지합니다.
- API의 기존 신뢰 판정 정의를 원본 보존·재검증·중단 후 재개 가능한 명시적 마이그레이션으로 이행합니다. 알 수 없는 정의와 SQL식 변조는 거절합니다.
- 공유 계약과 SQL 분석 bucket 제한을 통일해 유효한 81·89·90개 조합을 보존합니다.
- Codex 기본 프롬프트 길이 제한과 독립적인 Core/Insights 설치 계약을 유지합니다. 수집 동의·목적·기존 이벤트 기록을 자동으로 재설정하지 않습니다.

## 0.25.5

- Preserve complete lifecycle records when provider usage has not arrived yet, including CLI SessionStart counters and receipt visibility.
- Keep missing usage explicit in reports and retain normal records for the configured period. Incomplete reads and incoherent usage provenance remain quarantined with the short TTL.
- Correct existing view and retention classification without rewriting payloads, IDs, counters, or collection periods. The ingest contract remains revision 6.

## 0.25.4

- Accept overlapping failure labels and outputs from calls in an earlier window.
- Preserve bounded signal keys and counts, strict event integrity, and durable retry windows. Upgrade the API before the collector and use an explicit retry to recover paused collection.
- Require ingest contract revision 6 before sending events to prevent rejection by an older API.

## 0.25.3

- Derive and validate canonical cache ratios, including null for zero denominators.
- Enforce the shared payload projection in ClickHouse and reject overlapping or unbounded ingestion windows.
- Retain quarantined receipts for seven days; preserve the configured retention for trusted data.
- Add offline event validation and require ingest contract revision 5. Review and repair historical data before upgrading the API, then upgrade the plugins.

## 0.25.2

- Validate usage bounds and provenance in the producer, API, and root storage constraint.
- Keep trusted analysis separate from incomplete receipts; expose quarantine counts in reports and Grafana.
- Preserve no-usage lifecycle receipts, idempotency, consent, cursors, and configured retention.
- Require ingest contract revision 4. Upgrade the API before the plugins.

## 0.25.1

- Align the shared release with Core's evidence-based GPT-6/GPT-5.6 workflow review.
- Preserve the existing collection, report, and storage contracts; no historical data reset or migration is required.

## 0.25.0

- Share Codex App/CLI environment validation across collection and enrollment;
  exclude foreign runtime records without rewriting native data.
- Preserve unsupported stored state and reject invalid current metadata before
  creating state or sending events.
- Repair mixed-history report coverage reasons, retain business data, and bound
  ClickHouse internal diagnostic logs to 7 or 30 days.
- Verify existing JWT-authenticated Grafana deployments without changing login
  policy or falling back to Basic authentication.
- Retire the separate Desktop app; existing plugin and CLI collection continues.

## 0.22.2

- Wait for authenticated API and Grafana readiness before the release stack
  checks anonymous dashboard access. Preserve the redirect and semantic assertions.

## 0.22.1

- Recover collection windows containing a directly proven fresh native usage
  baseline. Preserve ambiguous earlier usage as incomplete.
- Accept larger native compaction/completion records with borrowed unused
  bodies and separate raw/projection bounds; preserve strict input validation.

## 0.22.0

- Count absent/current reporters correctly with explicit join-presence markers. Keep
  unsupported stored history intact and distinguish report-contract rejection
  from transport/storage failure in the API and report client.

## 0.21.4

- Keep immutable collection windows recoverable when an inactive old thread is
  archived or resumed. Exclude it only after reading and checking every record's
  timestamp; retain current-window ownership and malformed-input protections.

## 0.21.3

- Preserve executable permissions in published API images and verify both
  downloaded architecture artifacts through the image entrypoint before publish.

## 0.21.2

- Collect active native tasks using their newest update clock and independent
  native/UI usage baselines, without adding the two totals.
- Stream large histories into bounded audit projections. Preserve unknown
  ownership and selected-source resets as incomplete; never reset cursors.
- Correct source-tag versus packaged-install guidance, document API-first
  upgrades and preserved historical gaps, and consolidate obsolete notes.

## 0.21.1

- Retrieve the server's active collection generation during authenticated
  enrollment while preserving the collector identity, token, and prior data.
- Require ingest contract revision 3 and permit the configured health endpoint
  through the collector URL guard.

## 0.21.0

- Collect directly from native Codex App/CLI without Core or an inference proxy.
- Preserve frozen collection windows and prepared outbox events. Start new
  collection with a seven-day lookback and limit automatic failed reads to three.
- Accept only current consent, policy, and status contracts. Unsupported state
  is preserved for operator review rather than silently converted.
- Share bounded model/effort and usage-source labels with the API. Upgrade the
  API before its collectors and preserve rejected pending events.
- Exclude raw GitHub event payloads from image provenance while retaining
  checksums, signed attestations, and SBOMs.

## 0.20.x

- Establish the independent public Insights package, owner-issued enrollment,
  private configuration, API-owned ClickHouse migrations, and Grafana reports.
- Qualify six native targets and a rendered stack using a strict infrastructure
  compatibility profile. Keep real deployment inputs outside public CI.
- Add bounded request/storage quotas, retention, durable delivery backoff,
  trigger markers, and separate collector versus administrator credentials.

## Earlier details

The [historical package changelog](https://github.com/jukqaz/groundline/blob/v0.21.1/plugins/groundline-insights/CHANGELOG.md)
preserves earlier release notes, including retired Python runtimes and state
formats. It is historical evidence, not current installation or migration advice.
