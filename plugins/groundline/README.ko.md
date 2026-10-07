# GroundLine

GroundLine은 로컬 Codex 활동과 실제 작업 결과로 GPT-6 Astra·Sol·Luna의
모델·effort·서브에이전트 선택을 돕습니다. 실행, 설정, 권한, 에이전트,
worktree, 리뷰와 업그레이드는 Codex가 담당하며 이전 기록은 보존합니다.

흐름은 **audit → delivery → route**입니다. 활동 집계는 표본을 설명하고,
조건이 맞는 직접 결과는 비교에 사용합니다. 추천이나 합성 테스트 통과만으로
품질 개선·비용 절감을 입증하지 않습니다.
[최적화 흐름](references/codex-optimization-loop.md)을 참고하세요.

## 스킬

| 스킬 | 범위 |
| --- | --- |
| `align-agent-home` | 요청한 설치·설정·지침 정비 |
| `audit-agent-history` | 명시적으로 요청한 이력 조사와 비식별 사용 근거 |
| `optimize-codex-workflow` | 작업별 GPT-6 선택과 업무 결과 검토 |

설정 정비와 최적화는 작업에 맞춰 호출할 수 있으며, 이력 조사는 명시 요청이
필요합니다. 계획, Goal, 인계와 승인된 수정은 네이티브 Codex가 수행합니다.

## 설치와 업그레이드

설치와 적용에는 실행 파일이 포함된 검토한 `stable` 배포본의 `install.sh`를
사용합니다. 네이티브 패키지 설치만 수행할 수도 있습니다.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
```

Core는 Insights를 추가하거나 활성화하지 않습니다. 패키지 설치만으로 개인
설정을 수정하지도 않습니다. 적용 요청에는 `$groundline:align-agent-home`과
[설치·적용 절차](references/installation-alignment.md)를 사용합니다.
setup은 기존 선택과 네이티브 기본값을 보존합니다. 명시 설정, 백업, 미리보기와
컨텍스트 복원 범위는 [설정 점검](references/codex-configuration.md)을 따릅니다.

후보 API 검사, 커밋 고정과 복구는 [업그레이드 절차](references/native-upgrade.md)를
따릅니다. 설치 패키지와 실제 동작은 구분해 확인합니다.

```console
groundline provider-smoke --plugin-root /path/to/installed/groundline --require-installed --json
groundline doctor --plugin-root /path/to/installed/groundline --json
```

Apple Silicon macOS(ARM64)와 Linux(ARM64·x86_64)를 지원합니다. 실행 파일은 설치된 `bin/<target>`에서
[명령 경로 안내](references/platform-commands.md)에 따라 찾습니다. shell `PATH`
등록은 보장하지 않으며 소스 태그에는 실행 파일이 없습니다.

## 사용 근거와 완료 결과 비교

공개 저장소 밖의 비공개 디렉터리에서 실행합니다.

```console
groundline audit weekly --days 7 --review --json > weekly.json
groundline audit review --input weekly.json --json
groundline efficiency record-delivery --input manifest.json --output receipts/delivery.json --json
groundline efficiency delivery-summary --deliveries receipts --json
groundline efficiency route --input routing.json --catalog native-models.json --audit weekly.json --deliveries receipts --json
```

[주간 감사](references/weekly-usage-audit.md)는 선택한 기간의 작업 표본을 다루며
전체 모집단 coverage는 미확인으로 둡니다. 저장 결과의 review는 이력을 다시
읽지 않고 추천을 한 번 재계산합니다. `groundline audit store --json`는 별도의
전체 저장소 진단입니다.

[완료 기록](references/delivery-evidence.md)은 실제 선택, 품질·재작업·자원,
실패와 알 수 없는 값을 남깁니다. [라우팅](references/evidence-routing.md)에는
전용 기록 디렉터리와 빈 `outcomes` 배열을 사용합니다. 집계 보고서는 선택적
맥락이며, route는 설정을 바꾸거나 자동 학습된 개선을 입증하지 않습니다.
기존 `personal status`/`rollback`은 [복구 전용](references/personal-recovery.md)입니다.

요청한 지속 개선에는 [학습 루프](references/learning-loop.md)를 사용합니다. 당시 환경
capture와 결과 연결 초안, 후보별 후속 상태, 공식 자료 snapshot 변경 확인, 비공개 지침
bundle을 연결합니다. 명시 결정과 관측된 효과는 구분하고 실행은 native Codex가 담당합니다.
활성화한 학습 프로필에서는 의미 있는 작업 시작에 완료 기준을 선언하고 검증 결과를
`learning assess`로 제출합니다. 실제 종료 뒤 소비기가 소유 응답 비용과 결과를 연결하며,
`learning patterns`의 readiness가 대기 기록과 빠진 근거를 보여줍니다. 자동 연결은
품질 향상이나 후보 채택을 자동으로 판정하지 않습니다.

## 개인정보 경계

Core에는 hook, 백그라운드 프로세스, 스케줄러, 수집 식별자와 네트워크 클라이언트가
없습니다. audit는 범위가 제한된 읽기 전용 검사이고 receipt는 비공개 로컬 파일입니다.
감사 결과와 receipt는 native 원문 대화·경로·설정 값을 내보내지 않습니다. 비공개 환경
bundle에는 선택한 관리 지침 본문과 공통 기준이 들어갑니다. 본문에 개인 정보가 있는지
전송 전에 확인하고 공개 저장소에 넣지 않습니다. provider smoke는 owner hook manifest를
거절하며, 선택형 Insights는 별도 설치·동의를 요구합니다.
[보안](SECURITY.md)과 [개인정보 정책](https://github.com/jukqaz/groundline/blob/main/docs/privacy.md)을
참고하세요.

개발 검증은 [CONTRIBUTING](https://github.com/jukqaz/groundline/blob/main/CONTRIBUTING.md), 구조와 설치 프로필은
[아키텍처](https://github.com/jukqaz/groundline/blob/main/docs/architecture.md)와
[연동 안내](https://github.com/jukqaz/groundline/blob/main/docs/ko/integrations.md)에 있습니다.
