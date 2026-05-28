# MCP 레시피

GroundLine core는 MCP가 필수가 아닙니다. 기본 패키지는 Codex, Claude Code,
Antigravity에서 portable하게 쓰이도록 skills-only로 둡니다.

MCP는 skill만으로 부족한 live/private tool access가 필요할 때만 붙입니다.

## 판단 기준

skill만으로 충분한 경우:

- 현재 상태 증명
- handoff
- release cut
- side-effect boundary
- dogfood evidence
- AI usage 평가
- 제공된 source를 바탕으로 한 research, compare, recommend

MCP가 필요한 경우:

- GitHub issue, PR, CI, release, branch 상태가 proof에 필요함
- 최신 외부 문서 조회가 필요함
- private runbook이나 내부 문서가 필요함
- 큰 private codebase의 indexed search가 필요함
- 검토된 private service에 구조화된 read access가 필요함

## 권장 구조

- GroundLine은 profile과 evidence boundary를 제안합니다.
- provider가 MCP 연결, 인증, 권한 범위, 실행을 담당합니다.
- repository에는 token, raw private doc, transcript archive, provider home dump를 저장하지 않습니다.

## 대표 profile

| Profile | 쓸 때 | 같이 쓰는 skill |
| --- | --- | --- |
| GitHub evidence | PR, issue, CI, release, tag 확인 | `reconcile-current-state`, `close-live-work`, `compare-release-delta` |
| Documentation lookup | 최신 provider/API/package 문서 확인 | `research-agent-ecosystem`, `recommend-groundline-upgrades` |
| Ecosystem research | agent tool, skill, MCP, workflow 비교 | `agent-ecosystem-radar`, `compare-agent-workflows` |
| Private docs | 내부 runbook, 운영 문서 확인 | `audit-agent-history`, `package-agent-task`, `align-agent-home` |
| Private code search | local search로 부족한 큰 private codebase 탐색 | `reconcile-current-state`, `evaluate-groundline-pack` |

상세 profile은 영어 canonical 문서 `references/optional-mcp-profiles.md`를 봅니다.

## 안전 체크

- 이 작업에 MCP가 꼭 필요한지 확인합니다.
- read/write 범위를 분리합니다.
- write는 기본 비활성으로 둡니다.
- private data는 짧은 evidence summary로만 남깁니다.
- provider-native permission과 approval을 우회하지 않습니다.

