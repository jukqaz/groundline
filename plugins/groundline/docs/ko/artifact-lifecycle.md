# Artifact Lifecycle

영어 `docs/artifact-lifecycle.md`가 canonical입니다. 이 문서는 사람이 빠르게
흐름을 이해하기 위한 한국어 companion입니다.

GroundLine artifact는 다음 흐름으로 이동합니다.

```text
research -> compare -> recommend -> implement -> dogfood -> release -> post-release review
```

새 artifact type을 만들기 전에 기존 output contract로 표현할 수 있는지 먼저
확인합니다.

## 흐름 요약

| Artifact | Primary skill | Output contract | 다음 |
| --- | --- | --- | --- |
| research packet | `research-agent-ecosystem` | `GroundLine Research` | comparison report |
| comparison report | `compare-agent-workflows` | `GroundLine Comparison` | upgrade decision |
| upgrade decision | `recommend-groundline-upgrades` | `GroundLine Recommendation` | implementation task 또는 watch item |
| implementation task | `package-agent-task` | `GroundLine Task Packet` | dogfood evidence |
| dogfood evidence | `close-live-work` 또는 `stabilize-release-cut` | `Status: PASS / PARTIAL / FAIL` 또는 `GroundLine Release Cut` | release cut |
| release cut | `stabilize-release-cut` | `GroundLine Release Cut` | release delta |
| release delta | `compare-release-delta` | `GroundLine Release Delta` | next-work item 또는 rollback note |

## 규칙

- source-backed facts와 unverified claims를 분리합니다.
- raw transcript, credential, provider cache, full home dump는 repository
  artifact로 남기지 않습니다.
- provider-native feature duplication은 피합니다. Codex, Claude Code,
  Antigravity가 이미 제공하는 기능은 GroundLine이 다시 만들지 않고 setup
  recommendation이나 boundary로 기록합니다.
- release cut 중에는 반복된 dogfood failure가 기존 contract로 표현되지 않을
  때만 새 artifact type을 검토합니다.
- Stop condition은 항상 명시합니다. 다음 artifact로 넘어가는 조건이 없으면
  현재 단계에서 멈춥니다.

## 승격 기준

- research packet은 source와 uncertainty가 분리되어야 comparison report로 갑니다.
- comparison report는 기존 GroundLine skill과의 overlap을 적어야 upgrade decision으로 갑니다.
- upgrade decision은 adopt 또는 adapt와 side-effect boundary가 있어야 implementation task가 됩니다.
- dogfood evidence는 staged harness와 real provider activation proof를 분리해야 release cut에 쓸 수 있습니다.
- release delta는 실제 released version과 previous version을 비교한 뒤에만 next-work item을 만듭니다.
