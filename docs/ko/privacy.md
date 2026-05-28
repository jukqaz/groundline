# Privacy

영어 `docs/privacy.md`가 canonical입니다. 이 문서는 한국어 companion입니다.

## GroundLine이 읽을 수 있는 것

- GroundLine checkout 안의 repository files
- 명시적으로 넘긴 fake home 또는 temp home
- 사용자가 요청한 read-only command 결과

## 수집하면 안 되는 것

- credentials
- provider auth files
- raw transcripts
- local session databases
- shell history 전체
- private prompt archives
- full default home paths

## 출력 원칙

- secret value는 출력하지 않습니다.
- 기본 home path는 `~`로 줄여서 표시합니다.
- `mutation_performed=false`는 해당 command에 대한 증거일 뿐, 전체 작업의 무변경을 뜻하지 않습니다.
- provider home 쓰기, deploy, access, billing, destructive git 작업은 명시적 승인 뒤에만 진행합니다.

## 테스트할 때

가능하면 temp home을 씁니다.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
```

실제 provider home을 stage target으로 쓰려는 시도는 거부되어야 합니다.
