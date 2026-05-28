# Provider Activation Matrix

영어 `docs/provider-activation-matrix.md`가 canonical입니다. 이 문서는 한국어
companion입니다.

이 문서는 staged contract harness가 아니라 실제 provider session에서
GroundLine skill이 선택되는지를 추적합니다. raw transcript, auth material,
provider cache dump, full home path, runtime state는 저장하지 않습니다.

## 추적할 prompt family

| prompt family | 기대 skill | 기대 output contract | 상태 |
| --- | --- | --- | --- |
| `handoff` | `package-agent-task` | `GroundLine Task Packet` | PASS |
| `side-effect-guard` | `guard-side-effects` | `Boundary` | PENDING |
| `release-cut` | `stabilize-release-cut` | `GroundLine Release Cut` | PASS |
| `ecosystem-evaluation` | `evaluate-agent-capability` | `GroundLine Capability Evaluation` | PENDING |
| `ai-usage-maturity` | `evaluate-ai-usage-maturity` | `GroundLine AI Usage Maturity` | PENDING |

## 기록 형식

```text
provider: Codex | Claude Code | Antigravity
prompt_family: handoff | side-effect-guard | release-cut | ecosystem-evaluation | ai-usage-maturity
selected_skill: <groundline skill name>
output_contract: <contract name>
mutation_status: none | explicit-user-approved | blocked
result: PASS | PARTIAL | FAIL | PENDING
evidence: 짧은 sanitized summary
raw_transcript_stored: false
provider_home_dumped: false
```

## 원칙

- staged harness 결과는 `docs/provider-dogfood.md`에 둡니다.
- 실제 provider 선택 증거만 이 matrix에 추가합니다.
- `PARTIAL`이나 `FAIL`이 반복되면 바로 새 skill을 만들지 말고
  `recommend-groundline-upgrades` 입력으로 보냅니다.

## 수집 Runbook

실제 provider activation proof는 외부 LLM 서비스를 호출하거나 provider
session metadata를 만들 수 있습니다. provider CLI proof command를 실행하기
전에는 사용자 승인을 받습니다. 승인이 없으면 row는 `PENDING`으로 둡니다.

공통 prompt family는 영어 canonical 문서의 wording을 따릅니다. 기록할 때는
아래 조건을 확인합니다.

- selected skill이 있음
- output contract가 있음
- `mutation_status`가 `none`이거나 승인된 mutation이 명시됨
- `raw_transcript_stored=false`
- `provider_home_dumped=false`
- evidence가 짧고 sanitized됨
- staged harness evidence는 `docs/provider-dogfood.md`에 남김

live provider activation proof로 세지 않는 것:

- `groundline_dogfood.py` staged harness output
- `groundline_provider_smoke.py` install doctor output
- provider version probe
- raw transcript
- provider cache listing
- full provider home path
- mutation status가 없는 증거
