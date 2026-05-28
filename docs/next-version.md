# Next Version Plan

Target: v0.3.3

v0.3.2 tightens first-use skill routing, separates staged dogfood checks from
real provider invocation proof, and links provider-history inventory to AI usage
maturity assessment. The next version focuses on install posture, version drift
control, and compact proof workflows instead of expanding the skill surface.

## Completed Foundation

These are already done and should not be re-opened unless validation fails:

- Codex marketplace metadata points to `plugins/groundline`.
- Claude Code marketplace metadata points to `plugins/groundline`.
- Antigravity can validate and import the package.
- `plugins/groundline` contains the installable payload.
- English and Korean provider packaging docs exist.
- The v0.3.2 baseline passed local validation, provider validation, and CI on
  `main`; the current patch draft still needs the closeout gates before tag.

## Current Status: v0.3.3 Patch Draft

The narrow v0.3.2 slice keeps the sanitized evidence path and improves prompt
routing clarity without adding new skills or mandatory provider setup. Local
installation through the GitHub guide succeeded for Codex, Claude Code, and
Antigravity. The current v0.3.3 patch draft starts by closing the exposed
install posture and version drift gap.

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
- `docs/maturity-assessment.md` records the 85/100 public beta assessment and
  the next bounded work items
- version-aware provider smoke reports source version, installed version,
  payload presence, skill count drift, and `install_doctor_status`
- validation compares provider manifest versions against canonical
  `plugin.json`
- `docs/provider-activation-matrix.md` defines the five live prompt families
  and separates real proof rows from staged contract coverage
- staged provider dogfood now covers six scenario contracts, including
  side-effect guard, ecosystem evaluation, and AI usage maturity
- `docs/skill-graduation-plan.md` classifies all 12 experimental skills as
  graduate, keep experimental, merge, or defer
- `references/skill-index.json` records machine-readable graduation decisions
  and rationales for experimental skills
- `docs/workflow-cookbook.md` maps five representative prompts to selected
  skills, output contracts, verification evidence, and stop conditions
- `docs/artifact-lifecycle.md` maps research, comparison, recommendation,
  implementation, dogfood, release, and post-release review artifacts to
  existing skills and output contracts
- `scripts/groundline_release_gate.py` prints or executes the local release gate
  sequence and excludes approval-required publishing commands
- no new skills added

## 1. Install Posture And Version Drift

The next patch should make provider install state inspectable without relying
on manual list output interpretation.

Deliverables:

- Version-aware install doctor for Codex, Claude Code, and Antigravity:
  implemented in `scripts/groundline_provider_smoke.py`.
- Fake-home tests for installed version, source ref drift, stale cache, missing
  package payload, and skill count mismatch: added to script contract tests.
- Canonical manifest version comparison instead of hard-coded patch versions:
  implemented in `scripts/validate_pack.py`.
- Install and provider packaging docs updated with the new confirmation command:
  implemented for English and Korean human docs.

Ship gate:

- install doctor reports `PASS`, `PARTIAL`, or `FAIL`
- no provider auth, session, log, or raw home dump is printed
- provider package sync and validation still pass
- local provider install can be confirmed after a GitHub install

## 2. Provider Invocation Dogfood

The first repeatable evidence path is in place. It should be improved by
reducing accepted partials, not by adding more skills.

Status:

- Codex: PASS for the handoff family.
- Claude Code: PASS after the proof prompt allowed read-only skill doc
  inspection and returned `stabilize-release-cut` with `GroundLine Release Cut`.
- Antigravity: PARTIAL for expansion control because constrained print mode
  timed out before returning a proof.
- Staged contract coverage now covers handoff, side-effect guard, release cut,
  ecosystem evaluation, AI usage maturity, and completion proof across all three
  providers.
- No raw transcript archives or auth material were added to the repository.

Ship gate:

- all staged dogfood checks still pass
- each provider has at least one sanitized invocation proof or an accepted defer
- `docs/provider-dogfood.md` explains how to repeat the proof without writing
  raw transcripts into the repository
- accepted partials are named in release notes or reduced before tagging

## 3. Skill Graduation

The package has 7 active skills and 12 experimental skills. That is acceptable
for v0.3, but too broad for 1.0.

Deliverables:

- classify all experimental skills as graduate, keep experimental, merge, or
  defer: implemented in `docs/skill-graduation-plan.md`
- graduate only skills with docs, examples, output contracts, tests, and
  dogfood evidence: enforced as the promotion rule, but lifecycle values are
  not promoted in this patch
- keep watch items in `docs/next-work.md`: implemented

Status:

- `package-agent-task` and `stabilize-release-cut` are graduate candidates.
- `agent-ecosystem-radar` is a merge candidate.
- `compare-release-delta` is deferred until post-release comparison evidence
  exists.
- The remaining experimental skills stay experimental with explicit reasons.

## 4. Workflow Cookbook

Make GroundLine easier to understand during first use after install posture is
solid.

Status: implemented as a compact cookbook in the current patch draft. Expand
only if real provider proof shows a workflow remains unclear.

Deliverables:

- five complete workflows: handoff recovery, release cut, ecosystem radar, AI
  usage maturity review, and side-effect guarding: implemented
- each workflow maps prompt -> skill -> output contract -> verification
- examples stay short enough for an active agent session

Ship gate:

- people can pick a workflow without reading the full skill index first
- LLM agents can cite the same workflow without extra interpretation
- each cookbook entry names the stop condition so the task does not expand

## 5. Artifact Lifecycle

Clarify how GroundLine outputs move between skills.

Status: implemented as a compact lifecycle map in the current patch draft.

Deliverables:

- templates for research packets, comparison reports, upgrade decisions,
  release cuts, and release deltas: implemented
- lifecycle map from research to release review: implemented
- explicit rejection rule for provider-native feature duplication: implemented

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
validation, lint, unit tests, offline doctor, offline radar, safety eval,
provider smoke, staged dogfood, macOS local scenario, Linux Docker dry-run,
Linux Docker execution, and at least one remote install proof when the package
is meant to be installed from GitHub.
