# Next Work

This backlog captures work intentionally left outside the v0.2.x release line.
Provider marketplace packaging and the first adoption proof slice shipped in
v0.3.0. The next work should reduce accepted partials and improve first-use
clarity before adding more install surfaces. Pick one item, define evidence,
implement, and close it before adding more.

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

## Now: Remote Install Proof

Goal: prove the package can be installed from the pushed GitHub state before
adding more workflows.

Acceptance:

- Push `main` with the v0.3.1 patch.
- Tag and publish v0.3.1 after local gates pass.
- Confirm CI passes for the pushed commit.
- Run at least one provider install or update from the remote source.
- Keep provider-home writes explicit and user-approved.

Scope:

- provider invocation partial triage
- final package sync and provider validation
- release notes and checklist cleanup
- only the smallest workflow examples needed to explain the evidence
- optional MCP and provider guardrail docs

Out of scope:

- new skills
- new provider runtimes
- mandatory hooks or MCP setup
- official catalog submission
- broad ecosystem refresh

## P1: Provider Invocation Follow-up

Goal: reduce the two accepted partials from provider invocation dogfood without
expanding the skill surface.

Acceptance:

- Decide whether Claude Code contract naming is a docs/prompt issue or a
  release blocker.
- Decide whether Antigravity constrained print mode needs a repeatable runbook
  or stays an accepted defer.
- Keep `docs/dogfood.md` sanitized: no raw prompt archives, secrets, full
  default home paths, or provider runtime state dumps.
- Add no new skill unless repeated provider proof shows an existing skill cannot
  express the flow.
- Keep hooks, rules, MCP servers, commands, and provider-level agents out of the
  default package unless a specific reviewed use case requires opt-in setup.

Immediate tasks:

- Keep the Claude Code proof prompt in the runbook: allow read-only skill doc
  inspection, forbid mutation, and require canonical skill and contract names.
- Re-run Antigravity expansion-control proof only when it can be constrained
  without a tool loop or app-data write failures.
- If either remains partial, keep the release decision explicit instead of
  masking it as PASS.
- Re-run validation and provider package sync after any document change.
- Keep `docs/provider-guardrails.md`, `docs/mcp-recipes.md`, and
  `references/optional-mcp-profiles.md` aligned as the opt-in extension path.

## P1: Safety And Eval Harness Follow-up

Goal: keep the new offline safety harness useful without turning GroundLine into
a heavy eval platform.

Acceptance:

- Keep fixtures synthetic.
- Emit a machine-readable PASS/FAIL report with `mutation_performed=false`.
- Add the harness to CI only when it is deterministic and offline.
- Add a prompt-injection fixture only if it tests a distinct boundary rather
  than duplicating the existing safety cases.

The first version covers secret-like output, destructive command pressure, false
completion, and unsafe provider-home writes.

## P2: Representative Workflows

Goal: make the package easier for humans and LLM agents to understand by
showing complete before/after workflows.

Acceptance:

- Add examples for handoff recovery, release cut, ecosystem radar, AI usage
  maturity review, and side-effect guarding.
- Each example names the trigger, selected skill, expected output contract,
  verification evidence, and stop condition.
- Keep examples short enough to be scanned during an active agent session.

For v0.3.1, include only examples that support the invocation dogfood evidence.
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

These items are not required to keep v0.3.0 valid. The current release line
remains complete if the release URL, tag, CI run, package validation, provider
smoke, provider package validation, and staged dogfood remain PASS.
