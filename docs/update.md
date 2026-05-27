# Update

Use fast-forward updates so the local package stays aligned with the private
repository.

```bash
cd groundline
git pull --ff-only
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

For a release gate, also run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

Do not update provider homes, rules, hooks, or skills until validation passes
and the intended mutation is explicit.
