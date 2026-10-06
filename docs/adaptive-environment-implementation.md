# 환경 기준과 실제 결과를 연결하는 실행 경로

이 문서는 새 소스의 로컬 환경·학습 CLI를 설명합니다. 설치된 stable 플러그인,
다른 기기의 실제 적용, 작업 품질 향상까지 확인됐다는 뜻은 아닙니다.
기존 [설계](adaptive-environment-design.md)와 [개선 기준](adaptive-environment-improvements.md),
[파일 안전성](adaptive-environment-safety-research.md)의 책임 경계를 유지합니다.

## 저장 단위와 기존 데이터

기존 ClickHouse의 기간 집계와 private delivery receipt는 변경하지 않습니다.
기간 집계는 사용량·모델·위임·관측 누락을 설명하는 맥락입니다. 집계 행을
완료 작업 수로 쓰거나 당시 지침·환경·수락·재작업을 역추정하지 않습니다.
정확한 모델 패턴도 과거의 계열 표기를 정확한 모델 ID로 소급 변환하지 않습니다.

새 학습 기록은 기존 receipt의 원본 바이트 digest, unit, cohort, phase, 완료 시각에
작업 당시의 환경·대상 지침·공식 자료 revision과 정정 근거를 연결하는 별도 sidecar입니다.
현재 환경을 조회해서 과거 작업에 자동 부착하지 않습니다. 추가 요구·방향 변경·
실제 assistant 오류·수락·unknown을 구분합니다.

환경 기준에는 논리적 대상과 원하는 digest를 담습니다. 기기별 bindings에는
로컬 owner root와 상대 경로를 담습니다. 공통 기준, 기기 경로, 후보 원문,
적용 기록을 public 저장소나 기존 Insights 전송에 넣지 않습니다.
새 state 디렉터리는 owner-private이고 입력과 기록은 크기·소유자·링크 상태를 검사합니다.

## 환경을 등록하고 차이를 확인한다

첫 구현의 일반 writer 대상은 등록한 개인 skill 파일과 AGENTS.md의 명시 구간입니다.
모델·추론 강도·권한·네트워크·인증·native memory·plugin/provider cache는
이 writer로 변경하지 않습니다. config 수정이 필요하면 기존 setup/config-repair의
해당 명시 범위를 사용합니다.

- baseline: kind=groundline-environment-baseline, schema=1, revision과 parent_revision,
  source_revision, authority_revision, exception_revision, authority_ref,
  managed_targets를 담습니다.
- target: target_id, kind(skill_file 또는 agents_block), desired_sha256,
  dependencies, managed_block을 선언합니다. AGENTS 구간은 고유한 시작·종료 marker를 사용합니다.
- bindings: kind=groundline-environment-device-bindings, schema=1, revision,
  device_id, roots(root_id/path/aliases), targets(target_id/root_id/relative_path)를 담습니다.
- proposal: kind=groundline-environment-proposal, schema=1, proposal_id,
  basis_revision, source_revision, authority_ref, changes(target_id/content)를 담습니다.

skill_file digest는 전체 파일 내용, agents_block digest는 marker를 제외한 관리 본문입니다.
모든 revision과 authority 참조에 일관된 SHA-256를 사용하면 학습 기록과 직접 연결할 수 있습니다.
제공된 authority hash는 기존 승인 범위를 가리키며 새 권한이나 승인 진위를 만들어 주지 않습니다.
첫 등록의 parent_revision은 null입니다. 이후 기준 갱신은 현재 revision과 같은 부모를
제시해야 하며, 임의 병합이나 오래된 기준 덮어쓰기는 하지 않습니다.

    groundline environment register --state-dir PRIVATE_STATE --baseline baseline.json --bindings bindings.json --json
    groundline environment inspect --state-dir PRIVATE_STATE --json
    groundline environment plan --state-dir PRIVATE_STATE --proposal proposal.json --json
    groundline environment apply --state-dir PRIVATE_STATE --proposal-id PROPOSAL_ID --json
    groundline environment recover --state-dir PRIVATE_STATE --operation-id OPERATION_ID --json
    groundline environment rollback --state-dir PRIVATE_STATE --operation-id OPERATION_ID --json

PRIVATE_STATE와 입력 파일은 사용자가 지정한 비공개 위치입니다. 입력 예시는 계약을
설명하는 형식이며 사용자가 관리 대상으로 등록하지 않은 경로를 자동 등록하지 않습니다.
기기별 parent 경로는 미리 존재해야 합니다.

