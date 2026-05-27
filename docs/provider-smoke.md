# Provider Smoke

Provider smoke checks prove that GroundLine has the manifests and path plan
needed for Codex, Claude Code, and Antigravity.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

Expected safety fields:

- `mutation_performed=false`
- `real_home_touched=false`
- each provider reports `manifest_present=true`
- default home paths are displayed with `~`

Use `--home` with a temporary directory when testing install plans:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --home /tmp/groundline-home --json
```

The smoke command does not install, copy, link, or rewrite runtime state.
