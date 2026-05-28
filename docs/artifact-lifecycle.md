# Artifact Lifecycle

GroundLine artifacts should move in a bounded loop:

```text
research -> compare -> recommend -> implement -> dogfood -> release -> post-release review
```

Each artifact type belongs to one primary skill and one primary output
contract. Do not create a new artifact type when an existing contract can carry
the evidence.

## Lifecycle Map

| Artifact | Primary skill | Output contract | Next artifact | Stop condition |
| --- | --- | --- | --- | --- |
| research packet | `research-agent-ecosystem` | `GroundLine Research` | comparison report | Sources, confirmed facts, unverified claims, and uncertainty are separated. |
| comparison report | `compare-agent-workflows` | `GroundLine Comparison` | upgrade decision | Candidates are scored against GroundLine scope, context cost, and setup surface. |
| upgrade decision | `recommend-groundline-upgrades` | `GroundLine Recommendation` | implementation task or watch item | Adopt, adapt, watch, and reject are separated without expanding the current release. |
| implementation task | `package-agent-task` | `GroundLine Task Packet` | dogfood evidence | Goal, constraints, non-goals, mutation boundary, success criteria, and verification are clear. |
| dogfood evidence | `close-live-work` or `stabilize-release-cut` | `Status: PASS / PARTIAL / FAIL` or `GroundLine Release Cut` | release cut | Evidence says what passed, what remains partial, and what cannot be claimed. |
| release cut | `stabilize-release-cut` | `GroundLine Release Cut` | release delta | Ship, hold, or continue is explicit, with gate evidence and a single next action. |
| release delta | `compare-release-delta` | `GroundLine Release Delta` | next-work item or rollback note | Expected changes, unexpected changes, runtime evidence, install evidence, and rollback note are recorded. |

## Template Rules

- Start from the output contract in `references/output-contracts.md`.
- Keep source-backed facts separate from unverified claims.
- Put raw transcripts, credentials, provider caches, and full home dumps outside
  repository artifacts.
- Keep provider-native feature duplication out of GroundLine. If Codex, Claude
  Code, or Antigravity already owns the feature, GroundLine should record a
  setup recommendation or boundary, not rebuild the runtime feature.
- Do not create a new artifact type during a release cut unless an existing
  contract cannot express repeated dogfood failure.

## Escalation Rules

- A research packet may become a comparison report only when it has primary or
  clearly labeled secondary sources.
- A comparison report may become an upgrade decision only when it names overlap
  with existing GroundLine skills.
- An upgrade decision may become implementation work only when it says adopt or
  adapt and names side-effect boundaries.
- Dogfood evidence may support a release cut only when it separates staged
  harness output from real provider activation proof.
- A release delta may create next work only after comparing the released version
  with the previous installed or tagged version.

## Minimal Templates

### research packet

```text
GroundLine Research:
- scope:
- primary sources:
- secondary sources:
- candidate list:
- confirmed facts:
- unverified claims:
- uncertainty:
```

### comparison report

```text
GroundLine Comparison:
- candidates:
- scoring axes:
- strongest matches:
- overlap with GroundLine:
- context and setup cost:
- rejected or deferred:
- comparison gaps:
```

### upgrade decision

```text
GroundLine Recommendation:
- adopt:
- adapt:
- watch:
- reject:
- next GroundLine changes:
- side-effect boundary:
- verification checklist:
```

### release cut

```text
GroundLine Release Cut:
- current conclusion:
- scope lock:
- change budget:
- must fix:
- defer:
- reject:
- release gates:
- dogfood evidence:
- regression check:
- ship decision: ship|hold|continue
- next action:
```

### release delta

```text
GroundLine Release Delta:
- current conclusion:
- previous version:
- deployed version:
- comparison source:
- delta checklist:
- expected changes:
- unexpected changes:
- runtime evidence:
- install evidence:
- regression evidence:
- rollback note:
- decision: keep|monitor|rollback|not_enough_evidence
- follow-up:
```