plan은 기준·권한·기기 예외와 대상의 전후 내용·경로 binding을 고정합니다.
정확한 내용과 diff는 private 기록에 남고 stdout에는 내용·개인 경로를 내보내지 않습니다.
apply는 입력 proposal을 다시 해석해서 변경 범위를 확대하지 않고 저장한 plan을 사용합니다.
같은 실제 parent와 leaf의 중복 등록은 거부하여 하나의 대상을 두 번 쓰지 않습니다.

## 중단과 사용자 후속 편집을 보존한다

실제 변경 전 새 백업과 PREPARED 기록을 저장·동기화합니다. 신규 파일은 이미 생긴
다른 파일을 덮어쓰지 않습니다. 변경 전 내용·소유자·hardlink 수·root/parent binding을
다시 검사하고 파일 교체 후 부모 동기화와 재읽기를 수행합니다.
환경과 학습의 writer 잠금은 guard 종료 시 명시적으로 해제합니다.
복제·상속 descriptor가 남아도 종료한 writer의 잠금 수명을 연장하지 않도록
[Rust File 계약](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)에 맞춰 검증했습니다.

rename 뒤 sync 또는 완료 기록 저장이 실패하면 미적용으로 단정하지 않습니다.
recover는 현재 파일이 before/after/어느 쪽도 아님을 확인하며 준비 기록만으로
native 활성이나 과거 작업의 성공을 인증하지 않습니다.

rollback은 적용 후 내용이 유지된 항목만 복구합니다. 사용자 편집이 남은 consumer와
그 consumer의 전이 의존 자료를 보존합니다. 부분 성공·충돌·복구 보류는 각각 기록합니다.
여러 파일과 기기의 원자적 적용, 전원 차단 영속성, 임의 외부 editor와의 완전한
상호 배제까지 보장하지 않습니다.

## App과 PATH 관측을 분리한다

    groundline environment-observe --app-cli APP_CLI --path-cli PATH_CLI --codex-home CODEX_HOME_PATH --target SKILL_MD

각 CLI의 지원·버전과 config 읽기, skill 발견·enabled, hook의 노출된 trust metadata를
읽기 전용으로 따로 관측합니다. 실제 경로를 stdout에 출력하지 않고 지문과 제한된 상태만 남깁니다.
관련 native 방법이 제공되지 않으면 unsupported 또는 unknown입니다.
기존 채팅의 지침 재로딩·실제 호출·업무 효과는 목록 노출이나 파일 digest로 확정하지 않습니다.
관측 실패는 다른 CLI의 성공을 덮어쓰지 않습니다. 새 모델 턴을 실행하지 않습니다.

desired revision, disk digest, native discovery/metadata, execution effect를 구분합니다.
일부 기기의 성공을 전체 환경 통일로 보고하지 않습니다. 기록의 true 필드 하나를
외부에서 입력하여 activation_verified를 승격하는 경로도 없습니다.

## 실제 결과에서 작은 후보를 남긴다

기존 efficiency record-delivery로 결과와 소유된 root/child/실패/재시도 자원을 기록합니다.
다음 learning 명령은 sidecar와 후보를 private 상태에 저장하고 평가합니다.

    groundline learning link-outcome --input link.json --receipt delivery.json --state PRIVATE_LEARNING
    groundline learning propose --input candidate.json --state PRIVATE_LEARNING
    groundline learning evaluate --input evaluation.json --deliveries PRIVATE_DELIVERIES --state PRIVATE_LEARNING --operation application.json
    groundline learning status --state PRIVATE_LEARNING

link는 receipt의 실제 바이트 hash와 필드를 대조합니다. 미관측 당시 revision은 null로
남기며, 작업 이후 시각을 당시 관측 시각으로 넣으면 거부합니다. provenance hash는
내용 식별자이고 native 진위 인증은 아닙니다.

후보 본문은 기존 native Codex 작업에서 만듭니다. 발동 조건, 작은 대상 변경,
문제 가설, 예상 결과, 반증 조건, 기존 authority와 rollback 근거를 명시합니다.
GroundLine이 매 턴 reflection 모델을 호출하거나 별도 실행기를 운영하지 않습니다.
같은 근거·범위·관련 revision의 후보는 재사용하며 성공한 no_change와 분석 실패를
구별합니다. 실패한 시도만으로 근거를 처리 완료로 버리지 않습니다.

