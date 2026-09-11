# GroundLine

[English](README.md) · [한국어 문서](docs/ko/index.md)

GroundLine은 공개 Rust 모노레포 하나에서 서로 독립적으로 설치할 수 있는 Codex
플러그인 두 개를 제공합니다. Codex의 실행, 설정, 권한, 에이전트, worktree,
리뷰, compaction, 업그레이드 기능을 대체하지 않습니다.

| 플러그인 | 역할 | 기본 네트워크 동작 |
| --- | --- | --- |
| `groundline` | 로컬 가이드, 프로젝트 감사, 증거 경계, 집계 사용량 분석 | 오프라인, hook·collector identity 없음 |
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

## 설치와 업그레이드

**GroundLine은 Codex 플러그인과 네이티브 CLI로 동작합니다.**
Codex App과 CLI에서 같은 플러그인을 사용합니다.
Git과 Codex만 있으면 됩니다. 아래 명령은 개인 모델·추론·권한 설정을 바꾸지 않습니다.
수집은 Insights 연결 설정과 명시적 동의 후 Codex 훅이 실행될 때 동작합니다.

아래 `codex`는 실제 사용하는 Codex의 CLI를 뜻합니다. macOS에서 Codex App만
설치했다면 `/Applications/ChatGPT.app/Contents/Resources/codex` 전체 경로로
바꿔 실행할 수 있습니다. App과 CLI가 같은 설정을 사용하려면 같은 `CODEX_HOME`을 유지합니다.

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

새 버전은 하나의 marketplace snapshot을 갱신해 적용합니다.

```console
codex plugin marketplace upgrade groundline --json
codex plugin list --json
```

`main`과 버전 태그에는 소스가 있고, 설치에 필요한 실행 파일은 `stable`의
`bin` 디렉터리에 포함됩니다. 설치 채널을 소스 태그로 바꾸면 실행 파일이
없을 수 있습니다. 특정 버전으로 고정하려면 검증한 배포용 리비전이 필요합니다.
갱신 후 설치 버전이 그대로라면 같은 플러그인 ID의 `plugin add`를 다시 실행하고
설치 버전과 체크섬을 확인합니다.

marketplace 갱신, 설치 package checksum, hook 신뢰, collector upload,
ClickHouse 반영, Grafana frame, image 게시, 운영 배포, stable 승격은 서로 다른
증거 lane입니다.

### Insights 연결과 보고서

Insights CLI에서 연결 설정, 수집 활성화·중지, 상태 확인을 수행합니다.
Codex 훅도 같은 실행 파일을 호출합니다. 집계 보고서는 CLI 또는 운영자의
Grafana 대시보드에서 확인합니다. [Insights 명령 안내](plugins/groundline-insights/README.ko.md)를 참고하세요.

별도 GroundLine Desktop 앱은 제공을 종료했습니다. 기존 앱을 제거해도 플러그인,
서버 설정, 수집 동의, 인증 정보, 커서와 미전송 이벤트는 유지됩니다.

### 선택 작업: 기존 Codex 설정 보정

설치와 설정 보정을 함께 원할 때만 검토한 `stable` 배포본의 설치 스크립트를 실행합니다.
이 스크립트는 Core를 설치하고 `gpt-6-astra / xhigh / Fast 끔`을 적용하며,
수동 컨텍스트 제한과 퇴역한 Core hook 승인 기록 4종을 정리합니다.
기존 파일은 비공개 백업을 남기며 다른 설정은 보존합니다. Insights는 설치하지 않습니다.

```console
git clone --branch stable --single-branch https://github.com/jukqaz/groundline.git groundline-install
bash groundline-install/install.sh
```

Windows에서는 두 번째 줄 대신 `powershell -File groundline-install/install.ps1`을
실행합니다. 실제 App 런타임의 CLI가 자동 탐지와 다르면 shell 스크립트의 첫 인자,
PowerShell의 `-Codex` 인자로 경로를 전달합니다. 같은 설정에 재적용하면 쓰기와
추가 백업이 없습니다. 배포본과 설치된 실행 파일이 다르면 중단합니다.

## 개인 스킬 관리

설정 외에 지침까지 정리할 때는 다음 요청을 사용합니다.

> `$groundline:align-agent-home`으로 GroundLine을 적용하고, 기존 Codex 설정과
> 지침의 오류를 현재 모델의 공식 지침에 맞춰 백업·수정·검증해 줘.

