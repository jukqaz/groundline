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
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --json --include-docker-execution
```

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_safety_eval.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
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
- previous version 대비 delta
- rollback note
