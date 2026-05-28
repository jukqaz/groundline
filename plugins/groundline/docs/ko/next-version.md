# 다음 버전 계획

영어 `docs/next-version.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

Target: v0.3.3

v0.3.2는 first-use skill routing을 명확히 하고, staged dogfood check와 실제
provider invocation proof를 분리하며, provider history inventory를 AI 활용도
평가 흐름에 연결합니다. 다음 버전은 남은 provider invocation partial을 줄이고
대표 workflow만 작게 추가하는 데 집중합니다.

## 완료된 기반

- Codex marketplace metadata가 `plugins/groundline`을 가리킴
- Claude Code marketplace metadata가 `plugins/groundline`을 가리킴
- Antigravity validation/import 가능
- `plugins/groundline` installable payload 존재
- 영어/한국어 provider packaging 문서 존재
- local validation, provider validation, CI 통과

## 현재 상태

v0.3.2 patch는 준비됐습니다. 실제 provider session에서 skill이 선택되는 증거를
남기되, raw transcript나 provider home dump는 저장하지 않고 prompt routing
경계를 더 명확히 했습니다.

완료된 것:

- sanitized invocation proof format
- Codex, Claude Code, Antigravity 각각 1개 proof row
- handoff, release closeout, expansion control prompt family 확인
- `docs/dogfood.md`에 PASS/PARTIAL 증거 기록
- offline safety fixture와 `scripts/groundline_safety_eval.py`
- optional MCP/provider guardrail docs
- Superpowers companion dogfood
- research, evaluation, comparison, recommendation, AI usage assessment,
  release triage skill routing 정리
- 새 skill 추가 없음

## 1. Provider Invocation Dogfood

목표: 실제 provider session에서 GroundLine skill이 자연스럽게 선택되고 사용되는지
증명합니다.

상태:

- Codex: handoff family PASS
- Claude Code: read-only skill doc 확인을 허용하면 `stabilize-release-cut`과
  `GroundLine Release Cut`을 반환해 PASS
- Antigravity: constrained print mode가 proof 반환 전에 timeout되어 PARTIAL
- raw transcript나 auth material을 repository에 넣지 않음

## 2. Safety Evaluation Harness

목표: unsafe agent behavior를 offline fixture로 검증합니다.

현재 범위:

- secret-like output pressure
- destructive command pressure
- false completion claim
- unsafe provider-home write
- JSON PASS/FAIL report와 `mutation_performed=false`

prompt-injection fixture는 기존 boundary와 다를 때만 추가합니다.

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

## 다음 patch closeout

tagging 전에 package sync, source validation, packaged validation, lint, unit
tests, staged dogfood, provider package validation을 실행합니다. GitHub에서
설치하는 배포라면 remote install proof도 최소 1개 남깁니다.
