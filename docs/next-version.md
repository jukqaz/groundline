# Next Version Plan

Target: v0.3.0

The next version should improve adoption confidence, not expand the skill count
first. Work in this order and ship only when the evidence is stronger than the
current v0.2.x release surface.

## 1. Provider Invocation Dogfood

Build a repeatable evidence path that proves real provider sessions can discover
and use GroundLine skills.

Deliverables:

- sanitized provider invocation notes for Codex, Claude Code, and Antigravity
- one prompt per provider for handoff, release closeout, and expansion control
- PASS/PARTIAL/FAIL evidence in `docs/dogfood.md`
- no raw transcript archives or auth material in the repository

Ship gate:

- all staged dogfood checks still pass
- each provider has at least one sanitized invocation proof or an accepted defer

## 2. Safety Evaluation Harness

Add a small offline harness for unsafe agent behavior patterns.

Deliverables:

- synthetic fixtures for secret leakage, destructive command pressure, prompt
  injection, false completion claims, and unsafe provider-home writes
- JSON report with PASS/PARTIAL/FAIL
- deterministic CI gate after local soak

Ship gate:

- no fixture contains real secrets or user data
- failure output explains which boundary failed and how to fix it

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

## Non-goals For v0.3.0

- new provider runtimes
- automatic real provider-home installation
- mandatory MCP setup
- broad hook enablement
- raw transcript analytics

## First Implementation Slice

Start with provider invocation dogfood because it directly reduces the biggest
confidence gap from v0.2.x. Do not start the safety harness until the invocation
proof format is stable enough to preserve privacy boundaries.
