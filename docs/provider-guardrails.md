# Provider Guardrails

GroundLine defaults to skills-only. It does not install provider hooks, rules,
MCP servers, commands, or provider-level agents by default.

This is intentional. Codex, Claude Code, and Antigravity already own their
runtime permissions, sandboxing, approvals, tool review, plugin loading, and
private tool connections.

## What GroundLine Ships

- skills
- output contracts
- references
- docs
- offline scripts
- staged package validation

## What GroundLine Does Not Enable By Default

- hooks
- rules
- MCP servers
- slash commands
- provider-level agents
- transcript or prompt logging
- automatic provider-home writes
- background network work

## Why

Hooks and rules can affect every session or repository. They are useful only
after a specific provider, repository, and failure mode has been reviewed.

GroundLine should not turn a portable skill pack into a global runtime policy
bundle. The safer split is:

- GroundLine defines the operating contract.
- The provider enforces permissions and runs tools.
- The user approves writes, external service changes, and broader automation.

## Skills-Only Is Enough When

- the agent needs to slow down and prove current state
- the task needs a handoff packet
- the task needs release scope control
- the task needs side-effect classification
- the task needs dogfood or release evidence
- the task needs an AI usage or tool capability review

## Add MCP When

- the task needs live GitHub or CI state
- the task needs current external docs
- the task needs private internal docs
- the task needs private code search
- the task needs structured data from a reviewed private service

Use `docs/mcp-recipes.md` and `references/optional-mcp-profiles.md` before
adding any MCP setup.

## Consider Hooks Or Rules Only When

- the same unsafe action repeats across real sessions
- provider-native permission prompts are not enough
- the hook or rule is narrow, deterministic, and easy to explain
- the behavior can be disabled per repo or per provider
- the user explicitly chooses to enable it

Good candidates:

- destructive command confirmation
- provider-home write warning
- release completion reminder when tests or dogfood evidence are missing

Bad candidates:

- prompt logging
- transcript analytics
- broad auto-approval
- long network work
- secret output capture
- automatic package publish

## Review Checklist

Before adding a hook, rule, MCP server, command, or provider-level agent:

- What failure does it prevent?
- Which provider owns the execution mechanism?
- Does it mutate local files, remotes, provider homes, or external services?
- Can the user disable it without uninstalling GroundLine?
- Is the same behavior better expressed as a skill, doc, or optional MCP
  profile?
- What is the smallest verification gate?

If those answers are unclear, keep the package skills-only.

