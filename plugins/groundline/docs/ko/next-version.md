# 다음 버전 계획

영어 `docs/next-version.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

Target: v0.3.5

v0.3.3은 install posture, version drift control, compact proof workflow,
release gate를 닫았습니다. 현재 후보는 v0.3.4의 local proof-quality 작업을
포함하면서, 원격 설치와 업데이트 proof를 v0.3.5의 핵심 경계로 추가합니다.

## 완료된 기반

- Codex marketplace metadata가 `plugins/groundline`을 가리킴
- Claude Code marketplace metadata가 `plugins/groundline`을 가리킴
- Antigravity validation/import 가능
- `plugins/groundline` installable payload 존재
- 영어/한국어 provider packaging 문서 존재
- v0.3.3은 `main`에서 tag와 GitHub Release까지 완료됐고 source/package
  validation, provider-native validation, staged dogfood, staged provider smoke,
  scenario evidence, release-version preflight, remote CI proof를 갖췄습니다.
- real provider smoke는 이 단계의 install confirmation check입니다. 설치된
  provider target이 현재 source payload와 같을 때만 PASS이고, release 뒤 source
  content가 바뀌어 same-version install target이 stale이면 refresh action과 함께
  PARTIAL을 냅니다. v0.3.3 tag/release는 더 이상 pending이 아닙니다.

## 현재 상태

v0.3.3은 release와 로컬 설치 확인이 완료된 현재 baseline입니다. 다음 작업은 실제
GitHub 가이드 설치 과정에서 드러날 수 있는 provider별 설치 상태와 cache/version
drift를 반복 확인하고, live activation proof를 늘리는 방향입니다.

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
- staged provider smoke가 `--stage-package --require-installed`로 fake refreshed
  install을 증명
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
- release gate는 `--release-version`을 받아 실제 release cut에서 source 또는
  packaged manifest가 이전 public version에 남아 있거나 요청 version이 plain
  `X.Y.Z` semver가 아니면 실패
- staged provider smoke는 PASS이고, real provider smoke는 same-version source
  content 변경 뒤 설치 target refresh 여부를 보는 gate
- 명시적인 `--release-version 0.3.3` preflight는 released manifest와 package sync
  guard 역할
- 새 skill 추가 없음

## v0.3.5 release cut

결론: v0.3.5는 remote install/update proof patch입니다. public package를 fresh
install했을 때 정상인지, version bump 뒤 이전 설치본이 stale로 감지되는지, refresh
후 다시 PASS로 돌아오는지를 실제 provider home 없이 fake-home에서 증명합니다.

범위:

- v0.3.4의 local proof-quality evidence를 포함합니다.
- `groundline_remote_install_probe.py`로 Codex, Claude Code, Antigravity의 fresh
  install, previous-version detection, post-update refresh를 증명합니다.
- release gate, install/update docs, release checklist에 이 proof를 포함합니다.
- live provider CLI activation proof는 외부 provider runtime에 current worktree
  context를 보낼 수 있으므로 별도 명시 승인 대상으로 둡니다.

must fix:

- `groundline_remote_install_probe.py --json`이 `status=PASS`를 반환하고
  `fresh_install`, `stale_update_detection`, `post_update_refresh`를 모두 증명
- release gate가 `--release-version 0.3.5`에서 remote install/update proof를 포함
- source와 packaged manifest가 package sync 뒤 `0.3.5`로 일치
- provider smoke의 예상 밖 `FAIL` 없음
- public update confidence를 주장하기 전에 install/update 문서가 새 proof를 안내

현재 상태:

- source와 packaged manifest는 `0.3.5`
- `groundline_remote_install_probe.py --json`은 fake home에서 fresh install,
  stale update detection, post-update refresh를 증명하며 `PASS`
- Codex, Claude Code, Antigravity local provider target은 `0.3.5` package로 refresh
  되었고 provider smoke는 `PASS`
- `groundline_release_gate.py --json --keep-going --include-docker-execution
  --release-version 0.3.5`는 `PASS`

Ship decision: live provider activation proof와 published-ref proof를 수집하거나
explicit accepted partial로 분류할 때까지 `continue`입니다.

## v0.3.4 local proof-quality work

결론: v0.3.4는 capability 확장이 아니라 proof-quality patch입니다. published
v0.3.3 package를 refresh할 수 있고, 실제 provider session이 의도한 GroundLine
skill을 고르며, 이전 release와 비교 가능한지를 증명하는 데 집중합니다.

범위:

- Codex/Claude Code provider install refresh와 확인. 현재 로컬 direct target은
  `0.3.4` package와 일치하고 provider smoke는 PASS입니다.
- `docs/provider-activation-matrix.md`의 5개 prompt family live proof 정리
- published v0.3.3 baseline에 대한 release delta review
- v0.3.4 release gate, privacy scan, staged provider smoke, staged dogfood,
  provider smoke, scenario check, remote CI, GitHub Release proof

must fix:

- `groundline_provider_smoke.py --json --require-installed`에서 예상 밖 `FAIL`이
  없어야 합니다. same-version `content_fingerprint_mismatch`는 provider target
  refresh로 닫거나 explicit accepted partial과 `next_actions`로 남깁니다.
- `side-effect-guard`, `ecosystem-evaluation`, `ai-usage-maturity` matrix row는
  더 이상 `PENDING`이면 안 됩니다. `PASS` 또는 sanitized evidence가 있는 explicit
  `PARTIAL`이어야 합니다.
- `docs/dogfood.md`에는 sanitized invocation proof만 남기고 raw transcript,
  auth material, provider cache dump, full home path는 넣지 않습니다.
- `compare-release-delta`로 v0.3.3 baseline 대비 짧은 release delta judgment를
  남기거나, 비교 가능한 install evidence 부재를 partial로 명시합니다.
- manifest를 `0.3.4`로 함께 올리고 package sync 뒤
  `--release-version 0.3.4` gate를 통과해야 합니다.

defer:

- `package-agent-task`, `stabilize-release-cut`, `evaluate-agent-capability`의
  active promotion
- official catalog polish, screenshot/marketplace media, broad ecosystem refresh,
  optional MCP/hooks, provider-level agents, automatic provider-home install

reject:

- 새 skill, 새 provider runtime, default hook/MCP, raw transcript analytics,
  fresh activation proof 없는 lifecycle promotion

ship decision: provider refresh evidence와 activation matrix row가 닫힐 때까지
`continue`입니다. 그 뒤 v0.3.4는 좁은 patch로 release하고 stable-core promotion은
다음 patch로 넘깁니다.

현재 harness 결과:

- source와 packaged manifest는 `0.3.4`
- Codex/Claude Code direct provider target은 현재 packaged payload로 refresh됨
- `groundline_provider_smoke.py --json --require-installed`: `status=PASS`,
  `install_doctor_status=PASS`, `next_actions=[]`
- `groundline_release_gate.py --json --keep-going --include-docker-execution
  --release-version 0.3.4`: `status=PASS`

release delta 판단:

- previous version: `v0.3.3`
- candidate version: local `0.3.4`
- expected changes: manifest bump, post-release planning refresh, local provider
  target refresh evidence, proof-quality release cut docs
- unexpected changes: source diff에서 발견되지 않음. package copy 변경은 source
  payload mirror입니다.
- runtime/install evidence: full local release gate와 provider smoke가 PASS
- decision: live provider activation proof row를 수집하거나 explicit accepted
  partial로 분류할 때까지 monitor/continue

## 1. Install posture와 version drift

목표: GitHub 설치 뒤 provider별 GroundLine 설치 상태를 버전까지 명확히 확인합니다.

완료 기준:

- Codex, Claude Code, Antigravity 설치 상태를 확인하는 version-aware install doctor:
  `scripts/groundline_provider_smoke.py`에 구현
- source ref drift, stale cache, package payload 누락, skill count mismatch 탐지:
  fake-home unit test 추가
- fake refreshed install을 위한 staged provider smoke:
  `scripts/groundline_provider_smoke.py`에 구현
- provider auth, session, log, raw home dump 출력 금지
- hard-coded patch version 대신 canonical manifest 비교:
  `scripts/validate_pack.py`에 구현
- 실제 release cut에서는 `--release-version`으로 source와 packaged manifest가
  모두 의도한 version인지 tag 생성 전에 확인
- real provider install refresh를 주장하기 전 staged provider smoke가 PASS

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

상태: v0.3.3에 compact cookbook으로 구현했습니다. 실제 provider proof에서 헷갈리는
workflow가 확인될 때만 확장합니다.

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

상태: v0.3.3에 compact lifecycle map으로 구현했습니다.

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
`content_fingerprint_mismatch`를 보고하면, pushed package로 target을 새로
설치하기 전까지 full closeout은 PARTIAL로 봅니다. v0.3.3은 이미 tag/release됐으므로
다음 blocker는 published ref install confirmation을 반복하고, 1.0 표현 전 최소
5개 real activation proof row를 확보하는 것입니다.
