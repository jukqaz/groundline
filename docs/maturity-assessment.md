# GroundLine Maturity Assessment

Date: 2026-05-29

Scope: GroundLine v0.3.5 local release candidate, packaged plugin, local
provider install/update proof, CI, and public release surface.

## Current Conclusion

Overall maturity: 85/100.

GroundLine is a Public beta: useful, installable, and safe enough for real
agent sessions, but not yet a 1.0 package. The core idea is mature: a small
control plane for state proof, side-effect boundaries, handoff, release
discipline, and evidence. The 1.0 gap is now narrower: Real provider activation proof
and post-install confirmation need to be stronger before calling the package
stable. Version drift control, skill graduation decisions, compact first-use
workflow examples, and fake-home remote install/update proof are implemented
or in the current local candidate. Repeated published-ref confirmation and live
activation proof still need to be stronger before calling the package stable.

## Evidence Used

- Published source version: `0.3.3`. The local `v0.3.3` tag, remote `main`, and
  remote `v0.3.3` tag point at the same commit for the published patch.
- Remote CI evidence: the GroundLine push workflow for the v0.3.3 release commit
  completed successfully; the scheduled radar workflow after release also
  completed successfully.
- Current local evidence: `groundline_release_gate.py --json --keep-going
  --include-docker-execution --release-version 0.3.5` returns `status=PASS`
  after source/package validation, unit tests, privacy scan, fake-home remote
  install/update proof, real provider smoke, staged dogfood, and Linux Docker
  execution. Tag, push, and GitHub Release creation remain excluded and require
  explicit approval.
- Current provider install evidence: `groundline_provider_smoke.py --json
  --require-installed` returns `status=PASS`, `install_doctor_status=PASS`,
  `next_actions=[]`, `real_home_touched=false`, and `secret_value_printed=false`
  after local Codex, Claude Code, and Antigravity direct provider targets were
  refreshed from the current `0.3.5` packaged payload.
- Source validation: `validate_pack.py --json` returned `status=PASS`.
- Packaged validation: `plugins/groundline` validation returned `status=PASS`.
- Lint validation: `lint.py --json --require-actionlint` returned
  `status=PASS`.
- Provider-native validation: `groundline_provider_validate.py --json`
  returned `status=PASS` for local Claude Code and Antigravity validators.
- Unit tests: `python3 -m unittest discover -s tests -v` returned 129 tests OK.
- Safety eval: `groundline_safety_eval.py --json` returned `status=PASS` with
  4 synthetic cases and `mutation_performed=false`.
- Privacy scan: `groundline_privacy_scan.py --json` returned `status=PASS`
  with `finding_count=0`.
- Offline doctor and radar checks: `groundline_doctor.py --json --offline
  --probe-tools` and `groundline_radar.py --json --offline --command-sources`
  returned `status=PASS`.
- Package surface: 19 skills, 18 references, 45 packaged docs after this
  assessment.
- Skill lifecycle: 7 active skills and 12 experimental skills.
- Graduation decisions: 2 graduate candidates, 1 merge candidate, 1 defer, and
  8 keep-experimental decisions recorded in docs and metadata.
- Workflow cookbook: five representative workflows map prompts to skills,
  output contracts, verification evidence, and stop conditions.
- Artifact lifecycle: research, comparison, recommendation, implementation,
  dogfood, release, and post-release review artifacts map to existing skills
  and output contracts.
- Provider install evidence: current local Codex, Claude Code, and Antigravity
  direct targets match the `0.3.5` package version, skill count, and content
  fingerprint. Published-ref confirmation is still needed before 1.0.
- Staged provider smoke evidence: `groundline_provider_smoke.py --json
  --stage-package --require-installed` returns `status=PASS`,
  `fake_home_used=true`, and `real_home_touched=false`.
- Staged dogfood evidence: six scenario contracts pass for Codex, Claude Code,
  and Antigravity with `real_home_touched=false`.
- Scenario evidence: macOS local, Linux Docker dry-run, and Linux Docker
  execution report `status=PASS`.
- Release gate runner: `groundline_release_gate.py --plan --json` records the
  gate order and excludes approval-required publishing commands.
- Provider cache evidence: Codex and Claude Code resolve GroundLine 0.3.2 from
  provider cache paths; Antigravity resolves package shape and 19 skills even
  when its installed semantic version is unavailable.
- Safety posture: hooks, rules, MCP servers, commands, provider-level agents,
  raw transcripts, and provider runtime state are not enabled or copied by
  default.

## Scorecard

| Axis | Score | Status | Reason |
| --- | ---: | --- | --- |
| Repository readiness | 92 | PASS | Public docs, license, security policy, manifests, release notes, CI, and provider packaging are present. |
| Skill completeness | 80 | PARTIAL | All 19 skills have triggerable instructions and contracts, and all 12 experimental skills now have graduation decisions. |
| Trigger clarity | 84 | PASS | v0.3.2 clarified ecosystem, maturity, and release routing; more provider activation evidence is still needed. |
| Context weight | 86 | PASS | Long guidance mostly lives in docs and references; skills stay short enough to load on demand. |
| Workflow coverage | 82 | PARTIAL | Core loops and compact workflow examples are covered; representative real-provider workflows are still sparse. |
| Verification strength | 88 | PARTIAL | Source, package, lint, unit, staged dogfood, provider smoke, fake-home remote install/update proof, and macOS/Linux Docker scenario gates are covered locally; live provider activation proof is still incomplete. |
| Security posture | 90 | PASS | Default behavior is read-only/offline, staged smoke refuses real-home writes, secret-sensitive docs exist, and provider state is kept out of source. |
| Provider install posture | 88 | PARTIAL | The install doctor now detects provider cache, installed version, payload, skill count drift, same-version content drift, and fake-home update transitions; published-ref confirmation is still needed before 1.0. |
| Maintenance discipline | 84 | PASS | Manifest version checks now use canonical `plugin.json`, and package validation catches conflict-copy artifacts. |

