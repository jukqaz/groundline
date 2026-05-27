# Release Discipline Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an evidence-first release discipline layer that classifies new ideas before they enter GroundLine release scope.

**Architecture:** This is a documentation-and-contract change. The release stabilization reference defines the classifier, the affected skills repeat the operational rule, dogfood records the release boundary, and tests lock the wording so the rule cannot drift.

**Tech Stack:** Markdown skills and references, Python `unittest`, stdlib-only validation scripts.

---

## File Structure

- Modify `tests/test_groundline_reference_contract.py`: add contract coverage for release discipline surfaces.
- Modify `references/release-stabilization.md`: define expansion classification and evidence rules.
- Modify `skills/stabilize-release-cut/SKILL.md`: require classification before scope changes.
- Modify `skills/recommend-groundline-upgrades/SKILL.md`: make `watch` the default for attractive but unproven ideas.
- Modify `skills/curate-groundline-skills/SKILL.md`: require a new skill gate before creating skills.
- Modify `docs/dogfood.md`: keep new skill creation outside release scope without dogfood or repeated failure evidence.

No commit step is included because the current repository already has a broad uncommitted release candidate and the user has not requested commits. Stop after verified diffs.

## Task 1: Add Failing Contract Coverage

**Files:**
- Modify: `tests/test_groundline_reference_contract.py`

- [ ] **Step 1: Add the failing test**

Append this method inside `GroundLineReferenceContractTests`, after `test_release_stabilization_defines_scope_lock_and_dogfood_gates`:

```python
    def test_release_discipline_surfaces_align_on_expansion_control(self) -> None:
        release = self.read_reference("release-stabilization.md")
        stabilize = (PACK_ROOT / "skills/stabilize-release-cut/SKILL.md").read_text(encoding="utf-8")
        recommend = (PACK_ROOT / "skills/recommend-groundline-upgrades/SKILL.md").read_text(encoding="utf-8")
        curate = (PACK_ROOT / "skills/curate-groundline-skills/SKILL.md").read_text(encoding="utf-8")
        dogfood = (PACK_ROOT / "docs/dogfood.md").read_text(encoding="utf-8")

        required_by_surface = {
            "release": [
                "Expansion Classifier",
                "release blocker evidence",
                "watch is the default",
                "must fix requires",
                "new idea cannot enter release scope without classification",
            ],
            "stabilize": [
                "Classify new ideas before accepting scope",
                "must fix requires release-blocking evidence",
                "watch is the default",
                "Do not add a new skill during stabilization",
            ],
            "recommend": [
                "watch is the default",
                "promising but unproven",
                "failure evidence",
            ],
            "curate": [
                "New Skill Gate",
                "repeated work or dogfood",
                "too broad",
                "output contract",
            ],
            "dogfood": [
                "new skills derived from dogfood results",
                "dogfood or repeated failure evidence",
                "release scope",
            ],
        }
        surfaces = {
            "release": release,
            "stabilize": stabilize,
            "recommend": recommend,
            "curate": curate,
            "dogfood": dogfood,
        }

        for surface, terms in required_by_surface.items():
            for term in terms:
                with self.subTest(surface=surface, term=term):
                    self.assertIn(term, surfaces[surface])
```

- [ ] **Step 2: Run the target tests and confirm RED**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.test_groundline_reference_contract.GroundLineReferenceContractTests.test_release_discipline_surfaces_align_on_expansion_control -v
```

Expected result: `FAILED` with `AssertionError` for missing terms such as `Expansion Classifier` and `New Skill Gate`.

## Task 2: Define Release Stabilization Classifier

**Files:**
- Modify: `references/release-stabilization.md`

- [ ] **Step 1: Add classifier text**

Insert this section after `## Scope Lock`:

````markdown
## Expansion Classifier

A new idea cannot enter release scope without classification.

```text
new idea
-> release blocker evidence exists? must fix
-> fixes broken in-scope behavior? adapt
-> promising but unproven? watch
-> useful but outside this cut? defer
-> unsafe, duplicated, too broad, or unsupported? reject
```

`watch` is the default for attractive ideas that lack failure evidence. The
classifier keeps the release cut from absorbing every good idea.
````

- [ ] **Step 2: Add evidence rules**

Replace the `## Triage Labels` section with:

```markdown
## Triage Labels

- `must fix`: blocks release readiness or user trust. `must fix requires`
  release blocker evidence such as a failing release gate, public release
  blocker, privacy or security finding, broken install, provider smoke failure,
  runtime failure, docs contract failure, or repeated dogfood failure.
- `adapt`: fixes broken in-scope behavior and has a verification path.
- `watch`: promising but unproven; keep as a tracked idea without implementation.
- `defer`: useful, but outside the current scope lock.
- `reject`: unsafe, too broad, duplicated, or outside supported scope.
```

