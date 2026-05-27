# Release Checklist

Before a GroundLine release:

- Confirm `.github/workflows/test.yml` runs the offline gates on push and pull request.
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json`
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json`

Report any unavailable Docker or network proof as partial, not passing.
