# GroundLine

GroundLine is a lightweight control plane for Codex, Claude Code, and
Antigravity. It keeps capability blueprints, detects runtime and ecosystem
drift, prepares research and upgrade packets, and guides agents through
current-state proof, side-effect boundaries, live evidence, and handoff.

GroundLine is not a synced config dump, a heavy bootstrap CLI, or an always-on
MCP bundle.

GroundLine also does not reimplement provider-native features. Built-in goal
modes, subagents, hook engines, plugin installers, MCP launchers, browser
controls, and app context features stay owned by Codex, Claude Code, or
Antigravity. GroundLine provides the provider-neutral task contracts, safety
boundaries, and verification language around them.

## Languages

English is the default and canonical language for GroundLine documentation.
Korean companion docs are available for human-facing setup and workflow reading:

- Korean README: `README.ko.md`
- Korean human docs index: `docs/ko/index.md`
- Language policy: `docs/language-policy.md`

LLM-readable references, output contracts, manifests, and skill instructions
stay in English unless a release explicitly changes that policy.

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
| `agent-ecosystem-radar` | One pass should research, compare, and recommend agent workflow upgrades. |
| `research-agent-ecosystem` | External agent tools, skills, plugins, MCP servers, hooks, or workflow sources need source-backed research. |
| `compare-agent-workflows` | Researched candidates need scoring against GroundLine scope, safety, context cost, and setup weight. |
| `recommend-groundline-upgrades` | Research and comparison findings need `adopt`, `adapt`, `watch`, or `reject` decisions. |
| `evaluate-agent-capability` | Existing tools, skills, plugins, MCP servers, hooks, agents, or workflow packs need quality, security, context cost, and fit evaluation. |
| `evaluate-ai-usage-maturity` | A person or team's AI workflow maturity needs evidence-backed scoring and next upgrades. |
| `package-agent-task` | A broad, resumed, or high-context request needs a concise LLM-ready task packet. |
| `hold-the-line` | A task starts expanding with new ideas, extra research, more tools, or unclear finish criteria before current work is closed. |
| `polish-release-candidate` | A release candidate needs docs polish, duplicate cleanup, privacy checks, gate ordering, or commit split planning before ship judgment. |
| `stabilize-release-cut` | A growing change set needs scope lock, release gates, dogfood evidence, and a ship decision. |
| `compare-release-delta` | A deployed release needs comparison with the previous version, runtime evidence, regression checks, or rollback notes. |
| `evaluate-groundline-pack` | GroundLine itself needs review for repository readiness, skill completeness, safety, tests, and release fitness. |
| `curate-groundline-skills` | Skills, references, scripts, agents, hooks, MCP recommendations, or docs need create, adapt, merge, split, deprecate, or reject decisions. |

## Runtime Manifests

- Codex: `.codex-plugin/plugin.json`
- Claude Code: `.claude-plugin/plugin.json`
- Antigravity: `plugin.json`

## Install And Update

- Human guide: `docs/human-guide.md`
- Korean human guide: `docs/ko/human-guide.md`
- LLM guide: `docs/llm-guide.md`
- Install: `docs/install.md`
- Korean install: `docs/ko/install.md`
- Provider packaging: `docs/provider-packaging.md`
- Korean provider packaging: `docs/ko/provider-packaging.md`
- Update: `docs/update.md`
- Korean update: `docs/ko/update.md`
- Provider smoke: `docs/provider-smoke.md`
- Provider dogfood: `docs/provider-dogfood.md`
- Provider guardrails: `docs/provider-guardrails.md`
- Optional MCP recipes: `docs/mcp-recipes.md`
- Public release: `docs/public-release.md`
- Privacy: `docs/privacy.md`
- Korean privacy: `docs/ko/privacy.md`
- Terms: `docs/terms.md`
- Korean terms: `docs/ko/terms.md`
- Git history privacy: `docs/git-history-privacy.md`
- Language policy: `docs/language-policy.md`
- Next work: `docs/next-work.md`
- Next version: `docs/next-version.md`
- Skill portfolio: `docs/skill-portfolio.md`
- Korean skill portfolio: `docs/ko/skill-portfolio.md`
- Skill lifecycle: `references/skill-lifecycle.md`
- LLM-readable skill index: `references/skill-index.json`
- Optional MCP profiles: `references/optional-mcp-profiles.md`

## Operating Loop

```text
doctor -> package-agent-task -> hold-the-line -> skill-guided work -> verify -> polish-release-candidate -> stabilize-release-cut -> compare-release-delta
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
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
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

## Marketplace Install

GroundLine includes provider-facing package metadata for local and public
marketplace-style installation.

```bash
codex plugin marketplace add jukqaz/groundline --ref main
codex plugin add groundline@groundline
```

```bash
claude plugin marketplace add jukqaz/groundline
claude plugin install groundline@groundline
```

```bash
agy plugin install https://github.com/jukqaz/groundline
```

See `docs/provider-packaging.md` for local development, validation, and official
catalog submission boundaries.

GroundLine should not collect prompt text, raw transcripts, credentials, or
provider runtime state. Reports should keep mutation status, verification
evidence, and setup gaps explicit enough for both humans and LLM agents.
