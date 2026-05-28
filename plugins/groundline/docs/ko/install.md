# 설치

영어 `docs/install.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

## Clone

```bash
git clone https://github.com/jukqaz/groundline.git
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

GitHub CLI를 쓰는 경우:

```bash
gh repo clone jukqaz/groundline
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
```

## 확인할 출력

- `status=PASS`
- `mutation_performed=false`
- `real_home_touched=false`
- provider target path는 기본적으로 `~`로 표시

## 주의

provider smoke와 staged dogfood는 실제 provider home에 설치하지 않습니다.
실제 provider home에 쓰는 작업은 사용자가 명시적으로 요청한 경우에만 진행합니다.

Context7, Exa, GitHub 같은 외부 도구는 optional입니다. 없으면 doctor가 setup
recommendation만 출력해야 합니다.

## Marketplace 설치

Codex:

```bash
codex plugin marketplace add jukqaz/groundline --ref main
codex plugin list --marketplace groundline
codex plugin add groundline@groundline
```

Claude Code:

```bash
claude plugin marketplace add jukqaz/groundline
claude plugin install groundline@groundline
```

Antigravity:

```bash
agy plugin install https://github.com/jukqaz/groundline
```

로컬 provider package 검증은 `docs/ko/provider-packaging.md`를 기준으로 합니다.
