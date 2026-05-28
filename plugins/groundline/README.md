# GroundLine

GroundLine helps an AI coding agent slow down at the moments that usually go
wrong: resuming someone else's work, touching risky systems, claiming work is
complete too early, or letting a release grow without a stop point.

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

## Start Here

If you only want to try GroundLine, do this:

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

`validate_pack.py` should report `status=PASS`. The provider smoke command is
read-only: it should report `mutation_performed=false` and no real provider
home writes, but it may return `PARTIAL` when an existing provider install is
stale relative to the checked-out package. Use `--require-installed` when the
goal is post-install release proof rather than package/path validation. Read
top-level `next_actions` before installing or refreshing Codex, Claude Code, or
Antigravity.

If you want to install from the public repository:

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

## When To Use It

Use GroundLine when the next step needs evidence or a boundary:

| Situation | Ask the agent |
| --- | --- |
| A previous agent worked on this | "Recheck the current state before continuing." |
| A thread is too long | "Package this task so another agent can continue." |
| The task keeps expanding | "Hold the line and decide what ships now." |
| Tests passed but the runtime may still be wrong | "Close this with live evidence." |
| The action may change files, remotes, production, access, or secrets | "Classify side effects before doing anything." |
| You are near release | "Polish the release candidate and lock the release cut." |
| You are comparing agent tools or skills | "Evaluate this capability before adopting it." |

## What Changes And What Does Not

By default, GroundLine is skills-first:

- It adds skill instructions, references, docs, and validation scripts.
- It does not install hooks, rules, MCP servers, slash commands, provider-level
  agents, or background jobs.
- It does not read raw transcripts or copy provider runtime state into the
  repository.
- It can recommend optional MCP use when a task needs live GitHub, current docs,
  private docs, or private code search.

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

## Read Next

- New users: `docs/human-guide.md`
- Korean overview: `README.ko.md`
- Install and update: `docs/install.md`, `docs/update.md`
- Provider install details: `docs/provider-packaging.md`
- Provider activation proof: `docs/provider-activation-matrix.md`
- Workflow examples: `docs/examples.md`
- Workflow cookbook: `docs/workflow-cookbook.md`
- Artifact lifecycle: `docs/artifact-lifecycle.md`
- Skill overview: `docs/skill-portfolio.md`
- Skill graduation plan: `docs/skill-graduation-plan.md`
- Maturity and next work: `docs/maturity-assessment.md`
- LLM-facing contract guide: `docs/llm-guide.md`

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
- Provider activation matrix: `docs/provider-activation-matrix.md`
- Provider guardrails: `docs/provider-guardrails.md`
- Optional MCP recipes: `docs/mcp-recipes.md`
- Workflow cookbook: `docs/workflow-cookbook.md`
- Artifact lifecycle: `docs/artifact-lifecycle.md`
- Public release: `docs/public-release.md`
- Korean public release: `docs/ko/public-release.md`
- Privacy: `docs/privacy.md`
- Korean privacy: `docs/ko/privacy.md`
- Terms: `docs/terms.md`
- Korean terms: `docs/ko/terms.md`
- Git history privacy: `docs/git-history-privacy.md`
- Language policy: `docs/language-policy.md`
- Maturity assessment: `docs/maturity-assessment.md`
- Next work: `docs/next-work.md`
- Next version: `docs/next-version.md`
- Skill portfolio: `docs/skill-portfolio.md`
- Korean skill portfolio: `docs/ko/skill-portfolio.md`
- Skill graduation plan: `docs/skill-graduation-plan.md`
- Korean skill graduation plan: `docs/ko/skill-graduation-plan.md`
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

To inspect the release gate order without running the checks:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json
```

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_safety_eval.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_privacy_scan.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json
```

CI runs the deterministic offline subset from `.github/workflows/test.yml`.
Before a release, also run the full release checklist in
`docs/release-checklist.md`, including provider smoke and the real Linux Docker
scenario:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --json --keep-going --include-docker-execution
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json --require-installed
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
