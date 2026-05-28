# GroundLine

한국어 companion 문서입니다. 영어 `README.md`가 기본이자 canonical 문서입니다.

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
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

기본 검증은 실제 provider home을 수정하지 않습니다. 출력에서
`mutation_performed=false`와 `real_home_touched=false`를 확인하세요.

## 언제 쓰나

- 이전 agent 작업을 이어받기 전에 branch, runtime, CI, endpoint 상태를 다시 확인할 때
- 배포나 권한 변경처럼 side effect가 큰 작업의 승인 경계를 정할 때
- 테스트 통과 뒤에도 실제 runtime, release, endpoint 증거가 필요할 때
- 긴 대화나 큰 작업을 다음 LLM이 이어받을 수 있게 task packet으로 압축할 때
- release 직전에 privacy, docs, gate, dogfood evidence를 정리할 때

## 한국어 문서

- 한국어 문서 인덱스: `docs/ko/index.md`
- 설치: `docs/ko/install.md`
- 업데이트: `docs/ko/update.md`
- 사람용 가이드: `docs/ko/human-guide.md`
- 스킬 포트폴리오: `docs/ko/skill-portfolio.md`
- 예시 workflow: `docs/ko/examples.md`
- privacy: `docs/ko/privacy.md`
- release checklist: `docs/ko/release-checklist.md`
- 다음 버전 계획: `docs/ko/next-version.md`

LLM용 reference, output contract, skill 본문은 영어를 기준으로 유지합니다.
