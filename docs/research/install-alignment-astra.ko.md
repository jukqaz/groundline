# Astra 지침에 따른 GroundLine 설치와 기존 설정 정비

GroundLine은 설치 뒤 기존 환경을 그대로 둔 채 사용을 시작하는 흐름에서,
설치와 적용을 요청하면 기존 설정·활성 지침을 점검하고 확인된 오류를 고치는
흐름으로 확장하는 것이 적절하다. 설정 파일을 새 기본값으로 통째로 교체하는
방식보다, 문제의 근거와 소유 파일을 확인하고 백업한 뒤 좁게 수정하는 방식이
사용자의 의도와 Codex의 설정 계층을 보존한다.

이 권고는 Astra의 명령 준수·지속 실행 지침, Codex의 설정 및 지침 로딩 규칙,
플러그인 설치와 hook 신뢰 계약, GroundLine의 현재 소스를 함께 검토한 설계
판단이다. 공식 문서는 특정한 GroundLine 복구 알고리즘을 권장하지 않는다.
컨텍스트 수정 규칙과 적용 전 해시 검사는 여기서 선택한 구현 정책이다.[^1][^2]

## Astra 지침이 요구하는 점검 범위

Astra 공식 가이드는 스킬과 `AGENTS.md`의 모호하거나 충돌하는 지시가 작업을
예상보다 일찍 중단시킬 수 있다고 설명한다. 따라서 설정 정비는 모델 이름,
추론 강도, 컨텍스트 숫자만 검사해서는 충분하지 않다. 실제로 읽힌 지침이
기존 사용자 요청과 어떻게 상호작용하는지 확인해야 한다.[^1]

| 점검 항목 | 확인할 문제 | 적용 원칙 |
| --- | --- | --- |
| 작업 지속 | 구현 요청에 계획만 제시하고 종료하는 지침 | 이미 허용된 작업은 수정과 검증까지 진행 |
| 확인 질문 | 매 단계마다 같은 허가를 다시 요구하는 지침 | 실질적인 선택·권한이 부족한 경우만 질문 |
| 지침 충돌 | 스킬이 사용자 의도를 가리거나 오래된 전역 지침이 개입 | 실제 활성 파일과 우선순위를 확인하고 소유 위치 수정 |
| 위임 | 추론 수준만으로 무조건 에이전트를 생성 | 사용자의 위임 정책 유지 |
| 검증 | 모든 사소한 변경에 전체 테스트를 반복 | 변경 위험에 맞는 검증, 새 근거가 있을 때 확대 |
| 응답 | 과도한 형식과 설명이 실제 결과를 가림 | 결과·핵심 증거·남은 한계를 간결하게 보고 |

Astra 문서의 하위 에이전트 예시는 작업 방식에 맞추는 선택 가능한 가이드다.
그 예시가 기존의 위임 금지 또는 명시 요청 시에만 위임한다는 사용자 정책을
무효화하지는 않는다. 동일하게 “자율적으로 진행”은 보안 경계나 외부 배포
권한을 변경하라는 뜻이 아니다. 필요한 실행 권한과 기존 작업 승인을 구분해야
불필요한 중단과 무단 변경을 함께 피할 수 있다.[^1]

## 설정과 지침은 서로 다른 계층을 따른다

현재 공식 설정 문서에서 CLI override, 신뢰된 프로젝트 설정, 선택된 profile,
사용자 설정, 시스템 설정, 내장 기본값은 서로 다른 우선순위를 가진다. 사용자
`config.toml` 하나가 정상이어도 프로젝트 또는 profile에서 같은 키를 덮어쓸 수
있다. 반대로 낮은 계층에서 오래된 값이 보여도 현재 작업에 적용되지 않을 수
있다. GroundLine이 이 해석기를 별도로 구현하면 native 동작과 어긋날 위험이
커지므로 최종 유효 설정 판정은 Codex에 맡긴다.[^2]

