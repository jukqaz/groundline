# GroundLine

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

## 설치와 업그레이드

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

marketplace 갱신, 설치 package checksum, hook 신뢰, collector upload,
ClickHouse 반영, Grafana frame, image 게시, 운영 배포, stable 승격은 서로 다른
증거 lane입니다.

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
[Insights 셀프호스팅](docs/ko/self-hosting.md), 영문 README,
[release checklist](docs/release-checklist.md)를 참조하세요.
