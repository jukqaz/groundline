# 문자열 처리 crate 선택과 검증

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