지침 파일은 별도의 탐색 규칙을 따른다. 전역 및 각 프로젝트 경로에서
`AGENTS.override.md`가 일반 `AGENTS.md`보다 우선할 수 있고, 프로젝트 안에서도
현재 위치에 가까운 지침이 뒤에 연결된다. 그러므로 일반 `AGENTS.md`만 수정하고
활성 override를 남겨 두면 수정이 실제 동작에 반영되지 않을 수 있다. 정비는
파일 존재보다 실제 활성 체인을 기준으로 수행해야 한다.[^3]

Hook은 또 다르다. 여러 계층의 일치하는 hook이 함께 실행되며, 같은 계층의
`hooks.json`과 inline `[hooks]`도 합쳐질 수 있다. 이를 단순한 “높은 설정이
낮은 설정을 덮어쓴다”는 규칙으로 처리하면 중복 실행을 놓친다. 중복 hook을
정리할 때는 출처·실행 내용·신뢰 상태를 함께 확인해야 한다.[^4]

## 대안 비교

| 방식 | 장점 | 중요한 한계 | 판단 |
| --- | --- | --- | --- |
| 전체 설정을 새 템플릿으로 교체 | 구현과 설명이 단순 | 개인 모델·권한·MCP·profile 설정을 잃을 수 있음 | 채택하지 않음 |
| 시작 hook에서 매번 자동 수정 | 재발을 빠르게 감지 | 별도 신뢰, 반복 실행, 숨은 쓰기, 사용자 의도 판정 문제 | Core에 추가하지 않음 |
| 검사 결과만 출력 | 쓰기 위험이 작음 | 수정 요청이 추천 단계에서 끝남 | 진단 명령으로 유지 |
| 설치·적용 스킬과 좁은 수정 명령 결합 | 맥락을 판단하고 확실한 수정은 검증 가능 | 모든 의미적 오류를 결정적으로 판정할 수 없음 | 채택 |

Native 플러그인 설치와 hook 실행은 같은 행위가 아니다. 공식 문서는 비관리
hook이 검토와 신뢰를 거쳐야 하며, 설치만으로 hook 신뢰를 얻지 않는다고
설명한다. 따라서 사용자가 별도 동작을 기대하지 않는 `plugin add`에 숨은
설정 수정 효과를 연결하는 것보다, 설치·적용 요청에서 명시적으로 정비 흐름을
이어 가는 방식이 예측 가능하다.[^5]

배포 경로는 그대로 Codex native marketplace를 사용한다. GroundLine은 자체
업데이터를 만들거나 설치 캐시를 직접 고치지 않는다. 설치 UI의 시작 요청과
`align-agent-home` 스킬을 정비 진입점으로 사용하고, 이미 작업 중인 설치·적용
요청이라면 같은 작업에서 이어 간다. 설치만 수행한 사실을 정비 완료로 보고하지
않는다.

다른 PC에서 같은 결과를 얻도록 배포본의 `install.sh`와 `install.ps1`을 추가했다.
Codex의 native 설치 명령 뒤에 설치 artifact 일치 검사, `setup --apply`, strict
doctor를 연결한다. 별도 대화 요청이 없어도 이 명시적인 설치 명령 안에서 설정
보정까지 끝난다. native GUI의 설치 버튼 자체에 지원되지 않는 post-install
callback이 있다고 가정하지 않는다. Core의 hook 0개 계약도 유지한다.

## 수정 대상 판정

오래된 설정이라는 사실만으로 잘못됐다고 판정하면 안 된다. 수동 컨텍스트 크기,
특정 추론 강도, provider override는 의도적인 선택일 수 있다. “스키마상 잘못됨”,
“현재 native 모델 증거와 충돌함”, “명시한 사용자 의도와 충돌함”, “현재 작업에서
불필요하다고 확인됨”을 구분해 기록하는 편이 좋다.