평가는 후보와 실제 application 영수증, before/followup 직접 결과를 연결합니다.
baseline_revision은 기준 결과 당시 관측한 environment_revision입니다. 후보를 계획하는
basis_revision과 별도로 두어 이전 적용 plan에 연결된 자연 업무 결과를 그대로 재사용합니다.
proposal_revision은 적용한 private plan의 정확한 바이트 SHA-256입니다.
followup의 environment_revision은 이 plan SHA-256이며 skill.revision은 각각 대상의
before/after 내용 digest입니다. 기준 이름만 같거나 다른 스킬의 결과는 비교하지 않습니다.
이 digest는 application entry의 전체 파일 내용 기준입니다. AGENTS 관리 본문만의
desired_sha256과 구분하여 계획에 기록된 전후 digest를 사용합니다.
작업 cohort·phase·관측 모델/effort·runtime 및 기준/처리 지침 revision이 맞지 않거나
결과·비용이 미관측이면 INCONCLUSIVE입니다. 실패·재작업은 평균 비용 개선으로 숨기지 않습니다.
충분히 비교 가능한 결과라도 관측한 before/after 결과를 보여주는 것이며
인과효과나 최적 모델을 자동 인증하지 않습니다.

분석 비용도 소유 response를 기준으로 기록합니다. 이미 delivery에 포함된 응답을
다시 합산하지 않고 누락된 비용은 unknown으로 남깁니다. 병렬 child 시간을 더해
전체 완료 시간으로 쓰거나 토큰을 구독 잔여량으로 환산하지 않습니다.
완료한 업무를 표본 확보만을 위해 다시 실행하지 않습니다.

## 후속 범위

공식 자료 변경은 검토한 URL·시각·digest와 영향받는 대상/모델/runtime 근거를
후보에 연결합니다. native catalog 변화만으로 모델 권장 지침이 바뀌었다고 추정하지 않습니다.
새 모델의 효과는 기존 모델로부터 자동 상속하지 않습니다.

첫 구현은 명시된 private 입력·기준·후보를 처리하는 로컬 경로입니다. private Git 동기화,
다른 실제 기기의 적용, 요청하지 않은 일정·알림·상주 분석기는 자동으로 만들지 않습니다.
native memory의 생성 상태도 직접 편집하지 않습니다.
계약·소스 회귀, 실제 파일 적용·native 관측, 이후 자연 업무의 효과는 별도로 보고합니다.

## 첫 검증의 범위

2026-10-06에 환경·잠금·실패주입·복구 25개, 학습 계약 12개·CLI 7개,
native 관측 9개와 CLI 간 왕복 1개를 포함한 관련 crate 회귀를 통과했습니다.
정적 검사와 소스·플러그인 metadata/참조·CI 계약도 확인합니다.
저장소의 고정된 호환 이미지로 격리한 ClickHouse에서 실제 SQL 통합 6개도 통과했습니다.
정확한 모델·기간 overflow·관측 coverage와 기존 schema/report/Grafana 질의를 포함합니다.
이 테스트 DB는 검사 후 제거했으며 운영 ClickHouse의 schema·기존 데이터는 변경하지 않았습니다.
각 macOS/Linux native CI에는 adaptive_environment_cli를 delivery/routing 검사와
함께 실행하도록 연결하고, 검사 제거·필터·컴파일만 수행하는 우회를 거부합니다.

실제 현재 Mac에서 개인 skill 참조문 한 곳에 집계의 해석 한계를 적용했습니다.
준비 기록·백업·실제 application·반복 무변경·전후 결과 sidecar·평가를 저장했고,
App/PATH CLI 0.160.0의 개인 skill 발견·enabled metadata를 각각 확인했습니다.
reference 본문의 다음 작업 사용, 기존 채팅 재로딩과 기억 생성/사용 건강은
이 metadata 확인으로 입증하지 않습니다. config·AGENTS·agent·rules는 그대로 유지했습니다.

이 실행의 환경 검사 결과는 기록했지만 root 실행 모델·소유 자원과 분석 비용을
완전히 관측하지 못했고, 새 요구에 따른 변경입니다. 평가 결과는 INCONCLUSIVE이며
업무 품질·효율 향상을 주장하지 않습니다. 이후의 자연 업무 결과를 연결할 수 있습니다.
private 원문·로컬 경로·영수증은 이 문서나 Insights 업로드 범위에 포함하지 않습니다.
전원 차단 영속성·다른 실제 기기·지속적인 업무 효과·stable 패키지 승격은 별도 검증입니다.
