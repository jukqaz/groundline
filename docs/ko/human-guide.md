# 사람용 가이드

영어 `docs/human-guide.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

## GroundLine이 하는 일

GroundLine은 agent 작업을 더 안전하고 검증 가능하게 만드는 스킬 패키지입니다.
핵심은 세 가지입니다.

- 현재 상태를 다시 확인한다.
- side effect와 승인 필요성을 분리한다.
- 완료 주장 전에 검증 증거를 남긴다.

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

## 자주 쓰는 요청

| 상황 | 사용할 skill |
| --- | --- |
| 이전 agent 작업을 이어받음 | `reconcile-current-state` |
| 위험한 변경, 배포, 권한 변경 | `guard-side-effects` |
| 테스트는 통과했지만 runtime 증거가 필요함 | `close-live-work` |
| 대화가 길어져 다음 LLM에게 넘겨야 함 | `package-agent-task` |
| 아이디어가 계속 늘어남 | `hold-the-line` |
| release 직전 polish가 필요함 | `polish-release-candidate` |
| release scope를 잠그고 ship 여부를 판단함 | `stabilize-release-cut` |
| release 뒤 이전 버전과 비교함 | `compare-release-delta` |
| 외부 agent 도구를 조사하고 비교함 | `agent-ecosystem-radar` |
| AI 활용 습관을 평가함 | `evaluate-ai-usage-maturity` |

## 종료 기준

작업을 끝낼 때는 다음을 같이 남깁니다.

- PASS/PARTIAL/FAIL
- 확인한 branch, runtime, CI, release, endpoint, file
- 실행한 command
- 남은 gap
- side effect 여부
