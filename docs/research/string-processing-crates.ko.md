# 문자열 처리 crate 선택과 검증

체크섬 한 줄의 LF·CRLF 처리는 표준 바이트 슬라이스 `strip_suffix`로 제한한다.
현재 계약은 계산한 SHA-256, 공백 두 개, 정확한 실행 파일명으로 고정돼 있어
정규식이나 일반 체크섬 파서 crate가 필요하지 않다. 한 개의 줄 끝만 제거하고
나머지 바이트를 정확히 비교한다. 추가 줄·공백·BOM·다른 파일명·다른 해시는
거절하며 Core·Insights·배포 검증기가 같은 함수를 사용한다. Git의 실제
`core.autocrlf=true` checkout과 CRLF 설치 경로를 회귀 검증한다.

문자열에 형식이 있으면 해당 형식의 파서를 사용한다. 새 직접 구현이나 오류를
검토할 때 표준 라이브러리와 유지보수되는 crate를 함께 확인하고, 실제 호출부와
실패 입력을 연결할 수 있는 의존성만 도입한다. 최신 안정 버전의 호환성, 필요한
feature, 라이선스, 보안 공지는 잠금 파일과 배포 대상별로 검증한다.

| 형식 | 사용 도구 | 적용 경계 |
| --- | --- | --- |
| JSON | `serde_json`, `serde_path_to_error` | 엄격한 스키마와 값이 노출되지 않는 오류 분류 |
| URL | `url`, `percent-encoding` | URL 검증과 Markdown 로컬 경로의 UTF-8 디코딩 |
| TOML / YAML | `toml_edit`, `serde-saphyr` | 기존 주석 보존과 명시적 메타데이터 파싱 |
| Markdown | `pulldown-cmark 0.13.4` | 참조형 링크·이미지·이스케이프, 코드 예제 구분 |
| Rust 소스 | `syn 3.0.5`, `proc-macro2 1.0.107` | 개인정보 검사 예외의 정확한 바이트 범위 |
| 고정 패턴 | `regex`, `aho-corasick` | 일반 텍스트 패턴과 여러 개인정보 표식의 바이트 검색 |
| 비밀 문자열 | `secrecy`, `zeroize` | 비밀값 접근과 메모리 정리 |

0.24.3에서 Markdown의 참조형 잘못된 링크와 Rust 문자열·주석의 중괄호 때문에
검사 대상이 누락되는 입력을 먼저 재현한 뒤 전용 파서로 수정했다. 파싱할 수 없는
Rust 소스나 UTF-8에는 개인정보 예외를 적용하지 않는다. 예외는 기존 검사기 파일의
정확한 상수와 `#[cfg(test)]` 모듈의 지정된 테스트 함수에만 한정한다. Markdown의
디코딩된 로컬 경로는 기존 절대 경로·다른 스킴·패키지 밖 경로·심볼릭 링크 제한을
통과해야 한다. 코드를 표시하는 예제는 실제 링크로 취급하지 않는다.

Markdown과 Rust 파서는 `xtask`에만 직접 추가하며 불필요한 기본 feature는 끈다.
문자열을 검색하는 기존 Aho-Corasick 경로는 유지한다. `bstr`, `memchr`를 새 직접
의존성으로 추가할지는 비 UTF-8 입력 요구나 측정한 검색 병목이 생겼을 때 판단한다.
한글 표시 폭이나 문자 자르기는 실제 UI 문제를 재현한 뒤 Unicode 관련 crate를
검토한다. 의존성 개수 자체를 개선 지표로 쓰지 않는다.

Windows 파일 교체 문제에는 Rust 표준 `std::fs::rename`의 지원이 충분했다.
열린 읽기 핸들이 있는 대상을 교체할 때 Windows의 `FileRenameInfoEx` 처리를 사용하며,
새 파일의 소유자 전용 ACL을 보존한다. 권한을 완화하거나 무제한 재시도하지 않는다.

검증:

```console
cargo test --locked -p xtask --bin xtask
cargo test --locked -p groundline-runtime --lib --all-features
cargo run --locked -p xtask -- verify-source --root . --json
cargo run --locked -p xtask -- verify-history --root . --json
```

