# GroundLine

GroundLine은 평소 Codex 활동과 실제 완료 결과를 바탕으로 GPT-6 Astra·Sol·Luna의
모델·effort·서브에이전트 선택을 돕습니다. 실행, 설정, 권한, 에이전트, worktree,
리뷰와 업그레이드는 Codex가 담당합니다. 이전 세대의 기록은 보존하지만 최적화
대상으로 사용하지 않습니다.

증거 흐름은 **audit → delivery → route**입니다. 범위가 제한된 활동 표본을
확인하고, 실제 결과를 기록한 뒤, 조건이 맞는 결과를 비교합니다. 작업 판단이나
집계 사용량만으로 실측 최적 조합 또는 자동 개선을 입증하지 않습니다.
[제품 구조](docs/architecture.md)와
[근거 기반 선택 계약](plugins/groundline/references/evidence-routing.md)을 참고하세요.

[English](README.md) · [한국어 문서](docs/ko/index.md)

공개 Rust 모노레포에서 서로 독립적으로 설치할 수 있는 플러그인 두 개를 제공합니다.

| 플러그인 | 역할 | 기본 네트워크 동작 |
| --- | --- | --- |
| `groundline` | 로컬 사용 근거, 완료 기록, 범위가 제한된 작업 개선 추천 | 오프라인, hook·collector identity 없음 |
| `groundline-insights` | 선택형 집계 수집과 ClickHouse·Grafana 공개 self-hosting preview | owner profile과 enrollment credential 설정 전에는 비활성 |

설치 package의 canonical source는 `plugins/` 아래 두 디렉터리뿐입니다. 실제
endpoint, credential, dataset 경로, 배포 receipt, 인프라 inventory는 Git 밖의
owner-private 상태에 둡니다.
Compose template에는 인프라 버전을 직접 넣지 않습니다. strict compatibility
profile이 release-tested 조합 또는 더 최신 후보 조합을 선택하고, 최신 후보도 동일한
실제 stack verifier를 통과해야 합니다.

