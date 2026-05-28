# Next Version Plan

Target: v0.3.1

The next version should reduce accepted partials from v0.3.0, not expand the
skill count first. GroundLine now has provider marketplace packaging for Codex,
Claude Code, and Antigravity. v0.3.0 proved provider invocation with sanitized
evidence, added an offline safety check, and kept provider-specific partials
visible instead of hiding them.

## Completed Foundation

These are already done and should not be re-opened unless validation fails:

- Codex marketplace metadata points to `plugins/groundline`.
- Claude Code marketplace metadata points to `plugins/groundline`.
- Antigravity can validate and import the package.
- `plugins/groundline` contains the installable payload.
- English and Korean provider packaging docs exist.
- Local validation, provider validation, and CI pass on `main`.

## Current Status: v0.3.0 Adoption Proof Implemented

The narrow v0.3.0 slice proves the sanitized evidence path and records provider
invocation results without raw transcript collection or provider-home dumps.

Completed:

- sanitized invocation proof format in `docs/provider-dogfood.md`
- one sanitized proof row each for Codex, Claude Code, and Antigravity
- one proof row for each core prompt family: handoff, release closeout, and
  expansion control
- `docs/dogfood.md` updated with PASS/PARTIAL evidence
- offline safety fixture and `scripts/groundline_safety_eval.py`
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

## Non-goals For v0.3.1

- new provider runtimes
- automatic real provider-home installation
- mandatory MCP setup
- broad hook enablement
- raw transcript analytics

## v0.3.1 Closeout

Before tagging v0.3.1, run package sync, source validation, packaged validation,
lint, unit tests, staged dogfood, and provider package validation. If the
Antigravity partial remains, record whether it is still an accepted defer or has
been reduced.
