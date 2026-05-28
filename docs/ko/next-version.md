# 다음 버전 계획

영어 `docs/next-version.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

Target: v0.3.0

다음 버전은 skill 수를 먼저 늘리는 것보다, 다른 사용자가 믿고 설치하고 사용할 수
있는 증거를 강화하는 데 집중합니다.

## 1. Provider Invocation Dogfood

목표: 실제 provider session에서 GroundLine skill이 자연스럽게 선택되고 사용되는지
증명합니다.

완료 기준:

- Codex, Claude Code, Antigravity 각각 sanitized invocation note가 있음
- handoff, release closeout, expansion control prompt를 확인함
- `docs/dogfood.md`에 PASS/PARTIAL/FAIL 증거를 남김
- raw transcript나 auth material을 repository에 넣지 않음

## 2. Safety Evaluation Harness

목표: unsafe agent behavior를 offline fixture로 검증합니다.

완료 기준:

- secret leakage, destructive command pressure, prompt injection, false
  completion claim, unsafe provider-home write fixture가 있음
- JSON PASS/PARTIAL/FAIL report를 출력함
- deterministic해진 뒤 CI gate에 연결함

## 3. Workflow Cookbook

목표: 처음 쓰는 사람이 skill index 전체를 읽지 않아도 대표 workflow를 고를 수 있게
합니다.

완료 기준:

- handoff recovery
- release cut
- ecosystem radar
- AI usage maturity review
- side-effect guarding

각 workflow는 prompt, skill, output contract, verification을 함께 보여야 합니다.

## 4. Artifact Lifecycle

목표: research, comparison, recommendation, implementation, dogfood, release,
post-release review 사이의 artifact 흐름을 명확히 합니다.

완료 기준:

- research packet, comparison report, upgrade decision, release cut, release
  delta template가 있음
- 각 template가 하나의 skill과 output contract에 연결됨
- provider-native 기능을 중복 구현하지 않는다는 거절 규칙이 있음

## 먼저 할 일

Provider invocation dogfood부터 시작합니다. 이 증거 형식이 안정되기 전에는 safety
harness를 크게 키우지 않습니다.
