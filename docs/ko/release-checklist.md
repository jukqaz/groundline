# Release Checklist

영어 `docs/release-checklist.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

## Release 전 확인

- release version을 하나로 정합니다. Manifest에는 `X.Y.Z`, git tag에는 `vX.Y.Z`를 씁니다.
- README, install, update, privacy, security, license 문서가 현재 package를 정확히 설명하는지 확인합니다.
- 개인 신원, default home path, secret-like value가 문서나 script 출력에 남지 않았는지 확인합니다.
- CI에서 offline gate가 실행되는지 확인합니다.
- Docker나 network 증거가 없으면 PASS가 아니라 PARTIAL로 보고합니다.
- 현재 세션에서 사용자가 명시 승인하기 전에는 tag, push, GitHub Release 생성, repository visibility 변경을 하지 않습니다.

## Version bump 순서

```bash
RELEASE_VERSION=X.Y.Z
TAG="v$RELEASE_VERSION"
```

1. source manifest version만 수정합니다.
   - `plugin.json`
   - `.codex-plugin/plugin.json`
   - `.claude-plugin/plugin.json`
2. `plugins/groundline` manifest는 직접 고치지 않습니다.
   `PYTHONDONTWRITEBYTECODE=1 python3 scripts/sync_provider_package.py --json`로 package payload를 다시 생성합니다.
3. tag 생성 전에 source와 packaged validation을 모두 실행합니다.
   - `PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json`
   - `(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)`
4. 실제 release를 자를 때만 `CHANGELOG.md`의 `Unreleased` 항목을
   `v$RELEASE_VERSION - YYYY-MM-DD`로 옮깁니다. ship decision이 `hold`이면
   `Unreleased`에 그대로 둡니다.
5. package sync 뒤 `plugins/groundline/plugin.json`,
   `plugins/groundline/.codex-plugin/plugin.json`,
   `plugins/groundline/.claude-plugin/plugin.json`가 source manifest version과
   같은지 확인합니다.

## 기본 gate

아래 개별 명령이 기준입니다. Wrapper는 사람이 gate를 빠뜨리는 일을 줄이기
위한 도구이며 tag, push, GitHub Release 생성 명령은 실행하지 않습니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json --release-version "$RELEASE_VERSION"
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --json --keep-going --include-docker-execution --release-version "$RELEASE_VERSION"
```

ship decision이 아직 `hold`이고 source manifest가 현재 public version에 머무는
상태라면 `--release-version`은 생략합니다. 실제 release를 자를 때만 붙이며,
source 또는 packaged manifest가 `RELEASE_VERSION`과 다르거나 `RELEASE_VERSION`이
plain `X.Y.Z` semver가 아니면 gate는 실패합니다.

release closeout에서는 `--keep-going`을 사용합니다. version bump 뒤 설치된
provider cache가 이전 버전이라 provider smoke가 정상적으로 `PARTIAL`을 내더라도
뒤의 dogfood와 scenario gate 증거를 계속 수집하기 위해서입니다. 단 하나라도
partial gate가 있으면 wrapper 결과도 계속 `PARTIAL`입니다.

gate가 JSON을 출력하면 wrapper는 `status`, `install_doctor_status`,
`install_issues`, `stage_package`, `temp_state_created`, `next_actions` 같은
핵심 필드를 `json_summary`로 보존합니다. 긴 `stdout_tail`을 읽기 전에 이 요약을
먼저 봅니다.
top-level `non_passing_gates`와 `next_actions`는 현재 blocker set을 사람과
LLM handoff용으로 요약합니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_validate.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_safety_eval.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_privacy_scan.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json --stage-package --require-installed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json --require-installed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

## Release decision

- `CHANGELOG.md`가 release scope를 설명하는지 확인합니다.
- `docs/maturity-assessment.md`가 `ship`, `hold`, `continue` 중 하나를 기록하는지 확인합니다.
- ship decision이 `hold`이면 사용자가 부족한 증거를 이번 release에서 수용한다고 명시하기 전에는 여기서 멈춥니다.
- live provider activation proof가 없으면 accepted partial인지 release blocker인지 기록합니다.
- provider-native validation이 `claude` 또는 `agy` 부재로 `PARTIAL`이면 이번
  release에서 수용 가능한 local validator gap인지 기록합니다. validator 실패는
  release blocker로 봅니다.

## 승인 필요 배포 명령

아래 명령은 remote repository나 public release 상태를 변경합니다. 현재 세션에서
명시 승인을 받기 전에는 실행하지 않습니다.

```bash
git tag "$TAG"
git push origin "$TAG"
gh release create "$TAG" --repo jukqaz/groundline --title "$TAG" --notes-file CHANGELOG.md
```

## 배포 후 확인

- GitHub Release URL
- tag와 main의 target commit
- CI run success
- release page HTTP 200
- published ref에서 provider install confirmation 실행
- `groundline_provider_smoke.py --json --require-installed`로 설치된 provider target이 published ref와 맞는지 확인
- previous version 대비 delta
- rollback note
