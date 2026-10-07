# 작업 결과 증거 연결과 로컬 완성 기준

2026-10-08의 조사와 구현 범위는 GroundLine을 Codex의 로컬 증거 연결 계층으로
고정한다. 실행·기억·모델·권한·위임은 native Codex가 담당한다. GroundLine은 명시한
작업 기준과 직접 결과, 소유 응답 비용, 당시 환경 revision을 연결한다. 별도 agent
harness, 상주 reflection 모델, 자동 권한 변경은 이 범위에 넣지 않는다.

## 이미 있는 기능의 재사용

| 기존 기능 | 이번 결정 |
| --- | --- |
| [Codex Memories](https://learn.chatgpt.com/docs/customization/memories) | eligible한 이전 채팅에서 native가 만드는 로컬 기억을 재사용한다. 기억 파일을 GroundLine이 다시 생성하거나 직접 수정하지 않는다. |
| [Codex Hooks](https://learn.chatgpt.com/docs/hooks) | native 경계와 명시 trust를 재사용한다. 훅은 설치된 실행 기기에서만 작동하며 Stop이나 ACK는 성공·수락 근거가 아니다. |
| [Codex OTEL 설정](https://learn.chatgpt.com/docs/config-file/config-reference) | native exporter·metrics·trace를 재사용한다. prompt logging은 opt-in이며 GroundLine의 작업 집계 때문에 자동 활성화하지 않는다. |
| [ccusage Codex](https://ccusage.com/guide/codex/)와 [parser](https://github.com/ccusage/ccusage/blob/a2a2ae45619bc6df4159d7a90f15c5fac842b9ea/rust/adapters/codex/src/parser.rs) | 로컬 사용량 집계의 대안이다. deduplication·parent replay 처리와 API 단가 추정은 작업 결과·수락·구독 실청구를 증명하지 않는다. |
| [Langfuse Codex](https://langfuse.com/integrations/developer-tools/codex)와 [experiment 비교](https://langfuse.com/docs/evaluation/experiments/compare-experiments) | 중앙 tracing·설정한 기준/dataset 평가의 대안이다. Codex 연결의 raw transcript 전달을 GroundLine의 raw-free 수집으로 가져오지 않는다. |
| [Hermes background review](https://github.com/NousResearch/hermes-agent/blob/main/agent/background_review.py), [provider](https://hermes-agent.nousresearch.com/docs/integrations/providers), [Letta reflection](https://github.com/letta-ai/letta-code/blob/main/src/agent/subagents/builtin/reflection.md), [skill learning](https://www.letta.com/blog/skill-learning/) | 경험 기반 reflection을 운영하는 별도 harness다. 해당 모델 호출·기억 계층을 Codex App에 중복 설치하지 않는다. |
| [Superpowers](https://github.com/obra/superpowers)와 [ECC Codex plugin](https://github.com/affaan-m/ECC/blob/main/.codex-plugin/README.md) | 계획·작업 분할·TDD·리뷰는 workflow 중복이다. 짧은 작업 handoff와 제한된 리뷰 회차만 참고하고 모든 작업에 별도 절차를 강제하지 않는다. |

ECC의 [continuous-learning-v2](https://github.com/affaan-m/ECC/blob/main/skills/continuous-learning-v2/SKILL.md)는
관측과 정정에서 confidence를 갱신해 instinct를 보관·승격한다. Claude Code observer와
현재 [Codex SessionStart 훅](https://github.com/affaan-m/ECC/blob/main/hooks/codex-hooks.json)의
지원 범위를 구분한다. 사용자 미정정이나 높은 confidence를 GroundLine의 수락·효과
증거로 사용하지 않는다. 위 프로젝트의 제작자 주장·사용자 일화·자체 benchmark는
개인 Codex의 이후 업무에서 얻은 비교 가능한 결과를 대체하지 않는다.

## 현재 연결 경로

작업 시작에 native Codex가 scope와 완료 기준을 선언한다. 독립 native artifact가
있으면 경계를 대조하고 없으면 unmatched로 남긴다. 당시 capture는 과거 작업에
현재 revision을 소급하지 않는다.
작업 후 native Codex가 직접 결과와 검증 근거를 assessment로 제출한다. 기록은
immutable이며 원본 prompt나 사용자 원문을 포함하지 않는다. 비용 reader는 명시한
root/child의 소유 turn만 사용한다. parent/session이 같다는 이유로 다른 응답을 합산하지 않는다.

assessment 제출과 task-outcome 생성은 서로 다른 단계다. 직접 receipt가 부족하면
연결은 pending 또는 unknown으로 남는다. 검증한 원본 receipt SHA와 unit/cohort/phase가
일치할 때만 outcome과 sidecar를 연결하고 기존 evaluator로 후속 결과를 비교한다.
미관측 비용·모델·runtime·verification은 0이나 성공으로 채우지 않는다. 후보 본문은
native Codex가 작성하며 적용은 기존 trial/adoption gate와 rollback 보호를 재사용한다.

종료 훅의 detached worker는 Core 프로세스의 종료 코드와 소비 결과를 구분한다.
결과 출력은 제한된 크기로 읽고 status·대기 이유·연결 건수만 비공개 상태에 보존한다.
worker의 연결·대기 건수는 마지막 Core 시도 기준이며, 전체 작업 집계는 immutable
outcome과 readiness를 사용한다.
일시적인 잠금이나 실제 완료 기록 대기는 같은 제한 시간 안에서만 재시도한다.
영구적인 입력·소유권 오류와 충분한 native 근거가 없는 작업은 대기로 유지하며,
worker 실행 자체를 업무 완료·비용 관측·수락 근거로 사용하지 않는다.

## readiness가 말하는 범위

readiness는 CLI가 이미 검증한 active record와 참조된 archive의 제한된 closure 및
읽어 확인한 receipt만 받는 순수 집계다. 파일·native 입력·네트워크를 다시 읽거나
모델을 호출하지 않는다. 입력 record/receipt 각각 1,000개를 넘으면 명시 오류로
거절하며, 기존 reader의 크기·소유권·원본 SHA·collection 검사를 우회하지 않는다.
archive 전체를 조사하지 않으므로 `historical_quality_coverage_complete=false`다.

- `standalone_capture_count`는 task-start가 참조하지 않은 snapshot 수다.
  `link_only_count`는 task-outcome이 참조하지 않은 sidecar 수다. 둘 다 완료 업무 수가 아니다.
- `pending_assessment_count`는 assessment와 outcome 모두 없는 시작 수다.
  `assessment_pending_connection_count`는 동일 task-outcome이 없는 assessment 수이고,
  `unlinked_start_count`는 outcome이 없는 시작 수다. 진행 중인 작업을 실패로 판정하지 않는다.
- `by_task_category`는 명시한 `TaskScope.task_category`별 숫자다.
  `installation_verification`, `fixture_verification`, `natural_work`를 선언하면 각각
  별도 행으로 남는다. 다른 category나 category 없는 capture/link를 자연 업무로 추정하지 않는다.
- 실제 outcome→link→원본 SHA의 receipt 연결이 있는 경우만 delivery의 비용·모델·검증
  집계를 재사용한다. receipt 미제공은 비용·selection·verification unknown이고, 부분 비용은
  known sum과 missing count를 함께 보존한다. analysis 비용도 기존 소유 응답 계약을 재사용한다.
- `native_activation=UNVERIFIED`, `causal_effect_verified=false`,
  `efficiency_improvement_verified=false`를 유지한다. 관측된 검증 상태와 실패·재작업 수는
  기록 내용의 집계이며 native 진위나 개선 효과의 인증이 아니다.

## 완료 판정

이번 구현의 완성 근거는 관련 계약·CLI 회귀, qualified package, 로컬 설치판의 실제
경계 조회와 작업 시작/assessment/receipt/outcome 연결 왕복이다. source, package,
installed binary, native 실행을 각각 확인하고 receipt 원본과 private 기록 보존을 대조한다.
설치·fixture category 표본은 제품 경로의 검증이며 일반 업무 품질 표본으로 보고하지 않는다.

실제 개선 효과는 이후 자연 업무의 기준과 후속 결과가 동일 cohort·phase·완료 기준 및
관측 모델/effort/runtime과 관련 revision을 만족할 때 기존 evaluator의 관측 비교로만
보고한다. 부족한 자료는 INCONCLUSIVE로 유지한다. 로컬 경로 완성을 기다리는 동안
업무를 재실행하거나 부족한 비용·품질 자료를 만들어 채우지 않는다.
