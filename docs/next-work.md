# Next Work

This backlog captures work intentionally left outside the v0.2.1 release. Do
not expand the current release scope from this document. Pick one item, define
evidence, implement, and close it before adding more.

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

## P1: Safety And Eval Harness

Goal: add a small offline evaluation harness inspired by coding-agent red-team
checks without turning GroundLine into a heavy eval platform.

Acceptance:

- Add fixtures for secret leakage, destructive command pressure, prompt
  injection, fake completion claims, and unsafe provider-home writes.
- Emit a machine-readable PASS/PARTIAL/FAIL report.
- Add the harness to CI only when it is deterministic and offline.
- Keep all fixtures synthetic.

## P2: Representative Workflows

Goal: make the package easier for humans and LLM agents to understand by
showing complete before/after workflows.

Acceptance:

- Add examples for handoff recovery, release cut, ecosystem radar, AI usage
  maturity review, and side-effect guarding.
- Each example names the trigger, selected skill, expected output contract,
  verification evidence, and stop condition.
- Keep examples short enough to be scanned during an active agent session.

## P2: Artifact Lifecycle

Goal: clarify how GroundLine artifacts move from research to comparison,
recommendation, implementation, dogfood, release, and post-release review.

Acceptance:

- Add or refine templates for research packets, comparison reports, upgrade
  decisions, release cuts, and release delta reports.
- Tie each template to an existing skill and output contract.
- Reject new templates that duplicate existing provider-native behavior.

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

## Release Boundary

These items are not required to keep v0.2.1 valid. The v0.2.1 release remains
complete if the release URL, tag, CI run, package validation, provider smoke,
and staged dogfood remain PASS.
