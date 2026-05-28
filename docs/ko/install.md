# 설치

영어 `docs/install.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

설치는 두 단계로 나눕니다.

1. provider home을 건드리지 않고 clone과 검증을 먼저 합니다.
2. 실제 사용하는 provider에 plugin으로 설치합니다.

1단계를 먼저 해야 설치 전에 패키지 자체가 정상인지 알 수 있습니다.

## 1. Clone과 검증

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

확인할 출력:

- `status=PASS`
- `mutation_performed=false`
- `secret_value_printed=false`
- `install_doctor_status=PASS | PARTIAL | FAIL`
- `real_home_touched=false`
- provider target path는 기본적으로 `~`로 표시

## 주의

provider smoke와 staged dogfood는 실제 provider home에 설치하지 않습니다.
provider smoke는 설치된 target이 있으면 source version, installed version,
payload 존재 여부, skill count drift, provider cache candidate를 비교합니다.
`PARTIAL`은 stale version, stale provider cache, payload 누락, skill count
mismatch를 의미하며 JSON을 출력한 뒤 exit 2로 끝납니다. 실제 provider home에
쓰는 작업은 사용자가 명시적으로 요청한 경우에만 진행합니다.

Context7, Exa, GitHub 같은 외부 도구는 optional입니다. 없으면 doctor가 setup
recommendation만 출력해야 합니다.

## 2. Runtime 상태 확인

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
```

Superpowers가 있으면 `recommended_mode=companion-superpowers`가 나올 수 있습니다.
없어도 GroundLine은 standalone으로 동작해야 합니다.

## 3. Provider 설치

아래 명령들은 provider plugin state를 씁니다. read-only smoke와는 다릅니다.

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

## 4. 설치 확인

Codex:

```bash
codex plugin list --marketplace groundline
```

Claude Code:

```bash
claude plugin list
```

Antigravity:

```bash
agy plugin list
```

provider list가 version을 명확히 보여주지 않으면 clone에서 read-only install
doctor를 다시 실행합니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

설치 뒤 첫 요청 예시:

```text
GroundLine으로 현재 상태를 다시 확인하고 이어서 진행해줘.
```

release 작업이면:

```text
GroundLine으로 release candidate를 polish하고 release cut을 잠가줘.
```

로컬 provider package 검증은 `docs/ko/provider-packaging.md`를 기준으로 합니다.
