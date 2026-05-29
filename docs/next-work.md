# Next Work

This backlog captures work intentionally kept out of each release cut. Provider
marketplace packaging shipped in v0.3.0, routing clarity shipped in v0.3.2, and
v0.3.3 shipped install posture, version drift, and proof quality improvements.
Pick one item, define evidence, implement, and close it before adding more.

## Current Release Cut: v0.3.5 Remote Install And Update Proof

Goal: prove that a public package can be freshly installed, stale installed
targets can be detected after a version bump, and a refreshed install returns
to `PASS` without expanding the skill surface or mutating real provider homes by
default.

Must fix:

- Keep the v0.3.4 local proof-quality work inside the cut: provider target
  refresh evidence, release delta docs, and full local gate coverage.
- Add a fake-home remote install/update proof that covers fresh install,
  previous-version detection, and post-update refresh for Codex, Claude Code,
  and Antigravity.
- Add the new proof to release gate, install docs, update docs, and release
  checklist so update confidence is checked before a public release claim.
- Record sanitized live activation rows for the three still-pending prompt
  families: `side-effect-guard`, `ecosystem-evaluation`, and
  `ai-usage-maturity`.
- Keep `handoff` and `release-cut` proof rows from regressing after provider
  refresh.
- Run a release delta review against the published v0.3.3 baseline and the
  local v0.3.5 candidate.
- Bump manifests to `0.3.5`, sync `plugins/groundline`, and run the release gate
  with `--release-version 0.3.5` before tagging.

Defer:

- Lifecycle promotion for graduate candidates.
- Official catalog submission polish and richer marketplace media.
- Broad ecosystem comparison refresh.
- Optional hooks, MCP setup, provider-level agents, or automatic provider-home
  installation.

Reject:

- New skills, new provider runtimes, default provider-home writes, raw transcript
  storage, provider cache dumps, and full home-path evidence.

Ship boundary: v0.3.5 can ship when source/package validation, fake-home remote
install/update proof, provider smoke, staged dogfood, privacy scan, scenario
checks, and release delta evidence are explicit. Live provider activation rows
may remain accepted partials only if the missing proof is documented.

Current harness status:

- `groundline_remote_install_probe.py --json`: added for fake-home fresh
  install, stale update detection, and post-update refresh; current local run
  reports `PASS`.
- `groundline_provider_smoke.py --json --require-installed`: `PASS` after local
  Codex, Claude Code, and Antigravity provider targets were refreshed to the
  v0.3.5 package.
- `groundline_release_gate.py --json --keep-going --include-docker-execution
  --release-version 0.3.5`: `PASS` with source/package validation, 129 tests,
  privacy scan, remote install/update proof, provider smoke, staged dogfood,
  macOS scenario, and Linux Docker execution.
- Live provider CLI proof rows still require explicit approval because they send
  the current worktree context to external provider runtimes.

## Completed: v0.3.0 Adoption Confidence Slice

- `docs/provider-dogfood.md` defines the sanitized invocation proof shape.
- `docs/dogfood.md` records one provider invocation proof row per supported
  runtime without raw transcript storage.
- Codex returned `package-agent-task` and `GroundLine Task Packet` for the
  handoff family.
- Claude Code selected an installed GroundLine skill for release closeout, but
  did not return the canonical contract name.
- Antigravity print mode could not complete the constrained proof in this
  environment before timeout.
- `scripts/groundline_safety_eval.py` validates four synthetic safety cases
  offline.
- `tests/test_groundline_script_contract.py` covers the safety eval contract.

## Completed: v0.3.1 Partial Reduction

Goal: reduce or intentionally keep the accepted partials from v0.3.0 before
adding new workflows.

Current status:

- Claude Code contract naming partial is reduced: allowing read-only skill doc
  inspection returns `stabilize-release-cut` and `GroundLine Release Cut`.
- Antigravity print-mode proof remains partial: `agy --print` still enters tool
  exploration and hits CLI app-data write constraints before returning a
  sanitized proof.
- GroundLine remains skills-only by default. Hooks, rules, MCP servers,
  commands, and provider-level agents stay opt-in.
- Optional private MCP guidance is now the preferred extension path when skills
  need live or private tool access.
- Local Superpowers companion dogfood is recorded and verified.

## Completed: v0.3.2 First-use And Routing Clarity

Goal: make user prompts route to the correct existing skill without expanding
the skill set.

Current status:

- Human onboarding docs were expanded with clearer start paths.
- Ecosystem research skills now separate source gathering, single-candidate
  evaluation, multi-candidate comparison, and recommendation.
- Provider-history AI usage assessment now routes through
  `audit-agent-history -> evaluate-ai-usage-maturity`.
- Release triage now names when to use `hold-the-line`,
  `polish-release-candidate`, and `stabilize-release-cut`.
- Dogfood docs now separate staged contract harness checks from real provider
  invocation proof.

## Released: v0.3.3 Install Posture And Drift Control

