# GroundLine Desktop

Insights 연결과 전송 상태를 화면에서 관리하는 **선택형 GUI**입니다.
Codex App 자체와는 다른 앱이며, Codex App/CLI에서 GroundLine을 사용하는 데 필요하지 않습니다.
기본 설치는 [Codex 플러그인](../../README.ko.md#설치와-업그레이드)입니다.

## 선택 설치

현재 배포 대상은 macOS Apple Silicon 미리보기입니다.
[공식 릴리스](https://github.com/jukqaz/groundline/releases/latest)의
`groundline-desktop-aarch64-apple-darwin.zip`이 별도 GUI 패키지입니다.
Node.js나 Rust 개발 도구 없이 압축을 풀어 앱을 Applications에 옮기는 방식입니다.
같은 릴리스의 `SHA256SUMS`와 GitHub artifact attestation으로 배포 파일을 검증할 수 있습니다.

**Developer ID 서명과 Apple 공증은 아직 완료하지 않았습니다.** 임시(ad hoc) 서명과
파일 무결성 검증만 되어 있어 Gatekeeper가 실행을 거부할 수 있습니다.
일반 사용자의 설치 완료를 보장하는 정식 GUI 배포로 취급하지 않습니다.
공증된 GUI가 필요한 경우 플러그인만 사용하세요. Windows·Linux·Intel Mac용 GUI는 배포하지 않습니다.

자동 수집에는 별도로 설치하고 신뢰한 `groundline-insights` 플러그인이 필요합니다.
GUI 설치는 플러그인 설치·훅 승인·수집 동의를 대신하지 않습니다.
Core는 로컬 가이드가 필요할 때만 설치합니다. 일반 HTTPS 연결에는 Tailscale도 필요하지 않습니다.

## GUI 없이 사용하거나 제거하기

서버 연결 설정과 수집 동의를 마치면 GUI를 종료해도 됩니다. Codex 훅은 설치된
Insights 실행 파일을 직접 호출하며 GUI 실행 파일을 사용하지 않습니다.
GUI가 필요 없어지면 완전히 종료한 후 `GroundLine Desktop.app`만 휴지통으로 옮깁니다.
`CODEX_HOME`과 그 안의 `groundline/insights` 설정·동의·전송 대기는 유지합니다.
플러그인 업그레이드는 제거한 GUI를 다시 설치하지 않습니다.

처음부터 GUI를 쓰지 않는 연결·중지·상태 확인은
[Insights CLI 안내](../../plugins/groundline-insights/README.ko.md#owner-설정)를 따릅니다.
App과 CLI는 같은 Codex 홈의 서버 연결을 공유하지만 수집 동의와 상태는 대상 환경별로 구분합니다.

## 사용 흐름

주 메뉴는 **개요·서버·설정** 세 개입니다. 서버 연결과 Docker Compose 구성을 하나의
서버 메뉴로 통합했습니다. 기존 서버 연결을 먼저 보여 주고,
관리자용 **새 서버 구성**은 하단의 선택 도구로 제공합니다.
두 작업은 API·Grafana 주소 초안을 공유하므로 다시 입력하지 않아도 됩니다.
새 서버의 주소를 편집해도 저장된 기기 연결은 자동으로 바뀌지 않습니다.

- **개요 (기본 화면)**: 연결 여부·수집 동의·오류 상태에 따라 다음 작업을 안내하고,
  최근 서버 수신·전송 대기·전송 이력을 표시합니다. 기기 사용량은 요청할 때 로컬에서 읽습니다.
- **서버 → 기존 서버 연결**: 이미 연결된 환경은 관리 화면을 먼저 엽니다. 수집·전송 확인, 대기 건수,
  최근 확인·재시도·격리 상태, 원인별 안내와 같은 서버의 등록키 재확인을 제공합니다.
  Grafana 주소는 수집 재등록 없이 독립적으로 수정할 수 있으며 모든 환경에서 공통으로 사용합니다.
  수집 범위, 수집 중지·명시적 재동의 후 재개도 여기서 관리합니다.
  중지는 기존 로컬 데이터나 서버 데이터를 삭제하지 않습니다.
  진행 중인 요청이나 읽기가 있으면 정리가 끝난 뒤 중지 완료를 표시합니다.
  이미 서버로 보낸 요청은 회수할 수 없으며, 아직 전송하지 않은 데이터는 보존합니다.
- **설정**: 창 닫기(기본 트레이 / 완전 종료)와 테마(기본 시스템 / 라이트 / 다크)를 관리합니다.
  문제 알림 선택란은 트레이를 사용할 때만 표시합니다.
- **환경 선택**: 개요·서버의 연결 화면에서 Codex App / CLI를 선택합니다. 수집 동의와 제어는
  선택한 환경에만 적용되며 환경 변경 중에는 이전 상태를 표시하거나 작업하지 않습니다.
- **기존 서버 연결 순서**: API 주소, 선택형 Grafana 주소, 등록키 입력 → 서버/ClickHouse 준비 상태와
  등록키 확인 → 수집 범위와 초기 기록 동기화 동의 → 설정 저장 → 첫 전송 확인.
- **서버 → 새 서버 구성 (관리자용)**: 주소와 연결 방식 → 저장소·포트·필수 이미지 digest → 검토·생성으로
  진행합니다. 선택형 서비스 자격증명은 비우면 64자리로 생성합니다. 생성 후 해당 폴더를 열거나
  같은 메뉴의 연결 화면으로 이어갈 수 있습니다. 등록키는 직접 입력합니다.
- **일반 HTTPS**: 호스트 포트를 loopback에 바인딩하고, 운영자의 호스트 HTTPS 프록시를 연결합니다.
  Tailscale 설치가 필요하지 않습니다. API 자체가 TLS를 종료하지는 않습니다.
- **Tailscale**: 선택한 경우에만 서버 Tailnet IPv4가 필요합니다. Compose와 API에서 전용 네트워크 정책을 켭니다.
- **Grafana**: 연결 시 입력한 주소를 비공개 데스크톱 설정에 저장하고 기본 브라우저로 엽니다.

생성 파일은 다운로드 폴더의 새 GroundLine 폴더에 저장합니다.
`compose.yaml`, `secrets.json`은 비공개 파일입니다. 나머지는 주소만 있는
`connection.json`과 서버에서 수행할 절차를 설명하는 `README.ko.md`입니다.
생성은 서버 배포나 원격 설정 변경을 수행하지 않습니다.

ClickHouse DB `groundline`, ingest 사용자 `groundline_ingest`는 기존 저장소 계약의 고정값입니다.
Grafana는 별도의 읽기 전용 계정을 사용합니다. 컨테이너 이미지와 데이터 소스 플러그인은
`infrastructure/compatibility.json`의 고정된 조합을 그대로 사용합니다.

## 창 닫기와 전송 확인

앱은 하나만 실행됩니다. 다시 실행하면 기존 창을 복원하고, 메뉴 막대의
`GroundLine 열기`로 숨긴 창을 다시 엽니다. 닫기의 기본값은 트레이로 숨기기입니다.
설정에는 창 닫기와 테마를 표시하고, 트레이를 사용할 때만 문제 알림 선택란을 표시합니다.
진단 파일 내보내기는 서버 연결 상세에서 사용할 수 있습니다. Core 패키지 진단은 CLI에서 수행합니다.
설정에서 완전 종료로 바꿀 수 있고, 메뉴의 `완전히 종료`와 Cmd+Q는 설정과 무관하게
앱의 진행 중인 작업을 정리하고 종료합니다. 설정은 비공개 `desktop/behavior.json`에 저장합니다.

자동 수집·전송은 Codex의 SessionStart·Stop·PostCompact·SessionEnd 훅이 담당합니다.
데스크톱 앱에는 별도 자동 수집 타이머가 없으며, 앱을 종료해도 기존 동의에 따른 훅은
독립적으로 실행됩니다. 훅 호출이 없는 동안의 주기적 재전송을 보장하지는 않습니다.
앱에서 수동으로 시작한 작업은 트레이로 숨겼을 때 계속됩니다.

개요와 서버 화면은 최근 서버 수신 확인 건수·시각과 대기 건수를 보여줍니다.
수신 기록은 검증된 서버 ACK 뒤에만 비공개 파일로 저장합니다. 최근 전송 묶음의
집계 이벤트 수이며 전체 누적 또는 고유 이벤트 수는 아닙니다. 중복 수신 ACK도 포함됩니다.
수집 성공이나 단순 연결 확인만으로 전송 성공을 표시하지 않습니다.
화면이 보일 때 주기적으로 상태를 읽고, 창으로 복귀하면 다시 갱신합니다.

## 경계와 현재 검증 범위

Rust의 기존 수집기와 Compose 렌더러를 공유합니다. 수집 작업은 같은 앱 바이너리의 제한된
worker 프로세스로 실행해 Codex App/CLI 환경을 분리하고 전역 환경 변수를 변경하지 않습니다.
등록키는 명령행 인자로 전달하지 않습니다. 확인된 연결은 최대 10분 동안 Rust 메모리에만 보관합니다.
취소·메뉴 이탈·서버 작업 전환·환경 변경 시 해제하며, Rust에서도 만료 시 폐기합니다. 설정 저장에 실패해
확인 티켓이 소모되면 재확인 폼으로 복구합니다.
기존 서버 주소를 바꾸려 하면 대기 데이터를 다른 서버로 보내지 않도록 중단합니다.

앱의 로컬 창만 명시적으로 허용된 명령을 호출할 수 있습니다. 생성 폴더 열기는 이번 앱 실행에서
실제로 생성한 경로만 사용하며 프런트엔드가 임의 경로를 지정할 수 없습니다. 원격 웹 페이지의 명령 접근,
임의 셸 실행, 범용 파일 읽기 권한은 프런트엔드에 제공하지 않습니다.
HTTPS 인증서 확인과 리다이렉트 거부는 기존 Rust HTTP 클라이언트를 따릅니다.

이 앱은 macOS Apple Silicon용 공개 미리보기입니다. 앱 번들 전체에 임시(ad hoc) 서명을
적용하고 무결성을 검증합니다. Developer ID 서명과 Apple 공증은 완료하지 않았습니다.
Tauri의 Unicode 간접 의존성 5개에 유지보수 중단 공지가 있어, 사용 경로와 제한된
예외를 [데스크톱 의존성 정책](SECURITY.md)에 공개했습니다. 새 취약점 검사는 유지합니다.
릴리스의 `groundline-desktop-aarch64-apple-darwin.zip`에서 앱을 받을 수 있습니다.
Windows·Linux 및 Intel Mac의 데스크톱 앱은 이번 배포에 포함하지 않습니다.
데스크톱 빌드도 macOS Apple Silicon으로 제한합니다. Linux용 잠금 의존성에는
미해결 `glib` 보안 공지가 있어, 호환 가능한 수정 버전을 적용한 뒤 별도로 검증해야 합니다.
앱 빌드나 GUI 설치는 이미 설치된 플러그인과 운영 서버를 자동으로 교체하지 않습니다.
실제 서버 등록, 수신 확인, Grafana 로그인, ClickHouse 저장은 배포 환경에서 별도 검증해야 합니다.
서버는 등록키 확인 API와 수집기의 ingest 계약을 지원해야 합니다.
계약이 바뀐 릴리스라면 API를 먼저 업그레이드합니다.

## 개발 실행과 검증

소스에서 개발할 때만 Node.js와 Rust stable, macOS의 Tauri 빌드 도구가 필요합니다.

```console
cd apps/desktop
npm ci
npm run tauri -- dev
```

브라우저 화면만 확인하려면 `npm run dev`를 사용합니다. 브라우저에서는 로컬 자격증명 접근,
파일 생성, 서버 연결을 실행하지 않으며 결과를 모의 성공으로 표시하지 않습니다.

```console
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri -- build --bundles app
```

이 하위 Cargo workspace는 기본 Core/Insights 네이티브 배포에 GUI 시스템 의존성을 추가하지 않습니다.

설계 기준은 [Tauri 구조](https://v2.tauri.app/concept/architecture/),
[명령 권한](https://v2.tauri.app/security/capabilities/),
[공식 Opener](https://v2.tauri.app/plugin/opener/),
[Compose 변수 이스케이프](https://docs.docker.com/compose/how-tos/environment-variables/variable-interpolation/)를 확인했습니다.

메뉴 회귀 검사는 [React Testing Library 공식 설정](https://testing-library.com/docs/react-testing-library/setup/)에
따라 Vitest와 jsdom을 사용합니다. 외부 인증·전송을 모의하는 UI 검사는 실제 서버 수신 증거와 구분합니다.
