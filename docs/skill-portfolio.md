# Skill Portfolio

This document is for people reviewing GroundLine. It mirrors the
LLM-readable catalog in `references/skill-index.json`, but uses a format that
is easier to scan during planning, release review, and cleanup.

## How To Read This

- Stage shows where the skill sits in the operating loop.
- Risk shows the highest boundary the skill may reason about.
- Status shows whether the skill is stable or still being shaped.
- Keep detailed taxonomy rules in `references/skill-lifecycle.md`.
- Keep exact machine-readable fields in `references/skill-index.json`.

## Skill Portfolio

| Skill | Stage | Risk | Status | Human summary |
| --- | --- | --- | --- | --- |
| `reconcile-current-state` | orient | read-only | active | Proves current branch, runtime, and handoff state before acting. |
| `audit-agent-history` | research | secret-sensitive | active | Inventories local agent histories and prepares redacted evidence packets without dumping raw transcripts. |
| `guard-side-effects` | decide | secret-sensitive | active | Classifies side effects and approval needs before risky actions. |
| `close-live-work` | verify | read-only | active | Closes work with runtime, endpoint, release, or user-flow evidence. |
| `align-agent-home` | maintain | provider-home-write | active | Separates shareable provider config from local runtime state. |
| `recover-worktree-branch` | orient | local-write | active | Proves branch and worktree state before recovery or cleanup. |
| `package-agent-task` | orient | secret-sensitive | experimental | Turns broad or resumed requests into concise LLM-ready task packets. |
| `agent-ecosystem-radar` | research | read-only | experimental | Runs the research, evaluation, comparison, and recommendation skill set. |
| `research-agent-ecosystem` | research | read-only | experimental | Collects source-backed external agent workflow candidates. |
| `compare-agent-workflows` | compare | read-only | experimental | Scores researched workflow candidates against GroundLine scope. |
| `recommend-groundline-upgrades` | decide | read-only | experimental | Turns research and comparison into adopt, adapt, watch, or reject decisions. |
| `evaluate-agent-capability` | verify | read-only | experimental | Scores one existing tool, skill, plugin, MCP server, hook, agent, or workflow pack before adoption. |
| `evaluate-ai-usage-maturity` | verify | secret-sensitive | experimental | Assesses AI workflow maturity from artifacts or a redacted Provider Evidence Packet. |
| `hold-the-line` | decide | read-only | experimental | Stops scope growth and chooses finish, budget, defer, watch, or reject before more work starts. |
| `polish-release-candidate` | verify | secret-sensitive | experimental | Runs final docs, duplicate, privacy, gate, and commit-plan cleanup before release judgment. |
| `stabilize-release-cut` | verify | local-write | experimental | Locks release scope, classifies remaining work, and proves release readiness. |
| `compare-release-delta` | verify | read-only | experimental | Compares deployed and previous versions with runtime, install, regression, and rollback evidence. |
| `evaluate-groundline-pack` | verify | read-only | active | Reviews repository readiness, skill completeness, safety, and release fitness. |
| `curate-groundline-skills` | maintain | read-only | experimental | Maintains the skill portfolio taxonomy and lifecycle decisions. |

## Lifecycle Notes

- Active skills are part of the supported GroundLine surface.
- Experimental skills are usable, but their names and scope can still change
  before a stable release.
- A skill should move from experimental to active only after examples, index
  metadata, output contracts, and tests agree.
- A merged or deprecated skill should leave a note explaining where its behavior
  moved.
- Human-facing docs should stay readable without requiring provider internals.

## Routing Notes

- Use `research-agent-ecosystem` only for source gathering.
- Use `evaluate-agent-capability` for one candidate.
- Use `compare-agent-workflows` for two or more researched candidates.
- Use `recommend-groundline-upgrades` after findings exist.
- Use `agent-ecosystem-radar` when the user asks for the whole research,
  comparison, and recommendation loop.
- Use `audit-agent-history -> evaluate-ai-usage-maturity` when provider
  histories are evidence for an AI usage assessment.
