# Skill Portfolio

영어 `docs/skill-portfolio.md`와 `references/skill-index.json`이 canonical입니다.
이 문서는 사람이 빠르게 훑기 위한 한국어 companion입니다.

## 핵심 active skills

| Skill | 언제 쓰나 |
| --- | --- |
| `reconcile-current-state` | 이전 agent 작업, stale handoff, 현재 branch/runtime/CI 증명이 필요할 때 |
| `audit-agent-history` | agent history에서 반복 패턴과 개선점, AI 활용도 평가용 redacted evidence packet을 만들 때 |
| `guard-side-effects` | 파일, git, remote, production, access, secret, provider home을 건드릴 수 있을 때 |
| `close-live-work` | 테스트 통과 뒤에도 실제 runtime, endpoint, release 증거가 필요할 때 |
| `align-agent-home` | provider home, rules, hooks, skill, plugin 경계를 정리할 때 |
| `recover-worktree-branch` | worktree나 branch 상태가 꼬였거나 cleanup 전 증명이 필요할 때 |
| `evaluate-groundline-pack` | GroundLine 자체의 완성도와 release 준비도를 볼 때 |

## 실험적이지만 유용한 skills

| Skill | 언제 쓰나 |
| --- | --- |
| `agent-ecosystem-radar` | 외부 agent workflow를 조사, 평가, 비교, 추천까지 한 번에 묶을 때 |
| `research-agent-ecosystem` | source-backed 후보 수집만 필요할 때 |
| `compare-agent-workflows` | 두 개 이상의 후보를 GroundLine scope, safety, setup cost 기준으로 비교할 때 |
| `recommend-groundline-upgrades` | 이미 있는 조사/비교 결과를 adopt, adapt, watch, reject 결정으로 바꿀 때 |
| `evaluate-agent-capability` | 하나의 tool, skill, plugin, MCP server, hook, agent를 도입 전에 평가할 때 |
| `evaluate-ai-usage-maturity` | 개인이나 팀의 AI 활용도를 artifact나 redacted Provider Evidence Packet으로 평가할 때 |
| `package-agent-task` | 큰 작업이나 긴 대화를 다음 LLM에게 넘길 task packet으로 만들 때 |
| `hold-the-line` | 현재 목표가 끝나기 전에 scope가 커질 때 |
| `polish-release-candidate` | release 직전 docs, privacy, gate, commit split을 정리할 때 |
| `stabilize-release-cut` | scope lock, dogfood, regression, ship decision이 필요할 때 |
| `compare-release-delta` | 배포 뒤 이전 버전과 비교하고 rollback note를 남길 때 |
| `curate-groundline-skills` | skill을 만들지, 합칠지, 나눌지, 거절할지 결정할 때 |

## 선택 기준

- 먼저 current state를 증명합니다.
- side effect가 있으면 `guard-side-effects`를 먼저 씁니다.
- release 전에는 `polish-release-candidate`와 `stabilize-release-cut`을 씁니다.
- release 뒤에는 `compare-release-delta`로 확인합니다.
- skill을 늘리기 전에는 `curate-groundline-skills`로 중복을 막습니다.
- provider history를 근거로 AI 활용도를 평가할 때는
  `audit-agent-history -> evaluate-ai-usage-maturity` 순서로 씁니다.
