# Examples

These examples show how GroundLine should activate and what kind of answer it
should produce.

## Resume Prior Agent Work

Prompt:

```text
Claude Code was working on this. Recheck where it stopped and finish safely.
```

Expected flow:

```text
reconcile-current-state -> guard-side-effects when needed -> close-live-work when runtime proof matters
```

Expected answer shape:

```text
GroundLine Assessment:
- current conclusion: partial
- verified state: branch and PR checked
- capability gaps: live endpoint proof missing
- side-effect boundary: read_only
- recommended mode: companion-superpowers
- recommended next skill or tool: close-live-work
- exact next prompt: Verify the deployed endpoint and report PASS/PARTIAL/FAIL.
- verification checklist: endpoint, version, smoke
```

## Audit History For Reusable Patterns

Prompt:

```text
Find repeated failure patterns across my current agent histories and suggest improvements.
```

Expected skill:

```text
audit-agent-history
```

## Guard A Risky Operation

Prompt:

```text
Deploy this to production and update access permissions.
```

Expected skill:

```text
guard-side-effects
```

Expected answer shape:

```text
Boundary:
- classification: external_mutation
- approval needed: yes
- intended side effect: production deploy and access update
- read-only checks: current branch, CI status, target surface
- execution allowed: false
- secret value printed: false
```

## Close Live Work

Prompt:

```text
CI passed. Check whether the dev runtime actually serves this change.
```

Expected skill:

```text
close-live-work
```

## Align Agent Home

Prompt:

```text
Align my current agent homes and tell me what should be shared.
```

Expected skill:

```text
align-agent-home
```

## Recover Worktree And Branch

Prompt:

```text
The previous worktree path is gone. Prove the branch state before recreating it.
```

Expected skill:

```text
recover-worktree-branch
```

## Package A Task For Another Agent

Prompt:

```text
This thread is getting long. Package the current goal so another agent can continue without guessing.
```

Expected skill:

```text
package-agent-task
```

Expected answer shape:

```text
GroundLine Task Packet:
- current conclusion:
- goal:
- context:
- constraints:
- non-goals:
- mutation boundary:
- success criteria:
- verification:
- handoff:
```

## Research And Recommend Workflow Upgrades

Prompt:

```text
Find good external agent workflow repos and information, compare them, and recommend what GroundLine should add.
```

Expected flow:

```text
agent-ecosystem-radar -> research-agent-ecosystem -> evaluate-agent-capability -> compare-agent-workflows -> recommend-groundline-upgrades
```

Expected answer shape:

```text
GroundLine Research:
- scope: GroundLine supported runtimes
- primary sources: ...
- candidate list: ...

GroundLine Comparison:
- strongest matches: ...
- rejected or deferred: ...

GroundLine Recommendation:
- adopt: ...
- adapt: ...
- watch: ...
- reject: ...
- verification checklist: ...
```

## Evaluate Existing Tool Or Skill

Prompt:

```text
Evaluate this MCP server or skill pack before we decide whether GroundLine should use it.
```

Expected skill:

```text
evaluate-agent-capability
```

Expected answer shape:

```text
GroundLine Capability Evaluation:
- target:
- artifact type:
- source evidence:
- context cost:
- security risk:
- maintenance signal:
- GroundLine fit:
- decision: adopt|adapt|watch|reject
- verification needed:
```

## Evaluate AI Usage Maturity

Prompt:

```text
Assess how well I am using AI agents and what I should improve next.
```

Expected skill:

```text
evaluate-ai-usage-maturity
```

Expected answer shape:

```text
GroundLine AI Usage Maturity:
- current conclusion:
- provider coverage:
- evidence mode:
- provider evidence packet:
- evidence used:
- evaluation method:
- overall score:
- level:
- axis scores:
- fluency overlay:
- longitudinal comparison:
- evidence-to-score map:
- strengths:
- gaps:
- problem diagnosis:
- development edges:
- improvement plan:
- next upgrades:
- safety notes:
- verification needed:
```

## Stabilize A Release Cut

Prompt:

```text
Stop adding new capability and decide whether this package is ready to release.
```

Expected skill:

```text
stabilize-release-cut
```

Expected answer shape:

```text
GroundLine Release Cut:
- current conclusion:
- scope lock:
- change budget:
- must fix:
- defer:
- release gates:
- dogfood evidence:
- regression check:
- ship decision: ship|hold|continue
- next action:
```

## Hold Scope Before Expanding

Prompt:

```text
This looks useful. Also research more tools and add whatever else improves it.
```

Expected skill:

```text
hold-the-line
```

Expected answer shape:

```text
GroundLine Scope Hold:
- current conclusion:
- current work:
- expansion trigger:
- evidence:
- decision: finish_current|accept_with_budget|defer|watch|reject
- budget:
- non-goals:
- next action:
- verification:
- parked ideas:
```

## Polish A Release Candidate

Prompt:

```text
Before release, clean up docs, duplicate wording, privacy risk, and commit plan.
```

Expected skill:

```text
polish-release-candidate
```

Expected answer shape:

```text
GroundLine Release Polish:
- current conclusion:
- polish scope:
- cleanup findings:
- fix now:
- defer:
- watch:
- privacy sweep:
- identity sweep:
- gate order:
- commit split:
- verification:
- release readiness:
```

## Compare A Deployed Release

Prompt:

```text
Compare the deployed release with the previous version and give me the post-deploy checklist.
```

Expected skill:

```text
compare-release-delta
```

Expected answer shape:

```text
GroundLine Release Delta:
- current conclusion:
- previous version:
- deployed version:
- comparison source:
- delta checklist:
- expected changes:
- unexpected changes:
- runtime evidence:
- install evidence:
- regression evidence:
- rollback note:
- decision: keep|monitor|rollback|not_enough_evidence
- follow-up:
```

## Evaluate This Package

Prompt:

```text
Evaluate this GroundLine repo and tell me whether the skills are complete enough for public release.
```

Expected skill:

```text
evaluate-groundline-pack
```

Expected answer shape:

```text
GroundLine Pack Evaluation:
- current conclusion: PASS/PARTIAL/FAIL
- skill completeness: ...
- trigger clarity: ...
- verification strength: ...
- recommended fixes: ...
- release decision: ...
```

## Curate Skill Portfolio

Prompt:

```text
Review these GroundLine skill ideas and decide what should be created, merged, split, or rejected.
```

Expected skill:

```text
curate-groundline-skills
```

Expected answer shape:

```text
GroundLine Skill Curation:
- current conclusion:
- candidate:
- classification:
- decision: create|adapt|merge|split|deprecate|reject
- human-readable update:
- LLM-readable update:
- verification checklist:
```