이 적용 과정은 잘못된 설정을 보고만 하고 끝내지 않습니다. 근거가 확인된
오류는 요청 범위에서 수정하고, 의도적인 모델·추론·권한 선택은 보존합니다.
단순 `plugin add`는 설치만 수행하며 개인 설정 수정 hook을 실행하지 않습니다.
[설치·적용 절차](plugins/groundline/references/installation-alignment.md)와
[Astra 기반 조사 보고서](docs/research/install-alignment-astra.ko.md)를 참고하세요.

설치된 `groundline setup --catalog <native-models.json> --apply`로도 공통 기본값을
적용할 수 있습니다. `CODEX_HOME`을 자동으로 찾고, `--apply`를 빼면 미리보기만
수행합니다. 해당 PC에서 지원되지 않는 모델·추론 수준이나 해석되지 않은
profile/provider/catalog override는 파일을 바꾸지 않고 명시적으로 거부합니다.

`groundline config-repair --config <config.toml> --catalog <native-models.json>`은
잘못된 컨텍스트 제한의 수정안을 미리 보여줍니다. `--apply --expect-plan <hash>
--backup <new-file>`로 검토한 입력과 일치할 때만 백업 후 적용하며,
정상적인 수동 제한을 해제하려면 `--restore-native-context`를 명시합니다.

Codex 설정·모델 점검은 `groundline config-audit --config <config.toml>
--catalog <native-models.json> --json`으로 실행합니다. `--catalog -`로 네이티브
카탈로그를 바로 전달할 수도 있습니다. 선택된 항목만 비교하며 설정값을 출력하거나
수정하지 않습니다. 실제 유효 설정은 Codex의 strict doctor로 별도 검증합니다.
[설정 점검](plugins/groundline/references/codex-configuration.md)을 참고하세요.

`$groundline:align-agent-home`에 가져온 스킬의 점검·업데이트를 요청하면 됩니다.
GroundLine은 `guidance audit|snapshot`으로 스킬의 추가·삭제·변경, 원본 비교,
메타데이터와 파일 지문을 관리합니다. 기기별 경로 설정과 경로 없는 비교 기록을
분리하며, 이전 개인 JSON 형식을 런타임 호환 코드로 유지하지 않습니다. Codex는 원본 변경을 리뷰하고
승인된 수정을 적용한 뒤 관련 테스트를 실행합니다. 개인 스킬·설정·출처 기록은
공개 저장소 밖에 남으며 플러그인 업그레이드가 이를 덮어쓰지 않습니다.
자세한 절차는 [스킬 관리](plugins/groundline/references/skill-maintenance.md)를 참고하세요.

## 사용 패턴에 따른 개인 개선

`$groundline:improve-personal-workflow`는 Insights 보고서, 로컬 감사, 현재 모델의
공식 지침과 직접 확인한 완료 결과를 함께 검토합니다. `groundline personal
review|evaluate|rollback`은 승인된 전용 개인 지침에 한 가지 변경만 시험하고,
동일한 조건의 별도 작업 결과를 비교해 유지하거나 복구합니다.

근거가 부족하면 `OBSERVE`, 비교 조건이 달라지면 `INCONCLUSIVE`로 남습니다.
선택한 모델·추론 수준·권한·프로젝트 지침은 자동으로 바꾸지 않습니다. 최신 모델
지침은 실행 시 Codex가 공식 문서와 실제 네이티브 카탈로그를 확인하며, 모델 이름을
고정된 목록으로 판정하지 않습니다. 파일 생성과 실제 지침 적용은 따로 검증합니다.
자세한 입력·개인정보·복구 조건은 [개인 개선 계약](plugins/groundline/references/personal-improvement.md)에 있습니다.

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
실행합니다. GitHub Actions의 전체 qualification과 6개 플랫폼·2개 제품 artifact
matrix는 수동 실행 또는 release tag에서만 동작합니다. public CI는 self-hosted
runner와 production credential을 요구하지 않습니다.

더 최신 ClickHouse·Nginx·Grafana·datasource plugin 후보는 수동 workflow에 네 값을
한 세트로 넣어 검증할 수 있습니다. 이 검증은 기본 profile이나 운영 배포를 자동으로
바꾸지 않습니다.

자세한 선택지는 [연동과 설치 프로필](docs/ko/integrations.md),
[Codex 업데이트 대응과 지원 범위](docs/ko/codex-compatibility.md),
[Insights 셀프호스팅](docs/ko/self-hosting.md), 영문 README,
[변경 기록](CHANGELOG.md), [release checklist](docs/release-checklist.md)를 참조하세요.
