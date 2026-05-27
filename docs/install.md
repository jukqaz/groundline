# Install

GroundLine is private by default. Install from an authenticated GitHub account
that can read `jukqaz/groundline`.

## Clone

```bash
gh auth status
gh repo clone jukqaz/groundline
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

The provider smoke command is read-only. It reports manifest presence and local
target paths for Codex, Claude Code, and Antigravity without writing to the home
directory.

## Use

Run the doctor before wiring the package into any runtime:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
```

If Context7 or Exa are unavailable, doctor output includes setup
recommendations. They remain optional because GroundLine must still work as a
local control plane.
