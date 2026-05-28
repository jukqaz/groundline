# LLM Guide

This guide is for LLM agents reading GroundLine as operating context.

## Role

Use GroundLine as a control plane for agent work. It helps decide what evidence
is needed before continuing, mutating, releasing, or handing off work.

Do not treat GroundLine as permission to mutate user files, provider homes,
remotes, production systems, billing, access, or secrets.

Do not reimplement provider-native features. If the runtime already provides
goal mode, subagents, hooks, plugin installation, browser control, app context,
or MCP setup, use GroundLine to define the task packet, safety boundary, and
verification evidence, then let the provider-owned feature execute it.

## Primary Flow

1. Read the user's latest request.
2. Check current repository, branch, runtime, and dirty state.
3. Select the smallest relevant GroundLine skill.
4. Keep mutation boundaries explicit.
5. Run the smallest credible verification.
6. Report PASS, PARTIAL, or FAIL with concrete evidence.

## Skill Selection

- `reconcile-current-state`: stale handoff, previous agent claim, current worktree proof
- `audit-agent-history`: derive reusable improvements from agent history
- `guard-side-effects`: any action touching files, remotes, money, access, secrets, or production
- `close-live-work`: CI or local tests passed but live runtime proof is still needed
- `align-agent-home`: provider home, config, hooks, rules, skill, or plugin boundary review
- `recover-worktree-branch`: missing worktree, detached branch, cleanup, or recovery
- `package-agent-task`: convert broad, resumed, or high-context requests into a concise task packet
- `agent-ecosystem-radar`: research, compare, and recommend external workflow upgrades in one pass
- `research-agent-ecosystem`: gather source-backed ecosystem candidates
- `evaluate-agent-capability`: score an existing tool, skill, plugin, MCP server, hook, agent, or workflow pack before adoption
- `evaluate-ai-usage-maturity`: assess a person or team's AI workflow maturity from artifacts without exposing raw transcripts
- `hold-the-line`: stop scope growth when new ideas, more research, or extra tools appear before current work is closed
- `polish-release-candidate`: run final docs, duplicate, privacy, gate, and commit-plan cleanup before release judgment
- `stabilize-release-cut`: lock scope, classify remaining work, run release gates, collect dogfood evidence, and make a ship decision
- `compare-release-delta`: compare a deployed release with the previous version and produce a post-deploy checklist
- `compare-agent-workflows`: score candidates against GroundLine scope and risk
- `recommend-groundline-upgrades`: convert findings into `adopt`, `adapt`, `watch`, or `reject` tasks
- `evaluate-groundline-pack`: review this package for skill completeness, trigger clarity, tests, safety, and release fitness
- `curate-groundline-skills`: maintain the skill portfolio and decide whether to create, adapt, merge, split, deprecate, or reject capabilities

Use `references/skill-index.json` when a structured skill catalog is more
useful than prose. Use `docs/skill-portfolio.md` when producing a maintainer
summary for people.

## Safety Rules

- Do not print secret values.
- Do not copy provider auth files, sessions, logs, caches, shell snapshots, or
  local databases into source control.
- Do not simulate provider-native tools inside GroundLine when a setup
  recommendation is enough.
- Treat `mutation_performed=false` as evidence only for that command, not for
  the whole task.
- Ask for explicit approval before external mutation or destructive git work.
- Use `--home` with a temporary directory when testing install plans.

## Output Shape

Prefer concise evidence:

```text
status: PASS|PARTIAL|FAIL
scope: files, runtime, branch, or provider checked
evidence: command output summary or file path
mutation_performed: true|false
next_action: only if needed
```

For public or user-shared output, redact default home paths as `~` and avoid
long transcript excerpts.

Before recommending a repository visibility change, check
`docs/git-history-privacy.md` and separate current-tree findings from git
history findings.

## Provider Packaging

When asked to prepare GroundLine for provider-native installation, keep the
canonical source tree and install package aligned:

- Codex marketplace index: `.agents/plugins/marketplace.json`
- Claude Code marketplace index: `.claude-plugin/marketplace.json`
- Packaged plugin payload: `plugins/groundline`
- Packaging guide: `docs/provider-packaging.md`

Run `PYTHONDONTWRITEBYTECODE=1 python3 scripts/sync_provider_package.py --json`
after changing manifests, skills, docs, references, scripts, or assets. Then
validate the package with the provider checks listed in `docs/provider-packaging.md`.
