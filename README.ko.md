# GroundLine

GroundLine은 반복되는 Codex 실수를 줄이기 위한 작은 도구입니다. 필요한 대화
맥락을 검토하고, 근거 있는 지침·스킬 수정을 적용한 뒤, 다음 실제 작업에서
다시 확인합니다. 실행·권한·설정·에이전트는 Codex가 담당합니다. 공통 기본값은
native AGENTS 계층에, 프로젝트별 요구는 해당 프로젝트에 둡니다.

[English](README.md) · [문서](docs/ko/index.md)

| 제품 | 역할 | 기본값 |
| --- | --- | --- |
| [Core](plugins/groundline/README.ko.md) | 개선 스킬 하나, 작은 비공개 변경 기록과 후속 내용 판단 | 오프라인·훅 없음 |
| [Insights](plugins/groundline-insights/README.ko.md) | 선택적 운영 사용량 수집·대시보드 | 별도 설정·동의 |

`$groundline:optimize-codex-workflow`로 반복 마찰을 검토합니다.
[개선 절차](plugins/groundline/references/conversation-improvement.md)는 실제 대상 캡처,
native 수정·검증, 변경 기록, 독립적인 자연 후속 관측입니다. 근거가 없으면
변경 없이 종료합니다. 사용량 감사는 필요한 진단에만 쓰며 매주 필수가 아닙니다.
Core는 모델 라우팅·기기별 환경 기준·지속 학습을 관리하지 않습니다.

기존 대화 기록은 변경 없이 조회합니다. 기준을 지켰다는 내용 판단과 인과적
품질·비용 개선은 구분합니다. 설치·테스트 통과는 행동 개선 증거가 아닙니다.

Git·Codex·Bash·jq가 있는 환경에서 완전한 stable 배포본을 검토하고 설치합니다.

```console
git clone --branch stable --single-branch https://github.com/jukqaz/groundline.git groundline-install
bash groundline-install/install.sh
```

기본은 Core이며 `--profile insights` 또는 `--profile both`로 Insights를 선택합니다.
모델·effort·권한·비활성 플러그인 선택은 명시 요청 없이 바꾸지 않습니다.
설치는 검토한 배포본에 고정됩니다. 소스 수정은 설치 갱신과 다릅니다.
[설치·복구](docs/installation.md) · [검증](docs/guidance-validation.md) ·
[개인정보](docs/privacy.md). Apple Silicon macOS와 Linux ARM64/x86_64를 지원합니다.
