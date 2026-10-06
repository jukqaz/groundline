# 사용 경험과 모델 변화에 따른 Codex 환경 개선

상태: 2026-10-06 재설계 및 [로컬 구현](adaptive-environment-implementation.md).
설계 기준과 구현·설치·활성·후속 효과의 검증을 구분합니다.
이 문서는 자동 학습·환경 동기화가 이미 동작한다는 설명이 아닙니다.
공식 자료와 유사 프로젝트 조사에 따른 우선순위·구현 완료 기준은
[개선사항](adaptive-environment-improvements.md)에 정리합니다.

## 목적과 완료 기준

GroundLine의 목표는 사용 패턴과 실제 작업 결과를 분석하고, 현재 모델의
공식 권장 방식을 반영해 사용자의 공통 Codex 환경을 계속 개선하는 것입니다.
환경 통일은 같은 파일의 복사뿐 아니라 같은 의도와 기준이 실제 적용됐는지
확인하는 일입니다. 기기별 차이와 프로젝트 규칙은 각 소유자가 유지합니다.

개선 순환은 **관측 → 개선 후보 → 기준 갱신 → 환경 적용 → 다음 결과 → 재평가**입니다.
학습의 대상은 운용 지침·작업 방식·환경 기준입니다. 성공 기준은 다음과 같습니다.

- 실제 완료 품질을 유지하며 재작업·반복 오류·불필요한 사용자 개입을 줄인다.
- 모델·추론 강도·위임·기능 사용과 부모·자식·실패·재시도 자원을 함께 설명한다.
- 공통 기준, 기기에 기록된 상태, 실제 활성 상태의 차이를 드러내고 복구한다.
- 모델/런타임 변화로 불필요해진 규칙을 수정하거나 제거한다.
- 분석·지침 로딩·검증·운영의 비용까지 포함해 이익을 평가한다.

현재 Mac의 Codex App과 PATH CLI를 첫 적용 대상으로 삼습니다. macOS/Linux의
다른 기기는 같은 계약을 사용하되 기기별 적용 결과를 따로 기록합니다.
모든 환경에서 같아야 하는 항목과 의도적으로 다른 항목을 먼저 지정합니다.

## 현재 구현과 필요한 연결

