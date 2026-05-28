# Next Work

This backlog captures work intentionally left outside the v0.2.x release line.
Provider marketplace packaging is now complete, so the next work should prove
adoption and safety instead of adding more install surfaces. Pick one item,
define evidence, implement, and close it before adding more.

## Now: v0.3.0 Adoption Confidence Slice

Goal: turn the current installable package into a trusted package by proving
real provider invocation, privacy-preserving evidence, and clear first-use
workflows.

Scope:

- provider invocation dogfood
- proof format and privacy rules
- dogfood documentation update
- only the smallest workflow examples needed to explain the evidence

Out of scope:

- new skills
- new provider runtimes
- mandatory hooks or MCP setup
- official catalog submission
- broad ecosystem refresh

## P1: Provider Invocation Dogfood

Goal: prove that an installed or staged GroundLine package is naturally selected
by Codex, Claude Code, and Antigravity for real prompts, not only by scripted
file and contract checks.

Acceptance:

- Record one sanitized invocation transcript per provider.
- Confirm the selected skill and output contract for each transcript.
- Update `docs/dogfood.md` with PASS/PARTIAL evidence and accepted defers.
- Avoid raw prompt archives, secrets, full default home paths, and provider
  runtime state dumps.

Immediate tasks:

- Define the sanitized proof schema in `docs/provider-dogfood.md`.
- Add a small synthetic fixture for each prompt family if needed.
- Run one local provider invocation at a time and summarize only the selected
  skill, output contract, verification evidence, and stop condition.
- Update `docs/dogfood.md` with a compact matrix.
- Re-run validation and provider package sync.

## P1: Safety And Eval Harness

Goal: add a small offline evaluation harness inspired by coding-agent red-team
checks without turning GroundLine into a heavy eval platform.

Acceptance:

- Add fixtures for secret leakage, destructive command pressure, prompt
  injection, fake completion claims, and unsafe provider-home writes.
- Emit a machine-readable PASS/PARTIAL/FAIL report.
- Add the harness to CI only when it is deterministic and offline.
- Keep all fixtures synthetic.

Start after provider invocation proof is stable. The first version should be a
scripted offline harness, not a provider-session transcript archive.

## P2: Representative Workflows

Goal: make the package easier for humans and LLM agents to understand by
showing complete before/after workflows.

Acceptance:

- Add examples for handoff recovery, release cut, ecosystem radar, AI usage
  maturity review, and side-effect guarding.
- Each example names the trigger, selected skill, expected output contract,
  verification evidence, and stop condition.
- Keep examples short enough to be scanned during an active agent session.

For v0.3.0, include only examples that support the invocation dogfood evidence.
Move the full cookbook to a later patch if it starts growing.

## P2: Artifact Lifecycle

Goal: clarify how GroundLine artifacts move from research to comparison,
recommendation, implementation, dogfood, release, and post-release review.

Acceptance:

- Add or refine templates for research packets, comparison reports, upgrade
  decisions, release cuts, and release delta reports.
- Tie each template to an existing skill and output contract.
- Reject new templates that duplicate existing provider-native behavior.

This becomes important after safety and invocation proof exist. Do not start
here first.

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

These items are not required to keep v0.2.2 valid. The current release line
remains complete if the release URL, tag, CI run, package validation, provider
smoke, provider package validation, and staged dogfood remain PASS.
