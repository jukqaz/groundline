# Public Release

영어 `docs/public-release.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

Repository visibility를 바꾸거나 새 release를 공개하기 전에 이 문서를 봅니다.
이 체크는 release checklist를 대체하지 않고, 공개 전 privacy와 proof 기준을
분리해서 확인하기 위한 보조 문서입니다.

## Identity

- `LICENSE`는 `GroundLine contributors`를 사용합니다.
- provider manifest는 `GroundLine` 또는 `GroundLine contributors`를 사용합니다.
- 문서는 private GitHub 계정을 요구하지 않습니다.
- 개인 경로, local hostname, provider runtime state가 commit된 파일에 남아 있지 않아야 합니다.
- commit author와 committer metadata에 공개하면 안 되는 개인/회사 식별자가 없는지 확인합니다.

## Safety

- secret scan에서 token, auth file, private key가 없어야 합니다.
- 기본 script 출력은 full user home path를 출력하지 않아야 합니다.
- 외부 tool probe는 read-only여야 합니다.
- radar command source는 opt-in이어야 하며 shell string을 거부해야 합니다.
- CI에서 내려받는 tool은 version pin과 checksum 검증을 유지해야 합니다.
- 현재 세션에서 사용자가 명시 승인하기 전에는 tag, push, GitHub Release 생성,
  repository visibility 변경을 하지 않습니다.
- `docs/maturity-assessment.md`의 ship decision이 `hold`이면, 사용자가 이번
  release의 부족한 proof를 수용한다고 명시하기 전에는 공개하지 않습니다.

## Release Evidence

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --json --keep-going --include-docker-execution
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_validate.py --json
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

모든 필수 evidence가 `PASS`이거나, 명시적으로 수용한 `PARTIAL`이 문서화되기
전에는 repository를 공개로 돌리지 않습니다.

배포 명령은 `docs/ko/release-checklist.md`의 `승인 필요 배포 명령`에만 둡니다.
read-only release evidence block에 tag, push, GitHub Release 생성 명령을 섞지 않습니다.

git history에 공개하면 안 되는 author metadata나 과거 file content가 있으면
기존 repository를 그대로 공개하지 말고 `docs/git-history-privacy.md`를 따라
sanitized history에서 공개합니다.
