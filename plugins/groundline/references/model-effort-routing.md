# Model and effort routing

Let Codex choose its recommended model and effort by default. Recommend an
override only for a concrete task benefit, using the active session's catalog.
Do not install a separate model registry or pin a global default in GroundLine.

Choose from the models and efforts actually available on the execution host:

- Clear, repeatable work with deterministic checks: favor speed and cost.
- Everyday implementation, exploration, and review: favor balanced capability.
- Ambiguous or high-value work: favor reasoning depth and judgment.
- Difficult end-to-end work spanning tools and several dependent stages: favor
  sustained reasoning and completion capability.

The official model guidance currently describes Luna, Terra, Sol, and Astra in
those roles, respectively. These are examples, not a closed availability list.
Preserve explicit user selections and intentional economy roles. Recommend the
lowest supported effort appropriate to the difficulty, without requiring a
low-effort trial for an obviously hard task. Max and Ultra are reasoning levels
when supported; neither requires nor authorizes subagents. Codex delegation
requires the user's explicit request in this workflow. Do not import ChatGPT
Work's proactive Ultra delegation policy into local Codex. Model, effort,
delegation, and service tier are distinct decisions.

Use `codex debug models` from the actual runtime for catalog evidence. A
`--bundled` entry, a release note, and account access are different observations.
Never infer token price, quota, or savings from an effort label or a Fast badge.
Check current official pricing only when a cost comparison is requested.

## Astra-specific review

The official Astra guide calls out sensitivity to conflicting skills and
instructions, unnecessary clarification pauses, verbose formatting, and
over-broad verification. Optimize the workflow for these observed risks:

- Keep a single authoritative instruction for each behavior. Move conditional
  detail out of skill entrypoints, and do not load unrelated skills.
- For an implementation request, complete authorized local work and verify it;
  ask only for information or authority that can materially change the result.
- Keep reports concise and evidence-backed; omit empty templates and repeated
  safety prose. Use lists only when they make the result easier to understand.
- Choose meaningful checks by changed behavior. Broaden or repeat them only for
  a new change, failure, or unresolved risk; do not run a full suite by habit.

Use [configuration review](codex-configuration.md) to compare explicit settings
with native catalog evidence. `none` and `minimal` are not supported by the
observed Astra catalog; do not silently translate them. For future models or
catalog updates, use the returned support list. Do not equate an API example's
parameters, context limit, or delegation prompt with Codex configuration or
permission to spawn agents.

Sources checked 2026-09-07:
- [Official model guidance](https://learn.chatgpt.com/docs/models)
- [Codex changelog](https://learn.chatgpt.com/docs/changelog)
- [Subagent configuration](https://learn.chatgpt.com/docs/agent-configuration/subagents)
- [Astra instruction and testing guidance](https://developers.openai.com/api/docs/guides/latest-model)
