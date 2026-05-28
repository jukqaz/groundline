# Install

GroundLine is a public plugin package. Install has two phases:

1. Clone and validate the package without touching provider homes.
2. Install it through the provider you actually use.

Do phase 1 first. It gives you a clean PASS/FAIL signal before any provider
state changes.

## 1. Clone And Validate

```bash
gh repo clone jukqaz/groundline
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

If you do not use GitHub CLI:

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
```

Expected result:

- package validation returns `status=PASS`
- staged dogfood returns `status=PASS`
- provider smoke returns `status=PASS` for a matching install, or `PARTIAL`
  when an existing provider target is stale relative to the checked-out package
- `mutation_performed=false`
- `real_home_touched=false` for staged dogfood
- provider target paths are printed with `~`, not full home dumps

Treat provider smoke `FAIL` as an install blocker. Treat provider smoke
`PARTIAL` as actionable only after reading top-level `next_actions`. Use
`--require-installed` after provider installation when missing provider targets
should also make provider smoke return `PARTIAL`.

If provider smoke is `PARTIAL` only because real provider targets are stale,
you can prove the package itself is install-ready with a fake refreshed provider
home. See `docs/provider-smoke.md` for the temporary layout command.
That proof should return `status=PASS`, `fake_home_used=true`, and
`real_home_touched=false`.

The provider smoke command is read-only. It reports manifest presence, local
target paths, installed package version, source package version, payload
presence, skill count, same-version content drift, and provider cache
candidates for Codex, Claude Code, and Antigravity without writing to the home
directory. Default home paths are displayed with `~`.

Read `install_doctor_status`:

- `PASS`: source manifests are present and any existing provider target matches
  the source package.
- `PARTIAL`: an installed provider target has stale version, stale provider
  cache, missing payload, skill count drift, or `content_fingerprint_mismatch`.
  With `--require-installed`, a missing provider target is also `PARTIAL`. The
  command exits 2 while still printing JSON.
- `FAIL`: source manifests needed for provider install are missing.

When the result is `PARTIAL` or `FAIL`, read top-level `next_actions` first.
Each provider also includes `recommended_actions` for the specific target.

## 2. Check Local Runtime Posture

Run the doctor before wiring the package into any runtime:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
```

If Context7 or Exa are unavailable, doctor output includes setup
recommendations. They remain optional because GroundLine must still work as a
local control plane.

Read `docs/provider-dogfood.md` when you need staged provider proof before a
release or manual provider setup.

## 3. Install Into A Provider

GroundLine can be installed through provider-native plugin surfaces after the
public repository is reachable. These commands write to provider plugin state.

Codex:

```bash
codex plugin marketplace add jukqaz/groundline --ref main
codex plugin list --marketplace groundline
codex plugin add groundline@groundline
```

Claude Code:

```bash
claude plugin marketplace add jukqaz/groundline
claude plugin install groundline@groundline
```

Antigravity:

```bash
agy plugin install https://github.com/jukqaz/groundline
```

## 4. Confirm Install

Codex:

```bash
codex plugin list --marketplace groundline
```

Claude Code:

```bash
claude plugin list
```

Antigravity:

```bash
agy plugin list
```

Look for `groundline` and the expected version. If the provider only shows an
import record, also run provider validation and the read-only install doctor
from the cloned repository:

```bash
claude plugin validate ./plugins/groundline --strict
agy plugin validate ./plugins/groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_validate.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json --require-installed
```

## What To Do First After Install

In a new agent session, ask:

```text
Use GroundLine to recheck current state before continuing this task.
```

For release work, ask:

```text
Use GroundLine to polish this release candidate, then lock the release cut.
```

For local provider packaging checks, see `docs/provider-packaging.md`.
