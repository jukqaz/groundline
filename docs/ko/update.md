# 업데이트

영어 `docs/update.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

## 기본 업데이트

```bash
cd groundline
git pull --ff-only
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_safety_eval.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_remote_install_probe.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

## Release gate

release 전에는 `docs/release-checklist.md`를 기준으로 전체 gate를 확인합니다.
최소한 아래 gate를 추가로 실행합니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_validate.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

## 원칙

- validation이 PASS 되기 전에는 provider home, rules, hooks, skills를 바꾸지 않습니다.
- side effect가 있는 변경은 사용자의 명시적 승인 뒤에 진행합니다.
- 실패하면 원인을 package validation, script lint, runtime layout, dogfood, Docker로 나눠서 봅니다.
- `groundline_provider_smoke.py`가 `PARTIAL`이면 `install_doctor_status`와 provider별
  `runtime_probe.issues`를 먼저 읽습니다. 흔한 원인은 stale installed version,
  provider payload 누락, skill count drift입니다.
- `groundline_remote_install_probe.py --json`은 fake provider home에서 fresh
  install, 이전 버전 감지, refresh 후 PASS까지 확인합니다. 실제 provider home은
  바꾸지 않습니다.
