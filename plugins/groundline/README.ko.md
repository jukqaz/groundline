# GroundLine Core

`$groundline:optimize-codex-workflow` 스킬 하나로 근거 있는 반복 마찰을 개선합니다.
Codex가 필요한 맥락을 읽고 기존 AGENTS·스킬을 수정·검증하며,
[개선 절차](references/conversation-improvement.md)는 작은 비공개 캡처·변경·후속 관측만
연결합니다. 일반 작업이나 변경 없는 검토에는 감사·영수증·모델 비교·학습 프로필이
필요하지 않습니다.

Core는 오프라인이며 훅·수집기·백그라운드 프로세스가 없습니다. Insights는 독립입니다.
기존 대화 기록은 그대로 조회하며 미지원 관리 형식은 변환·삭제 없이 명시적으로
거부합니다. `learning status --state PRIVATE_RECORDS`는 기존 기록을,
`improve status`는 작은 새 변경도 함께 요약합니다.

[선택적 감사](references/weekly-usage-audit.md) ·
[수동 설치 진단](references/installation-alignment.md) · [English](README.md)
