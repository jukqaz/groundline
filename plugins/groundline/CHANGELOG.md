# Changelog

## Unreleased

## v0.3.3 - 2026-05-28

- Version-aware provider smoke now reports installed version, source version,
  install source, cache candidates, payload presence, skill count drift,
  same-version content drift, `install_doctor_status`, and
  `secret_value_printed=false` for Codex, Claude Code, and Antigravity, with
  provider-level `recommended_actions` and top-level `next_actions`.
- Add `--require-installed` to provider smoke and use it from the release gate
  so missing provider targets are only accepted during package/path validation,
  not during post-install release proof.
- Add single-source version control so validation compares provider manifests
  against canonical `plugin.json` instead of a hard-coded patch version.
- Add a provider activation matrix and expand staged dogfood to six prompt
  families while keeping live provider proof separate from staged contract
  checks.
- Align the AI usage maturity activation matrix row with the canonical
  `GroundLine AI Usage Maturity` output contract.
- Add a skill graduation plan with machine-readable decisions for all 12
  experimental skills. No lifecycle values are promoted in this patch.
- Add a workflow cookbook that maps five common prompts to selected skills,
  output contracts, verification evidence, and stop conditions.
- Add an artifact lifecycle map for research packet, comparison report,
  upgrade decision, implementation task, dogfood evidence, release cut, and
  release delta handoffs.
- Add a release gate runner that prints or executes the local release gate
  sequence while excluding approval-required tag, push, and GitHub Release
  commands, and preserves compact JSON summaries plus top-level next actions
  for partial gates.
- Add an optional `--release-version` release gate preflight so actual release
  cuts fail when source or packaged manifests still point at the wrong version,
  or when the requested version is not plain `X.Y.Z` semver.
- Add staged provider smoke so a fake refreshed install can be proven with
  `--stage-package --require-installed` before touching real provider homes.
- Preserve staged provider smoke summary fields in release gate output and
  refresh maturity evidence against the current remote CI run.
- Add a provider-native validation gate for read-only Claude Code and
  Antigravity package validation during local release closeout.
- Redact local home paths from release gate and Docker scenario evidence
  outputs before they are copied into release review.
- Separate approval-required publishing commands from read-only release
  evidence in the public release checklist.
- Document the exact version bump sequence for source manifests, package sync,
  validation, changelog movement, and `v`-prefixed release tags.
- Add the deterministic offline safety eval harness to the default CI release
  gate and manual release evidence checklist.
- Add a deterministic privacy scan gate for local home paths, generic
  secret-like values, dynamically checked stale test proof counts, stale remote
  CI run claims, and overstated release claims.
- Align README and update validation docs with source, packaged, safety,
  privacy, smoke, dogfood, and scenario release gates.
- Keep package validation strict for conflict-copy payloads while ignoring
  empty conflict-copy directories that contain no files.
- Prevent installable provider package copies from running package sync and
  creating nested `plugins/groundline` payloads.

## v0.3.2 - 2026-05-28

- Clarify routing boundaries for ecosystem research, single-candidate
  evaluation, candidate comparison, and GroundLine upgrade recommendation.
- Separate staged dogfood contract checks from real provider invocation proof.
- Link provider-history inventory to AI usage maturity assessment through a
  redacted Provider Evidence Packet.
- Make release triage priority explicit across scope hold, pre-ship polish, and
  final ship decision skills.

## v0.3.1 - 2026-05-28

- Record the Claude Code follow-up proof that reduces the v0.3.0 release
  closeout partial when read-only skill document inspection is allowed.
- Keep the Antigravity constrained print-mode proof as an explicit accepted
  defer while package validation and install validation remain passing.
- Add optional provider guardrail and MCP recipe docs for Codex, Claude Code,
  and Antigravity without enabling hooks, rules, MCP servers, commands, or
  provider-level agents by default.
- Record local Superpowers companion dogfood showing GroundLine as the state,
  side-effect, live-proof, and release-control layer while Superpowers owns
  planning, TDD, debugging, review, and final verification discipline.

## v0.3.0 - 2026-05-28

- Add provider marketplace packaging for Codex and Claude Code, plus an
  Antigravity install surface and provider packaging guide.
- Add Korean companion docs for human-facing setup, workflow selection, skill
  overview, privacy, release, and next-version planning while keeping English
  as the default and canonical documentation language.
- Add a language policy and validation coverage for bilingual human docs.
- Add sanitized provider invocation proof schema and dogfood evidence for Codex,
  Claude Code, and Antigravity.
- Add an offline safety evaluation harness with synthetic cases for secret-like
  output, destructive command pressure, false completion claims, and unsafe
  provider-home writes.
- Keep Claude Code contract naming and Antigravity constrained print mode as
  explicit accepted partials for the next patch instead of masking them as
  passing.

## v0.2.2 - 2026-05-28

- Add the next work backlog for provider invocation dogfood, safety evaluation,
  representative workflows, artifact lifecycle, ecosystem refresh, and install
  UX.
- Link the backlog and next version plan from README and make package
  validation require both.

## v0.2.1 - 2026-05-28

- Add a provider dogfood harness for staged package, runtime probe, and shared
  scenario contract validation.
- Add provider dogfood runbook and release checklist gate.
- Record v0.2.1 dogfood evidence and accepted defers.

## v0.2.0 - 2026-05-28

- Add public release, privacy, contribution, and security documentation.
- Add separate human and LLM guides plus GitHub issue and pull request templates.
- Add git history privacy guidance for public repository preparation.
- Replace personal author metadata with GroundLine contributor branding.
- Redact default home paths in doctor and provider smoke output.
- Align upgrade packet secret-like input detection with doctor and radar checks.
- Verify the pinned actionlint archive checksum in CI.
- Add an agent ecosystem radar skill set for research, comparison, and upgrade recommendations.
- Add a GroundLine pack evaluation skill for skill completeness and release readiness review.
- Add human-readable skill portfolio docs and an LLM-readable skill index.
- Add skill lifecycle taxonomy and curation guidance.
- Add read-only provider smoke runtime probes for staged install targets.
- Add an existing capability evaluation skill and rubric for tools, skills, plugins, MCP servers, hooks, and agents.
- Add an AI usage maturity assessment skill and rubric for evidence-backed workflow improvement.
- Add task packet and release stabilization skills for context packaging, scope lock, dogfood evidence, and ship decisions.
- Add a release delta comparison skill for post-deploy checklists against the previous version.

## v0.1.1

- Force GitHub JavaScript actions onto Node 24 in validation and radar
  workflows.
- Pin GitHub Actions to current release tags and install pinned actionlint from
  its prebuilt Linux binary.

## v0.1.0

- Add GroundLine manifests for Codex, Claude Code, and Antigravity.
- Add six workflow skills for state reconciliation, history audit, side-effect
  boundaries, live evidence, provider home alignment, and worktree recovery.
- Add stdlib-only doctor, radar, upgrade packet, provider smoke, package
  validation, and scenario scripts.
- Add opt-in external tool probes and command sources with secret-like output
  redaction.
- Add GitHub Actions validation and scheduled radar workflows.
- Add macOS local and Linux Docker scenario gates.
- Add install, update, provider smoke, runtime support, examples, and release
  documentation.
