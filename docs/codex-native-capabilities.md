# Codex 기능을 지속 개선과 환경 통일에 연결하기

2026-10-06 공식 문서 조사. GroundLine은 Codex가 제공하는 기능을 우선 사용하고
사용 패턴·작업 결과·환경 검증에 필요한 연결을 보완합니다. 아래 표는 발동 조건과
설계상의 사용 위치이며 모든 기능이 현재 계정/기기/작업에서 활성화됐다는 목록은 아닙니다.

## 기능별 사용 위치

| Codex 기능 | GroundLine에서 쓸 위치 | 필요할 때의 선택·검증 | 공식 근거 |
| --- | --- | --- | --- |
| 설정 layer·profile·native App Server | 개인 공통 기준과 기기/프로젝트 예외의 차이 확인 | 관련 `config/read`와 App/PATH 해석을 비교. 디스크 계층과 실행 중 선택/override를 구분 | [Config](https://learn.chatgpt.com/docs/config-file/config-basic), [App Server](https://learn.chatgpt.com/docs/app-server) |
| `AGENTS.md`·skills·plugins | 공통 작업 원칙과 개인 workflow의 배포·갱신 | 짧은 사용 조건, 필요한 본문만 로드, 소스 revision·활성 경로·중복 확인. plugin cache 직접 수정 없음 | [Customization](https://learn.chatgpt.com/docs/customization/overview), [Build skills](https://learn.chatgpt.com/docs/build-skills) |
| 모델 catalog·effort·native subagents | 작업에 맞는 모델·추론·위임과 전체 비용 비교 | 명시 선택과 기본 상속을 유지하고 실제 child 선택을 관측. 독립 작업의 이득과 조율 비용으로 판단 | [Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents), [App Server](https://learn.chatgpt.com/docs/app-server) |
| Git worktrees·local environments | 환경 관리 코드/지침 변경과 검증의 격리 | 적합한 기존 worktree 재사용, Git 시작 상태·설정·의존성 확인. setup 비용과 외부 효과 포함 | [Worktrees](https://learn.chatgpt.com/docs/environments/git-worktrees) |
| `.worktreeinclude` | tracked 파일만으로 준비되지 않는 로컬 의존 파일 전달 | 로컬 App 관리 worktree에서 필요한 ignored 파일만. 원격·수동 Git worktree에 일반화하거나 비밀 파일 광범위 복사 없음 | [Worktree include](https://learn.chatgpt.com/docs/environments/git-worktrees#copy-ignored-local-files-into-managed-worktrees) |
| native review·diff·GitHub PR/checks | 지침/환경 코드 변경, 검토 피드백, 개선 근거 연결 | 실제 diff·결함·CI 결과 확인. 패널 생성과 리뷰 수행을 구분하고 같은 변경의 반복 리뷰를 강제하지 않음 | [Code review](https://learn.chatgpt.com/docs/code-review) |
| native 실행·대기·terminal | inspect/plan/apply와 명령 결과 검증 | 직접 도구와 실제 실행 handle 사용. 시작을 완료로 기록하지 않고 기존 작업을 기다림 | [Long-running work](https://learn.chatgpt.com/docs/long-running-work), [App Server](https://learn.chatgpt.com/docs/app-server) |
| MCP·서비스 connector | GitHub 관리·공식 자료·권한 있는 구조화 데이터 접근 | 정확한 tool·계정·인증·범위 확인, 저장 후 재읽기. plugin 등록만으로 연결 성공을 추정하지 않음 | [MCP](https://learn.chatgpt.com/docs/extend/mcp) |
| Browser·기존 Chrome 연결 | 로컬 웹 결과와 로그인 웹 경로 검증 | 로컬 웹은 Browser, 로그인 웹은 사용자 Chrome. 실제 세션·화면·저장 결과 확인 | [Browser](https://learn.chatgpt.com/docs/browser) |
| Computer Use | native API가 설명하지 못하는 로컬 앱 화면·동작 확인 | 해당 앱과 macOS 접근/화면 권한 확인. 요청한 UI 경로에서 실제 결과 확인 | [Computer Use](https://learn.chatgpt.com/docs/computer-use) |
| Remote·handoff·notification | 대상 기기의 적용/검증과 완료·판단 필요 상태 복귀 | 연결된 host에서 실행됐는지 확인. 기기별 영수증 유지, 의미 있는 변화·실패·결정 필요 상태만 알림 | [Remote engineering](https://learn.chatgpt.com/blog/mastering-codex-remote-for-engineering) |
| scheduled task·chat follow-up | 공식 변경·환경 차이·적용 후 결과의 후속 확인 | 사용자가 요청한 일정·변경 범위에서 native 자동화 사용. host 실행 조건과 분석 비용 확인 | [Automations](https://learn.chatgpt.com/docs/automations) |
| native Goal·task continuity·memory | 장기 작업의 완료 조건·정정·진행 상태 유지 | 명시 Goal 요청에는 native Goal 사용. 일반 작업은 같은 채팅에서 연속 처리. 관련 memory만 읽고 저장 사실과 현재 근거 구분 | [Long-running work](https://learn.chatgpt.com/docs/long-running-work), [Customization](https://learn.chatgpt.com/docs/customization/overview) |
| context compaction·usage | 컨텍스트 압박·중복 설명·장기 작업 비용 분석 | native usage/compaction 관측과 결과 연결. native context 관리 유지, API cache 옵션을 Codex TOML로 옮기지 않음 | [App Server](https://learn.chatgpt.com/docs/app-server#trigger-thread-compaction) |
| 파일·artifact preview·작업용 dependency | 사용 패턴 보고서와 개선안을 검토 가능한 결과로 제공 | 필요한 표·차트·파일을 만들고 실제 렌더링과 수치를 확인. App preview와 CLI의 파일 생성/경로 보고를 구분 | [Work with files](https://learn.chatgpt.com/docs/artifacts-viewer) |
| hooks·native trust | Insights의 결과 수집·정의 변경 뒤 활성 확인 | event 지원·정의 hash·trust·실제 실행을 구분. Core는 hook-free 유지, 턴마다 LLM 분석을 실행하지 않음 | [Hooks](https://learn.chatgpt.com/docs/hooks) |
| sandbox·rules·approvals·auto-review | 원래 요청한 작업의 권한 경계와 실제 실패 진단 | command network·MCP·browser·앱 승인을 구분. 이미 승인된 범위의 좁은 규칙과 native 정책 사용 | [Security](https://learn.chatgpt.com/docs/agent-approvals-security), [Auto-review](https://learn.chatgpt.com/docs/sandboxing/auto-review) |

## 환경 기준에 보존할 기능 근거

각 기능에 native surface, 공식 URL/확인일, host 지원, 현재 available/enabled/trusted,
발동 조건, 소유자, 마지막 검증 근거를 연결합니다. 복잡한 기능별 플랫폼이나 별도 updater는
만들지 않습니다. 공식 지원·tool 목록에 존재·계정/작업에서 사용 가능·실행 권한·성공 결과를
따로 기록하며 unknown/unsupported/disabled/native 승인 필요를 구분합니다.

CLI feature flag는 전체 App 기능 목록이 아닙니다. 동일한 이름이 보여도 runtime별 지원과
현재 task override는 별도입니다. 저장된 config와 명령의 feature 표시를 현재 채팅의 활성
기능으로 일반화하지 않습니다. Remote에 파일만 복사해 native trust가 같다고 판단하지 않습니다.

현재 API 문서의 prompt/cache 옵션은 Codex 사용자 설정 지원의 근거가 아닙니다.
모델/effort·권한·네트워크·feature 변경은 사용자 의도와 실제 필요를 확인한 범위에서 수행합니다.
기능을 사용한 횟수와 필요한 작업에서 유익했던 결과도 구분합니다.

## 패턴 분석에 연결하는 방식

작업 목적·기회·선택한 native 기능·실제 operation handle·검증된 결과가 관측되면 연결합니다.
기회나 활성 상태를 확인하지 못한 기록은 unknown으로 남깁니다. 호출이 없다는 이유만으로
활용 실패를 판정하거나 기능 활용 점수를 목표로 삼지 않습니다.

예를 들어 독립 조사가 있는 작업에서 subagent의 조사 품질·소요 시간과 전체 부모/자식
비용을 비교하고, 필요한 UI 검증이 있는 작업에서는 실제 Browser/Computer Use 결과가
누락됐는지 확인합니다. 단순 문서 수정에 UI·위임·Goal을 의무화하지 않습니다.

설계는 [개선 순환과 환경 기준](adaptive-environment-design.md), 현재 agent 판단은
[capability routing](../plugins/groundline/references/capability-routing.md)을 사용합니다.
지속적인 기능 기회/결과 기록은 후속 구현이며 현재 `efficiency route`에 해당 입력을
추가했다고 보고하지 않습니다.
