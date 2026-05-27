# Release Discipline Layer Design

Date: 2026-05-28

## Current Conclusion

GroundLine already has release stabilization, skill curation, recommendation,
dogfood, and validation surfaces. The next improvement should not add another
capability. It should make expansion control explicit so new ideas are sorted
before they enter release scope.

## Problem

GroundLine is becoming useful because it captures recurring agent-work patterns
as skills, references, scripts, and tests. The same strength creates a risk:
every promising idea can become another feature, skill, or integration before
there is dogfood evidence that it belongs in the current release.

The release flow needs a hard bias toward scope control:

- New ideas should not automatically become implementation work.
- `must fix` should require release-blocking evidence.
- `adapt` should require a broken or missing behavior inside the current scope.
- `watch` should be the default for promising but unproven ideas.
- `defer` should capture useful ideas outside the current release cut.
- `reject` should capture unsafe, duplicated, broad, or unsupported ideas.

## Goals

- Add a repeatable expansion-control rule to release stabilization.
- Keep GroundLine lightweight by requiring evidence before new skills or
  capability surfaces are added.
- Make recommendation, curation, dogfood, and release-cut language agree.
- Add contract tests so the rule cannot silently disappear.

## Non-Goals

- No new skill.
- No new MCP setup.
- No provider home mutation.
- No actual provider dogfood execution in this change.
- No automation job for recurring release reviews.
- No change to the supported runtime or platform scope.

## Design

### Expansion Classifier

Release stabilization should classify every new idea during a release cut:

```text
new idea
-> release blocker evidence exists? must fix
-> fixes broken in-scope behavior? adapt
-> promising but unproven? watch
-> useful but outside this cut? defer
-> unsafe, duplicated, too broad, or unsupported? reject
```

The classifier is a judgment rule, not a new script. It belongs in
`references/release-stabilization.md` and the `stabilize-release-cut` skill.

### Evidence Rules

`must fix` requires at least one of:

- failing release gate
- public release blocker
- privacy or security finding
- broken install, provider smoke, runtime check, or docs contract
- repeated dogfood failure in a supported provider

`adapt` requires a current-scope behavior gap and a verification path.

`watch` is the default when an idea is attractive but has no failure evidence.

`defer` is for useful work that would expand the current release cut.

`reject` is for unsafe setup, unsupported provider scope, broad context load,
duplicated behavior, or ideas that violate GroundLine's boundary.

### New Skill Gate

GroundLine should create a new skill only when at least one condition is true:

- the same failure appears in repeated work or dogfood
- adding the workflow to an existing skill would make that skill too broad
- the workflow needs its own output contract, risk boundary, or lifecycle entry

Otherwise, the idea should become a source-registry item, reference note,
example, or `watch` item.

### Surface Updates

Update these files:

- `references/release-stabilization.md`: add the classifier and evidence rules.
- `skills/stabilize-release-cut/SKILL.md`: require expansion classification
  before accepting new scope.
- `skills/recommend-groundline-upgrades/SKILL.md`: make `watch` the default
  for attractive but unproven external ideas.
- `skills/curate-groundline-skills/SKILL.md`: add the new skill gate.
- `docs/dogfood.md`: state that new skills need dogfood or repeated failure
  evidence before becoming release scope.
- tests: add contract coverage for the classifier, evidence rules, default
  `watch`, and new skill gate.

## Data Flow

1. A user or agent proposes a new idea.
2. `stabilize-release-cut` checks the current scope lock and release gates.
3. The idea is classified as `must fix`, `adapt`, `watch`, `defer`, or `reject`.
4. Only `must fix` and narrow `adapt` items may change the current release cut.
5. `watch`, `defer`, and `reject` items are documented without implementation.
6. Verification reruns the relevant release gates.

## Error Handling

- If evidence is missing, classify as `watch` or `defer`; do not invent a
  blocker.
- If the item touches provider homes, remotes, access, billing, production, or
  secrets, require explicit approval before mutation.
- If classification is ambiguous, choose the narrower status and state the
  missing evidence.

## Testing

Add or update contract tests to require:

- release stabilization reference contains the expansion classifier
- `must fix` requires release-blocking evidence
- `watch` is the default for promising but unproven ideas
- new skill creation requires repeated failure, dogfood evidence, or clear
  separation from existing skills
- dogfood docs keep new skill creation out of release scope without evidence

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
git diff --check
```

Provider smoke should remain read-only:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

## Success Criteria

- A release cut cannot accept a new idea without classification.
- New skills are harder to add than references, examples, or watch items.
- The docs and skill instructions agree on `must fix`, `adapt`, `watch`,
  `defer`, and `reject`.
- Validation, lint, unit tests, provider smoke, and diff whitespace checks pass.
