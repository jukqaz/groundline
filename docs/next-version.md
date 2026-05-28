# Next Version Plan

Target: v0.3.3

v0.3.2 tightens first-use skill routing, separates staged dogfood checks from
real provider invocation proof, and links provider-history inventory to AI usage
maturity assessment. The next version should reduce the remaining provider
invocation partial and add only the smallest representative workflows.

## Completed Foundation

These are already done and should not be re-opened unless validation fails:

- Codex marketplace metadata points to `plugins/groundline`.
- Claude Code marketplace metadata points to `plugins/groundline`.
- Antigravity can validate and import the package.
- `plugins/groundline` contains the installable payload.
- English and Korean provider packaging docs exist.
- Local validation, provider validation, and CI pass on `main`.

## Current Status: v0.3.2 Patch Ready

The narrow v0.3.2 slice keeps the sanitized evidence path and improves prompt
routing clarity without adding new skills or provider setup.

Completed:

- sanitized invocation proof format in `docs/provider-dogfood.md`
- staged contract harness language separated from real provider invocation proof
- one sanitized proof row each for Codex, Claude Code, and Antigravity
- one proof row for each core prompt family: handoff, release closeout, and
  expansion control
- `docs/dogfood.md` updated with PASS/PARTIAL evidence
- offline safety fixture and `scripts/groundline_safety_eval.py`
- optional provider guardrail and MCP recipe docs
- local Superpowers companion dogfood
- explicit routing for research, evaluation, comparison, recommendation, AI
  usage assessment, and release triage skills
- no new skills added

## 1. Provider Invocation Dogfood

The first repeatable evidence path is in place. It should be improved by
reducing accepted partials, not by adding more skills.

Status:

- Codex: PASS for the handoff family.
- Claude Code: PASS after the proof prompt allowed read-only skill doc
  inspection and returned `stabilize-release-cut` with `GroundLine Release Cut`.
- Antigravity: PARTIAL for expansion control because constrained print mode
  timed out before returning a proof.
- No raw transcript archives or auth material were added to the repository.

Ship gate:

- all staged dogfood checks still pass
- each provider has at least one sanitized invocation proof or an accepted defer
- `docs/provider-dogfood.md` explains how to repeat the proof without writing
  raw transcripts into the repository
- accepted partials are named in release notes or reduced before tagging

## 2. Safety Evaluation Harness

Add a small offline harness for unsafe agent behavior patterns. The first
version is implemented with four synthetic cases.

Current coverage:

- secret-like output pressure
- destructive command pressure
- false completion claims
- unsafe provider-home writes
- JSON report with PASS/FAIL and `mutation_performed=false`

Remaining:

- add a prompt-injection fixture only if it tests a distinct boundary
- keep all fixtures synthetic and deterministic
- keep failure output focused on which boundary failed and how to fix it

## 3. Workflow Cookbook

Make GroundLine easier to understand during first use.

Deliverables:

- five complete workflows: handoff recovery, release cut, ecosystem radar, AI
  usage maturity review, and side-effect guarding
- each workflow maps prompt -> skill -> output contract -> verification
- examples stay short enough for an active agent session

Ship gate:

- people can pick a workflow without reading the full skill index first
- LLM agents can cite the same workflow without extra interpretation
- each cookbook entry names the stop condition so the task does not expand

## 4. Artifact Lifecycle

Clarify how GroundLine outputs move between skills.

Deliverables:

- templates for research packets, comparison reports, upgrade decisions,
  release cuts, and release deltas
- lifecycle map from research to release review
- explicit rejection rule for provider-native feature duplication

Ship gate:

- each template points to one skill and one output contract
- duplicate or overlapping templates are merged before release

## Later, Not Now

- official catalog submission polish
- screenshots or richer marketplace media
- deeper ecosystem comparison refresh
- optional MCP setup recipes
- optional hooks after a specific reviewed use case exists

## Non-goals For The Next Patch

- new provider runtimes
- automatic real provider-home installation
- mandatory MCP setup
- broad hook enablement
- raw transcript analytics

## Next Patch Closeout

Before tagging the next patch, run package sync, source validation, packaged
validation, lint, unit tests, staged dogfood, provider package validation, and
at least one remote install proof when the package is meant to be installed
from GitHub.
