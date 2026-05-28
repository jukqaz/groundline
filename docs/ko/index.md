# GroundLine 한국어 문서

이 디렉터리는 사람이 읽기 위한 한국어 companion 문서입니다. 영어 문서가 기본이자
canonical입니다.

## 시작하기

1. 처음 5분이면 `README.ko.md`와 `install.md`만 봅니다.
2. 실제 agent session에서 어떻게 요청할지 막히면 `human-guide.md`를 봅니다.
3. prompt 예시가 필요하면 `examples.md`를 봅니다.
4. 어떤 skill을 골라야 할지 헷갈리면 `skill-portfolio.md`를 봅니다.
5. release 전에는 `release-checklist.md`와 영어 `docs/release-checklist.md`를 함께 봅니다.

## 가장 흔한 경로

```text
install.md -> human-guide.md -> examples.md -> skill-portfolio.md
```

provider에 설치하기 전에는 먼저 read-only 검증을 통과시키세요.

## 문서 목록

| 문서 | 용도 |
| --- | --- |
| `human-guide.md` | 사람용 운영 가이드 |
| `install.md` | 설치와 첫 검증 |
| `update.md` | 업데이트와 재검증 |
| `examples.md` | 대표 workflow 예시 |
| `skill-portfolio.md` | skill 선택 요약 |
| `provider-guardrails.md` | hooks, rules, MCP 기본 제외와 opt-in 기준 |
| `mcp-recipes.md` | private MCP와 optional tool profile 판단 기준 |
| `privacy.md` | privacy와 secret 경계 |
| `release-checklist.md` | release 전 확인 |
| `next-version.md` | 다음 버전 작업 방향 |
| `provider-packaging.md` | 세 provider 설치 패키징 |
| `terms.md` | 이용 조건 |

## 언어 정책

- 영어 문서가 기본입니다.
- 한국어 문서는 빠른 이해를 위한 companion입니다.
- LLM-readable reference와 skill 본문은 영어를 기준으로 봅니다.
- 한국어 문서와 영어 문서가 다르면 영어 문서를 먼저 믿고, 한국어 문서를 갱신합니다.
