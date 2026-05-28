# 업데이트

영어 `docs/update.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

## 기본 업데이트

```bash
cd groundline
git pull --ff-only
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

## Release gate

release 전에는 Linux Docker scenario도 확인합니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

## 원칙

- validation이 PASS 되기 전에는 provider home, rules, hooks, skills를 바꾸지 않습니다.
- side effect가 있는 변경은 사용자의 명시적 승인 뒤에 진행합니다.
- 실패하면 원인을 package validation, script lint, runtime layout, dogfood, Docker로 나눠서 봅니다.
