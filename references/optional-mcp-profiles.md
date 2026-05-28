# Optional MCP Profiles

These profiles are LLM-readable setup guidance. They are not bundled MCP server
configuration.

GroundLine may recommend a profile, but the provider owns connection,
authorization, scoping, and execution.

## Profile: github-evidence

- status: optional
- use_when: GitHub issues, pull requests, CI, releases, tags, or review comments
  are part of the proof.
- pair_with: `reconcile-current-state`, `close-live-work`,
  `compare-release-delta`
- read_scope: repository metadata, pull request metadata, issue metadata,
  workflow runs, release metadata
- write_scope: none by default
- approval_required_for: comments, labels, branch changes, releases, permission
  changes
- never_store: tokens, private comments copied in full, raw review dumps
- output: commit SHA, run URL, release URL, issue or PR number, PASS/PARTIAL/FAIL

## Profile: documentation-lookup

- status: optional
- use_when: current provider, API, package, or runtime documentation is needed.
- pair_with: `research-agent-ecosystem`, `compare-agent-workflows`,
  `recommend-groundline-upgrades`
- read_scope: official docs, package docs, source-backed pages
- write_scope: none
- approval_required_for: none unless the provider performs account-level setup
- never_store: large copied pages, private docs, auth material
- output: source link, date checked, decision, confidence, follow-up

## Profile: ecosystem-research

- status: optional
- use_when: adjacent tools, skills, MCP servers, prompt packs, or agent
  workflows need comparison.
- pair_with: `agent-ecosystem-radar`, `research-agent-ecosystem`,
  `evaluate-agent-capability`
- read_scope: public repositories, docs, release notes, issue summaries
- write_scope: none
- approval_required_for: filing issues, starring repos, joining communities,
  installing tools
- never_store: unverified claims as facts, private account data, copied articles
- output: adopt/adapt/watch/reject with evidence

## Profile: private-docs

- status: optional
- use_when: internal runbooks, team operating notes, or private project docs are
  needed for a task.
- pair_with: `audit-agent-history`, `package-agent-task`, `align-agent-home`
- read_scope: smallest relevant folder, collection, or project
- write_scope: none by default
- approval_required_for: edits, comments, sharing, access changes
- never_store: full private docs, secrets, private transcripts, provider cache
  dumps
- output: short evidence summary, assumptions, approval needs, next verification

## Profile: private-code-search

- status: optional
- use_when: local search is insufficient for a large private codebase.
- pair_with: `reconcile-current-state`, `evaluate-groundline-pack`,
  `guard-side-effects`
- read_scope: indexed code metadata and selected snippets needed for the task
- write_scope: none by default
- approval_required_for: edits through the MCP server, branch changes, comments,
  repository setting changes
- never_store: broad source dumps, credentials, generated cache files
- output: file paths, symbol names, verified local commands, remaining gaps

## Selection Rule

Recommend at most one profile at a time unless the user asks for a broader
operating setup. If a skill or local command can answer the question, do not
recommend MCP.

