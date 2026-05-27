# Release Checklist

Before a GroundLine release:

- Set the release version once, for example `VERSION=vX.Y.Z`.
- Confirm `README.md`, `docs/install.md`, `docs/update.md`, `SECURITY.md`,
  `CONTRIBUTING.md`, and `LICENSE` describe the public package accurately.
- Confirm author and maintainer fields do not expose personal identity unless
  that exposure is intentional.
- Confirm scripts do not print credentials, raw provider runtime state, or full
  default home paths.
- Confirm downloaded CI tools are pinned and checksum verified.
- Confirm `.github/workflows/test.yml` runs the offline gates on push and pull request.
- Confirm `.github/workflows/radar.yml` can run manually and upload `groundline-radar.json`.
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json`
- `git tag "$VERSION"`
- `git push origin "$VERSION"`
- `gh release create "$VERSION" --repo jukqaz/groundline --title "$VERSION" --notes-file CHANGELOG.md`

Report any unavailable Docker or network proof as partial, not passing.
