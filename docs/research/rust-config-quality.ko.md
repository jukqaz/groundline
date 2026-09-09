# 설정 보정의 Rust 품질·성능 개선

v0.23.2는 설정 보존 검증, 오류 위치 진단, 카탈로그 재사용에 집중한다.
다음 crate를 실제 실행 또는 검증 경로에 연결했다.

| crate | 적용 |
| --- | --- |
| `toml_edit 0.24.0` | 기존 항목의 값을 제자리에서 바꾸어 인용 키와 앞쪽 주석까지 보존 |
| `proptest 1.11.0` | 카탈로그 조회 256개, 설정 보존 32개, 거부 시 파일 보존 32개 조합 |
| `serde_path_to_error 0.1.20` | 카탈로그 디코딩 실패 시 알려진 스키마 필드만 고정 오류 코드로 표시 |
| `divan 0.1.21` | 동일 카탈로그의 두 번 파싱과 한 번 파싱을 시간·할당량으로 비교 |

`proptest`와 `divan`은 개발 의존성이며 배포 실행 경로에 포함되지 않는다.
오류 경로 추적은 실패한 입력에만 수행한다. 정상 입력은 일반 Serde 파서를
한 번 사용하고, 검증한 카탈로그를 수정 전후 감사에서 재사용한다. 모델 조회는
정렬된 목록의 이진 검색을 사용한다.

오류 출력은 모델명, 실제 값, 동적 map key, 원래 역직렬화 오류를 포함하지 않는다.
예를 들어 모델 이름 필드의 타입 오류는
`config_audit_invalid_catalog_model_slug`로 표시한다. 입력 크기, 모델·추론 수준
검증, 단일 UTF-8 BOM, trailing data 거부 계약은 유지한다.

## 발견한 결함

생성형 테스트가 CRLF 파일에서 인용된 `"model"` 키를 교체할 때 앞쪽 사용자
주석이 사라지는 사례를 찾았다. `Table::insert`가 기존 키의 표현을 다시
포맷하기 때문에 발생했다. 기존 키를 유지하고 값만 교체하도록 수정했으며,
축소된 입력을 별도의 고정 회귀 테스트로 남겼다.

생성형 파일 테스트는 실제 CLI를 실행하여 백업 원문 일치, 비대상 값 보존,
두 번째 적용의 쓰기·추가 백업 없음, 지원하지 않는 설정의 무변경 실패를 확인한다.
합성 입력만 사용하며 실행당 사례 수를 제한한다. 릴리스 CI는 여섯 네이티브
플랫폼에서 카탈로그와 설치·보정 검사를 실행한다.

## 성능 근거와 범위

설정 감사와 보정이 함께 쓰는 카탈로그 타입·검증을 공통 모듈로 분리했다.
설치 기본 정책은 실행당 한 번 파싱하여 적용 후보와 보고서에서 함께 사용한다.
카탈로그의 인코딩 검사는 공통 파서 테스트로 옮겨 플랫폼 CI에서 직접 실행한다.

워크스페이스의 미사용 직접 의존성 10개를 제거했다. API의 `hmac`, `tempfile`,
`zeroize`, runtime의 `sha2`, `walkdir`, 두 CLI의 `thiserror`, contracts의
`regex`, xtask의 `subtle`, `toml`이다. 필요한 전이 의존성은 유지한다.
서로 다른 major 버전을 요구하는 전이 의존성은 강제 통합하지 않는다.
`cargo machete --with-metadata`로 잔여 미사용 직접 의존성을 확인한다.

2026-09-09 Apple Silicon에서 최적화 빌드, 각 50 samples / 50 iterations로
카탈로그 파싱·검증 부분을 비교했다. 각 모델은 합성 지침 약 9 KiB를 포함한다.

| 모델 수 | 두 번 파싱 중앙값 | 한 번 파싱 중앙값 |
| --- | --- | --- |
| 8 | 23.20 µs | 11.53 µs |
| 128 | 365.5 µs | 182.0 µs |
| 512 | 2.067 ms | 745.9 µs |

128개 입력의 누적 할당 횟수는 1,538회에서 769회, 할당 바이트는 83.10 KB에서
41.55 KB로 줄었다. 최대 동시 할당량은 두 경우 모두 25.05 KB였다.
이 비교는 동일 파서에서 중복 작업 제거 효과를 분리한 측정이다. 전체 CLI 실행,
네트워크 설치 또는 Codex 추론의 속도 개선 수치로 해석하지 않는다.

재현:

```console
cargo bench --locked -p groundline-cli --bench config_catalog -- --sample-count 50 --sample-size 1
cargo test --locked -p groundline-cli --lib --test setup_cli --test config_audit_cli
```

근거: [Proptest 전략·축소](https://docs.rs/proptest/1.11.0/proptest/strategy/trait.Strategy.html),
[Serde 오류 경로](https://docs.rs/serde_path_to_error/0.1.20/serde_path_to_error/struct.Error.html),
[Divan 벤치마크와 할당 분석](https://docs.rs/divan/0.1.21/divan/).
