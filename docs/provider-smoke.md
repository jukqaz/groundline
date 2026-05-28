# Provider Smoke

Provider smoke checks prove that GroundLine has the manifests, path plan, and
version-aware install state needed for Codex, Claude Code, and Antigravity.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

Expected safety fields:

- `mutation_performed=false`
- `secret_value_printed=false`
- `real_home_touched=false`
- `install_doctor_status=PASS | PARTIAL | FAIL`
- `source_package.skill_index_consistent=true`
- `source_package.version` comes from canonical `plugin.json`
- `source_package.content_fingerprint` fingerprints the installable payload
  while ignoring manifest version fields
- each provider reports `manifest_present=true`
- each provider includes a read-only `runtime_probe`
- each provider includes `recommended_actions`
- top-level `next_actions` lists only actions needed for `PARTIAL` or `FAIL`
  provider states
- default home paths are displayed with `~`

Use `--home` with a temporary directory when testing install plans:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --home /tmp/groundline-home --json
```

The smoke command does not install, copy, link, or rewrite runtime state.

When a fake or real provider target already contains a GroundLine checkout, the
runtime probe reports target manifest presence, target skills presence, target
skill count, installed version, source version, content fingerprint, and whether
the target matches the source package. It still performs no mutation.

Important runtime probe fields:

- `installed_version`: version read from the installed provider target manifest
- `source_version`: canonical version read from repository `plugin.json`
- `version_matches_source`: whether the installed target matches the source
- `version_check`: `match`, `mismatch`, `unavailable`, or `not_installed`
- `target_skill_count_matches_source`: whether installed skills match source
- `content_matches_source`: whether the installable payload matches source even
  when the semantic version is unchanged
- `source_content_fingerprint` and `target_content_fingerprint`: SHA-256
  fingerprints of the installable payload, with manifest version fields ignored
- `install_source`: `direct` or provider cache
- `candidate_versions`: cache or target versions found while selecting the
  installed target
- `issues`: `version_mismatch`, `stale_cache_version`,
  `missing_manifest_payload`, `missing_skills_payload`, or
  `skill_count_mismatch`, `content_fingerprint_mismatch`
- `recommended_actions`: provider-specific read-only remediation guidance, such
  as refreshing a stale install or reinstalling a missing payload

`next_actions` is the deduplicated top-level list for release closeout. It is
empty when provider smoke is `PASS`; when provider smoke is `PARTIAL` or `FAIL`,
use it as the first human/LLM handoff summary.

Antigravity may expose package shape and skills without an installed semantic
version. In that case `version_check=unavailable` is acceptable when manifest
presence and skill count still match the source package.

Exit code behavior:

- `PASS` exits 0.
- `PARTIAL` exits 2 and still prints JSON. This is the expected signal for an
  installed target with stale version, missing payload, skill count drift, or
  same-version content drift.
- `FAIL` exits 1 when source manifests needed for provider install are missing.

Use `docs/provider-dogfood.md` and `scripts/groundline_dogfood.py` when a staged
package plus shared scenario contract check is required.
