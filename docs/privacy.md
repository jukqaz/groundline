# Privacy

GroundLine should be safe to run in a local agent environment without turning
that environment into source material.

## What GroundLine May Read

- repository files in the GroundLine checkout
- manifest presence for Codex, Claude Code, and Antigravity
- optional local tool versions when `--probe-tools` is passed
- optional command-source versions when `--command-sources` is passed

## What GroundLine Must Not Collect

- auth files
- secret values
- raw transcripts
- prompt logs
- provider sessions
- shell snapshots
- local databases
- caches

## Output Rules

- `mutation_performed` must be explicit.
- `real_home_touched` must be explicit for scenario and smoke commands.
- Default home paths should be shown as `~`.
- Explicit `--home` paths may be shown because they are normally temporary test
  homes.
- Secret-like command output must be rejected or redacted.

Use a temporary home when testing install plans:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --home /tmp/groundline-home --json
```

Do not paste raw local outputs into public issues until paths and provider state
have been reviewed.
