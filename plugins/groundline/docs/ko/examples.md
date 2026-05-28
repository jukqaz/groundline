# 예시 Workflow

영어 `docs/examples.md`가 canonical입니다. 이 문서는 대표 workflow를 짧게 고르는
한국어 companion입니다.

실제 session에서는 상황에 맞는 prompt를 그대로 복사해도 됩니다. skill 이름은
agent가 잘못 고를 때만 직접 지정하세요.

## 빠른 선택

| 상황 | 먼저 볼 예시 |
| --- | --- |
| 이전 agent나 stale handoff | 이전 agent 작업 이어받기 |
| 위험한 변경이나 provider home write | 위험한 작업 경계 잡기 |
| CI는 통과했지만 runtime 증거가 없음 | live work 닫기 |
| 긴 대화 handoff | task packet 만들기 |
| 아이디어가 계속 늘어남 | release scope 잠그기 |
| release 직전 | release scope 잠그기 |
| release 뒤 비교 | release 뒤 비교 |

## 1. 이전 agent 작업 이어받기

Prompt:

```text
이전 agent가 하던 작업을 현재 상태부터 다시 확인하고 이어서 마무리해줘.
```

Flow:

```text
reconcile-current-state -> guard-side-effects -> close-live-work
```

확인할 증거:

- branch와 dirty state
- 최근 commit과 CI
- 실제 runtime 또는 endpoint
- 남은 gap

## 2. Release scope 잠그기

Prompt:

```text
이제 더 확장하지 말고 release 가능한지 판단해줘.
```

Flow:

```text
hold-the-line -> polish-release-candidate -> stabilize-release-cut
```

확인할 증거:

- must fix, defer, reject 분류
- validation gate
- dogfood evidence
- ship decision

## 2-1. live work 닫기

Prompt:

```text
CI는 통과했는데 실제 runtime이나 endpoint에 반영됐는지 확인해줘.
```

Flow:

```text
close-live-work
```

확인할 증거:

- runtime, endpoint, release, queue, browser, user-flow 중 무엇을 봤는지
- `PASS`, `PARTIAL`, `FAIL`
- 남은 gap

## 3. 외부 agent workflow 조사

Prompt:

```text
비슷한 agent workflow 도구를 조사하고 GroundLine에 무엇을 흡수할지 추천해줘.
```

Flow:

```text
agent-ecosystem-radar -> research-agent-ecosystem -> compare-agent-workflows -> recommend-groundline-upgrades
```

결과는 adopt, adapt, watch, reject로 나눕니다.

## 4. AI 활용도 평가

Prompt:

```text
내 AI agent 활용 습관을 증거 기반으로 평가하고 개선점을 알려줘.
```

Skill:

```text
evaluate-ai-usage-maturity
```

주의:

- raw transcript를 기본으로 수집하지 않습니다.
- durable artifact와 provider evidence packet을 우선합니다.
- 문제점, 개선 계획, 다음 upgrade를 분리합니다.

## 5. 위험한 작업 경계 잡기

Prompt:

```text
배포하고 권한도 바꿔줘.
```

Skill:

```text
guard-side-effects
```

기대 출력:

```text
Boundary:
- classification:
- approval needed:
- intended side effect:
- read-only checks:
- execution allowed:
- secret value printed: false
```

## 6. release 뒤 비교

Prompt:

```text
방금 배포된 버전과 이전 버전을 비교해서 checklist와 regression risk를 정리해줘.
```

Skill:

```text
compare-release-delta
```

확인할 증거:

- previous version
- deployed version
- expected changes
- unexpected changes
- install/runtime/regression evidence
- rollback note
