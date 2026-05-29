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
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json --stage-package --require-installed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_remote_install_probe.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

GitHub CLI를 쓰는 경우:

```bash
gh repo clone jukqaz/groundline
cd groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
```

확인할 출력:

- package validation은 `status=PASS`
- staged provider smoke는 `status=PASS`, `fake_home_used=true`,
  `real_home_touched=false`
- remote install/update proof는 fake provider home에서 fresh install,
  stale update detection, post-update refresh를 증명하고 `status=PASS`
- staged dogfood는 `status=PASS`
- provider smoke는 설치본이 source와 일치하면 `status=PASS`, 기존 provider
  target이 checkout package보다 stale이면 `status=PARTIAL`
- `mutation_performed=false`
- `secret_value_printed=false`
- `install_doctor_status=PASS | PARTIAL | FAIL`
- `real_home_touched=false`
- provider target path는 기본적으로 `~`로 표시

provider smoke `FAIL`은 설치 blocker입니다. provider smoke `PARTIAL`이면
먼저 top-level `next_actions`를 보고 조치 범위를 판단합니다.
provider 설치 확인이 목적이라면 `--require-installed`를 붙여 provider target
부재도 `PARTIAL`로 처리합니다.

provider smoke가 실제 provider target의 stale 상태 때문에만 `PARTIAL`이면,
임시 fake provider home에 현재 package를 배치해서 package 자체가 설치 가능한지
증명할 수 있습니다. `groundline_provider_smoke.py --json --stage-package
--require-installed`를 실행하면 이 proof는 `status=PASS`,
`fake_home_used=true`, `real_home_touched=false`를 반환해야 합니다.

원격 설치와 업데이트 흐름까지 함께 증명하려면
`groundline_remote_install_probe.py --json`을 실행합니다. 이 probe는 실제
provider home을 건드리지 않고, 이전 버전 설치본이 `PARTIAL`로 감지되고 refresh
뒤 `PASS`로 돌아오는지 확인합니다.

## 주의

provider smoke와 staged dogfood는 실제 provider home에 설치하지 않습니다.
provider smoke는 설치된 target이 있으면 source version, installed version,
payload 존재 여부, skill count drift, same-version content drift, provider
cache candidate를 비교합니다. `PARTIAL`은 stale version, stale provider
cache, payload 누락, skill count mismatch, `content_fingerprint_mismatch`를
의미합니다. `--require-installed`를 쓰면 not installed도 `PARTIAL`입니다.
이 경우 JSON을 출력한 뒤 exit 2로 끝납니다. 실제 provider home에 쓰는 작업은
사용자가 명시적으로 요청한 경우에만 진행합니다.

결과가 `PARTIAL` 또는 `FAIL`이면 먼저 top-level `next_actions`를 봅니다.
각 provider 항목의 `recommended_actions`에는 해당 target을 위한 조치가
따로 들어갑니다.

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
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_validate.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json --require-installed
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
