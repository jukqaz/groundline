# GroundLine 성숙도 평가

영어 `docs/maturity-assessment.md`가 canonical입니다. 이 문서는 사람이 빠르게
판단하기 위한 한국어 companion입니다.

## 결론

현재 성숙도는 85/100입니다.

GroundLine은 실사용 가능한 public beta입니다. Codex, Claude Code,
Antigravity에 설치 target이 있고 19개 skill이 보입니다. 현재 로컬 `0.3.5`
candidate는 provider target refresh를 마쳤고, fake-home remote install/update
proof로 fresh install, stale update detection, post-update refresh를
증명합니다. staged provider smoke, validation, lint, provider-native
validation, unit test, safety eval, privacy scan, offline doctor/radar, staged
dogfood, macOS local scenario, Linux Docker gate는 release 전 다시 확인해야
합니다.
다만 1.0 안정판이라고 부르기에는 live provider activation proof와 published-ref
install confirmation이 아직 부족합니다.

핵심 부족분은 기능 수가 아니라 운영 증거입니다.

- provider별 설치 상태 진단, fake refreshed install proof, remote install/update
  proof는 구현됐고, 현재 로컬 real provider target도 v0.3.5 기준 PASS
- activation matrix와 staged 6-scenario contract coverage는 구현됐지만, 실제
  provider session에서 어떤 skill이 선택되는지 sanitized proof가 더 필요함
- 12개 experimental skill은 graduate, keep experimental, merge, defer로 정리됐지만 실제 active promotion은 아직 보류됨
- 대표 workflow 5개는 cookbook으로 정리됐고, 실제 provider proof로 더 다듬어야 함
- research부터 post-release review까지 artifact lifecycle은 기존 skill과 output contract에 연결됨

## 축별 판단

| 축 | 판단 |
| --- | --- |
| repository readiness | PASS |
| skill completeness | PARTIAL |
| trigger clarity | PASS |
| context weight | PASS |
| workflow coverage | PARTIAL |
| verification strength | PARTIAL |
| security posture | PASS |
| provider install posture | PARTIAL |
| maintenance discipline | PASS |

## 다음 작업

1. P0: Version-aware install doctor

   provider별 설치된 GroundLine version, source ref, cache 상태, skill count,
   same-version content drift를 확인합니다. provider auth, session, log, raw
   home dump는 출력하지 않습니다.
   v0.3.3에 구현됐고, published ref install/update로 반복 확인해야 합니다.

2. P1: Real provider activation matrix

   handoff, side-effect guard, release cut, ecosystem evaluation, AI usage
   maturity prompt에서 실제 provider가 어떤 skill과 output contract를 선택하는지
   sanitized proof로 기록합니다. Matrix 문서와 staged coverage는 v0.3.3에
   들어갔고, side-effect guard, ecosystem evaluation, AI usage maturity의
   live proof row가 남았습니다.

3. P1: Skill graduation plan

   experimental skill 12개를 graduate, keep experimental, merge, defer로
   분류했습니다. `package-agent-task`와 `stabilize-release-cut`은 promotion
   후보이고, 실제 active 변경은 반복 설치 확인과 provider proof 뒤에 합니다.

4. P2: Workflow proof cookbook

   사람이 바로 따라 할 수 있는 end-to-end workflow 예시를 만듭니다. 각 예시는
   prompt, skill, output contract, verification, stop condition을 포함합니다.
   `docs/workflow-cookbook.md`와 `docs/ko/workflow-cookbook.md`에 compact
   version을 구현했습니다.

5. P2: Single-source version control

   patch version을 여러 파일에 하드코딩하지 않도록 canonical manifest 기준으로
   version 일치 여부를 검증합니다. v0.3.3에 구현됐고, 다음 version bump 때 release
   gate로 다시 증명해야 합니다.

## 1.0 기준

1.0은 다음 조건이 충족될 때 붙입니다.

- GitHub fresh install/update 뒤 세 provider의 install doctor가 PASS
- 최소 5개 real provider activation proof row가 있음
- active core와 experimental skill lifecycle이 정리됨
- version drift check가 자동화되고 다음 version bump에서도 유지됨
- fresh clone에서 validate, install, confirm이 prior cache 없이 재현됨

## Release decision

v0.3.5는 현재 local release candidate입니다. install posture, version drift
control, staged provider activation coverage, skill graduation decision, compact
workflow cookbook, artifact lifecycle을 유지하면서 fake-home remote
install/update proof를 추가했습니다. source와 packaged manifest는 `0.3.5`이고,
real provider target refresh도 로컬에서 확인했습니다. full local release gate는
PASS이며, published-ref proof는 public release 전 추가로 필요합니다.

현재 decision은 `continue`입니다. 다음 작업은 live activation proof를 수집하거나
explicit accepted partial로 분류하는 것이며, 새 skill은 activation matrix가 기존
skill로 표현할 수 없는 반복 실패를 증명할 때만 추가합니다.
