# Update

Use fast-forward updates so the local package stays aligned with the upstream
repository.

```bash
cd groundline
git pull --ff-only
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_safety_eval.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

For a release gate, use `docs/release-checklist.md`. At minimum, also run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_validate.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

Do not update provider homes, rules, hooks, or skills until validation passes
and the intended mutation is explicit.

If `groundline_provider_smoke.py` returns `PARTIAL`, read
`install_doctor_status` and provider `runtime_probe.issues` before reinstalling.
Common causes are stale installed version, missing provider payload, or skill
count drift.