| 분류 | 예시 | 처리 |
| --- | --- | --- |
| 명확한 수치 오류 | 0 이하 컨텍스트 제한 | 제한값 제거 후보를 생성하고 native 기본값 사용 |
| 상호 충돌 | compaction 제한이 명시된 window보다 큼 | 두 override를 함께 제거하는 후보 제시 |
| 의도 확인이 필요한 값 | 양수 수동 window, 서비스 등급, 추론 강도 | 자동 교체하지 않고 현재 선택의 근거 확인 |
| native 증거 부족 | 다른 provider, 선택 profile, catalog override | 해당 계층을 해석한 뒤 재판정 |
| 형식 오류 | 중복 TOML 키, 깨진 문법, 필드 타입 오류 | 원문 보존; 확인한 의도로 좁게 수동 패치 |
| 지침 오류 | 적용되지 않는 파일 수정, 반복 승인 강제, 충돌 규칙 | 활성 소유 파일을 백업하고 문장 단위 수정 |
| 실행 환경 문제 | 터미널·네트워크·task-store 경고 | 설정 초기화와 분리해서 원인별 진단 |

공식 설정 참조는 compaction 제한을 생략하면 모델 기본값을 사용한다고 명시한다.
이것이 native 기본값 복원을 선택할 수 있는 근거다. 다만 양수 override 자체를
오류라고 규정하지는 않는다. 따라서 정상적인 양수 override 제거는
`--restore-native-context`라는 별도 선택으로 표현했다.[^6]

모델과 추론 강도의 유효성은 정적인 제품 목록으로 판정하지 않는다. 이번 확인에서도
일반 설정 참조의 추론 강도 설명과 실제 native catalog의 지원 범위가 완전히
일치하지 않았다. 일반 참조 표만으로 Astra의 유효한 선택을 삭제하면 오탐이다.
호스트·계정별 가용성은 현재 실행 환경의 카탈로그로 확인하고, 카탈로그의 최신성
또한 별도의 증거로 남긴다. 공급한 JSON을 읽었다는 사실만으로 계정 접근이나
실시간 최신성을 검증했다고 주장하지 않는다.

## 구현 계약

사용자가 공통 기본값을 **gpt-6-astra / xhigh / Fast 끔**으로 명시했으므로,
`setup`은 `config/setup-defaults.toml` 한 곳에서 이 설치 정책을 읽어 바이너리에
포함한다. 이 값은 사용자 선택이며 OpenAI가 권장한 보편 설정이라는 주장이 아니다.
Fast 끔은 이 PC의 native 검증을 통과한 `service_tier="default"`로 표현한다.
Fast 기능 자체를 비활성화하는 feature 변경은 필요하지 않다.[^7]

`setup`은 이 세 root 설정, 두 컨텍스트 override 제거, 확인된 퇴역 Core hook
승인 기록 4종만 보정한다. `toml_edit 0.24.0`으로 주석과 서식을 보존하고, 다시
파싱한 결과가 허용한 의미 변경만 포함하는지 검사한다. TOML 편집기의 dotted-key
순서 보존에는 한계가 있으므로 모든 원문 바이트가 같다고 주장하지 않는다.[^8]
백업은 원문 그대로 보존한다. 신규 config는 배타적으로 생성하고, 기존 config는
동일 디렉터리의 새 owner-private 백업과 쓰기 직전 재확인을 거친다. `CODEX_HOME`
또는 OS 홈을 사용하므로 특정 PC의 경로나 계정 정보는 배포본에 들어가지 않는다.
지원되지 않는 카탈로그 모델·추론 수준이나 해석되지 않은 profile/provider/catalog
override에서는 임의 대체 없이 실패한다. 재실행 시 이미 맞는 설정은 쓰지 않는다.

`config-audit`는 계속 읽기 전용이다. 새로운 `config-repair`는 기본적으로
수정안을 미리 보여주며 파일을 만들지 않는다. 제한된 입력 크기와 native
catalog 검증을 기존 코드와 공유한다. 이 좁은 명령은 설치 기본값 전체를 적용하지
않으며, 아래의 컨텍스트 수정 계획과 해시 계약을 따른다.

