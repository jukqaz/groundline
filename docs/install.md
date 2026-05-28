# Install

GroundLine is a public plugin package. Clone it with either GitHub CLI or git.

## Clone

```bash
gh repo clone jukqaz/groundline
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

Alternative:

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
```

The provider smoke command is read-only. It reports manifest presence and local
target paths for Codex, Claude Code, and Antigravity without writing to the
home directory. Default home paths are displayed with `~`.

## Use

Run the doctor before wiring the package into any runtime:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
```

If Context7 or Exa are unavailable, doctor output includes setup
recommendations. They remain optional because GroundLine must still work as a
local control plane.

Read `docs/provider-dogfood.md` when you need staged provider proof before a
release or manual provider setup.
