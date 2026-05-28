# Release Checklist

Use this checklist before a GroundLine release. Everything before the
approval-required publishing section is read-only or local-only.

Do not run publish commands unless the user approved publishing in this
session.

## Preflight

- Set the release version once, for example `RELEASE_VERSION=X.Y.Z`.
- Confirm `README.md`, `docs/install.md`, `docs/update.md`, `SECURITY.md`,
  `CONTRIBUTING.md`, and `LICENSE` describe the public package accurately.
- Confirm author and maintainer fields do not expose personal identity unless
  that exposure is intentional.
- Confirm scripts do not print credentials, raw provider runtime state, or full
  default home paths.
- Confirm downloaded CI tools are pinned and checksum verified.
- Confirm `.github/workflows/test.yml` runs the offline gates on push and pull request.
- Confirm `.github/workflows/radar.yml` can run manually and upload `groundline-radar.json`.

## Version Bump Sequence

Use plain semver for manifest files and a `v` prefix for git tags.

```bash
RELEASE_VERSION=X.Y.Z
TAG="v$RELEASE_VERSION"
```

1. Update source manifest versions only:
   - `plugin.json`
   - `.codex-plugin/plugin.json`
   - `.claude-plugin/plugin.json`
2. Do not edit `plugins/groundline` manifests directly. Run
   `PYTHONDONTWRITEBYTECODE=1 python3 scripts/sync_provider_package.py --json`
   to regenerate the packaged payload.
3. Run source and packaged validation before any tag is created:
   - `PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json`
   - `(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)`
4. Move `CHANGELOG.md` entries from `Unreleased` to
   `v$RELEASE_VERSION - YYYY-MM-DD` only after the release is actually being
   cut. Keep the entries under `Unreleased` while the ship decision is `hold`.
5. Confirm `plugins/groundline/plugin.json`,
   `plugins/groundline/.codex-plugin/plugin.json`, and
   `plugins/groundline/.claude-plugin/plugin.json` match the source manifest
   version after package sync.

## Verification Gates

The individual commands below are authoritative. The wrapper exists to reduce
missed steps during local release closeout and never runs tag, push, or release
creation commands.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --json --keep-going --include-docker-execution
```

Use `--keep-going` during release closeout so an expected provider smoke
`PARTIAL`, such as stale installed cache after a version bump, does not prevent
later dogfood and scenario gates from producing evidence. The wrapper still
returns `PARTIAL` when any gate is partial.

When a gate emits JSON, the wrapper preserves a compact `json_summary` with
fields such as `status`, `install_doctor_status`, `install_issues`, and
`next_actions`. Use that summary before reading long `stdout_tail` output.
The top-level `non_passing_gates` and `next_actions` fields summarize the
current blocker set for people and LLM handoff.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_validate.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_safety_eval.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_privacy_scan.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

Report any unavailable Docker or network proof as partial, not passing.

## Release Decision

- Confirm `CHANGELOG.md` describes the unreleased scope before tagging.
- Confirm `docs/maturity-assessment.md` records `ship`, `hold`, or `continue`.
- If the ship decision is `hold`, stop before tag, push, or release creation
  unless the user explicitly accepts the missing proof for this release.
- If live provider activation proof is missing, record whether that is an
  accepted partial or a release blocker.
- If provider-native validation is `PARTIAL` because `claude` or `agy` is not
  installed, record whether that missing local validator is accepted for this
  release. Treat validator failures as release blockers.

## Approval-required Publishing Commands

These commands mutate the remote repository or public release state. Do not run
them without explicit user approval in the current session.

```bash
git tag "$TAG"
git push origin "$TAG"
gh release create "$TAG" --repo jukqaz/groundline --title "$TAG" --notes-file CHANGELOG.md
```

## Post-publish Check

- Confirm the tag points at the intended commit.
- Confirm CI passed for the published commit.
- Confirm the GitHub Release page is reachable.
- Run provider install confirmation from the published ref.
- Record any accepted partial proof in the release notes or next-work document.
