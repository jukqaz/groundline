# Next Version Plan

Target: v0.3.5

v0.3.3 closed install posture, version drift control, compact proof workflows,
and release gates without expanding the skill surface. The current local
candidate carries the v0.3.4 proof-quality work forward and adds remote install
and update proof as the next release boundary.

## Completed Foundation

These are already done and should not be re-opened unless validation fails:

- Codex marketplace metadata points to `plugins/groundline`.
- Claude Code marketplace metadata points to `plugins/groundline`.
- Antigravity can validate and import the package.
- `plugins/groundline` contains the installable payload.
- English and Korean provider packaging docs exist.
- v0.3.3 is tagged and published from `main` with source/package validation,
  provider-native validation, staged dogfood, staged provider smoke, scenario
  evidence, explicit release-version preflight coverage, and remote CI proof.
- Real provider smoke is the install confirmation check for this phase. It
  reports `PASS` only when installed provider targets match the current source
  payload, and `PARTIAL` with refresh actions when post-release source changes
  make same-version installed targets stale. The v0.3.3 tag and GitHub Release
  are no longer pending.

## Current Status: v0.3.3 Published Baseline

The narrow v0.3.3 release keeps the sanitized evidence path and prompt routing
clarity from v0.3.2 while closing the exposed install posture and version drift
gap. Local installation through the GitHub guide succeeded for Codex, Claude
Code, and Antigravity.

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
  payload presence, skill count drift, same-version content drift, and
  `install_doctor_status`
- staged provider smoke proves a fake refreshed install with
  `--stage-package --require-installed`
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
- release gate accepts `--release-version` so the actual release cut fails when
  source or packaged manifests still point at the previous public version or the
  requested version is not plain `X.Y.Z` semver
- staged provider smoke passes; real provider smoke is the refresh gate for
  installed targets after same-version source content changes
- the explicit `--release-version 0.3.3` preflight is the manifest and
  package-sync guard for this released patch
- no new skills added

## v0.3.5 Release Cut

Current conclusion: v0.3.5 is a remote install and update proof patch. It
should prove the public installable package can be freshly installed, stale
installed targets can be detected after a version bump, and a refreshed install
returns to `PASS` without mutating real provider homes by default.

Scope lock:

- Keep v0.3.4 local proof-quality evidence: provider target refresh, release
  delta docs, and local release gate coverage.
- Add `groundline_remote_install_probe.py` for fake-home fresh install,
  previous-version detection, and post-update refresh across Codex, Claude
  Code, and Antigravity.
- Add the remote install/update proof to release gate, install docs, update
  docs, release checklist, and package sync.
- Keep live provider CLI activation proof separate because it can send current
  worktree context to external provider runtimes and still needs explicit
  approval.

Change budget:

- Script, tests, release gate wiring, docs, changelog, and package sync.
- Validation-only fixes when an existing gate cannot express install/update
  confidence.
- No new skills, provider runtimes, hooks, MCP setup, provider-level agents, or
  default real provider-home mutation.

Must fix before ship:

- `groundline_remote_install_probe.py --json` returns `status=PASS` and proves
  `fresh_install`, `stale_update_detection`, and `post_update_refresh`.
- `groundline_release_gate.py --release-version 0.3.5` includes the new
  remote install/update proof gate.
- Source and packaged manifests are `0.3.5` after package sync.
- Provider smoke still reports no unexpected `FAIL`; any real provider target
  drift is either refreshed or recorded as an accepted partial.
- Install and update docs name the remote install/update proof before claiming
  public update confidence.

Current harness result:

- Source and packaged manifests are `0.3.5`.
- `groundline_remote_install_probe.py --json` returns `status=PASS` and proves
  fresh install, stale update detection, and post-update refresh in a fake home.
- Local Codex, Claude Code, and Antigravity provider targets were refreshed to
  the `0.3.5` package and provider smoke returns `PASS`.
- `groundline_release_gate.py --json --keep-going --include-docker-execution
  --release-version 0.3.5` returns `status=PASS`.

Ship decision: `continue` until live provider activation proof and
published-ref proof are collected or explicitly accepted as partial.

## v0.3.4 Local Proof-Quality Work

Current conclusion: v0.3.4 is a proof-quality patch, not a capability
expansion. It should prove that the published v0.3.3 package can be refreshed,
selected by real provider sessions, and compared against the previous release
without changing the skill surface.

Scope lock:

- Provider install refresh and confirmation for Codex and Claude Code after
  same-version content drift is detected. Current local direct targets now match
  `0.3.4` and provider smoke reports `PASS`.
- Live provider activation proof for all five prompt families in
  `docs/provider-activation-matrix.md`.
- A release delta review for the published v0.3.3 baseline before tagging
  v0.3.4.
