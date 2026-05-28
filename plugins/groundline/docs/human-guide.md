# Human Guide

This guide is for a person installing, reviewing, or publishing GroundLine.

## What GroundLine Is

GroundLine is a small plugin package for Codex, Claude Code, and Antigravity.
It provides skills, references, and stdlib-only scripts for agent handoff,
current-state checks, side-effect boundaries, release evidence, and runtime
drift review.

It is not a config sync dump, background daemon, MCP bundle, or installer that
rewrites provider homes by default.

## First Run

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

Expected behavior:

- no provider home writes
- `mutation_performed=false`
- `real_home_touched=false`
- default home paths displayed with `~`

## Daily Use

Run doctor when you need to know the local runtime posture:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
```

Run radar when you want an LLM-ready packet for current docs and ecosystem
follow-up:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
```

Ask for `agent-ecosystem-radar` when you want GroundLine to research external
agent workflow tools, compare them, and recommend what to adopt, adapt, watch,
or reject.

Ask for `package-agent-task` when a request has grown across several turns and
you want the next LLM to start from a clear goal, constraints, non-goals,
verification, and handoff.

Ask for `hold-the-line` when the work starts attracting more ideas, tools,
research, or cleanup before the current goal has been closed.

Ask for `polish-release-candidate` when a locked release candidate needs final
docs, duplicate wording, privacy, identity, validation, and commit-plan cleanup.

Ask for `evaluate-groundline-pack` before public release or a larger version
bump when you want the repository, skills, docs, safety posture, and validation
gates reviewed together.

Ask for `stabilize-release-cut` when the package is good enough to stop adding
new capability and start proving a release candidate.

Ask for `compare-release-delta` after deploy when you want to compare the
deployed version with the previous version and capture runtime, install,
regression, and rollback evidence.

Ask for `evaluate-ai-usage-maturity` when you want a private, evidence-backed
review of how a person or team uses AI agents and what operating habits should
improve next.

Read `docs/skill-portfolio.md` when you want a maintainer-friendly overview of
the current skills. Use `references/skill-index.json` when an LLM or script
needs stable taxonomy fields.

## Before Publishing

Use `docs/public-release.md` and `docs/git-history-privacy.md`. Pay special
attention to git history: old commits can expose author names, emails, or
earlier file contents even after the current working tree is cleaned.

Do not make the existing repository public until the current tree and git
history are both acceptable for public disclosure.
