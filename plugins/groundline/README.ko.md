# GroundLine

GroundLine은 평소 Codex 활동과 실제 작업 결과를 바탕으로 GPT-6 Astra·Sol·Luna의
모델·effort·서브에이전트 선택을 돕습니다. 이전 기록은 보존합니다. 실행, 설정,
권한, 에이전트, worktree, 리뷰와 업그레이드는 Codex가 담당합니다.

흐름은 **audit → delivery → route**입니다. 활동 집계는 표본을 설명하고, 조건이
맞는 직접 결과는 실측 비교에 사용합니다. 추천이나 합성 테스트 통과만으로 품질
개선·비용 절감을 입증하지 않습니다. [최적화 흐름](references/codex-optimization-loop.md)을
참고하세요.

## 스킬

| 스킬 | 범위 |
| --- | --- |
| `align-agent-home` | 요청한 설치·설정·지침 정비 |
| `audit-agent-history` | 명시적으로 요청한 이력 조사와 비식별 사용 근거 |
| `optimize-codex-workflow` | 작업별 GPT-6 선택과 업무 결과 검토 |

설정 정비와 최적화 스킬은 작업에 맞춰 호출할 수 있으며, 이력 조사는 명시 요청이
필요합니다. 계획, Goal, 인계와 승인된 파일 수정은 네이티브 Codex가 수행합니다.
별도의 작업 실행 계층이나 개인·외부 스킬 관리 CLI는 제공하지 않습니다.

## 개인정보 경계

Core에는 lifecycle hook, 백그라운드 프로세스, 스케줄러, 수집 식별자와 네트워크
클라이언트가 없습니다. audit는 범위가 제한된 로컬 상태를 읽기 전용으로 열고,
원문 prompt·transcript·경로·설정 값을 제외한 집계를 반환합니다. 명시적인 완료
기록은 비공개 로컬 receipt로 저장하며 업로드하지 않습니다.

`groundline provider-smoke --plugin-root <path> --json`는 owner hook manifest가
있으면 실패합니다. 선택 기능인 Insights는 별도 설치·동의 계약을 사용합니다.

## 설치와 업그레이드

실행 파일이 포함된 검토한 `stable` 배포본의 `install.sh` 또는 `install.ps1`을
사용하거나 네이티브 Codex로 패키지를 설치합니다.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
```

Core 설치는 Insights를 설치하거나 활성화하지 않습니다. 패키지 설치만으로 개인
설정 수정 hook이 실행되지 않습니다. 적용·수정을 요청하려면
`$groundline:align-agent-home`과 [설치·적용 절차](references/installation-alignment.md)를
사용합니다.

setup은 기존 선택과 Codex 기본값을 보존합니다. 명시한 `--model`, `--effort`,
`--service-tier`만 적용하며 고정 모델 프리셋은 없습니다. 모델·effort는 제공한
네이티브 카탈로그로 검증합니다. 컨텍스트 복원에는 `--restore-native-context`가
필요합니다. 변경한 설정은 비공개 백업을 만들고 `--apply`를 빼면 쓰기 없이
미리 봅니다. 자세한 범위는 [설정 점검](references/codex-configuration.md)에 있습니다.

[네이티브 업그레이드](references/native-upgrade.md) 후 설치 패키지·checksum과
실제 동작을 구분해 확인합니다.

```console
groundline provider-smoke --plugin-root /path/to/installed/groundline --require-installed --json
groundline doctor --plugin-root /path/to/installed/groundline --json
```

Apple Silicon/Intel macOS, ARM64/x86_64 Linux·Windows를 지원합니다. 실행 파일은
설치된 플러그인의 `bin/<target>`에서 찾습니다. 설치가 shell `PATH` 등록까지
보장하지 않으며 소스 태그에는 설치용 실행 파일이 없습니다.

## 사용 근거와 완료 결과 비교

다음 예제는 공개 저장소 밖의 비공개 작업 디렉터리에서 실행합니다.

```console
groundline audit weekly --days 7 --review --json > weekly.json
groundline audit review --input weekly.json --json
groundline efficiency record-delivery --input manifest.json --output receipts/delivery.json --json
groundline efficiency delivery-summary --deliveries receipts --json
groundline efficiency route --input routing.json --catalog native-models.json --audit weekly.json --deliveries receipts --json
```

필요에 따라 새 audit를 한 번 수집하거나 저장된 결과를 재사용합니다. 저장 결과의
review는 이력을 다시 읽지 않고 현재 코드로 추천을 한 번 재계산합니다. 주간 audit는
선택한 기간의 작업 표본을 다루며 전체 모집단 coverage는 미확인으로 둡니다.
`groundline audit store --json`는 전체 저장소 metadata를 별도로 진단합니다.
범위와 제한은 [주간 감사](references/weekly-usage-audit.md)에 정리되어 있습니다.

[완료 기록 계약](references/delivery-evidence.md)은 manifest·비공개 receipt,
관측한 실제 모델·effort, 품질·재작업·소유 자원을 정의합니다. 실패한 작업과 알 수
없는 값도 남깁니다. 라우팅 패킷의 `outcomes`를 비우고 전용 기록 디렉터리를
`route --deliveries`로 전달합니다. 비교 조건과 카탈로그 요구사항은
[라우팅 계약](references/evidence-routing.md)을 따릅니다. 집계 보고서는 선택적
맥락이며, route는 설정을 변경하거나 자동 학습된 개선을 입증하지 않습니다.

`personal status`와 `personal rollback`은 기존 비공개 실험 상태 확인·복구만
지원합니다. 새 실험을 만들지 않습니다. [기존 상태 복구](references/personal-recovery.md)를
참고하세요.

개발 검증 명령은 [영문 README](README.md), 제품 구조는
[아키텍처](https://github.com/jukqaz/groundline/blob/main/docs/architecture.md),
Insights 선택 기준은 [연동과 설치 프로필](https://github.com/jukqaz/groundline/blob/main/docs/ko/integrations.md)을
참고하세요.
