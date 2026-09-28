# GPT-6 model, effort, and delegation

GroundLine optimizes for `gpt-6-astra`, `gpt-6-sol`, and `gpt-6-luna` only.
Do not recommend, tune, benchmark for adoption, or silently fall back to earlier
generations. Preserve historical records and explicit user selections; an
older selected model requires a separately authorized move to an available
GPT-6 model before optimization. Configuration audit remains catalog-based and
can describe existing settings without making them optimization targets.

## Resolve the execution surface

Use the active host catalog or the actual App-bundled `codex debug models`.
Intersect the GPT-6 scope with available models and supported efforts. Never
invent an alias, transfer API options into Codex TOML, or claim account access
from bundled metadata. Refresh evidence after a relevant runtime/catalog change.
If no suitable GPT-6 model is available, report that boundary instead of choosing
an older generation. Preserve the user's explicit task-level model/effort and
service tier; dynamic selection applies only within the delegated scope.

The September 28 App and PATH bundled catalogs list low/medium/high/xhigh/max for all three
models and ultra for Astra/Sol only. Max allocates more reasoning to one task;
Ultra combines maximum reasoning with suitable parallel delegation. Choose by
the work's shape, not an assumption that Ultra is always better than Max.
Apply the host's delegation controls and user policy. Ultra is not required for
explicitly authorized subagents with individually selected efforts.

API Sol/Luna also support none; this does not establish native none support.
API context sizes and reasoning modes are not Codex defaults. The API's
configuration_update mechanism does not prove that a Codex agent can change its
own running effort. Use only controls exposed by the active runtime.

## Select per work item

Select model and effort independently using ambiguity, dependencies, verification
difficulty, failure impact, context needs, and the user's time/quality preference.
Official starting points are Sol/medium, Luna/high, and Astra/low; these are
recommendations, not the catalog's default values or ceilings. The observed App/PATH bundled catalogs default to Astra/low and Sol/Luna/medium.
These observations are versioned metadata, not verified live account defaults. Use the whole supported effort range
when justified, without quotas or a fixed explorer/worker/reviewer ladder. Start
from the actual phase and acceptance check, not task length or token history.
Ordinary task selection needs no history audit or empirical sample. Preserve an
accepted baseline when claiming a measured replacement; missing data cannot
establish that a cheaper choice preserves its quality.

| Model | Work fit | Effort choice |
| --- | --- | --- |
| Luna | Clear briefs, coordinated edits, context gathering across apps, and problems with explicit constraints and objective checks | Low for fine-grained work; medium/high for creation and reasoning; xhigh for constrained analysis and prioritization; max for greater depth when workload checks support it |
| Sol | Implementation, research and ambiguous multi-step work | Low through max according to depth; do not cap demanding Sol work at high |
| Astra | Nuanced work, broad projects, coupled reasoning and synthesis across tools | Low for a scoped task needing Astra capability; medium through max for increasing depth and uncertainty |

| Effort | Select when | Example, not a mandatory role mapping |
| --- | --- | --- |
| low | Steps and success criteria are clear, with little ambiguity | Known-symbol inventory or a mechanical follow-up |
| medium | Routine planning and several dependent steps need checking | A bounded implementation with established tests |
| high | Logic tracing, assumptions and edge cases need sustained attention | A state-machine change or a focused correctness review |
| xhigh | Interacting hypotheses or constraints need deep analysis | A difficult concurrency diagnosis or conflicting research evidence |
| max | The hardest single problem warrants depth over latency/usage | A subtle invariant, algorithm or architectural decision with a concrete acceptance check |
| ultra | A supported model has a complex task with useful independent lanes | Coordinated end-to-end work under the host's delegation policy |

Max is a first-choice option for a known hard problem, not a last resort after
every lower effort has failed. All three GPT-6 models may use Max when supported;
Luna/Max is neither equivalent to nor guaranteed cheaper than Sol/medium. Choose
a stronger model directly when the issue is capability or broad judgment.
Use Ultra only when decomposition adds value; a hard serial bottleneck can use
Max while other agents handle independent work at lower efforts.

Reassess at a new lane or a meaningful phase change. Raise effort for an unresolved
reasoning problem; lower it for mechanical follow-up after the uncertainty is
resolved. Do not replay completed work just to exercise another setting. Count
coordination, failures and retries in total quality/time/usage; do not infer
subscription savings from API prices or prefer diversity for its own sake.
Frequent high-effort use alone is not evidence of wasted work.