- [ ] **Step 3: Run the target test and confirm partial progress**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.test_groundline_reference_contract.GroundLineReferenceContractTests.test_release_discipline_surfaces_align_on_expansion_control -v
```

Expected result: still `FAILED`, but release-surface failures are gone. Remaining failures should point to skill and dogfood files.

## Task 3: Tighten Stabilize Release Cut Skill

**Files:**
- Modify: `skills/stabilize-release-cut/SKILL.md`

- [ ] **Step 1: Update workflow**

Replace the entire `## Workflow` list with:

```markdown
1. Check current branch, dirty files, version, and changed surface.
2. Declare a scope lock: what is in, what is out, and what may still change.
3. Classify new ideas before accepting scope: `must fix`, `adapt`, `watch`,
   `defer`, or `reject`.
4. Set a change budget for `must fix` and narrow `adapt` items only.
5. Classify remaining requests as `watch`, `defer`, or `reject`.
6. Run release gates: validation, lint, tests, scenario runs, provider smoke,
   privacy review, and docs review as appropriate.
7. Collect dogfood evidence from supported providers or explain the missing
   evidence.
8. Make a ship decision: `ship`, `hold`, or `continue`.
```

- [ ] **Step 2: Update rules**

Add these bullets under `## Rules`:

```markdown
- Classify new ideas before accepting scope.
- `must fix` requires release-blocking evidence.
- `watch` is the default for promising but unproven ideas.
- Do not add a new skill during stabilization unless repeated failure,
  dogfood evidence, or a clear output-contract boundary proves the need.
```

- [ ] **Step 3: Run the target test and confirm partial progress**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.test_groundline_reference_contract.GroundLineReferenceContractTests.test_release_discipline_surfaces_align_on_expansion_control -v
```

Expected result: still `FAILED`, but stabilize-surface failures are gone.

## Task 4: Align Recommendation And Curation Skills

**Files:**
- Modify: `skills/recommend-groundline-upgrades/SKILL.md`
- Modify: `skills/curate-groundline-skills/SKILL.md`

- [ ] **Step 1: Update recommendation labels**

In `skills/recommend-groundline-upgrades/SKILL.md`, replace the decision bullets with:

```markdown
- `adopt`: add the pattern directly because it fits GroundLine with low risk
  and has current release evidence.
- `adapt`: borrow the pattern but keep GroundLine lighter or safer; requires a
  current-scope behavior gap and a verification path.
- `watch`: track source changes before adding behavior. `watch` is the default
  for promising but unproven ideas without failure evidence.
- `reject`: keep it out because it adds risk, context load, provider sprawl, or
  duplicates existing GroundLine behavior.
```

- [ ] **Step 2: Add curation gate**

In `skills/curate-groundline-skills/SKILL.md`, add this section before `## Rules`:

```markdown
## New Skill Gate

Create a new skill only when at least one condition is true:

- the same failure appears in repeated work or dogfood
- adding the workflow to an existing skill would make that skill too broad
- the workflow needs its own output contract, risk boundary, or lifecycle entry

Otherwise classify the idea as a source-registry item, reference note, example,
or `watch` item.
```

- [ ] **Step 3: Run the target test and confirm partial progress**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.test_groundline_reference_contract.GroundLineReferenceContractTests.test_release_discipline_surfaces_align_on_expansion_control -v
```

Expected result: still `FAILED`, but only dogfood-surface failures remain.

## Task 5: Document Dogfood Release Boundary

**Files:**
- Modify: `docs/dogfood.md`

- [ ] **Step 1: Add release discipline note**

Insert this section before `## Must Fix`:

```markdown
## Release Discipline

New skills derived from dogfood results should not enter release scope until
there is dogfood or repeated failure evidence. A promising idea without that
evidence stays in `watch` or `defer`.
```

- [ ] **Step 2: Run the target test and confirm GREEN**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.test_groundline_reference_contract.GroundLineReferenceContractTests.test_release_discipline_surfaces_align_on_expansion_control -v
```

Expected result: `OK`.

## Task 6: Full Verification

**Files:**
- Verify all changed files.

- [ ] **Step 1: Run pack validation**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
```

Expected result: JSON with `"status": "PASS"` and `"mutation_performed": false`.

- [ ] **Step 2: Run lint and actionlint**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
```

Expected result: JSON with `"status": "PASS"` and `"actionlint": "passed"`.

- [ ] **Step 3: Run full unit tests**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
```

Expected result: `OK` with all tests passing.

- [ ] **Step 4: Run provider smoke read-only**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

Expected result: JSON with `"status": "PASS"`, `"mutation_performed": false`, and `"real_home_touched": false`.

- [ ] **Step 5: Run diff whitespace check**

Run:

```bash
git diff --check
```

Expected result: exit code 0 with no output.

- [ ] **Step 6: Report verified status**

Final report must include:

```text
status: PASS|PARTIAL|FAIL
scope: release discipline layer
evidence: validation, lint/actionlint, tests, provider smoke, diff check
mutation_performed: false for scripts that report it
next_action: provider dogfood remains separate and requires approval before real home mutation
```