공식 근거: [Markdown 이벤트](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/),
[퍼센트 인코딩](https://docs.rs/percent-encoding/latest/percent_encoding/),
[Rust 파서](https://docs.rs/syn/latest/syn/),
[바이트 범위](https://docs.rs/proc-macro2/latest/proc_macro2/struct.Span.html),
[표준 파일 교체](https://doc.rust-lang.org/std/fs/fn.rename.html).

활동 기록 집계에서는 기존 `serde_json::RawValue`와 `serde::de::DeserializeSeed`를
사용한다. 큰 도구 결과를 `Value` 트리로 만들기 전에 결과 분류만 순회해 남기며,
원본 JSON 문법과 64 MiB 레코드 한도는 유지한다. 새 파서나 crate 없이 기존
Serde의 스트리밍 방문자로 이미지·본문 트리의 메모리 할당을 피한다. 작은 결과의
분류 동등성, 이스케이프 문자열, 큰 이미지 결과, 중복 순번의 설정 알림과 실제
사용량 충돌을 회귀 검증한다. 읽기 예산은 8 GiB, 보관한 집계 기록은 512 MiB로
독립 제한하며 실제 수집 창의 일부만 읽었을 때 전송하지 않는다.

공식 근거: [RawValue](https://docs.rs/serde_json/latest/serde_json/value/struct.RawValue.html),
[DeserializeSeed](https://docs.rs/serde/latest/serde/de/trait.DeserializeSeed.html).

0.25.6 검증 결과 판정은 잠금 파일의 `serde 1.0.229`, `serde_json 1.0.151`을
그대로 사용한다. 현재 Codex의 text/content 봉투와 settled batch를 깊이 6으로
제한해 해석하고, JSON 결과의 종료 코드·상태 외에는 `IgnoredAny`로
건너뛴다. 네이티브 텍스트 머리말은 표준 `str::lines`, `strip_prefix`, 정수
`parse`로 읽으며 stdout 전에 멈춘다. 고정된 머리말 몇 개에는 새 정규식
의존성이 필요하지 않다. 성공 테스트 이름의 timeout/rejected 오인, 비영 종료
텍스트의 성공 오인, 실행 중 결과, 미확인 결과, 종료 결과 중복, 대형 본문,
이스케이프와 JSON 문자열 봉투를 회귀 입력으로 고정한다. 실제 종료 증거가
없는 결과는 미확인으로 보존하며 종료 코드 0을 만들어 내지 않는다.

후속 대기 연결에는 `oxc_parser`, `oxc_ast`, `oxc_allocator`, `oxc_span`
0.149.0을 사용한다. 잠금 파일과 실제 빌드로 Rust 요구사항 1.96.0이 현재
1.98.1과 호환됨을 확인했다. JSON 형태의 직접 poll은 기존 Serde로 읽는다.
JavaScript `exec`는 문자열·주석·조건 분기 안의 poll을 실행으로 오인할 수
있으므로 정규식 대신 Oxc AST를 사용한다. 8 KiB와 문장부호 수 제한을 먼저
적용하고, 최상위에서 순서대로 실행되는 `await`와 정적 인자만 인정한다.
복수 poll은 각 `text` 출력과 네이티브 결과 봉투의 개수·순서가 일치할 때만
연결한다. 출력되지 않은 호출은 미확정으로 보존한다. 코드를 실행하거나
일반 제어 흐름을 추론하지 않는다. 동적 ID, spread, 중복 속성, 대화형 입력,
조건 분기, 파싱 오류는 연결하지 않는다. 외부로 실행 handle을 출력하지 않는다.

공식 근거: [Oxc parser](https://oxc.rs/docs/guide/usage/parser),
[고정 버전 API](https://docs.rs/oxc_parser/0.149.0/oxc_parser/).

Oxc의 숫자 변환 의존성 `dragonbox_ecma 0.1.12`는 포함된 `LICENSE-Boost`와
공식 [Boost Software License 1.0](https://www.boost.org/LICENSE_1_0.txt)을 확인해
BSL-1.0 선택을 해당 버전에만 허용한다. 전역 허용 목록이나 보안 권고 검사는
완화하지 않는다. 범위는 [cargo-deny의 패키지별 예외](https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html#the-exceptions-field-optional)로 고정한다.
