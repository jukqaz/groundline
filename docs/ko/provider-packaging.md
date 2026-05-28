# Provider 패키징

이 문서는 repository clone만 하는 경우가 아니라 Codex, Claude Code,
Antigravity의 plugin 설치 표면으로 GroundLine을 쓰고 싶은 사람을 위한 문서입니다.

GroundLine은 하나의 canonical source tree를 유지하고, 세 provider용 설치
표면을 제공합니다.

- Codex: `.agents/plugins/marketplace.json`이 `plugins/groundline`을 가리킵니다.
- Claude Code: `.claude-plugin/marketplace.json`이 `plugins/groundline`을 가리킵니다.
- Antigravity: repository root와 `plugins/groundline` 모두 같은 `plugin.json` 정체성을 가집니다.

## 설치 전 확인

먼저 read-only 검증을 실행합니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

이 단계는 provider home을 쓰지 않습니다. 아래 provider install 명령은 provider
plugin state를 씁니다.

`groundline_provider_smoke.py`는 version-aware install doctor 역할도 합니다.
`install_doctor_status`, source version, installed version, payload 누락,
skill count mismatch, provider cache candidate, package drift를 보고하되 auth,
session, log, provider home dump는 출력하지 않습니다.

## 어떤 경로를 쓰나

| 목표 | 경로 |
| --- | --- |
| 일반 사용자처럼 공개 package 설치 | GitHub marketplace/source 추가 후 `groundline@groundline` 설치 |
| release 전 local edit 테스트 | `./plugins/groundline` 또는 local checkout marketplace 사용 |
| package shape만 확인 | `validate_pack.py`, `claude plugin validate`, `agy plugin validate` |
| provider catalog 제출 | validation 뒤 provider review 절차 진행 |

## Codex

```bash
codex plugin marketplace add jukqaz/groundline --ref main
codex plugin list --marketplace groundline
codex plugin add groundline@groundline
```

로컬 개발 중에는 checkout 경로를 marketplace로 추가합니다.

```bash
codex plugin marketplace add .
codex plugin add groundline@groundline
```

`plugins/groundline` 패키지는 `skills/`, `docs/`, `references/`, `scripts/`,
`assets/`를 포함합니다. 단, source-only release planning material인
`docs/superpowers/`는 installable provider package에서 의도적으로 제외합니다.

확인:

```bash
codex plugin list --marketplace groundline
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

## Claude Code

```bash
claude --plugin-dir ./plugins/groundline
claude plugin validate ./plugins/groundline --strict
claude plugin marketplace add jukqaz/groundline
claude plugin install groundline@groundline
```

공식/verified 노출은 provider 검토 영역입니다. 제출 전에는 strict validation을
통과시켜야 합니다.

확인:

```bash
claude plugin list
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

## Antigravity

```bash
agy plugin validate .
agy plugin validate ./plugins/groundline
agy plugin install https://github.com/jukqaz/groundline
agy plugin install ./plugins/groundline
```

설치 후 `agy plugin list`로 import 상태를 확인합니다.

Antigravity list가 semantic version 대신 import metadata만 보여줄 수 있습니다.
그 경우 clone에서 `agy plugin validate ./plugins/groundline`으로 package shape를
확인하고, `groundline_provider_smoke.py`로 installed payload와 skill count를
비교합니다.

## 배포 규칙

- 실제 catalog에 올라가기 전에는 공식 등재라고 표현하지 않습니다.
- 세 provider manifest의 version을 canonical `plugin.json`과 맞춥니다.
- runtime 참조는 packaged plugin directory 내부에 둡니다.
- hooks와 MCP 서버는 기본 활성화하지 않고 opt-in으로 문서화합니다.
- 릴리스 전 provider check를 실행합니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
claude plugin validate ./plugins/groundline --strict
agy plugin validate ./plugins/groundline
```
