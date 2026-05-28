# Provider Guardrails

GroundLine 기본값은 skills-only입니다. hooks, rules, MCP servers, commands,
provider-level agents는 기본으로 설치하거나 활성화하지 않습니다.

Codex, Claude Code, Antigravity가 각자 sandbox, approval, permission, tool
review, plugin loading, private tool connection을 담당하기 때문입니다.

## 기본 포함

- skills
- output contracts
- references
- docs
- offline scripts
- staged package validation

## 기본 제외

- hooks
- rules
- MCP servers
- slash commands
- provider-level agents
- prompt/transcript logging
- 자동 provider-home write
- background network work

## 왜 제외하나

hooks와 rules는 모든 session이나 repository에 영향을 줄 수 있습니다. 특정 provider,
repo, failure mode가 충분히 검토된 뒤 opt-in으로 켜는 것이 맞습니다.

안전한 분리는 다음과 같습니다.

- GroundLine은 operating contract를 정의합니다.
- provider는 permission과 tool execution을 담당합니다.
- 사용자는 write, external service change, broader automation을 승인합니다.

## MCP를 붙일 때

MCP는 live GitHub/CI state, 최신 문서, private docs, private code search처럼
skill만으로 부족한 tool access가 필요할 때만 씁니다.

먼저 `docs/mcp-recipes.md`와 `references/optional-mcp-profiles.md`를 확인합니다.

## hooks/rules를 고려할 때

다음 조건을 만족할 때만 opt-in으로 고려합니다.

- 같은 unsafe action이 실제 session에서 반복됨
- provider-native permission만으로 부족함
- 동작이 좁고 deterministic함
- repo나 provider별로 끌 수 있음
- 사용자가 명시적으로 켜기로 함

좋은 후보:

- destructive command confirmation
- provider-home write warning
- release completion reminder

나쁜 후보:

- prompt logging
- transcript analytics
- broad auto-approval
- long network work
- secret output capture
- automatic package publish

