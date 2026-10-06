# 환경 적용·복구의 경계 조건 조사

2026-10-06. [환경 개선 설계](adaptive-environment-design.md)의 중단 복구,
오래된 공통 기준, 부분 복구 의존성, 경로 소유권을 조사한 결과입니다.
아래 계약은 후속 구현의 기준입니다. 환경 동기화 엔진이나 실패 복구가 현재
제품에서 구현·검증됐다는 뜻은 아닙니다. 제품 코드·개인 설정·스킬은 이번 조사에서
변경하지 않았습니다.

## 결론과 적용 범위

| 경계 조건 | 후속 구현에 사용할 최소 계약 |
| --- | --- |
| 파일 반영 뒤 중단·영수증 실패 | 대상 변경 전에 백업과 준비 기록을 영속화하고, 재시작 시 실제 디스크 상태로 복구 판정 |
| 두 기기의 오래된 기준 | 기준의 부모 revision을 조건부로 갱신하고, 적용 전에 기준·예외·관리 권한도 재검증 |
| 사용자 편집이 있는 부분 복구 | 남은 consumer를 보존 대상으로 삼아 필요한 의존 파일과 버전을 유지 |
| 같은 bytes의 다른 경로 | 승인된 owner root·상대 경로·파일 종류·소유권·link 상태를 digest와 함께 확인 |

기존 파일 저장 함수와 이미 사용하는 rustix를 우선 재사용합니다. 새 DB·배포 서비스·
분산 잠금 시스템은 초기 구현에 추가하지 않습니다. 이 계약은 GroundLine writer를
조율하며, 임의의 외부 editor까지 잠그거나 여러 파일/기기를 원자적으로 바꾸는 계약은
아닙니다. 실제 변경이 없는 작업에 준비 기록·백업·LLM 분석을 생성하지 않습니다.

## 1. 변경 전에 복구 근거를 남긴다

### 확인한 사실

- [Rust File::sync_all](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)은
  파일 내용·메타데이터의 동기화를 시도하며 오류를 반환합니다. 파일 close 시 무시되는
  오류를 성공 근거로 삼으면 안 됩니다.
