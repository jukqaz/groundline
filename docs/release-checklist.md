# Release Checklist

Before a GroundLine release:

- Confirm `.github/workflows/test.yml` runs the offline gates on push and pull request.
- Confirm `.github/workflows/radar.yml` can run manually and upload `groundline-radar.json`.
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json`
- `git tag v0.1.0`
- `git push origin v0.1.0`
- `gh release create v0.1.0 --repo jukqaz/groundline --title v0.1.0 --notes-file CHANGELOG.md`

Report any unavailable Docker or network proof as partial, not passing.