Goal: make local provider install state and version drift obvious before adding
more workflows. This is the highest-value next step from
`docs/maturity-assessment.md`.

Post-release status:

- Version-aware provider smoke now reports source version, installed version,
  payload presence, skill count drift, same-version content drift, and
  `install_doctor_status`.
- Staged provider smoke proves that a fake refreshed install passes without
  touching real provider homes.
- Validation now compares provider manifest versions against canonical
  `plugin.json` instead of a hard-coded patch version.
- Package sync, source validation, packaged validation, lint, provider-native
  validation, unit tests, safety eval, privacy scan, offline doctor, offline
  radar, staged dogfood, macOS local scenario, Linux Docker dry-run, and Linux
  Docker execution passed for the v0.3.3 release cut.
- Staged provider smoke proves that a refreshed package would pass. Real
  provider smoke reports `PARTIAL` when installed provider targets still contain
  the previous same-version payload after post-release source changes; use its
  `next_actions` to refresh Codex and Claude Code before claiming installed
  targets match the current source.
- A release gate runner now prints or executes the same local gate sequence
  without including approval-required publish commands.
- The remaining work is v0.3.4 planning: keep provider install confirmation
  green, collect more live activation proof rows, and run a release delta review
  before changing lifecycle values.

Acceptance:

- Add a version-aware install doctor for Codex, Claude Code, and Antigravity.
- Detect installed GroundLine version, source ref drift, stale cache versions,
  missing package payload, skill count mismatch, and same-version content
  drift.
- Report `PASS`, `PARTIAL`, or `FAIL` without printing provider auth, sessions,
  logs, or raw home dumps.
- Prove fake refreshed installs with `--stage-package --require-installed`.
- Add fake-home unit tests for all three providers.
- Replace hard-coded package version checks with canonical manifest comparison.
- Update install and provider packaging docs with the new confirmation command.

Scope:

- install posture diagnostics
- version drift control
- docs for repeatable confirmation
- unit tests and package sync

Out of scope:

- new skills
- new provider runtimes
- mandatory hooks or MCP setup
- official catalog submission work
- broad ecosystem refresh

## P0: Version-aware install doctor

Goal: make the real installed package version visible after a GitHub install.

Status: implemented and released in v0.3.3; source/package, staged dogfood, and
scenario gates pass. Keep as P0 while real provider smoke can become `PARTIAL`
after source content changes until installed provider targets are refreshed.

Acceptance:

- The check reports installed version for Codex and Claude Code when the
  provider exposes semantic versions.
- The check reports Antigravity package shape and skill count even when list
  output only shows import metadata.
- The check detects stale marketplace ref, cache state, or same-version content
  drift when the repository payload is newer than the installed plugin.
- The check uses fake provider homes in tests.
- The check reports `mutation_performed=false` and `secret_value_printed=false`.

## P0: Single-source version control

Goal: prevent future patch releases from drifting between manifests, packaged
payload, tests, and validation scripts.

Status: implemented and released in v0.3.3; package sync and validation prove
source and packaged manifests stay aligned. Keep as P0 until the next version
bump proves the same canonical manifest path.

Acceptance:

- `validate_pack.py` reads the expected version from a canonical manifest.
- Tests assert `.codex-plugin/plugin.json`, `.claude-plugin/plugin.json`,
  root `plugin.json`, and packaged manifests all match.
- Release docs state the exact version bump sequence.
- Package sync preserves the same version across source and packaged payload.

## P1: Real provider activation matrix

Goal: prove live provider sessions select the intended GroundLine skill and
output contract for representative prompts.

Status: matrix document and six-scenario staged contract coverage shipped in
v0.3.3. The live proof collection runbook and row update checklist are also
documented. Live provider proof rows still need to be collected for side-effect
guard, ecosystem evaluation, and AI usage maturity before this item is complete.

Acceptance:

- Cover handoff, side-effect guard, release cut, ecosystem evaluation, and AI
  usage maturity prompt families in both staged contract evidence and real
  provider proof tracking.
- Record only sanitized proof: provider, prompt family, selected skill, output
  contract, mutation status, result, and short evidence.
- Keep raw transcripts, auth material, provider cache dumps, and full home dumps
  out of the repository.
- Keep staged contract harness results separate from real provider activation
  proof.

## P1: Skill graduation plan

Goal: reduce the experimental surface before calling GroundLine stable.

Status: implemented as a lifecycle decision plan in v0.3.3. No lifecycle values
are promoted yet; promotion waits for repeated install confirmation and the
remaining provider activation rows.

Acceptance:

- Classify all 12 experimental skills as `graduate`, `keep experimental`,
  `merge`, or `defer`.
- Graduate only skills with examples, output contracts, docs, tests, and
  dogfood evidence.
- Do not create new skills during this pass.
- Record lifecycle decisions in `docs/skill-portfolio.md` and
  `references/skill-index.json`.

Current decisions:

