# 다음 버전 계획

영어 `docs/next-version.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

Target: v0.3.3

v0.3.2는 first-use skill routing을 명확히 하고, staged dogfood check와 실제
provider invocation proof를 분리하며, provider history inventory를 AI 활용도
평가 흐름에 연결합니다. 다음 버전은 skill surface를 넓히기보다 install posture,
version drift control, compact proof workflow에 집중합니다.

## 완료된 기반

- Codex marketplace metadata가 `plugins/groundline`을 가리킴
- Claude Code marketplace metadata가 `plugins/groundline`을 가리킴
- Antigravity validation/import 가능
- `plugins/groundline` installable payload 존재
- 영어/한국어 provider packaging 문서 존재
- v0.3.2 baseline은 `main`에서 local validation, provider validation, CI를
  통과했고, 현재 patch draft는 local validation, provider-native validation,
  staged dogfood, scenario evidence가 통과합니다. full release gate는 provider
  install이 published ref에서 refresh되기 전까지 PARTIAL입니다. tag는 명시적인
  ship decision 뒤에만 진행합니다.

## 현재 상태

v0.3.2는 release와 로컬 설치가 완료됐습니다. 현재 v0.3.3 patch draft는 실제
GitHub 가이드 설치 과정에서 드러난 provider별 설치 상태와 cache/version drift
문제를 먼저 닫는 방향입니다.

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
- `docs/maturity-assessment.md`에 85/100 public beta 평가와 다음 작업 기록
- provider smoke가 source version, installed version, payload 존재 여부,
  skill count drift, same-version content drift, `install_doctor_status`를 보고
- validation이 hard-coded patch version 대신 canonical `plugin.json` 기준으로
  provider manifest version을 비교
- `docs/provider-activation-matrix.md`가 다섯 live prompt family를 정의하고
  staged contract coverage와 real proof row를 분리
- staged provider dogfood가 side-effect guard, ecosystem evaluation, AI usage
  maturity까지 포함한 6개 scenario contract를 확인
- `docs/skill-graduation-plan.md`에 12개 experimental skill의 graduate,
  keep experimental, merge, defer 결정 기록
- `references/skill-index.json`에 experimental skill별 machine-readable
  graduation decision과 rationale 추가
- `docs/workflow-cookbook.md`에 5개 대표 prompt, selected skill, output
  contract, verification evidence, stop condition 추가
- `docs/artifact-lifecycle.md`에 research, comparison, recommendation,
  implementation, dogfood, release, post-release review artifact 흐름 추가
- `scripts/groundline_release_gate.py`가 local release gate 순서를 출력하거나
  실행하고 승인 필요한 publish 명령은 제외
- 새 skill 추가 없음

## 1. Install posture와 version drift

목표: GitHub 설치 뒤 provider별 GroundLine 설치 상태를 버전까지 명확히 확인합니다.

완료 기준:

- Codex, Claude Code, Antigravity 설치 상태를 확인하는 version-aware install doctor:
  `scripts/groundline_provider_smoke.py`에 구현
- source ref drift, stale cache, package payload 누락, skill count mismatch 탐지:
  fake-home unit test 추가
- provider auth, session, log, raw home dump 출력 금지
- hard-coded patch version 대신 canonical manifest 비교:
  `scripts/validate_pack.py`에 구현

## 2. Provider Invocation Dogfood

목표: 실제 provider session에서 GroundLine skill이 자연스럽게 선택되고 사용되는지
증명합니다.

상태:

- Codex: handoff family PASS
- Claude Code: read-only skill doc 확인을 허용하면 `stabilize-release-cut`과
  `GroundLine Release Cut`을 반환해 PASS
- Antigravity: constrained print mode가 proof 반환 전에 timeout되어 PARTIAL
- raw transcript나 auth material을 repository에 넣지 않음
- staged contract coverage는 handoff, side-effect guard, release cut,
  ecosystem evaluation, AI usage maturity, completion proof를 세 provider에서 확인

## 3. Skill graduation

목표: 12개 experimental skill을 stable core로 올릴 것과 보류할 것으로 나눕니다.

완료 기준:

- graduate, keep experimental, merge, defer 분류: 구현됨
- docs, examples, output contract, tests, dogfood evidence가 있는 skill만 active로 승격:
  promotion rule로 기록했고 이번 patch에서는 lifecycle 값을 바로 바꾸지 않음
- 새 skill 추가 금지: 유지
- lifecycle 결정은 `docs/skill-portfolio.md`와 `references/skill-index.json`에 반영: 구현됨

현재 결정:

- `package-agent-task`, `stabilize-release-cut`: graduate 후보
- `agent-ecosystem-radar`: merge 후보
- `compare-release-delta`: post-release evidence 전까지 defer
- 나머지 experimental skill: keep experimental

## 4. Workflow Cookbook

목표: 처음 쓰는 사람이 skill index 전체를 읽지 않아도 대표 workflow를 고를 수 있게
합니다.

상태: 현재 patch draft에 compact cookbook으로 구현했습니다. 실제 provider proof에서
헷갈리는 workflow가 확인될 때만 확장합니다.

완료 기준:

- handoff recovery: 구현됨
- release cut: 구현됨
- ecosystem radar: 구현됨
- AI usage maturity review: 구현됨
- side-effect guarding: 구현됨

각 workflow는 prompt, skill, output contract, verification을 함께 보여야 합니다.

## 5. Artifact Lifecycle

목표: research, comparison, recommendation, implementation, dogfood, release,
post-release review 사이의 artifact 흐름을 명확히 합니다.

상태: 현재 patch draft에 compact lifecycle map으로 구현했습니다.

완료 기준:

- research packet, comparison report, upgrade decision, release cut, release
  delta template가 있음: 구현됨
- 각 template가 하나의 skill과 output contract에 연결됨: 구현됨
- provider-native 기능을 중복 구현하지 않는다는 거절 규칙이 있음: 구현됨

## 다음 patch closeout

tagging 전에 package sync, source validation, packaged validation, lint,
provider-native validation, unit tests, offline doctor, offline radar,
safety eval, privacy scan, provider smoke, staged dogfood, macOS local
scenario, Linux Docker dry-run, Linux Docker execution을 실행합니다.
GitHub에서 설치하는 배포라면 remote install proof도 최소 1개 남깁니다.
provider smoke가 같은 version의 local target에서
`content_fingerprint_mismatch`를 보고하면, published ref로 target을 새로
설치하기 전까지 full closeout은 PARTIAL로 봅니다.
