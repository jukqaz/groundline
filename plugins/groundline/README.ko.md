# GroundLine

GroundLine은 Codex 작업 준비, 증거 기반 완료, 프로젝트 설정 감사, 집계 사용량
분석을 반복 가능하게 만드는 공개 로컬 우선 플러그인입니다. Codex의 실행,
설정, 권한, 에이전트, worktree, 리뷰, 업그레이드 기능을 대체하지 않습니다.

## 개인정보 경계

공개 플러그인은 다음 불변식을 지킵니다.

- lifecycle hook, 백그라운드 프로세스, 스케줄러, 수집 식별자가 없습니다.
- 네트워크 클라이언트, 업로드 목적지, 인증 토큰, 원격 저장소가 없습니다.
- prompt, transcript, 경로, 저장소 이름, 설정 값을 출력하지 않습니다.
- 로컬 감사 명령은 크기가 제한된 일반 파일을 읽기 전용으로 열고 집계 수치와
  안정적인 reason code만 반환합니다.

`groundline provider-smoke --plugin-root <path> --json`는 owner hook manifest가
있으면 실패합니다. 저장소 qualification은 개인·secret 표식, Python runtime
의존성, 중복 package root, CI 계약 이탈을 거부합니다.

## 설치와 업그레이드

Codex marketplace에 `https://github.com/jukqaz/groundline.git`을 추가하고
`groundline` 플러그인을 설치합니다. 이 명령은 Core만 설치하며
`groundline-insights`를 설치하거나 활성화하지 않습니다. refresh와 upgrade는
Codex가 담당하며, GroundLine은 자체 업데이트나 trust 변경을 수행하지 않습니다.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
```

갱신 절차는 [네이티브 업그레이드](references/native-upgrade.md)를 따릅니다.
버전 태그에는 소스가 있으므로 설치에는 실행 파일이 포함된 `stable`을 사용합니다.

업그레이드 후 설치 package와 native artifact를 각각 검증합니다.

```console
groundline provider-smoke --plugin-root /path/to/installed/groundline --require-installed --json
groundline doctor --plugin-root /path/to/installed/groundline --json
```

Apple Silicon/Intel macOS, ARM64/x86_64 Linux, ARM64/x86_64 Windows를
지원합니다. release artifact는 이동하는 Rust `stable` 채널로 빌드되며 엄격한
manifest와 SHA-256 checksum을 포함합니다.
실행 파일은 설치된 plugin의 `bin/<target>`에서 찾습니다. plugin 설치가 사용자
shell의 `PATH` 등록까지 보장하는 것은 아닙니다.

## 주요 명령

```console
groundline platform --json
groundline project-audit --repo . --json
groundline config-audit --config /private/config.toml --catalog /private/models.json --json
groundline guidance audit --profile /private/review/profile.json --baseline /private/review/baseline.json --json
groundline audit weekly --days 7 --json
groundline efficiency batch --input batch.json --json
groundline efficiency compare --input comparison.json --json
```

`project-audit`는 Codex guidance, config, skill, agent, rule, plugin,
`.worktreeinclude` 개수만 세고 내용은 읽거나 반환하지 않습니다. audit는 로컬
Codex state store를 수정하지 않으며, efficiency 입력은 외부로 전송하지 않습니다.

`config-audit`는 제공한 네이티브 모델 카탈로그와 설정을 비교합니다.
`align-agent-home`은 개인 스킬 점검을 담당하며, `guidance audit`는 추가·삭제·변경을
확인하고 `guidance snapshot`은 기존 파일을 덮어쓰지 않는 비공개 기준 기록을
만듭니다. 자세한 절차는 [스킬 관리](references/skill-maintenance.md)를 참고하세요.

state database는 현재 사용자 소유의 symlink가 아닌 8 GiB 이하 regular file이어야
합니다. audit은 thread metadata를 최대 100,000개만 읽고, canonical하며 symlink가
아닌 `sessions` 또는 `archived_sessions` 아래 기록만 읽습니다. 압축 해제 입력은
파일당 1 GiB, 감사당 8 GiB, 보관할 집계 레코드는 512 MiB로 제한합니다.
사용량 기준값과 불완전 이력의 처리는 [주간 감사](references/weekly-usage-audit.md)에
정리되어 있습니다.

Insights 선택 기준은 [연동과 설치 프로필](https://github.com/jukqaz/groundline/blob/main/docs/ko/integrations.md), 개발
검증 명령은 [영문 README](README.md)와 [release checklist](https://github.com/jukqaz/groundline/blob/main/docs/release-checklist.md)를
참조하세요.