- `graduate`: `package-agent-task`, `stabilize-release-cut`.
- `merge`: `agent-ecosystem-radar`.
- `defer`: `compare-release-delta`.
- `keep experimental`: `research-agent-ecosystem`,
  `compare-agent-workflows`, `recommend-groundline-upgrades`,
  `evaluate-agent-capability`, `evaluate-ai-usage-maturity`,
  `hold-the-line`, `polish-release-candidate`, and
  `curate-groundline-skills`.

## P1: Provider Invocation Follow-up

Goal: keep reducing provider invocation partials without expanding the skill
surface.

Status: Claude Code contract naming is reduced when the proof prompt allows
read-only skill document inspection and requires the canonical skill and
contract names. The remaining accepted defer is Antigravity constrained
print-mode proof.

Acceptance:

- Keep the Claude Code proof prompt as the documented PASS path and refresh it
  after published install proof.
- Decide whether Antigravity constrained print mode needs a repeatable runbook
  or stays an accepted defer.
- Keep `docs/dogfood.md` sanitized: no raw prompt archives, secrets, full
  default home paths, or provider runtime state dumps.
- Add no new skill unless repeated provider proof shows an existing skill cannot
  express the flow.
- Keep hooks, rules, MCP servers, commands, and provider-level agents out of the
  default package unless a specific reviewed use case requires opt-in setup.

Immediate tasks:

- Refresh Claude Code release-cut proof after the published ref is installed;
  do not reopen the old contract naming partial unless it regresses.
- Re-run Antigravity release-cut proof only when it can be constrained
  without a tool loop or app-data write failures.
- If Antigravity remains partial, keep the release decision explicit instead
  of masking it as PASS.
- Re-run validation and provider package sync after any document change.
- Keep `docs/provider-guardrails.md`, `docs/mcp-recipes.md`, and
  `references/optional-mcp-profiles.md` aligned as the opt-in extension path.

## P1: Safety And Eval Harness Follow-up

Goal: keep the new offline safety harness useful without turning GroundLine into
a heavy eval platform.

Status: implemented for the current synthetic fixture set and added to the
default CI release gate in v0.3.3.

Acceptance:

- Keep fixtures synthetic: implemented.
- Emit a machine-readable PASS/FAIL report with `mutation_performed=false`:
  implemented.
- Add the harness to CI only when it is deterministic and offline:
  implemented.
- Add a prompt-injection fixture only if it tests a distinct boundary rather
  than duplicating the existing safety cases.

The first version covers secret-like output, destructive command pressure, false
completion, and unsafe provider-home writes.

## P2: Representative Workflows

Goal: make the package easier for humans and LLM agents to understand by
showing complete before/after workflows.

Status: implemented as a compact workflow cookbook in v0.3.3. Keep it compact
unless live provider proof shows a workflow remains confusing.

Acceptance:

- Add examples for handoff recovery, release cut, ecosystem radar, AI usage
  maturity review, and side-effect guarding.
- Each example names the trigger, selected skill, expected output contract,
  verification evidence, and stop condition.
- Keep examples short enough to be scanned during an active agent session.

For v0.3.3, `docs/workflow-cookbook.md` and
`docs/ko/workflow-cookbook.md` cover the five representative workflows without
adding new skills.

## P2: Artifact Lifecycle

Goal: clarify how GroundLine artifacts move from research to comparison,
recommendation, implementation, dogfood, release, and post-release review.

Status: implemented as a compact artifact lifecycle map in the current patch
draft.

Acceptance:

- Add or refine templates for research packets, comparison reports, upgrade
  decisions, release cuts, and release delta reports.
- Tie each template to an existing skill and output contract.
- Reject new templates that duplicate existing provider-native behavior.

For v0.3.3, `docs/artifact-lifecycle.md` and
`docs/ko/artifact-lifecycle.md` map each artifact to one primary skill, output
contract, next artifact, and stop condition.

## P2: Ecosystem Comparison Refresh

Goal: keep GroundLine aligned with adjacent tools without copying their scope.

Acceptance:

- Refresh source-backed notes for Superpowers, Spec Kit, Agent OS, BMAD,
  gstack, grill-me, coding-agent red-team checks, and AI fluency assessment
  tools.
- Classify each finding as adopt, adapt, watch, or reject.
- Create no new skill unless repeated evidence shows the existing skill set
  cannot express the workflow.

## P3: Installation UX

Goal: reduce first-run friction while staying a provider-neutral companion
layer.

Acceptance:

- Improve install/update docs with provider-specific manual verification steps.
- Add only dry-run or staged install helpers unless the user explicitly asks for
  real provider-home writes.
- Keep MCP and hook setup optional.

This is mostly done for the current package. Re-open only if a new user cannot
install through Codex, Claude Code, or Antigravity from the documented commands.

## Release Boundary

These backlog items are not required to keep the already-published v0.3.3 patch
bounded. The next release remains shippable only when the release decision is
explicit and the current validation gates and diff checks pass. Treat real
provider smoke as a release blocker when it reports missing source manifests or
unsafe output, and as an accepted `PARTIAL` only when `next_actions` clearly
point to post-publish provider install refresh.
