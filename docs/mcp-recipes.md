# MCP Recipes

GroundLine core does not require MCP. The default package stays skills-only so
it remains portable across Codex, Claude Code, and Antigravity.

Use MCP only when the task needs live or private tool access that a skill cannot
provide by itself.

## Decision Rule

Use skills alone when the task is about:

- current-state proof
- handoff
- release cuts
- side-effect boundaries
- dogfood evidence
- AI usage assessment
- research, comparison, and upgrade recommendations from provided sources

Add an MCP server when the task needs:

- live GitHub issues, pull requests, CI, release, or branch state
- current external documentation lookup
- source-backed ecosystem discovery
- private internal docs, runbooks, or code search
- structured access to a private service with reviewed permissions

Do not add MCP just to make a workflow feel more automated. If a normal CLI,
repo file, or skill output is enough, keep the run skills-only.

## Optional Profiles

| Profile | Use when | Typical skill pairing |
| --- | --- | --- |
| GitHub evidence | PRs, issues, CI runs, release state, tags, or review comments are part of the proof. | `reconcile-current-state`, `close-live-work`, `compare-release-delta` |
| Documentation lookup | Current library, provider, API, or runtime docs are needed. | `research-agent-ecosystem`, `recommend-groundline-upgrades` |
| Ecosystem research | New agent tools, skills, MCP servers, or workflow packs need source-backed comparison. | `agent-ecosystem-radar`, `compare-agent-workflows` |
| Private docs | Internal runbooks or project-specific operating notes are needed. | `audit-agent-history`, `package-agent-task`, `align-agent-home` |
| Private code search | A large private codebase needs indexed lookup beyond local `rg`. | `reconcile-current-state`, `evaluate-groundline-pack` |

The LLM-readable profile details live in
`references/optional-mcp-profiles.md`.

## Provider Setup Boundary

GroundLine should not write MCP configuration by default.

- Codex owns `codex mcp` setup and authorization.
- Claude Code owns `claude mcp` setup and authorization.
- Antigravity owns its provider-native MCP or private tool setup.

GroundLine can recommend a profile, name the evidence needed, and explain the
side-effect boundary. The provider should connect, authorize, scope, and run the
MCP server.

## Private MCP Checklist

Before using a private MCP server:

- Confirm the server is needed for this task.
- Confirm the server scope is narrower than the whole account when possible.
- Confirm the agent may read the requested data.
- Confirm whether writes are disabled, gated, or explicitly approved.
- Confirm output must not include tokens, raw private docs, full provider home
  dumps, or transcript archives.
- Record only a short evidence summary in repository docs.

## Recipes

### GitHub Release Evidence

Use when a release claim depends on GitHub state.

1. Use `reconcile-current-state` to identify branch, tag, commit, and expected
   release.
2. Use a GitHub MCP server or GitHub CLI to inspect CI, release, and tag state.
3. Use `close-live-work` or `compare-release-delta` to report PASS, PARTIAL, or
   FAIL.
4. Store the run URL, commit SHA, release URL, and gap list. Do not store auth
   material.

### Current Documentation Lookup

Use when a provider or library behavior may have changed.

1. Use `research-agent-ecosystem` to define the question and source list.
2. Use a docs MCP server only for current source retrieval.
3. Use `compare-agent-workflows` or `recommend-groundline-upgrades` to classify
   findings as adopt, adapt, watch, or reject.
4. Store source links and decisions, not large copied pages.

### Private Runbook Lookup

Use when the answer depends on team-specific deployment or operating notes.

1. Use `guard-side-effects` to decide whether the task is read-only or mutating.
2. Use private MCP read access for the smallest relevant runbook set.
3. Use `package-agent-task` to create a continuation packet with verified facts,
   assumptions, non-goals, approval needs, and verification.
4. Do not copy private runbooks into GroundLine source.

### Private Code Search

Use when local search is insufficient for a large private codebase.

1. Use `reconcile-current-state` to prove checkout, branch, and scope.
2. Use private code search MCP for navigation only.
3. Apply edits through the local workspace, not through the MCP server, unless
   the user explicitly approves a write-capable flow.
4. Verify with the repo's local gates.

