# Contributing

GroundLine is a small stdlib-only plugin package. Changes should keep the
runtime easy for humans to run and easy for LLM agents to inspect.

## Local Setup

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
```

No package install is required for the core checks.

## Change Rules

- Keep scripts stdlib-only unless a new dependency is clearly justified.
- Keep Codex, Claude Code, and Antigravity as the only supported runtimes.
- Keep macOS on Apple Silicon and Linux as the only supported platforms.
- Keep default behavior read-only and offline.
- Do not commit provider auth files, sessions, shell snapshots, logs, caches,
  raw prompts, transcripts, or secret values.
- Do not add hooks that run by default across every user repository.

## Verification

Run the smallest credible gate for the change. For release-sized changes, run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

If Docker or a network check is unavailable, report the result as partial and
include the exact skipped command.

## Pull Requests

- Use one logical intent per pull request.
- Explain the user-facing behavior change.
- Include verification commands and results.
- Call out any mutation boundary, provider runtime path, or external command
  change explicitly.