| 영역 | 현재 구현 | 추가할 연결 |
| --- | --- | --- |
| 사용 관측 | native audit, root/child 소유·중복·미관측 처리, Insights 집계 | 작업 결과와 당시 환경 revision 연결 |
| 모델 패턴 | [PR #57](https://github.com/jukqaz/groundline/pull/57)의 실제 ID·effort·응답·토큰·coverage | 해당 PR 병합 후 패턴을 비교 보고서의 설명 자료로 연결 |
| 결과 기록 | `efficiency record-delivery`의 증거 hash, 완료/실패/unknown, 재작업·자원 | 환경/공식 지침 revision, 명시 정정과 결과 근거의 연결 |
| 비교 | `efficiency route --deliveries`, phase/cohort의 직접 결과 비교 | 후보 채택·적용·후속 결과의 지속 기록, 모델/환경 변화에 따른 분리 |
| 설정 변경 | `setup`, `config-repair`의 preview·백업·재읽기·적용 | 개인 공통 기준, 기기 예외, 관리 범위, 환경 변경 영수증 |
| 복구 | 설치 source rollback, 폐기된 개인 trial의 status/rollback | 새 환경 변경의 충돌 감지·부분 복구·실제 활성 재확인 |
| 모델 변화 | 현재 native catalog의 모델·effort 지원 확인 | 공식 문서 변경과 검토된 지침 revision 연결 |

현재 `personal`은 기존 상태 복구 전용입니다. 새 학습 상태를 폐기된 trial 형식에
추가하거나 기존 상태를 자동 변환하지 않습니다. 기존 route의 표본 수·개선율
조건은 현재 모델 비교의 운영 조건이며 모든 지침 수정에 적용할 학습 기준이 아닙니다.
현재 최적화의 모델 범위도 native catalog 전체 지원과 구분해 확장해야 합니다.

## 소유 경계

| 대상 | 기준과 책임 |
| --- | --- |
| Codex | 실행·설정 우선순위·모델 적용·권한·agents·skills/MCP·hook trust |
| 공용 GroundLine | 관측·비교·변경 계획·검증·복구 기능과 일반적인 사용법 |
| 개인 공통 기준 | 사용자의 명시 선호, 관리 가능한 항목, 보호 항목, 변경 근거·revision |
| 기기별 예외 | OS·경로·설치 위치·지원 기능·정상 연결 참조와 예외 이유 |
| 프로젝트 | 가까운 `AGENTS.md`와 프로젝트 config, 빌드·테스트·배포 규칙 |

공용 저장소에 개인 설정·접속 주소·인증·대화 원문을 포함하지 않습니다.
개인 공통 기준은 비공개 로컬 파일 또는 사용자가 선택한 비공개 Git에서 관리합니다.
배포 플러그인 revision과 개인 기준 revision은 독립적입니다.
GroundLine은 native 설정 우선순위를 재구현하거나 별도 실행기·GUI를 만들지 않습니다.

## 학습에 사용하는 근거

사용량은 개선 후보를 찾는 근거이고 실제 결과는 효과를 판단하는 근거입니다.
사용자 정정·수락·요청 변경, 테스트·검토·배포의 확인, assistant 자기 보고를 구분합니다.
응답 횟수는 완료 작업 수가 아니며 사용자 침묵은 수락으로 기록하지 않습니다.

개인 선호는 명시한 범위에서 즉시 기준이 될 수 있습니다. 반복 관측은 학습 가설입니다.
가설만으로 명시 선호를 바꾸지 않습니다. Codex memory의 자동 편집도 학습 경로로 삼지 않습니다.
모델/작업/환경이 다른 결과를 무조건 합치거나 미관측을 0으로 채우지 않습니다.
토큰·시간이 작아도 미완료·품질 저하가 있으면 개선으로 판정하지 않습니다.

후보 하나에 최소한 다음을 연결합니다.

| 필드 | 의미 |
| --- | --- |
| `proposal_id`, `basis_revision` | 후보 ID와 출발한 개인 기준 |
| `evidence_refs`, `source_revision` | 결과·명시 정정 근거와 공식 자료의 검토 revision |
| `scope`, `managed_targets` | 작업 종류·모델/런타임·기기와 수정할 파일/키/구간 |
| `hypothesis`, `expected_result` | 발생한 문제, 제안하는 변경, 확인할 결과 |
| `authority_ref` | 사용자 요청 또는 사전에 허용한 관리 범위 |
| `verification`, `rollback_ref` | 적용 검사, 반증 조건, 되돌릴 기준 |
| `followup_refs`, `status` | 이후 결과와 관측/적용/효과 확인 상태 |

이 필드들은 추가 구현 계약이며 현재 CLI 입력 형식은 아닙니다.
보고서에는 원문 대신 범위와 확인된 의미만 남깁니다. 로그·외부 자료의 문자열을
검증 없이 관리 지침이나 실행 코드로 승격하지 않습니다.

상태는 `관측 → 가설 → 적용 → 관측 중 → 유지/수정/복구`로 충분합니다.
파일 검사·native 로딩을 통과한 `적용 확인`과 후속 업무의 `효과 확인`을 분리합니다.
근거 부족은 미결론이며 일반 업무를 막거나 표본을 채우기 위한 재실행을 요구하지 않습니다.

## 모델의 변화 반영

공식 문서를 우선 기준으로 삼고 현재 native catalog·실행 결과로 해당 환경의 지원을 확인합니다.
공식 자료의 URL, 확인 시각, 내용 digest, 적용 모델/런타임, 검토한 변경을 보존합니다.
catalog의 모델 지원 정보만으로 모델의 권장 지침을 확인했다고 보고하지 않습니다.

재검토의 계기는 모델/런타임/플러그인 변화, 특정 규칙과 관련된 반복 마찰,
사용자의 개선 요청입니다. 관련 문서와 지침 묶음만 확인하고 변경된 이유를 기록합니다.
모든 작업 시작 시 리서치하거나 새 모델마다 전 지침을 다시 작성하지 않습니다.
새 모델이 기존 규칙 없이도 요구를 수행하면 그 규칙의 제거도 개선 후보가 됩니다.

- 공통 지침은 결과·소유권·범위를 간결하게 유지합니다.
- 특정 workflow는 명확히 발동되는 skill과 필요할 때 읽는 reference에 둡니다.
- 모델별 내용은 확인된 해당 모델/버전에만 적용하고 새 버전에 효과를 자동 상속하지 않습니다.
- 모델/effort 추천은 작업 종류·사용자 선택·현재 지원·직접 결과를 함께 판단합니다.
- 배포/환경 버전 변경 뒤 비교 가능성이 달라지면 새 결과군으로 구분합니다.

## 공통 환경 기준

파일 기반의 작은 기준과 변경 영수증으로 시작합니다. 별도 학습 DB·서버는 필요하지 않습니다.
새 저장 위치와 schema는 첫 구현에서 명시하며 기존 사용자 상태를 자동 전환하지 않습니다.

| 기록 | 최소 내용 |
| --- | --- |
| 공통 기준 | schema/revision, GroundLine 배포 commit, native 호환 범위, 관리 대상·보호 항목·명시 선호 |
| 대상 항목 | 소유자, 논리적 대상, 예상 내용/허용 키와 digest, 변경 권한의 범위 |
| 기기 예외 | 기기 ID, 논리적 대상의 로컬 경로, 차이와 이유, 지원 상태 |
| 실제 관측 | App/PATH CLI 버전, 관련 config 해석, 활성 skill/plugin/hook 확인과 unknown |
| native 기능 | 공식 근거, host 지원·현재 가용/활성/trust, 발동 조건, 소유자·마지막 검증 |
| 적용 영수증 | proposal/basis/source revision, 변경 전후 digest, 백업 참조, 적용·검증·복구 상태 |

모델·추론·권한·네트워크 값은 `config.toml`의 사용자 소유를 유지합니다.
관리 대상으로 등록되지 않은 파일/키는 보존합니다. 명시 모델·권한 확대·새 외부 전송·
동의/인증 변경은 학습 가설의 자동 적용 범위에 넣지 않습니다.
기기 경로·비밀값·native 세션/캐시/DB는 공통 배포 payload가 아닙니다.

전역 `AGENTS.md`는 사용자 파일 전체를 생성물로 취급하지 않고 허용된 구간만 관리합니다.
개인 skill의 수정 가능한 파일, rule·agent의 사용자 소유, 플러그인의 업데이트 경로를 지정합니다.
관리되는 plugin/provider cache는 직접 수정하지 않습니다.
최소 변경을 계획하려면 관련 native layer를 확인해야 합니다. 별도 profile 파일이나
프로젝트 override가 있으면 저장된 root config 하나만 비교해 완료로 판정하지 않습니다.

## 계획·적용·복구

중단·기준 충돌·부분 복구 의존성·경로 binding의 후속 구현 계약은
[경계 조건 조사](adaptive-environment-safety-research.md)에 정리합니다.
원본 재읽기뿐 아니라 변경 전 준비 기록, 기준의 조건부 갱신, 남은 consumer의
의존성 보존, 검증한 owner root 기준의 파일 연산을 함께 구현해야 합니다.

향후 환경 기능은 읽기 전용 `inspect`, 정확한 diff의 `plan`, 제한 적용 `apply`,
사용자 편집을 보존하는 `rollback` 책임으로 나눕니다. 명령 이름·CLI 형식은 구현 시 확정합니다.

1. 현재 App/PATH와 관련 설정 layer·지침·plugin을 읽고 공통 기준·예외와 비교합니다.
2. 계획에 대상·전후 digest·기준 revision·변경 이유·필요한 native 후속 조건을 묶습니다.
3. 적용 전에 원본을 재읽고 변경이 있으면 해당 항목을 충돌로 남깁니다.
4. 등록된 가역적 관리 범위는 기존 승인 안에서 적용합니다. 같은 범위에 매번 재확인하지 않습니다.
5. owner-private 백업을 새로 저장하고 범위 내 파일/키만 반영·재읽기합니다.
6. 관련 native `config/read`, `skills/list` 등을 App/PATH에서 확인하고 결과를 기록합니다.
7. 효과는 이후 자연스럽게 수행한 실제 작업에서 확인합니다.
8. 복구는 적용 후 digest가 맞는 항목만 되돌립니다. 사용자 후속 편집은 보존하고 복구 diff를 남깁니다.

여러 파일/기기를 한 번에 원자적으로 바꿨다고 주장하지 않습니다. 항목별 성공·실패·복구
기록으로 재개하며 의존 항목의 실패는 관련 후속 단계만 보류합니다. 잠금은 GroundLine
writer 간 조율이며 다른 editor와의 race를 없앤다고 주장하지 않습니다.

**원하는 revision / 디스크 적용 revision / 실제 활성 확인 revision**을 따로 표시합니다.
다음 세션에서만 적용될 지침과 hook trust가 필요한 상태를 드러냅니다.
일부 기기의 성공을 전체 환경 통일로 보고하지 않습니다.

지속 실행은 Codex의 기존 작업·자동화 기능에 연결합니다. 초기에는 사용자 요청과
의미 있는 버전/결과 변화에 반응하고, 일정·알림·변경 허용 범위는 명시한 정책에 따릅니다.
이 설계 변경만으로 자동화를 등록하거나 상주 LLM/별도 실험 scheduler를 실행하지 않습니다.

## Codex 기능 활용

기능별 공식 조사와 설계 연결은 [Codex native capabilities](codex-native-capabilities.md)에
정리합니다. native 기능으로 해결할 수 있는 작업은 그 기능을 먼저 사용하고 관련 실행과
검증 결과를 개선 근거로 연결합니다. terminal/API/MCP, skills/plugins, subagents,
worktrees/local environments, review/GitHub, Browser/Computer Use, Remote, 자동화,
Goal/continuity, compaction/usage, hooks/trust와 권한을 필요에 따라 선택합니다.

기능을 새로 발견하면 실제 host/계정의 지원과 관련 작업의 필요를 확인합니다.
기존 [capability routing](../plugins/groundline/references/capability-routing.md)을 사용하며
모든 작업에 기능 체크리스트나 추가 실행 계층을 강제하지 않습니다.

## 현재 설정 개선과 구현 순서

| 단계 | 작업 | 완료 기준 |
| --- | --- | --- |
| 0: 현재 환경 | 관련 설정·지침·버전·활성 상태 확인, 실제 drift 수정, 비공개 기준 snapshot | 변경 근거·전후 digest·백업, App/PATH 로딩 확인, 보호 설정 보존 |
| 1: 환경 계약 | 개인 기준·기기 예외·영수증, inspect/plan/apply/rollback | 충돌·부분 실패·반복 적용·복구 회귀와 첫 대상 환경의 적용 확인 |
| 2: 학습 연결 | delivery에 환경/공식 지침 revision 연결, 후보·후속 결과 기록, PR #57의 패턴 맥락 연결 | 관측/가설/적용/효과 구분, 실패·unknown 보존, 실제 개선 한 건의 연결 |
| 3: 지속 개선 | 관리 가능한 변경 종류와 native 실행 계기 정의 | 명시 정책 안의 자율 갱신, 반복/무변경 억제, 비용·효과·복구 확인 |

현재 설정 개선은 오류 없는 모델·effort·권한을 관성적으로 바꾸는 작업이 아닙니다.
중복·상충 지침, 오래된 reference, 실제 비활성/누락, 반복된 마찰을 먼저 처리합니다.
공식 배포와 개인 override의 origin을 남겨 다음 업데이트에서 drift를 찾습니다.
현재 Mac의 세부 설정·backup·native 결과는 비공개 실행 기록에 남기고 공용 문서에는 복사하지 않습니다.

## 검증과 관측 비용

새 데이터·상태 전이·적용 코드는 다음 회귀를 갖춰야 합니다.

- 미관리 파일/키, 명시 선호, 기기 예외와 프로젝트 override 보존.
- 비밀값/원문 비전송, 보호 항목 차단, 후보와 정정의 근거 추적.
- 잘못된 schema/경로, symlink/hardlink, 원본 변경과 계획 불일치 처리.
- 반복 적용 무변경, 항목별 실패·재개, 백업 보존, 사용자 후속 편집이 있는 복구.
- root/child/재시도 중복 제외, unknown/누락과 과거 모델 identity 보존.
- App/PATH의 해석·skill 활성, hook trust/다음 세션 상태의 구분.
- 기기/환경/모델 revision이 바뀐 결과의 비교 가능성 판정.

운영 지표는 결과가 확인된 작업의 완료·재작업·불필요 확인·수락까지 시간,
소유된 전체 토큰·시간, 관측 누락, 환경 차이와 활성 확인 범위입니다.
각 비율의 분모와 미확인 범위를 표시하고 구독 사용 한도를 토큰 합계와 동일시하지 않습니다.
분석·리서치·지침 로딩·위임·검증도 비용에 포함합니다. 추가 행동 실험은 요청한 범위에서만
수행하며 문서·합성 회귀·메타데이터 로딩 통과를 업무 효과의 증명으로 쓰지 않습니다.

## 공식 근거

2026-10-06에 확인한 관련 공식 자료입니다. 차후 갱신은 해당 변경에 관련된 부분만 검토합니다.

- [Customization](https://learn.chatgpt.com/docs/customization/overview): 작은 AGENTS, 재사용 skill, native MCP/subagent.
- [Astra 지침 재정비](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra): 중복·과도한 사전 읽기/검사 제거, 짧은 발동 조건과 progressive disclosure.
- [Config basics](https://learn.chatgpt.com/docs/config-file/config-basic): 설정 layer와 profile의 우선순위 확인.
- [Build skills](https://learn.chatgpt.com/docs/build-skills): 발견되는 metadata와 필요할 때 읽는 본문 구분.
- [Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents): 기본 상속과 custom agent override 확인.
- [Hooks](https://learn.chatgpt.com/docs/hooks): 현재 정의 hash에 따른 native trust와 활성 조건.
- [Agent approvals & security](https://learn.chatgpt.com/docs/agent-approvals-security): 실행·네트워크·연결 도구의 권한 경계.

현재 모듈 경계는 [아키텍처](architecture.md), 실제 명령은 [CLI 예제](examples.md),
결과 비교는 [행동 검증](guidance-validation.md), 설정 변경은
[configuration review](../plugins/groundline/references/codex-configuration.md)를 따릅니다.