- Release gate, privacy scan, staged provider smoke, staged dogfood, provider
  smoke, scenario checks, remote CI, and GitHub Release proof for v0.3.4.

Change budget:

- Documentation, sanitized evidence rows, changelog, package sync, and
  validation-only fixes.
- Script or test changes only when an existing gate is wrong, stale, or unable
  to express the proof above.
- No default provider-home mutation inside repository scripts.

Must fix before ship:

- `groundline_provider_smoke.py --json --require-installed` has no unexpected
  `FAIL` state. Same-version `content_fingerprint_mismatch` must be resolved by
  refreshing the installed provider target or recorded as an explicit accepted
  partial with `next_actions`.
- The activation matrix has no `PENDING` rows for `side-effect-guard`,
  `ecosystem-evaluation`, or `ai-usage-maturity`; each row is `PASS` or an
  explicit `PARTIAL` with sanitized evidence.
- `docs/dogfood.md` records only sanitized invocation proof fields; no raw
  transcripts, auth material, provider cache dumps, or full home paths.
- `compare-release-delta` is used to produce a short release delta judgment for
  v0.3.3 as the previous published baseline, or the absence of comparable
  install evidence is named as a partial.
- Source and packaged manifests are bumped together to `0.3.4`, package sync
  runs, and the release gate uses `--release-version 0.3.4`.

Defer:

- Promoting `package-agent-task`, `stabilize-release-cut`, or
  `evaluate-agent-capability` from experimental to active.
- Official catalog submission polish, screenshots, richer marketplace media,
  broader ecosystem comparison refresh, optional MCP setup, optional hooks,
  provider-level agents, and automatic provider-home installation.

Reject:

- New skills, new provider runtimes, default hooks or MCP servers, raw
  transcript analytics, and lifecycle promotion without fresh activation proof.

Current harness result:

- Source and packaged manifests are `0.3.4`.
- Local Codex and Claude Code direct provider targets were refreshed from the
  current packaged payload.
- `groundline_provider_smoke.py --json --require-installed` reports
  `status=PASS`, `install_doctor_status=PASS`, and `next_actions=[]`.
- `groundline_release_gate.py --json --keep-going --include-docker-execution
  --release-version 0.3.4` reports `status=PASS`.

Release delta judgment:

- previous version: `v0.3.3`
- candidate version: local `0.3.4`
- expected changes: manifest bump, post-release planning refresh, local provider
  target refresh evidence, and proof-quality release cut docs
- unexpected changes: none found in the source diff; package copy changes mirror
  the source payload
- runtime evidence: full local release gate including Linux Docker execution
  reports `PASS`
- install evidence: provider smoke with `--require-installed` reports `PASS`
- decision: monitor until live provider activation proof rows are collected or
  explicitly accepted as partial

Ship decision: `continue` until activation matrix rows are complete or explicitly
accepted as partial. After that, cut v0.3.4 as a narrow release and keep
stable-core promotion for a later patch.

## 1. Install Posture And Version Drift

The next patch should make provider install state inspectable without relying
on manual list output interpretation.

Deliverables:

- Version-aware install doctor for Codex, Claude Code, and Antigravity:
  implemented in `scripts/groundline_provider_smoke.py`.
- Fake-home tests for installed version, source ref drift, stale cache, missing
  package payload, and skill count mismatch: added to script contract tests.
- Staged provider smoke for fake refreshed installs: implemented in
  `scripts/groundline_provider_smoke.py`.
- Canonical manifest version comparison instead of hard-coded patch versions:
  implemented in `scripts/validate_pack.py`.
- Install and provider packaging docs updated with the new confirmation command:
  implemented for English and Korean human docs.

Ship gate:

- install doctor reports `PASS`, `PARTIAL`, or `FAIL`
- no provider auth, session, log, or raw home dump is printed
- provider package sync and validation still pass
- staged provider smoke passes before real provider install refresh is claimed
- local provider install can be confirmed after a GitHub install
- release cut can pass an explicit `--release-version` and prove all source and
  packaged manifests match it before tag creation

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

Status: implemented as a compact cookbook in v0.3.3. Expand only if real
provider proof shows a workflow remains unclear.

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

Status: implemented as a compact lifecycle map in v0.3.3.

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
validation, lint, provider-native validation, unit tests, offline doctor,
offline radar, safety eval, privacy scan, provider smoke, staged dogfood,
macOS local scenario, Linux Docker dry-run, Linux Docker execution, and at least
one remote install proof when the package is meant to be installed from GitHub.
If provider smoke reports `content_fingerprint_mismatch` against same-version
local targets, treat the full closeout as PARTIAL until those targets are
refreshed from the pushed package. Since v0.3.3 is already tagged and published,
the next blocker is repeated published-ref install confirmation, followed by at
least five real activation proof rows before considering 1.0 language.
