# 사람용 가이드

영어 `docs/human-guide.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

README로 개념은 알겠는데 실제 agent session에서 어떻게 써야 할지 막힐 때 이
문서를 봅니다.

## GroundLine이 하는 일

GroundLine은 agent 작업을 더 안전하고 검증 가능하게 만드는 스킬 패키지입니다.
핵심은 세 가지입니다.

- 현재 상태를 다시 확인한다.
- side effect와 승인 필요성을 분리한다.
- 완료 주장 전에 검증 증거를 남긴다.

## 머릿속 모델

GroundLine은 일하는 agent 자체가 아니라 운영 레이어입니다.

- provider는 tool 실행, permission, plugin/MCP 연결을 담당합니다.
- Superpowers나 provider 내장 기능은 planning, TDD, debugging, review를 담당할 수 있습니다.
- GroundLine은 현재 상태 증명, side effect 경계, handoff, release cut, live proof를 맡습니다.

기본 흐름:

```text
orient -> bound -> act -> prove -> polish -> cut -> compare
```

매번 전부 쓸 필요는 없습니다. 지금 가장 위험한 지점 하나만 골라 쓰면 됩니다.

## 첫 실행

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

기대값:

- 실제 provider home을 쓰지 않음
- `mutation_performed=false`
- `real_home_touched=false`
- 기본 home path는 `~`로 표시

실패하면 아직 provider에 설치하지 말고, 실패한 command와 sanitized output부터
확인합니다.

## 자주 쓰는 요청

| 상황 | 사용할 skill | 이유 |
| --- | --- | --- |
| 이전 agent 작업을 이어받음 | `reconcile-current-state` | stale handoff를 믿지 않게 함 |
| 위험한 변경, 배포, 권한 변경 | `guard-side-effects` | read-only와 mutation을 분리함 |
| 테스트는 통과했지만 runtime 증거가 필요함 | `close-live-work` | 실제 endpoint, release, user-flow를 확인함 |
| 대화가 길어져 다음 LLM에게 넘겨야 함 | `package-agent-task` | goal, constraint, non-goal, verification을 압축함 |
| 아이디어가 계속 늘어남 | `hold-the-line` | finish, defer, watch, reject를 결정함 |
| release 직전 polish가 필요함 | `polish-release-candidate` | docs, privacy, gate, commit shape를 정리함 |
| release scope를 잠그고 ship 여부를 판단함 | `stabilize-release-cut` | must fix와 defer를 분리하고 ship decision을 냄 |
| release 뒤 이전 버전과 비교함 | `compare-release-delta` | expected/unexpected change와 rollback note를 남김 |
| 외부 agent 도구를 조사하고 비교함 | `agent-ecosystem-radar` | research, compare, recommend를 묶음 |
| AI 활용 습관을 평가함 | `evaluate-ai-usage-maturity` | 증거 기반 개선 계획을 만듦 |

## 그대로 써도 되는 prompt

```text
현재 branch, dirty files, 최근 commit, runtime 증거부터 다시 확인하고 이어서 진행해줘.
```

```text
대화가 길어졌으니 다음 agent가 이어받을 수 있게 goal, constraints, non-goals, verification, handoff를 task packet으로 정리해줘.
```

```text
실행 전에 side effect를 먼저 분류해줘. read-only인지, local file 변경인지, remote/provider home 변경인지, 승인이 필요한지 말해줘.
```

```text
테스트 통과 말고 실제 runtime, install, release, user-flow 증거로 완료 여부를 판단해줘.
```

```text
이 release는 더 확장하지 말고 must fix, defer, watch, reject로 나눈 뒤 ship 또는 hold를 결정해줘.
```

## 결과 읽는 법

- `PASS`: 필요한 증거를 확인했고 기준을 만족함
- `PARTIAL`: 일부 증거는 있지만 중요한 gap이 남음
- `FAIL`: 기준을 만족하지 못했거나 검증이 실패함

좋은 결과에는 다음이 있어야 합니다.

- 확인한 것
- 확인하지 못한 것
- mutation 여부
- secret value 출력 여부
- 다음 safe action

## 종료 기준

작업을 끝낼 때는 다음을 같이 남깁니다.

- PASS/PARTIAL/FAIL
- 확인한 branch, runtime, CI, release, endpoint, file
- 실행한 command
- 남은 gap
- side effect 여부
