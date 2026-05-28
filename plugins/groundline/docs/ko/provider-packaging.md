# Provider 패키징

GroundLine은 하나의 canonical source tree를 유지하고, 세 provider용 설치
표면을 제공합니다.

- Codex: `.agents/plugins/marketplace.json`이 `plugins/groundline`을 가리킵니다.
- Claude Code: `.claude-plugin/marketplace.json`이 `plugins/groundline`을 가리킵니다.
- Antigravity: repository root와 `plugins/groundline` 모두 같은 `plugin.json` 정체성을 가집니다.

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

## Claude Code

```bash
claude --plugin-dir ./plugins/groundline
claude plugin validate ./plugins/groundline --strict
claude plugin marketplace add jukqaz/groundline
claude plugin install groundline@groundline
```

공식/verified 노출은 provider 검토 영역입니다. 제출 전에는 strict validation을
통과시켜야 합니다.

## Antigravity

```bash
agy plugin validate .
agy plugin validate ./plugins/groundline
agy plugin install https://github.com/jukqaz/groundline
agy plugin install ./plugins/groundline
```

설치 후 `agy plugin list`로 import 상태를 확인합니다.

## 배포 규칙

- 실제 catalog에 올라가기 전에는 공식 등재라고 표현하지 않습니다.
- 세 provider manifest의 version을 맞춥니다.
- runtime 참조는 packaged plugin directory 내부에 둡니다.
- hooks와 MCP 서버는 기본 활성화하지 않고 opt-in으로 문서화합니다.
- 릴리스 전 provider check를 실행합니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
claude plugin validate ./plugins/groundline --strict
agy plugin validate ./plugins/groundline
```
