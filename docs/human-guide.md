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

## Before Publishing

Use `docs/public-release.md` and `docs/git-history-privacy.md`. Pay special
attention to git history: old commits can expose author names, emails, or
earlier file contents even after the current working tree is cleaned.

Do not make the existing repository public until the current tree and git
history are both acceptable for public disclosure.