## Main Findings

1. The package is already good enough for staged local use.
   Evidence: source/package validation, staged provider smoke, staged dogfood,
   and macOS/Linux scenarios pass. Real provider targets exist with 19 skills,
   and the current local content fingerprint checks pass after Codex, Claude
   Code, and Antigravity direct provider targets were refreshed to the `0.3.5`
   package. v0.3.5 also adds fake-home update proof.

2. The remaining risk is not feature absence. It is proof quality.
   The staged dogfood harness now covers six scenario contracts, including all
   five activation prompt families, and the provider activation matrix includes
   a live proof collection runbook. It still does not prove live LLM skill
   selection for every prompt family. Real provider invocation proof is still
   partly manual and sanitized.

3. Installation diagnostics are now first-class.
   Provider smoke reports `install_doctor_status`, cache candidates, installed
   versions, payload presence, skill count drift, and same-version content
   drift without printing provider homes or auth material. It now catches stale
   cache content even when the semantic version still matches, and staged
   provider smoke proves the refresh path in a temporary home.

4. Experimental surface is still large, but now bounded.
   Twelve experimental skills remain acceptable for v0.3, and each now has a
   graduation, merge, defer, or keep-experimental decision. 1.0 still needs the
   actual active core to be smaller and proven after install.

5. Documentation is now readable, but next-work was stale after v0.3.2.
   The backlog should become versioned release planning, not an open-ended idea
   pile.

## Next Work Created

These tasks are the next bounded work items. They intentionally avoid new
provider runtimes, mandatory hooks, mandatory MCP setup, or broad feature
expansion.

### P0: Version-aware install doctor

Goal: make local provider install state obvious after GitHub install.

Status: implemented in v0.3.3, exercised in the local v0.3.4 candidate, and
extended in v0.3.5 with fake-home remote install/update proof. Keep it as a
release gate until published-ref install or update confirmation is repeated
outside the current local targets.

Acceptance:

- Add a script or doctor mode that reports installed GroundLine version for
  Codex, Claude Code, and Antigravity.
- Detect source ref drift, stale cache versions, missing package payload, and
  skill count mismatch.
- Report `PASS`, `PARTIAL`, or `FAIL` without printing provider auth, sessions,
  logs, or raw home dumps.
- Add unit tests with fake provider homes.
- Update install and provider packaging docs with the new check.

### P1: Real provider activation matrix

Goal: prove that provider sessions select the intended skill and output
contract for representative prompts.

Status: matrix document and staged six-scenario contract coverage are
implemented in v0.3.3. Live provider proof rows are still needed for side-effect
guard, ecosystem evaluation, and AI usage maturity.

Acceptance:

- Cover at least five prompt families: handoff, side-effect guard, release cut,
  ecosystem evaluation, and AI usage maturity.
- Record sanitized proof only: provider, prompt family, selected skill, output
  contract, mutation status, result, and short evidence.
- Keep raw transcripts and provider home dumps out of the repository.
- Keep staged contract harness evidence separate from real provider activation
  proof.

### P1: Skill graduation plan

Goal: move from a broad v0.3 skill set to a stable supported core.

Status: graduation decisions are recorded in `docs/skill-graduation-plan.md`,
`docs/skill-portfolio.md`, and `references/skill-index.json`. The current
patch does not promote lifecycle values yet.

Acceptance:

- Define graduation criteria for experimental skills.
- Classify each experimental skill as graduate, keep experimental, merge, or
  defer.
- Promote only skills with examples, output contracts, docs, tests, and dogfood
  evidence.
- Leave watch items in `docs/next-work.md` instead of adding new skills.

### P2: Workflow proof cookbook

Goal: show complete workflows that humans and LLMs can follow without reading
the full index.

Status: compact cookbook implemented in `docs/workflow-cookbook.md` and
`docs/ko/workflow-cookbook.md`.

Acceptance:

- Add concise end-to-end examples for state recovery, risky operation, release
  cut, ecosystem radar, and AI usage maturity.
- Each example maps prompt -> selected skill -> output contract -> verification
  evidence -> stop condition.
- Keep examples provider-neutral unless a provider-specific proof is required.

### P2: Single-source version control

Goal: reduce release drift when the package version changes.

Status: implemented in v0.3.3; keep as release gate until the next version bump
proves the canonical manifest path.

Acceptance:

- Validation reads the expected version from canonical manifest data instead of
  hard-coding a patch version in multiple places.
- Tests assert all provider manifests match that canonical version.
- Release docs explain the version bump sequence.

## 1.0 Gate

GroundLine should not call itself 1.0 until all of these are true:

- Provider install doctor passes on Codex, Claude Code, and Antigravity after a
  fresh GitHub install or update.
- At least five real provider activation proof rows are recorded with sanitized
  evidence.
- The active skill core is explicit, and experimental skills have lifecycle
  decisions.
- Version drift checks are automated and survive the next version bump.
- A fresh clone can validate, install, and confirm GroundLine without relying on
  prior local cache state.

## Release Decision

v0.3.5 is the current local release candidate. It carries forward the v0.3.4
local proof-quality work and adds fake-home remote install/update proof. Source
and packaged manifests are `0.3.5`; real provider targets were refreshed
locally; the full local release gate passes. Published-ref proof must still be
collected before public release.

Current decision: continue until live activation proof rows are collected or
explicitly accepted as partial. Do not add a new skill unless the activation
matrix proves a repeated failure that existing skills cannot express.