수정 후보는 root의 `model_context_window`와
`model_auto_compact_token_limit`에만 적용된다. TOML parser의 값 위치 정보를
사용해 해당 정수 대입문 줄을 제거하고, 다시 파싱한 결과가 원래 구조에서
허용된 키만 제거한 결과와 같은지 검사한다. 이를 통해 다른 설정, 중첩 테이블,
여러 줄 문자열에 들어 있는 비슷한 텍스트를 건드리지 않는다. 제거된 대입문에
붙은 inline 주석은 함께 제거되고 그 밖의 원문은 보존된다.

실제 적용에는 다음 조건이 필요하다.

1. 현재 대상 경로, 설정 원문, 카탈로그 원문, 규칙 버전, 옵션으로 계산한
   계획 해시가 미리보기 해시와 같아야 한다.
2. 원본은 현재 사용자 소유의 제한된 일반 파일이어야 한다. 심볼릭 링크와
   Unix hard link는 거부한다.
3. 지정한 백업 파일은 존재하지 않아야 한다. 기존 백업을 덮어쓰지 않는다.
4. GroundLine 수정끼리 겹치지 않도록 advisory lock을 잡고, 백업과 최종 교체
   직전에 원문이 여전히 같은지 다시 읽는다.
5. 원본 백업을 owner-only 권한으로 저장하고 동기화한 뒤 원자적으로 교체한다.
6. 교체된 파일을 다시 읽어 후보 원문과 일치하는지 확인한다.

계획 해시는 승인을 대신하지 않는다. 설치·적용 및 오류 수정 요청에서 확보한
권한 안에서 실행하며, 해시는 검토한 대상과 실제 쓰기 대상이 같은지를 확인한다.
매 파일 또는 매 단계마다 같은 사용자 승인을 반복해서 요청할 이유는 없다.
권한·외부 서비스·관리 정책을 새로 바꾸는 경우에는 별도 판단이 필요하다.

## 실패와 복구

구문 오류나 unresolved profile/provider가 있으면 자동 수정 경로에서 원문을
보존한다. 잘못된 model/effort를 임의의 기본값으로 바꾸지 않는다. 이는 모든
수정을 포기한다는 뜻이 아니다. 설치·적용 스킬은 실제 의도와 올바른 owning
layer를 확인해 해당 항목을 native 편집 도구로 수정하고 독립적인 작업은 계속한다.

백업 실패는 설정 쓰기 전에 멈춘다. 교체 후 디렉터리 동기화에서 실패하면
교체가 이미 일어났을 수 있으므로 결과를 불명으로 표시한다. 복구할 때에는 현재
파일이 수정 직후의 내용과 같은지 비교해야 하며, 사용자가 이후 고친 내용을
오래된 백업으로 덮어쓰지 않는다.

Advisory lock은 GroundLine 프로세스끼리의 직렬화다. Codex나 외부 편집기는
같은 잠금을 사용하지 않으므로 마지막 재확인과 교체 사이의 경쟁을 완전히
제거하지 못한다. 적용 중 다른 설정 작성자를 멈추는 운영 조건이 필요하며,
출력에 `external_writers_locked: false`를 명시한다. 완전한 다중 작성자
트랜잭션이나 다중 파일 원자성을 보장한다고 주장하지 않는다.

## 설치 후 검증과 완료 기준

| 검증 범위 | 완료 증거 | 별도로 남는 것 |
| --- | --- | --- |
| 파일 수정 | 백업 일치, 변경 키 제한, 원문 보존, 재실행 시 무변경 | 실제 native 유효 설정 |
| 소스·패키지 구조 | CLI 테스트, lint, 스킬 metadata·link 검증 | 배포된 stable와 설치 캐시 |
| native 설정 | 실제 App CLI의 strict doctor 설정 항목 | 네트워크·터미널 등 독립 경고 |
| 지침 동작 | 허용된 환경에서 해당 작업이 기대대로 지속·검증됨 | 단순 문구 검사로는 대체 불가 |
| 배포와 설치 | release artifact, 설치 checksum, 새 실행 환경 | 소스 변경만으로는 충족 불가 |