Insights는 자기 서비스 연결 방식입니다. 서로 다른 운영자는 별도의 비공개 인스턴스,
저장소, credential을 사용하며 공개 플러그인 설치로 maintainer 서비스에 가입되지
않습니다. [개인 운영 경계](docs/ko/integrations.md#개인-운영-경계)를 참고하세요.

## 토큰 비용과 가치

Core의 스킬 설명과 읽은 지침은 모델 입력을 늘립니다. 모델이 수행하는 감사,
검증, 보고서 분석에도 추가 토큰이 들 수 있습니다. Insights 자동 수집은
네이티브 코드로 기존 활동을 집계해 운영자 서비스에 전송하며 언어 모델을 호출하지 않습니다.

실패한 작업까지 포함해 비슷한 요청 결과의 검증된 품질, 재작업, 사용자 개입, 소요 시간,
실측 토큰을 함께 평가합니다. 추가 토큰으로 얻는 품질이나 신뢰성이 충분하다면
합당한 비용일 수 있습니다. 총토큰 절감은 보장하지 않으며, 설치·전송 검증 성공만으로
효율 개선을 입증할 수는 없습니다. 지침은 간결하게 유지하고, 작업 위험에 맞게
검증하며, 유효한 증거는 재사용합니다.
[완료 기록 계약](plugins/groundline/references/delivery-evidence.md)은 요청한 선택과
관측한 실제 선택, 실패한 작업, 누락된 자원과 완료 근거를 구분합니다.

## 설치와 업그레이드

릴리스는 `2026.09.29-a`처럼 날짜와 당일 순번으로 표시합니다. 설치·업데이트에서
비교하는 숫자 버전은 `2026.929.1`이며 [버전 규칙](docs/versioning.md)에 따라
같은 날 재배포와 다음 날짜 모두 증가합니다.

실행 파일이 포함된 검토한 `stable` 배포본의 설치 스크립트를 사용합니다. 기본은
Core 설치와 기존 Codex 선택 보존이며 완료·대기 중인 단계를 각각 보고합니다.
Git과 Codex가 먼저 설치되어 있어야 하며 Bash 설치기에는 `jq`도 필요합니다.

```console
git clone --branch stable --single-branch https://github.com/jukqaz/groundline.git groundline-install
bash groundline-install/install.sh
```

macOS와 Linux의 ARM64·x86-64를 지원합니다.
Insights도 설치하려면 `--profile both`, Insights만 설치하려면
`insights`를 선택합니다. 모델 선택, 비공개 연결 입력, 동의, 첫 수집과 재실행은
[통합 설치 안내](docs/installation.md)를 참고하세요. 기존 비활성 플러그인은 그대로
유지합니다. 설치기는 검토한 커밋으로 고정하므로 App Refresh도 그 커밋에 머뭅니다.
다음 버전은 최신 전체 `stable` 배포본을 검토한 뒤 그 설치기를 다시 실행합니다.
패키지만 설치하는 명령도 아래에 있습니다.

**GroundLine은 Codex 플러그인과 네이티브 CLI로 동작합니다.**
Codex App과 CLI에서 같은 플러그인을 사용합니다.
패키지만 설치하는 native 명령에는 Git과 Codex가 필요합니다. 아래 명령은 개인 모델·추론·권한 설정을 바꾸지 않습니다.
수집은 Insights 연결 설정과 명시적 동의 후 Codex 훅이 실행될 때 동작합니다.

아래 `codex`는 실제 사용하는 Codex의 CLI를 뜻합니다. macOS에서 Codex App만
설치했다면 `/Applications/ChatGPT.app/Contents/Resources/codex-cli/bin/codex`
전체 경로로 바꿔 실행할 수 있습니다. 설치기는 시스템·사용자 Applications에서
번들 식별자를 확인하고, 이전 `Contents/Resources/codex` 배치도 인식합니다.
App과 CLI가 같은 설정을 사용하려면 같은 `CODEX_HOME`을 유지합니다.

moving `stable` branch를 한 번 등록한 뒤 설치 프로필을 선택합니다. 두 플러그인은
독립적이며 하나를 설치해도 다른 플러그인이 자동 설치·활성화되지 않습니다.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
```

외부 연동이 없는 Core만 설치하려면:

```console
codex plugin add groundline@groundline --json
```

owner가 운영하는 수집·분석 노드에 Insights만 설치하려면:

```console
codex plugin add groundline-insights@groundline --json
```

Core 기능과 비공개 집계 분석을 함께 쓸 때만 `plugin add` 두 개를 모두 실행합니다.

등록된 marketplace snapshot을 갱신합니다. 커밋으로 고정된 등록은 같은 버전에 머뭅니다.

```console
codex plugin marketplace upgrade groundline --json
codex plugin list --json
```

`main`과 버전 태그에는 소스가 있고, 설치에 필요한 실행 파일은 `stable`의
`bin` 디렉터리에 포함됩니다. 설치 채널을 소스 태그로 바꾸면 실행 파일이
없을 수 있습니다. 특정 버전으로 고정하려면 검증한 배포용 리비전이 필요합니다.
직접 실행하는 native 명령은 설치기의 API 호환성 검사를 거치지 않습니다.
업데이트에는 검토한 설치기를 사용합니다. 비활성 플러그인을 `plugin add`로
다시 추가하면 활성화됩니다.

marketplace 갱신, 설치 package checksum, hook 신뢰, collector upload,
ClickHouse 반영, Grafana frame, image 게시, 운영 배포, stable 승격은 서로 다른
증거 lane입니다.

### Insights 연결과 보고서

Insights CLI에서 연결 설정, 수집 활성화·중지, 상태 확인을 수행합니다.
Codex 훅도 같은 실행 파일을 호출합니다. 집계 보고서는 CLI 또는 운영자의
Grafana 대시보드에서 확인합니다. [Insights 명령 안내](plugins/groundline-insights/README.ko.md)를 참고하세요.

별도 GroundLine Desktop 앱은 제공을 종료했습니다. 기존 앱을 제거해도 플러그인,
서버 설정, 수집 동의, 인증 정보, 커서와 미전송 이벤트는 유지됩니다.

### 기존 설정과 이전

설치는 기존 선택을 검증하고 확인된 퇴역 Core hook 승인 항목 네 개만 정리합니다.
모델·effort·서비스 등급은 명시적인 옵션으로만 변경하며 고정 모델 프리셋은 없습니다.
컨텍스트 크기를 네이티브 기본값으로 복원하려는 경우에만 `--restore-native-context`
또는 `-RestoreNativeContext`를 사용합니다. 실제 설정 변경에는 비공개 백업을 만들고
같은 설정에 재적용하면 추가 쓰기·백업이 없습니다.
[이전 절차](plugins/groundline/references/installation-alignment.md#existing-settings-and-migration)는
이전 네이티브 옵션, profile, 지침 충돌과 복구 범위를 다룹니다.

## Core 스킬과 설정

| 스킬 | 범위 |
| --- | --- |
| `align-agent-home` | 요청한 설치·설정·지침 정비 |
| `audit-agent-history` | 명시적으로 요청한 이력 조사와 비식별 사용 근거 |
| `optimize-codex-workflow` | 작업별 GPT-6 선택과 관측한 작업 결과 검토 |

설정 정비와 최적화 스킬만 작업에 맞춰 자동 호출할 수 있습니다. 이력 조사는 명시
요청이 필요합니다. 일반적인 계획, Goal, 인계와 실행은 네이티브 Codex가 담당합니다.
가져온 스킬의 정비도 별도 GroundLine 목록·기준 기록 없이 네이티브 파일 검토로 수행합니다.

`groundline setup --catalog <native-models.json> --apply`는 기존 선택과 Codex
기본값을 보존합니다. `--apply`를 빼면 쓰기 없이 미리 봅니다. 지원되는 모델·effort를
명시적으로 선택할 때만 `--model <id> --effort <level>`을 전달합니다. 설정 점검과
범위가 제한된 수정에는 `config-audit`와 `config-repair`를 사용합니다.
네이티브 카탈로그 검증, 비공개 백업과 수정 계획 일치 조건은
[설치·적용 절차](plugins/groundline/references/installation-alignment.md)와
[설정 점검](plugins/groundline/references/codex-configuration.md)에 있습니다.

## 개인정보와 보안

Core는 lifecycle hook을 설치하거나 네트워크 요청을 하지 않습니다. Insights만
fail-open Codex lifecycle hook 4개를 소유합니다. Codex SQLite를 읽기 전용으로
열어 제한된 집계 counter만 만들며 raw prompt, response, transcript, command,
patch, path, 저장소명, task/rollout/account/hostname/IP 식별자를 wire contract에서
거부합니다.

Tailnet 연결은 권한이 아닙니다. 첫 enrollment에는 플러그인 밖의 비공개 파일에
저장한 owner-issued credential이 추가로 필요하고, 이후 collector마다 별도
token을 사용합니다. 공개 저장소에는 placeholder와 범용 template만 둡니다.

## 개발 검증

개발 중에는 변경 범위에 맞는 fast lane만 실행하고, 변경이 고정된 뒤 전체
workspace test·Clippy·현재 source와 reachable Git history 검증을 한 번
실행합니다. GitHub Actions의 전체 qualification과 4개 대상·2개 제품 artifact
matrix는 수동 실행 또는 release tag에서만 동작합니다. public CI는 self-hosted
runner와 production credential을 요구하지 않습니다.

더 최신 ClickHouse·Nginx·Grafana·datasource plugin 후보는 수동 workflow에 네 값을
한 세트로 넣어 검증할 수 있습니다. 이 검증은 기본 profile이나 운영 배포를 자동으로
바꾸지 않습니다.

자세한 선택지는 [연동과 설치 프로필](docs/ko/integrations.md),
[Codex 업데이트 대응과 지원 범위](docs/ko/codex-compatibility.md),
[Insights 셀프호스팅](docs/ko/self-hosting.md), 영문 README,
[변경 기록](CHANGELOG.md), [release checklist](docs/release-checklist.md)를 참조하세요.

## 작업 결과 근거

`$groundline:optimize-codex-workflow`로 작업 선택 또는 요청한 업무 개선 검토를
수행합니다. [최적화 흐름](plugins/groundline/references/codex-optimization-loop.md)과
[CLI 예제](docs/examples.md)를 따르며 Insights는 선택 사항입니다.

주간 audit는 선택한 기간의 작업 표본을 설명합니다. 전체 모집단 coverage는
미확인으로 두고, 전체 저장소 metadata는 별도 `audit store` 명령으로 진단합니다.
`audit review --input <saved-audit.json>`는 이력을 다시 읽지 않고 저장된 근거를
재사용하며 현재 코드로 추천을 한 번 재계산합니다.

모델·effort의 실측 개선을 주장하려면 실패·미완료 작업을 포함한 실제 완료 결과를
먼저 기록해야 합니다. `route`는 조건이 맞는 직접 결과를 비교하며 자동으로 이득을
학습하거나 설정을 적용하지 않습니다. 기존 비공개 개인 실험 상태는 `personal status`로
확인하고 `personal rollback`으로 복구할 수 있습니다.
[기존 상태 복구](plugins/groundline/references/personal-recovery.md)를 참고하세요.
