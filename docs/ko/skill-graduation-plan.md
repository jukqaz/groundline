# 스킬 졸업 계획

GroundLine은 현재 active skill 7개와 experimental skill 12개를 가지고
있습니다. v0.3 라인에서는 허용 가능한 상태지만, 안정된 core라고 부르기
전에는 각 experimental skill의 졸업 판단이 필요합니다.

## 졸업 기준

experimental skill을 active로 올리려면 다음 증거가 맞아야 합니다.

- `SKILL.md`의 trigger 문구
- `references/skill-index.json`의 metadata
- `docs/skill-portfolio.md`의 사람용 설명
- 일반 사용을 보여주는 examples 또는 copyable prompt
- 결과 형태를 고정하는 output contracts
- 실제 provider scenario에서 얻은 dogfood evidence
- privacy, side effect, context cost 문제가 남아있지 않음

## Graduation Decisions

| Skill | Decision | Rationale | Next proof |
| --- | --- | --- | --- |
| `agent-ecosystem-radar` | merge | 한 번에 실행하는 radar는 유용하지만 research, compare, recommend 흐름과 겹칩니다. | orchestrator 하나가 좋은지 작은 skill 조합이 좋은지 provider routing으로 확인합니다. |
| `research-agent-ecosystem` | keep experimental | source gathering 경계는 좋지만 실제 repo와 문서 기반 examples가 더 필요합니다. | source date와 제외 대상을 포함한 sanitized research packet을 추가합니다. |
| `compare-agent-workflows` | keep experimental | 비교 기준은 유용하지만 rejected/watch 사례가 더 필요합니다. | overlap, context cost, unsupported claim을 포함한 비교표를 추가합니다. |
| `recommend-groundline-upgrades` | keep experimental | 추천이 곧 scope growth가 되지 않도록 adopt, adapt, watch, reject 결과가 더 필요합니다. | 다음 ecosystem pass에서 네 가지 판단 결과를 기록합니다. |
| `evaluate-agent-capability` | graduate | 단일 후보 평가로 범위가 명확하고 read-only이며 provider dogfood에도 포함됐습니다. | public tool 하나에 대한 worked example을 추가합니다. |
| `evaluate-ai-usage-maturity` | keep experimental | 평가 축은 유용하지만 secret-sensitive evidence 처리가 더 검증되어야 합니다. | raw transcript나 secret 없는 Provider Evidence Packet 예시를 추가합니다. |
| `package-agent-task` | graduate | long-context handoff의 핵심이고 packet contract가 안정적입니다. | fresh install provider invocation이 packet contract로 이어지는지 확인합니다. |
| `hold-the-line` | keep experimental | scope control은 반복적으로 유용하지만 finish, defer, watch, reject 예시가 더 필요합니다. | scope를 자르는 cookbook prompt를 추가합니다. |
| `polish-release-candidate` | keep experimental | release polishing은 유용하지만 `stabilize-release-cut`과 경계가 겹칩니다. | release-cut을 중복하지 않는 pre-release cleanup 예시를 추가합니다. |
| `stabilize-release-cut` | graduate | gate 순서와 ship/hold/continue contract가 명확하고 현재 package dogfood에 쓰였습니다. | package sync 뒤 validation, dogfood, provider smoke, Docker evidence를 계속 통과시킵니다. |
| `compare-release-delta` | defer | 실제 배포된 patch와 install-after-release 증거가 필요합니다. | v0.3.3 배포 후 이전 설치 버전과 비교합니다. |
| `curate-groundline-skills` | keep experimental | portfolio 정리는 유용하지만 한 번의 release cycle에서 판단을 적용한 뒤 active 여부를 정해야 합니다. | 다음 patch 뒤 이 계획을 다시 실행하고 증거 없는 skill은 merge 또는 defer합니다. |

## Release Rule

이 계획이 열려 있는 동안 새 experimental skill을 추가하지 않습니다. 먼저
examples, output contracts, dogfood evidence, provider activation proof를
강화합니다.
