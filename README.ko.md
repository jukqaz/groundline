# GroundLine

GroundLine은 Codex 사용 패턴·실제 작업 결과·현재 모델의 공식 지침을 연결해
사용 방식과 개인의 공통 Codex 환경을 개선합니다. 실행·권한·설정·에이전트는
Codex가 담당합니다. 과거 관측은 보존하고 환경 변경은 사용자의 명시 선택과 범위를 따릅니다.

[English](README.md) · [문서 안내](docs/ko/index.md)

| 제품 | 역할 | 기본 동작 |
| --- | --- | --- |
| [Core](plugins/groundline/README.ko.md) | 로컬 감사, 작업 결과 기록, 개선 추천 | 오프라인, 훅 없음 |
| [Insights](plugins/groundline-insights/README.ko.md) | 선택형 집계 수집, ClickHouse, Grafana | 연결 설정과 명시적 동의 전에는 수집 중지 |

두 플러그인은 독립적으로 설치합니다. Insights는 운영자가 준비한 서비스에
연결하며 공개 설치로 개발자의 인프라에 가입되지 않습니다.
지원 환경은 **Apple Silicon macOS(ARM64)와 Linux(ARM64·x86_64)**입니다.

## 설치와 업데이트

Git·Codex·Bash·`jq`를 준비하고 실행 파일이 포함된 전체 `stable` 배포본을
검토한 뒤 설치기를 실행합니다.

```console
git clone --branch stable --single-branch https://github.com/jukqaz/groundline.git groundline-install
bash groundline-install/install.sh
```

기본은 Core입니다. 필요에 따라 `--profile insights` 또는 `--profile both`를
선택합니다. 별도로 변경을 요청하지 않은 기존 모델·effort·권한·비활성 플러그인은
보존합니다. Insights 수집기를 갱신하기 전에 운영 API가 새 계약을 지원해야 합니다.

설치기는 검토한 커밋으로 고정하므로 App Refresh도 그 커밋에 머뭅니다. 새 버전은
최신 전체 `stable` 배포본을 검토하고 해당 설치기를 다시 실행합니다. `main`과
버전 태그는 소스이며 생성된 실행 파일은 포함하지 않습니다.
App/CLI 선택, 패키지만 설치하는 명령, 비공개 연결 설정, 동의와 부분 실패 복구는
[설치 안내](docs/installation.md)에 있습니다. 날짜와 당일 순번을 사용하는
[버전 규칙](docs/versioning.md)을 따릅니다.

## 사용과 효과 확인

- `$groundline:align-agent-home`: 요청한 설치와 설정 정렬
- `$groundline:audit-agent-history`: 명시적으로 요청한 사용 기록 감사
- `$groundline:optimize-codex-workflow`: 작업별 모델·effort 선택과 작업 방식 검토

**audit → delivery → route** 순서로 관측·실제 결과·비교를 연결합니다.
[CLI 예제](docs/examples.md)에서 시작하고 실패·미완료 결과도 기록합니다.
주간 표본을 전체 이력으로 일반화하거나 집계 사용량만으로 최적 모델과 개선 효과를
단정하지 않습니다.

[지속 개선과 환경 통일 재설계](docs/adaptive-environment-design.md)는 이 순환을
개인 공통 기준·제한 적용·복구·후속 결과로 확장합니다.
[로컬 구현](docs/adaptive-environment-implementation.md)은 등록된 지침 계획과
복구 가능한 적용·비공개 결과 sidecar를 연결합니다. 설치된 플러그인의 활성과
이후 업무 효과는 별도로 확인합니다.

Core 지침과 모델이 수행하는 분석에는 토큰이 듭니다. Insights의 네이티브 수집기는
언어 모델을 호출하지 않습니다. 같은 조건의 작업에서 품질·재작업·사용자 개입·시간·
관측한 전체 자원을 함께 비교하며 토큰 절감은 보장하지 않습니다.
[행동 검증 기준](docs/guidance-validation.md)을 참고하세요.

## 개발과 운영

개발 검사는 [기여 안내](CONTRIBUTING.md), 코드 책임은 [아키텍처](docs/architecture.md),
공개 preview 서버 구축은 [셀프호스팅](docs/ko/self-hosting.md), 장애 진단과 실행 확인은
[운영 안내](docs/insights-operations.md)에 모았습니다. 자격증명·대화 원문·비공개 주소·
배포 기록은 Git 밖에 보관합니다.

[개인정보](docs/privacy.md) · [보안](SECURITY.md) · [변경 기록](CHANGELOG.md)
