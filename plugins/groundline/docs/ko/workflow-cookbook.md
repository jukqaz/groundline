# Workflow Cookbook

이 문서는 사람이든 LLM agent든 전체 skill index를 다 읽지 않고도 바로
GroundLine 흐름을 고를 수 있게 만든 요약 cookbook입니다.

각 항목은 다음 형태를 가집니다.

```text
prompt -> selected skill -> output contract -> verification evidence -> stop condition
```

## Handoff Recovery

Trigger: 이전 agent 작업, 긴 thread, 오래된 summary, 중단된 branch를 이어야 할 때.

Prompt:

```text
현재 상태를 다시 확인한 뒤, 다음 agent가 추측 없이 이어갈 수 있게 목표와 검증 기준을 패키징해줘.
```

Selected skill: `reconcile-current-state -> package-agent-task`

Expected output contract: `GroundLine Task Packet`

Verification evidence:

- 현재 branch와 dirty files
- 관련 changed files 또는 commits
- constraints와 non-goals
- 이미 실행한 command와 결과
- 아직 검증되지 않은 gap

Stop condition: 새 agent가 전체 transcript를 다시 읽지 않아도 이어갈 수
있고, raw secret이나 provider home dump가 포함되지 않은 상태입니다.

## Side-Effect Guard

Trigger: 파일, git, remote, production, money, access, provider home, secret,
user data가 바뀔 수 있을 때.

Prompt:

```text
실행 전에 side effect를 분류해줘. read-only 확인, local write, external mutation, approval-required 작업을 나눠줘.
```

Selected skill: `guard-side-effects`

Expected output contract: `Boundary`

Verification evidence:

- action inventory
- mutation boundary
- approval requirements
- blocked 또는 deferred actions
- 안전한 read-only command

Stop condition: agent가 무엇을 안전하게 확인할 수 있는지 명확히 말하고,
사용자 승인 전에는 approval-required mutation을 하지 않는 상태입니다.

## Release Cut

Trigger: 변경이 계속 커지거나, 테스트는 통과했지만 ship/hold/continue
판단이 필요할 때.

Prompt:

```text
이번 release cut을 잠가줘. must fix, defer, watch, reject를 분류하고 ship decision에 필요한 gate를 실행해줘.
```

Selected skill: `stabilize-release-cut`

Expected output contract: `GroundLine Release Cut`

Verification evidence:

- 현재 branch와 changed surface
- scope lock
- must-fix list와 change budget
- validation, lint, test, scenario, provider smoke, dogfood evidence
- 아직 부족한 evidence

Stop condition: output이 `ship`, `hold`, `continue` 중 하나를 gate evidence와
함께 말하고, release-blocking evidence 없이 새 capability를 추가하지 않는 상태입니다.

## Ecosystem Radar

Trigger: 새 agent tool, skill pack, MCP server, hook, provider feature,
workflow framework를 도입할지 판단해야 할 때.

Prompt:

```text
현재 source-backed 후보를 조사하고 GroundLine과 비교한 뒤, adopt, adapt, watch, reject 중 하나로 추천해줘. 이 release scope는 키우지 마.
```

Selected skill: `agent-ecosystem-radar`

Expected output contract: `GroundLine Research`, `GroundLine Capability Evaluation`,
`GroundLine Comparison`, `GroundLine Recommendation`

Verification evidence:

- 날짜나 retrieval note가 있는 source list
- confirmed facts와 unverified claims
- GroundLine scope와의 비교
- context/setup cost
- adopt, adapt, watch, reject decisions

Stop condition: 추천이 bounded next-work item으로만 남고, watch/reject 항목이
즉시 구현 작업으로 번지지 않는 상태입니다.

## AI Usage Maturity Review

Trigger: raw transcript 없이 개인이나 팀의 AI 활용도를 평가하고 싶을 때.

Prompt:

```text
redacted artifact를 기반으로 AI usage maturity를 평가해줘. 강점, gap, scoring evidence, 다음 upgrade를 알려줘.
```

Selected skill: `audit-agent-history -> evaluate-ai-usage-maturity`

Expected output contract: `GroundLine AI Usage Maturity`

Verification evidence:

- redacted Provider Evidence Packet
- 검토한 artifact types
- scoring rubric
- strengths and gaps
- 기존 skill 또는 docs로 이어지는 improvement actions

Stop condition: raw message history, auth material, provider runtime state를
repository에 저장하지 않고 실행 가능한 다음 upgrade가 나오는 상태입니다.
