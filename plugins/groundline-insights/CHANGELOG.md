# GroundLine Insights changes

This package summary covers the current release line. The repository
[changelog](https://github.com/jukqaz/groundline/blob/main/CHANGELOG.md) records shared changes. A source version entry
does not prove that its image or `stable` distribution has been published.

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
