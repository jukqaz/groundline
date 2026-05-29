# GroundLine

한국어 companion 문서입니다. 영어 `README.md`가 기본이자 canonical 문서입니다.

GroundLine은 AI coding agent가 자주 실수하는 지점에서 속도를 줄이게 하는
스킬 패키지입니다. 이전 작업을 이어받을 때, 위험한 변경을 할 때, 검증 없이
완료를 말하려 할 때, release scope가 계속 커질 때 사용합니다.

GroundLine은 Codex, Claude Code, Antigravity에서 쓰는 가벼운 control plane
스킬 패키지입니다. 설정 전체를 동기화하거나 provider 기능을 다시 만드는 도구가
아니라, agent 작업을 이어받고, 현재 상태를 증명하고, 위험한 변경을 구분하고,
검증 증거를 남기도록 돕습니다.

## 지원 범위

- Codex
- Claude Code
- Antigravity
- macOS on Apple Silicon
- Linux

## 빠른 시작

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_remote_install_probe.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

기본 검증은 실제 provider home을 수정하지 않습니다. 출력에서
`mutation_performed=false`와 `real_home_touched=false`를 확인하세요.
release 직전에는 `docs/ko/release-checklist.md`에 있는 전체 gate를 실행하세요.

## 무엇이 바뀌나

- skill, reference, docs, validation script를 제공합니다.
- hooks, rules, MCP server, command, provider-level agent는 기본으로 설치하지 않습니다.
- raw transcript, credential, provider runtime state를 repository에 넣지 않습니다.
- live GitHub, 최신 문서, private docs, private code search가 필요할 때만 optional MCP를 권합니다.

## 언제 쓰나

- 이전 agent 작업을 이어받기 전에 branch, runtime, CI, endpoint 상태를 다시 확인할 때
- 배포나 권한 변경처럼 side effect가 큰 작업의 승인 경계를 정할 때
- 테스트 통과 뒤에도 실제 runtime, release, endpoint 증거가 필요할 때
- 긴 대화나 큰 작업을 다음 LLM이 이어받을 수 있게 task packet으로 압축할 때
- release 직전에 privacy, docs, gate, dogfood evidence를 정리할 때

## 첫 요청 예시

| 상황 | 이렇게 요청 |
| --- | --- |
| 이전 작업 이어받기 | `현재 branch와 diff부터 확인하고 이어서 마무리해줘.` |
| 긴 대화 넘기기 | `다음 agent가 이어받을 수 있게 task packet으로 정리해줘.` |
| scope가 커짐 | `지금 release에 넣을 것과 미룰 것을 분류해줘.` |
| 완료 검증 | `테스트 말고 실제 runtime 증거까지 확인해줘.` |
| 위험한 변경 | `side effect와 승인 필요성을 먼저 분류해줘.` |
| release 직전 | `release candidate를 polish하고 ship 여부를 판단해줘.` |

## 어디를 읽나

- 처음 쓰는 사람: `docs/ko/human-guide.md`
- 설치: `docs/ko/install.md`
- workflow 예시: `docs/ko/examples.md`
- workflow cookbook: `docs/ko/workflow-cookbook.md`
- artifact lifecycle: `docs/ko/artifact-lifecycle.md`
- skill 선택: `docs/ko/skill-portfolio.md`
- skill 졸업 결정: `docs/ko/skill-graduation-plan.md`
- provider 설치: `docs/ko/provider-packaging.md`
- provider activation proof: `docs/ko/provider-activation-matrix.md`

## 한국어 문서

- 한국어 문서 인덱스: `docs/ko/index.md`
- 설치: `docs/ko/install.md`
- 업데이트: `docs/ko/update.md`
- 사람용 가이드: `docs/ko/human-guide.md`
- 스킬 포트폴리오: `docs/ko/skill-portfolio.md`
- 스킬 졸업 계획: `docs/ko/skill-graduation-plan.md`
- 예시 workflow: `docs/ko/examples.md`
- workflow cookbook: `docs/ko/workflow-cookbook.md`
- artifact lifecycle: `docs/ko/artifact-lifecycle.md`
- privacy: `docs/ko/privacy.md`
- 공개 전 확인: `docs/ko/public-release.md`
- release checklist: `docs/ko/release-checklist.md`
- 다음 버전 계획: `docs/ko/next-version.md`
- provider 패키징: `docs/ko/provider-packaging.md`
- provider activation matrix: `docs/ko/provider-activation-matrix.md`
- 이용 조건: `docs/ko/terms.md`

LLM용 reference, output contract, skill 본문은 영어를 기준으로 유지합니다.

## Marketplace 설치

```bash
codex plugin marketplace add jukqaz/groundline --ref main
codex plugin add groundline@groundline
```

```bash
claude plugin marketplace add jukqaz/groundline
claude plugin install groundline@groundline
```

```bash
agy plugin install https://github.com/jukqaz/groundline
```

공식 catalog 등재 여부와 직접 설치 가능 여부는 분리해서 판단합니다.
세부 가이드는 `docs/ko/provider-packaging.md`를 보세요.
