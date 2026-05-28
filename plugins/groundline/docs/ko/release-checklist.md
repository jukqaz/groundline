# Release Checklist

영어 `docs/release-checklist.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

## Release 전 확인

- release version을 하나로 정합니다.
- README, install, update, privacy, security, license 문서가 현재 package를 정확히 설명하는지 확인합니다.
- 개인 신원, default home path, secret-like value가 문서나 script 출력에 남지 않았는지 확인합니다.
- CI에서 offline gate가 실행되는지 확인합니다.
- Docker나 network 증거가 없으면 PASS가 아니라 PARTIAL로 보고합니다.

## 기본 gate

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

## 배포 후 확인

- GitHub Release URL
- tag와 main의 target commit
- CI run success
- release page HTTP 200
- previous version 대비 delta
- rollback note