## Adaptive delegation

When the user requests adaptive delegation, that request authorizes suitable
subagents throughout the task; do not ask again for every spawn. The applicable
optimize-codex-workflow skill also requests bounded delegation for independent
implementation/research lanes. Respect higher-priority instructions and explicit
user restrictions. An advisory review alone does not enable delegation or
override a no-delegation policy.

The root retains the goal, dependencies, integration, and final verification.
Before delegating, carry the agreed outcome, current acceptance criteria and
authority into each lane. Keep the serial critical path coherent; the number of
available slots is not a target. For broad visual work, establish a representative
accepted result before parallel production when that acceptance is still unknown.
Spawn only a concrete independent lane while useful root work can proceed.
Choose a supported GPT-6 model and effort together for each lane. Inspect custom
agent overrides before assuming the requested pair will take effect. Full-history
forks may inherit the parent's model; use the runtime's supported bounded-context
mode when a different model is needed, carrying forward decision-critical context.
Do not claim a running root switched itself when no supported control exists.
If an existing child cannot be reconfigured, reassign only the remaining work
with a compact handoff when justified. Asking it to "think harder" does not
prove its effective reasoning setting changed.

Start with the useful independent lanes, usually one or two, and stay within the
host concurrency limit. Do not manufacture parallel work or recursively fan out.
Assign disjoint edit ownership and separate browser/UI sessions where needed.
Give each agent its goal, scope, constraints, relevant evidence, completion check,
and compact return contract: result, evidence, checks, and unresolved issues.
Reuse an existing agent for related follow-up when its context/settings still fit.
For a long task, hand off the goal, user corrections, decisions, relevant files,
completed checks, rejected hypotheses and remaining acceptance work. Do not fork,
reset or copy the whole history merely because compaction occurred.

For example, independent lanes might use Luna/low for a known API inventory,
Sol/high for a bounded feature and Sol/max or Astra/max for a hard invariant
review. A later mechanical follow-up can use a lower effort. Each choice still
needs suitable context, a completion check and actual independence.

Validate returned evidence before integration. Missing information calls for
better inputs; network, permission, and tool failures need their actual cause
resolved. Raise effort or reassign to a stronger GPT-6 model only for demonstrated
reasoning/complexity needs. Carry completed work and rejected hypotheses forward;
never repeat an unchanged failure or run an unbounded escalation loop. Reuse
passing checks until a change or unresolved concern invalidates them.

## GPT-6 guidance and evaluation

For an empirical comparison of an existing choice, or preparing an `efficiency
route` packet, use the [evidence routing contract](evidence-routing.md). A plan
for the next task can use supplied history as descriptive context without
opening that contract or collecting more data. Label its proposed model/effort
as a task judgment, not a measured optimum. Matched direct outcomes are needed
when claiming an evidence-based replacement; missing outcomes do not block an
ordinary authorized choice.

Keep one authoritative instruction for each behavior. Load specialist detail
only when relevant. Continue authorized work across steering, ask only for a
material missing choice, and prepare independent work while awaiting an answer.
State outcomes, evidence and acceptance criteria; let native Codex plan the steps.
The official family guide's behavior examples originate with Astra; evaluate them
on the selected Sol/Luna workload before claiming the same effects.

Compare the same task, model/effort, tools, service tier, and delegated composition
when assessing a guidance change. Report completion, rework, failures, total owned
tokens and elapsed time. Historical unversioned astra/sol/luna cohorts and mixed
or unknown generations cannot establish GPT-6 improvement. Core validates evidence
offline; it does not add a model proxy, scheduler, or automatic inference service.

Sources checked 2026-09-28:
- [Reasoning effort and Max](https://developers.openai.com/api/docs/guides/reasoning)
- [Current skill and prompt guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)
- [GPT-6 family guidance](https://developers.openai.com/api/docs/guides/latest-model)
- [GPT-6 Sol](https://developers.openai.com/api/docs/models/gpt-6-sol)
- [GPT-6 Luna](https://developers.openai.com/api/docs/models/gpt-6-luna)
- [Task-specific model choice](https://developers.openai.com/api/docs/guides/model-selection)
- [Codex model selection](https://learn.chatgpt.com/docs/models)
- [Native subagent model and effort controls](https://learn.chatgpt.com/docs/agent-configuration/subagents)
