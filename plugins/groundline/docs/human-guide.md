# Human Guide

This guide is for a person installing, reviewing, or publishing GroundLine.

Use this guide when the README told you what GroundLine is, but you still need
to know how to operate it in a real agent session.

## What GroundLine Is

GroundLine is a small plugin package for Codex, Claude Code, and Antigravity.
It provides skills, references, and stdlib-only scripts for agent handoff,
current-state checks, side-effect boundaries, release evidence, and runtime
drift review.

It is not a config sync dump, background daemon, MCP bundle, or installer that
rewrites provider homes by default.

## Mental Model

GroundLine is an operating layer, not the worker.

- Provider runtimes run tools, enforce permissions, and own their plugin or MCP
  mechanics.
- Superpowers or the provider's own workflow features can handle planning,
  TDD, debugging, review, and delegation.
- GroundLine asks the agent to prove state, name side effects, choose the next
  skill, preserve handoff context, and stop expanding work when a release is
  good enough.

The normal flow is:

```text
orient -> bound -> act -> prove -> polish -> cut -> compare
```

You do not need every step for every task. Use the smallest step that answers
the current risk.

For complete prompt-to-evidence examples, read `docs/workflow-cookbook.md`.

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

If this fails, do not install the plugin yet. Fix the local package first or
open an issue with the failing command and the sanitized output.

## Choose The Right Request

| If you are thinking | Ask for this | Why |
| --- | --- | --- |
| "Another agent touched this." | `reconcile-current-state` | Prevents stale handoff assumptions. |
| "This task is too large for the current thread." | `package-agent-task` | Produces a compact task packet for the next LLM. |
| "This might deploy, push, delete, spend, or expose something." | `guard-side-effects` | Separates read-only checks from actual mutation. |
| "Tests passed, but I am not sure users see it." | `close-live-work` | Looks for runtime, endpoint, release, queue, browser, or user-flow proof. |
| "I keep adding ideas." | `hold-the-line` | Forces finish, defer, watch, or reject. |
| "We are close to release." | `polish-release-candidate` | Checks docs, privacy, identity, gates, and commit shape. |
| "Stop expanding and decide if it ships." | `stabilize-release-cut` | Locks scope and requires release evidence. |
| "The release is out." | `compare-release-delta` | Compares current release with the previous one. |
| "Should we adopt this tool, skill, hook, or MCP server?" | `evaluate-agent-capability` | Scores fit, risk, setup cost, and maintenance signal. |
| "How good is my AI workflow?" | `evaluate-ai-usage-maturity` | Turns artifacts into a private improvement plan. |

## Copyable Prompts

```text
Recheck the current branch, dirty files, recent commits, and runtime evidence before continuing this task.
```

```text
This conversation is too long. Package the current goal, constraints, non-goals, verification, and handoff for another agent.
```

```text
Classify side effects before doing anything. Tell me what is read-only, what mutates local files, what mutates remotes or provider homes, and what needs approval.
```

```text
Tests passed. Check whether this work is actually complete with runtime, install, release, or user-flow evidence.
```

```text
Stop expanding this release. Classify must fix, defer, watch, and reject, then make a ship or hold decision.
```

## Reading Results

GroundLine answers should make uncertainty explicit.

- `PASS`: the requested boundary or evidence was checked and met.
- `PARTIAL`: useful evidence exists, but at least one important gap remains.
- `FAIL`: the evidence contradicts the desired state or a required gate failed.

Good GroundLine output also says:

- what was checked
- what was not checked
- whether anything mutated
- whether secret values were printed
- what the next safe action is

## Daily Commands

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

## Install Into A Provider

Install only after local validation passes.

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
claude plugin list
```

Antigravity:

```bash
agy plugin install https://github.com/jukqaz/groundline
agy plugin list
```

Provider install commands write to provider-owned plugin state. They are not
part of the read-only smoke checks.

## Before Publishing

Use `docs/public-release.md` and `docs/git-history-privacy.md`. Pay special
attention to git history: old commits can expose author names, emails, or
earlier file contents even after the current working tree is cleaned.

Do not make the existing repository public until the current tree and git
history are both acceptable for public disclosure.
