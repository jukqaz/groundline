# Skill Graduation Plan

GroundLine currently has 7 active skills and 12 experimental skills. The
experimental set is acceptable for the v0.3 line, but it needs explicit
graduation decisions before GroundLine can claim a stable core.

This plan is intentionally conservative. A skill graduates only when examples,
output contracts, tests, and dogfood evidence all describe the same behavior.

## Graduation Criteria

A skill can move from `experimental` to `active` when it has:

- clear trigger language in `SKILL.md`
- matching metadata in `references/skill-index.json`
- a human-readable entry in `docs/skill-portfolio.md`
- examples or copyable prompts that show normal use
- output contracts that define the expected result shape
- dogfood evidence from at least one realistic provider scenario
- no unresolved privacy, side-effect, or context-cost concern

## Graduation Decisions

| Skill | Decision | Rationale | Next proof |
| --- | --- | --- | --- |
| `agent-ecosystem-radar` | merge | The one-pass radar is useful, but it duplicates the research, comparison, and recommendation chain. | Prove whether one orchestrator skill or three smaller skills gives better LLM routing. |
| `research-agent-ecosystem` | keep experimental | Source gathering is clear, but current-source research needs more examples across real repositories and docs. | Add a sanitized research packet with source dates and excluded provider-native features. |
| `compare-agent-workflows` | keep experimental | The scoring contract is useful, but needs more comparison examples with rejected and watched tools. | Add one comparison table that includes overlap, context cost, and unsupported claims. |
| `recommend-groundline-upgrades` | keep experimental | Upgrade recommendations are useful only if agents can avoid turning every finding into scope growth. | Record adopt, adapt, watch, and reject outcomes from the next ecosystem pass. |
| `evaluate-agent-capability` | graduate | Single-candidate evaluation is bounded, read-only, contract-backed, and covered by provider dogfood. | Add a worked example for one public tool evaluation before flipping lifecycle to active. |
| `evaluate-ai-usage-maturity` | keep experimental | The rubric is valuable, but secret-sensitive evidence handling needs more redacted packet examples. | Add a Provider Evidence Packet example that contains no raw transcript or secret values. |
| `package-agent-task` | graduate | Task packaging is central to long-context handoff and already has a stable packet shape. | Confirm fresh-install provider invocation still routes to the packet contract. |
| `hold-the-line` | keep experimental | Scope control is repeatedly useful, but needs more examples that show finish, defer, watch, and reject decisions. | Add cookbook prompts for cutting scope before more work starts. |
| `polish-release-candidate` | keep experimental | Release polishing is valuable, but overlaps with release-cut stabilization and needs clearer boundary examples. | Add a pre-release cleanup example that does not duplicate `stabilize-release-cut`. |
| `stabilize-release-cut` | graduate | Release-cut stabilization has a clear gate sequence, current package dogfood, and a ship/hold/continue contract. | Keep validation, dogfood, provider smoke, and Docker evidence green after package sync. |
| `compare-release-delta` | defer | Post-release comparison needs a real released patch and install-after-release evidence. | Run it after v0.3.3 is published and compare against the previous installed version. |
| `curate-groundline-skills` | keep experimental | Portfolio curation is useful, but should not become active until one release applies these decisions. | Re-run this plan after the next patch and remove or merge skills that still lack evidence. |

## Release Rule

Do not add another experimental skill while this plan is open. Improve examples,
output contracts, dogfood evidence, or provider activation proof first.