이번 구현의 테스트는 미리보기 무변경, 적용·백업, 계획 변경 거부, 선택 보존,
명시적인 native 기본값 복원, 재적용 무변경, 잘못된 입력 보존, link 거부,
동시 수정 거부를 다룬다. 개인정보 sentinel을 사용해 stdout/stderr에 원문과
개인 경로가 나타나지 않는지도 확인한다. 이 테스트들은 합성 파일을 사용하므로
실제 사용자 설정의 모든 오류를 제거했다는 증거가 아니다.

지침의 행동 효과는 별도의 검증이다. 설치 문구를 바꾸고 링크 검사가 통과한
것만으로 Astra가 모든 상황에서 원하는 대로 행동한다고 말할 수 없다. 해당
모델과 새로 로드된 스킬이 있는 환경에서 승인 유지, 질문 필요성, 위임 정책,
작업 범위에 맞는 검증을 관찰해야 한다. 실측이 없는 경우에는 그 범위를
미검증으로 보고한다.

## 구현 위치

- [수정 명령](../../crates/groundline-cli/src/config_repair.rs): 결정적인 수치 오류와 기본값 복원.
- [공통 설정 적용](../../crates/groundline-cli/src/setup.rs): PC별 경로 탐지, 명시한 기본값, 기존 설정 보존.
- [macOS/Linux 설치](../../install.sh), [Windows 설치](../../install.ps1): native 설치부터 setup과 doctor까지.
- [설치 회귀 테스트](../../crates/groundline-cli/tests/install_cli.rs): 실물 CLI와 모의 provider를 조합한 설치 흐름, 실패 차단, 재실행 검증.
- [CLI 회귀 테스트](../../crates/groundline-cli/tests/config_repair_cli.rs): 실제 명령 경계의 보존·백업·실패 검증.
- [설치·적용 계약](../../plugins/groundline/references/installation-alignment.md): 설정과 지침의 의미적 판단 및 복구.
- [설정 점검 계약](../../plugins/groundline/references/codex-configuration.md): preview/apply 사용법과 증거 한계.
- [적용 스킬](../../plugins/groundline/skills/align-agent-home/SKILL.md): 설치 완료 이후 정비까지 이어지는 진입점.

## 출처

공식 문서는 2026-09-09에 확인했다. 문서의 일반 예시와 실행 버전의 기능이
다르면 현재 호스트의 help·catalog·native 검증 결과로 적용 가능성을 판단한다.
아래 문서는 전체 설정 자동 교체나 특정 GroundLine 코드의 안전성을 보증하는
근거로 사용하지 않았다.

[^1]: OpenAI, [Model guidance — Prompting best practices](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices). Astra의 지침 민감도, 작업 지속, 위임 조정, 검증 범위.
[^2]: OpenAI, [Basic configuration — Configuration precedence](https://developers.openai.com/codex/config-basic#configuration-precedence). 설정 계층 및 native 해석 경계.
[^3]: OpenAI, [Custom instructions with AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md). 지침 탐색, override, 새 실행에서의 반영.
[^4]: OpenAI, [Hooks](https://learn.chatgpt.com/docs/hooks). 여러 출처의 hook 병합, 신뢰, 이벤트 동작.
[^5]: OpenAI, [Package your plugin](https://developers.openai.com/plugins/build/plugins#bundled-mcp-servers-and-lifecycle-hooks). 플러그인 패키징, native 설치와 hook 신뢰의 구분.
[^6]: OpenAI, [Configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference). 컨텍스트 및 compaction 설정의 의미와 기본값.
[^7]: OpenAI, [Fast mode](https://learn.chatgpt.com/docs/agent-configuration/speed#fast-mode). Fast 선택과 기능 활성화 구분.
[^8]: toml_edit, [0.24.0 API documentation](https://docs.rs/toml_edit/0.24.0%2Bspec-1.1.0/toml_edit/). 주석·서식 보존과 dotted key 순서 제한.
