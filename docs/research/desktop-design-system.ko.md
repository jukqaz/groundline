# Desktop 디자인 시스템

Desktop은 Radix Themes 3.3.0과 Lucide 아이콘을 사용한다. 개요·서버·설정에서 같은 색상, 컨트롤 크기, 간격을 적용한다.

- `DesignTheme`가 라이트/다크 색상과 시스템 테마 전환을 관리한다. 원격 폰트·이미지·스타일 CDN을 로드하지 않는다.
- `src/ui.tsx`의 Button, Input, Choice는 Radix의 Button, TextField, Select를 사용한다. 입력값과 저장·동의 정책은 기존 애플리케이션 상태가 관리한다.
- 간격은 Radix의 `--space-*` 토큰을 사용한다. 기본 단위는 4/8/12/16/24/32/40/48/64px이다. 내부 여백과 섹션 간격을 구분한다.
- 큰 선택 카드는 추가하지 않는다. 일반 설정은 왼쪽 라벨과 오른쪽 소형 컨트롤로 배치한다. 상세 설명은 필요할 때 펼친다.
- 버튼과 입력창은 기본 size 2를 사용한다. 아이콘은 16~20px, 탐색은 18px를 기준으로 한다. 아이콘만 있는 버튼에는 접근성 이름을 제공한다.
- 회색은 탐색·대기 상태, 청록은 확인된 정상 상태에 사용한다. 성공 수치는 실제 서버 응답이 있을 때만 표시한다.
- `style.css`에는 제품 레이아웃과 필요한 변형만 둔다. 과거 규칙을 끝에 반복 덮어쓰지 않고 원래 정의를 수정한다.
- 기본 창은 1200×820, 최소 창은 760×600이다. 라이트·다크·최소 폭과 네이티브 dropdown의 키보드 동작을 함께 확인한다.

공식 문서:

- [Radix Themes 시작하기](https://www.radix-ui.com/themes/docs/overview/getting-started)
- [Spacing](https://www.radix-ui.com/themes/docs/theme/spacing)
- [Select](https://www.radix-ui.com/themes/docs/components/select)
