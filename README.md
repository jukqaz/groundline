# GroundLine

GroundLine is a lightweight control plane for Codex, Claude Code, and
Antigravity. It keeps capability blueprints, detects runtime and ecosystem
drift, prepares research and upgrade packets, and guides agents through
current-state proof, side-effect boundaries, live evidence, and handoff.

GroundLine is not a synced config dump, a heavy bootstrap CLI, or an always-on
MCP bundle.

## Supported Scope

- Codex
- Claude Code
- Antigravity
- macOS on Apple Silicon
- Linux

## Skills

| Skill | Use when |
| --- | --- |
| `reconcile-current-state` | Continuing previous agent work after a handoff, stale summary, PR note, or agent claim. |
| `audit-agent-history` | Inspecting current agent histories and deriving reusable improvements. |
| `guard-side-effects` | An action may mutate files, git state, remotes, production, money, access, secrets, or user data. |
| `close-live-work` | Local tests or CI passed but runtime, endpoint, release, worker, queue, or browser evidence still matters. |
| `align-agent-home` | Agent home config, guidance, skills, rules, hooks, manifests, tool profiles, or private source boundaries need alignment. |
| `recover-worktree-branch` | A worktree path vanished, branch is detached or missing, or cleanup needs proof before deletion. |

## Runtime Manifests

- Codex: `.codex-plugin/plugin.json`
- Claude Code: `.claude-plugin/plugin.json`
- Antigravity: `plugin.json`

## Install And Update

- Human guide: `docs/human-guide.md`
- LLM guide: `docs/llm-guide.md`
- Install: `docs/install.md`
- Update: `docs/update.md`
- Provider smoke: `docs/provider-smoke.md`
- Public release: `docs/public-release.md`
- Privacy: `docs/privacy.md`
- Git history privacy: `docs/git-history-privacy.md`

## Operating Loop

```text
doctor -> radar -> research packet -> upgrade packet -> skill-guided work
```

GroundLine defaults to read-only and offline. Apply, install, sync, deploy,
access, billing, and destructive actions require explicit user approval.

## Standard Tool Profile

GroundLine core has no hard MCP dependency.

- GitHub: PR, issue, CI, release, branch, and adoption evidence
- Context7: current library and runtime documentation lookup
- Exa: ecosystem discovery and research

Missing tools produce setup recommendations only.

## External Tool Calls

GroundLine uses local tools only when the operator opts in.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
```

The doctor probe runs read-only version checks for git, gh, docker, and curl.
Radar command sources must be JSON arrays of command tokens; shell strings are
rejected.

## Validation

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json
```

CI should run the same offline gates from `.github/workflows/test.yml`. A real
Linux Docker run is a release gate:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

`.github/workflows/radar.yml` runs a scheduled radar packet and uploads the JSON
artifact for review.

## Project Docs

- Contributing: `CONTRIBUTING.md`
- Security: `SECURITY.md`
- License: MIT, see `LICENSE`

GroundLine should not collect prompt text, raw transcripts, credentials, or
provider runtime state. Reports should keep mutation status, verification
evidence, and setup gaps explicit enough for both humans and LLM agents.