- [Linux fsync](https://man7.org/linux/man-pages/man2/fsync.2.html)는 파일과 그 이름을
  담는 디렉터리의 영속화를 구분합니다. 파일 sync만으로 이름 변경을 보장하지 않습니다.
- [Apple fsync](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)는
  일반 fsync와 장치 cache flush를 요청하는 F_FULLFSYNC를 구분합니다.
  [조사 시점 Rust Unix 구현](https://raw.githubusercontent.com/rust-lang/rust/master/library/std/src/sys/fs/unix.rs)의
  Apple 경로는 F_FULLFSYNC를 사용합니다. 설치 toolchain을 확인하지 않고
  추가 호출이 필요하다고 단정하지 않습니다. master 링크는 이후 바뀔 수 있습니다.
- [tempfile persist](https://docs.rs/tempfile/3.23.0/tempfile/struct.NamedTempFile.html#method.persist)는
  기존 대상 교체와 sync를 구분합니다. 호출자가 파일·디렉터리 sync를 책임져야 합니다.
- [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html)의 복구 기록 선행과
  [WAL](https://www.sqlite.org/wal.html)의 동기화 조건은 DB 내부의 계약입니다.
  별도 SQLite에 기록해도 외부 설정 파일 교체까지 하나의 DB transaction에 들어가지는
  않습니다. 후자는 GroundLine 대상에 적용한 설계 판단입니다.

### 기존 코드와 추가 계약

[local_file.rs](../crates/groundline-runtime/src/local_file.rs)의 기존 원자적 저장 함수는
대상과 같은 디렉터리의 임시 파일을 쓰고 sync한 다음 rename·부모 sync를 합니다.
[config_repair.rs](../crates/groundline-cli/src/config_repair.rs)는 먼저 원본 백업을
sync하며, rename 이후 오류를 configuration_changed=null/write_outcome_unverified로
보고합니다. 이 저장 함수 위에 아래 복구 기록을 추가하는 것이 작은 변경입니다.
[audit_store.rs](../crates/groundline-runtime/src/audit_store.rs)의 Codex DB는 읽기 전용
관측 대상이며 새 복구 기록의 저장소로 사용하지 않습니다.

1. private 기록 root 아래 고유 operation을 준비합니다. 새 디렉터리 이름을 만드는
   경우 그 부모의 저장 조건도 확인합니다.
2. 원본 존재 여부를 기록하고, 기존 원본은 새 백업에 저장·sync합니다. 후보는 같은
   대상 디렉터리의 private 임시 파일에 완전히 쓰고 sync합니다.
3. 준비 기록을 완전한 작은 문서로 저장·sync합니다. operation/target ID,
   기준·예외·권한 revision, 전후 존재 여부·digest, 백업 참조, 경로 binding,
   의존 관계가 최소 내용입니다. 이는 현재 CLI schema를 추가했다는 설명이 아닙니다.
4. 원본·경로·기준을 마지막으로 확인한 뒤 반영합니다. 기존 파일은 rename,
   신규 파일은 exclusive-create/no-clobber 계약을 사용합니다. 준비 후 다른 writer가
   만든 신규 파일을 기존 파일처럼 덮어쓰지 않습니다.
5. 대상 부모 sync와 실제 내용 검사를 완료하고 완료 영수증을 안전하게 저장합니다.
   준비·백업 저장 실패는 대상 변경 전에 중단합니다. rename 이후 sync·영수증 실패는
   결과 미확정으로 남기고 백업을 보존하며 관련 후속 변경을 보류합니다.
6. 복구도 별도 operation으로 기록합니다. 실패한 operation의 백업을 재사용하면서
   원본 근거를 덮어쓰거나, 현재 사용자 파일을 무조건 원복하지 않습니다.

완료 기록이 없거나 불완전하면 현재 상태를 다시 읽습니다.

| 재조사 결과 | 처리 |
| --- | --- |
| 원본 상태와 같음 | 현재 변경 전 상태로 기록하고, 재실행 전에 계획 유효성을 확인 |
| 후보 상태와 같음 | 현재 후보 상태를 관측했다고 기록하고 권한·native 검증을 다시 확인 |
| 어느 상태와도 다름, 있어야 할 파일이 없음, 백업 불일치 | 충돌로 보존하고 자동 덮어쓰기 중단 |

처음부터 없었던 대상이 여전히 없는 경우도 변경 전 상태입니다. digest 일치만으로
과거 mutation 주체나 당시 native 활성 검증을 인증하지 않습니다. 준비 기록만 있는
상태를 완전한 적용 성공으로 승격하지 않습니다. 기록 없는 임시 파일도 완료 증거가
아니며 살아 있는 작업과 구분하기 전에는 일괄 삭제하지 않습니다.

동일 파일시스템 rename의 원자성과 전원 차단 후 영속성은 별개입니다.
SIGKILL 시험은 프로세스 중단을 다루며 전원 차단을 증명하지 않습니다. 필요한 sync를
지원하지 않으면 성공으로 대체하지 않습니다. 실제 macOS/APFS·Linux 대상의
영속성 수준은 후속 구현에서 검사하고, 미지원·미검증·반영 후 불확실을 구분합니다.

## 2. 공통 기준도 예상 revision이 맞을 때만 갱신한다

[Git update-ref](https://git-scm.com/docs/git-update-ref)는 예상 old OID가 현재 값과
같을 때만 ref를 갱신할 수 있습니다. 이는 기준의 조건부 갱신에 참고할 실제 계약입니다.
[Git push](https://git-scm.com/docs/git-push#_push_rules)는 보통 branch의 fast-forward
갱신만 허용합니다. 충돌 해결을 대신 수행하거나 모든 기기의 파일 적용을 묶는 기능은
아닙니다.

GroundLine에 적용할 최소 결정은 다음과 같습니다.

- 초기에는 현재 Mac을 개인 기준의 갱신 지점으로 두고, 추가 기기는 그 기준에 대한
  제안과 기기별 적용 결과를 연결합니다. 원격 private 저장소를 자동 생성하지 않습니다.
- 기준은 부모 revision을 가진 불변 snapshot과 현재 revision을 구분합니다.
  GroundLine writer 잠금 아래 예상 부모를 확인하고 현재 revision을 조건부 갱신합니다.
  다른 갱신이 먼저 채택됐으면 자동 병합·덮어쓰기 대신 새 기준으로 재계획합니다.
- apply는 대상 파일뿐 아니라 basis/기기 예외/관리 권한 revision을 다시 확인합니다.
  바뀐 항목에 대한 오래된 계획은 적용하지 않고 이미 적용한 항목은 영수증으로 보존합니다.
- 사용자가 private Git을 선택하면 commit 기반 snapshot과 일반 fast-forward 갱신을
  이용합니다. force push를 충돌 해결 수단으로 삼지 않습니다. Git ref의 조건부 갱신을
  임의 설정 파일의 전역 compare-and-swap 보장으로 확대하지 않습니다.
- 기준의 최신 상태를 확인할 수 없는 기기는 inspect/plan과 pending 상태를 사용할 수
  있습니다. offline 적용은 사전에 정의한 revision·관리 권한 범위가 있을 때만 허용하며,
  확인되지 않은 최신성이나 전체 기기 정렬 성공을 보고하지 않습니다.
- 한 기기의 최종 검사 뒤 기준이 또 바뀔 수 있습니다. 해당 적용은 확인한 revision의
  결과로 남기고 다음 inspect에서 drift를 표시합니다. 분산 동시 반영은 주장하지 않습니다.

조사 중 격리된 로컬 bare 저장소에서 실제 Git 2.54.0으로 확인했습니다. 같은 부모에서
만든 첫 갱신은 성공했고, 같은 예상 부모를 사용하는 후속 갱신은 거부됐습니다.
로컬 bare remote에서도 정상 fast-forward는 성공하고 분기된 후속 publish는 거부됐으며
기존 accepted revision은 유지됐습니다. 외부 remote·프로젝트 ref는 변경하지 않았습니다.
이 시험은 Git 조건부 갱신만 검증하며 환경 엔진·다중 기기 활성화 검증은 아닙니다.

## 3. 남아 있는 파일의 의존성도 복구에서 보존한다

[Nix GC](https://nix.dev/manual/nix/2.34/command-ref/nix-store/gc.html)는 root에서
도달 가능한 store path를 보존합니다.
[실제 GC 구현](https://raw.githubusercontent.com/NixOS/nix/master/src/libstore/gc.cc)은
reference closure를 계산해 살아 있는 항목을 표시합니다. GroundLine은 이 보존 원칙을
좁게 참고하며 Nix store·GC 시스템을 추가하지 않습니다.

- plan에 consumer → dependency와 필요한 content/revision을 기록합니다. SKILL에서
  reference를 쓰는 경우가 첫 대상입니다. 등록된 관리 대상만 다룹니다.
- 적용은 필요한 dependency를 준비·검증한 뒤 consumer를 바꿉니다. 여러 파일 사이
  원자성을 주장하지 않으며 각 항목의 적용 상태를 기록합니다.
- rollback은 consumer부터 처리합니다. 사용자 편집으로 남은 consumer가 있으면
  그것을 보존 대상으로 삼아 전이 의존성도 유지합니다.
- 의존 파일 삭제뿐 아니라 이전 버전 복원도 retained consumer를 깨뜨리면 보류합니다.
  shared dependency의 일부 consumer만 복구된 경우도 같습니다.
- 파괴적 복구 직전에 관련 consumer를 재확인합니다. 참조를 확실히 해석하지 못하면
  마지막으로 확인한 closure를 보존하고 retained_dependency/부분 복구로 기록합니다.
  임의 사용자 Markdown의 모든 의존성을 자동으로 알아냈다고 보고하지 않습니다.

예: 사용자가 수정한 SKILL이 신규 reference를 계속 사용하면 SKILL과 그 reference를
함께 유지합니다. 사용자가 바꾼 내용은 복구 diff로 제시하고 나머지 독립 항목만 복구합니다.

## 4. 내용과 경로의 소유 경계를 함께 확인한다

[Linux open](https://www.man7.org/linux/man-pages/man2/open.2.html)의 O_NOFOLLOW는
마지막 요소만 검사합니다. 앞선 symlink의 처리와 openat의 디렉터리 fd 사용은 별개의
문제입니다. [Apple open](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/open.2.html)도
대상 symlink 처리 조건을 명시합니다.
[hardlink](https://www.man7.org/linux/man-pages/man2/link.2.html)는 다른 이름이 같은
파일을 참조하는 경우여서 같은 inode라는 이유만으로 쓰기 대상 이름을 합치면 안 됩니다.

[cap-std 4.0.3](https://docs.rs/crate/cap-std/4.0.3)은 Linux/macOS 지원과 디렉터리
범위 내 symlink 처리를 설명합니다.
[Dir 구현](https://docs.rs/cap-std/4.0.3/src/cap_std/fs/dir.rs.html)은 열린 디렉터리를
기준으로 상대 경로 연산을 제공합니다. 일반 rename은 digest 조건부 교체 기능이 아닙니다.
cap-std는 비교 후보이며 이번 조사에서 추가한 의존성은 아닙니다.

기존 local_file은 leaf no-follow와 config hardlink 거부를 사용하고 config_repair는
parent를 canonicalize합니다. 이 경계를 유지하면서 다음을 추가합니다.

- 실제 owner root/parent의 검증한 핸들을 기준으로 상대 경로 연산을 합니다.
  기존 rustix로 필요한 macOS/Linux 계약을 유지할 수 있는지 첫 구현에서 확인합니다.
  직접 구현 범위가 커지면 유지보수되는 capability crate를 비교합니다.
- plan/apply/rollback에서 승인된 root·상대 경로·파일 종류·owner·link count·digest를
  확인합니다. 실제 binding이 달라지면 bytes가 같아도 충돌로 처리합니다.
- 등록된 home/skill alias는 inspect에서 설명합니다. 다른 root로 이어지는 alias는
  실제 root가 별도로 관리 대상으로 등록돼 있어야 합니다. 링크를 따라 임의 범위를
  새로 관리 대상으로 삼지 않습니다.
- App/PATH alias가 같은 실제 parent·leaf를 가리키면 한 apply 항목과 잠금으로 묶습니다.
  별개 hardlink 이름은 합치지 않으며 별도 topology 계약이 없으면 기존 nlink == 1
  경계를 유지합니다.
- 디렉터리 핸들은 rename된 옛 디렉터리를 계속 가리킬 수 있으므로 logical alias의
  binding도 확인합니다. 핸들 confinement만으로 현재 경로의 활성 상태를 인증하지 않습니다.

검사와 교체 사이에 협조하지 않는 editor가 바꾸는 race는 남습니다. 원본 재읽기와
경로 검사는 그 구간을 줄이며 협조하는 writer는 잠금으로 조율합니다. 이 범위 밖까지
사용자 편집이 절대 덮어써지지 않는다고 보장하지 않습니다.

## 구현 검증 조건

| 영역 | 필요한 회귀 |
| --- | --- |
| 준비 실패 | 백업·준비 기록의 write/sync 실패 또는 ENOSPC/EIO에서 대상 불변 |
| 중단 후 재개 | 후보 write·rename 직전/직후·완료 영수증 전의 프로세스 종료에서 현재 상태 판정 |
| 결과 미확정 | rename 뒤 sync/영수증 실패를 미적용으로 단정하지 않음 |
| 신규 대상 | absent 원본, 준비 뒤 다른 writer의 생성, no-clobber 및 반복 적용 |
| 기준 충돌 | 같은 부모의 두 갱신, 예외·권한 revision 변경, offline 계획, 이미 적용한 항목 보존 |
| 의존성 | 편집된 consumer, shared dependency, A→B→C, 이전 버전 복원이 깨뜨리는 경우 |
| 경로 변경 | 같은 bytes의 symlink, 조상 retarget, parent rename/recreation, 검사 뒤 hardlink 추가 |
| 정상 alias | 등록 alias 유지, App/PATH 중복 대상 한 번 쓰기, 미등록 root 거부 |
| 사용자 편집 | 영수증 실패 뒤 편집·복구 충돌, 재개 후에도 사용자 내용과 필요한 dependency 보존 |

실제 전원 차단·쓰기 유실/재정렬은 폐기 가능한 이미지/VM에서 별도로 다룹니다.
호스트 실제 파일을 강제 종료/전원 차단 대상으로 삼지 않습니다. 플랫폼별 실제
durability, 환경 엔진의 실패주입 회귀, App/PATH 적용·활성화는 아직 미검증입니다.
이번에 실행한 시험은 격리된 Git revision guard뿐입니다. 문서 검사 통과를 위 동작의
검증 완료로 보고하지 않습니다.
