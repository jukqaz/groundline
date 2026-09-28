# Native 주간 감사의 저장소 범위 검증

2026-09-28 구현. 원본 Codex SQLite·rollout 이력은 읽기 전용으로 다룬다.

기존 주간 감사는 DB에 등록된 경로만 순회했다. 따라서 DB에 없는 파일은 누락 통계에도 나타나지 않았고, 서로 다른 파일의 같은 thread ID가 중복 집계될 수 있었다. 읽기 실패가 있더라도 `eligible_root_count`를 선택된 수와 같게 두어 `selection_coverage=1.0`으로 표시했다.

주간 감사는 이제 `sessions`와 `archived_sessions`의 파일 목록을 DB 참조와 대조한다. `.jsonl`/`.jsonl.zst`는 같은 논리 rollout으로 합치며, 활성·보관별 파일 수, DB 행 수, 미등록 파일 수, 존재하지 않는 DB 경로 수를 별도로 반환한다. 기존 bounded reader로 session metadata까지만 읽어 같은 thread ID가 여러 논리 경로에 있는지 확인한다. 이런 충돌은 임의의 파일을 정답으로 선택하지 않고 주간 표본에서 제외한다. 원본 ID·경로·내용은 결과에 넣지 않는다.

`coverage.store_integrity`는 전체 로컬 이력의 대조 결과다. 주간·runtime별 누락 수로 해석해서는 안 된다. 불일치가 이번 기간에 미친 영향은 `unknown`이다. DB snapshot과 파일 목록은 원자적으로 고정되지 않으므로 동시 쓰기가 관측 차이를 만들 수 있으며, 결과에 `snapshot_atomic=false`를 명시한다. 감사는 복구·삭제·캐시 생성·migration을 하지 않는다.

분모를 확정할 수 없으면 주간 `scope.eligible_root_count`와 `selection_coverage`는 `null`, `collection_complete=false`, `status=PARTIAL`이다. `coverage.confirmed_eligible_root_count`는 실제 선택된 확인 표본 수이며 전체 모집단이 아니다. `recommendation_evidence_complete=false`와 `recommendation_limit=incomplete_observed_sample_only`를 통해 이 자료에 따른 추천의 한계를 표시한다. 표본 수가 적은 문제와 수집 완전성은 독립적이다.

추가 목록 탐색은 최대 400,000개 entry와 깊이 32로 제한한다. symlink는 따라가지 않는다. 탐색 오류·한도 도달은 하한 수치와 미완료 상태로 남기며, 이때 전체 DB 경로 부재 수는 `null`이다. metadata 읽기도 기존 호출의 8 GiB scan/512 MiB retained 예산을 공유한다. metadata ID를 읽지 못한 파일 수를 별도 보고한다. 이때 `identity_counts_complete=false`와 `counts_are_lower_bounds=true`로 중복 ID 0건이 확정값으로 읽히지 않게 한다. DB의 활성·보관 구분이 미상인 행도 저장소 완전성을 제한한다.

이 확장은 오프라인 주간 감사에 한정한다. activity audit, Insights wire payload와 API 계약은 바꾸지 않는다. CLI 리뷰 요약은 이 coverage를 전달해야 하며, 수집 불완전성을 완료 근거로 바꾸면 안 된다.

회귀 검증은 fixture의 활성 미등록 1개·보관 미등록 2개·오래된 DB 경로 3개·중복 ID 3그룹을 재현한다. 중복 파일을 제외한 선택 수, 알 수 없는 분모, 비공개 정보 미출력, DB/rollout 바이트 불변을 확인한다. 추가로 압축 sibling 단일 집계, 완전한 작은 표본, 목록 상한, 공유 읽기 예산, 중첩 symlink 경계를 검증한다.

주간 추천의 충분/제한표본 경로 모두 `coverage.recommendation_evidence_complete=true`를 요구한다. 누락·null·잘못된 타입은 unknown이며 false와 구분한다. 개인 trial은 감사 status가 PASS여도 이 coverage가 불완전/미상이면 OBSERVE로 남고 자동 적용하지 않는다. Routing도 집계 준비 완료를 거부한다. 별도 완전한 직접 결과의 모델 비교는 집계 맥락과 독립적으로 유지한다.
