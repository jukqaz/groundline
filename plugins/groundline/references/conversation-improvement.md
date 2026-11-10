# Improve one repeated friction

Native Codex reads meaning and performs the scoped edit. Core retains bounded
private references, target digests and contextual follow-up. It does not train a
model, infer quality from usage totals, or authenticate a caller's declarations.
The existing native AGENTS and skill layers carry useful changes into later work.

## Select a justified change

Read the relevant requirement, assistant action/tool outcome, correction and
recovery, plus a useful counterexample. Distinguish an assistant error from a
new requirement, exploration and missing context. A short “again” is not an
error label. Prefer code/tests for code defects and existing owning guidance for
a durable lesson. Do not add permanent instructions for a one-off fact.

Inspect a relevant earlier change first. If no independent follow-up or new
actionable friction exists, stop this review without a new audit or change.
Do not delay the user's actual work to fill a measurement quota.

Keep project-specific behavior in its closest project AGENTS or skill. Before
a common change, explain which decision transfers after removing project names,
APIs, device matrices and local authority assumptions, and test a different
project or counterexample. Use an actually loaded native instruction layer for
ordinary defaults. A specialist skill's installation alone is not exposure.

## Capture, edit, record

Resolve the installed binary through [platform commands](platform-commands.md).
Use the existing owner-private records directory; do not create an automation
ledger or copy this procedure into a scheduler prompt. Codex authors the small
private inputs; the owner does not fill out JSON or a rating form.

`brief.json` is one strict object with `pattern_id` (ASCII letters, digits, dash
or underscore, maximum 64), `source_work_sha256`, `context_sha256`,
`criterion_sha256`, `interpretation`, `scope`, `target_kind` and nullable
`transferability_sha256`. References are lowercase SHA-256 of actual native
source references or a locally derived review, never invented evidence. Store
no source quotations, prompts, transcripts or secrets. A digest identifies
reviewed evidence; it is not provider authentication or permission.

Interpretation is `assistant_error`, `durable_preference`, `requirement_change`,
`exploration` or `unknown`. The last three exit `NO_CHANGE` without creating
state. Scope is `project` or `common`; actionable common changes require the
transferability review's digest. Target kind is `agents_block` or `skill`.

```console
groundline improve status --state PRIVATE_RECORDS
groundline improve capture --input brief.json --target PROJECT/AGENTS.md --owner-project-root PROJECT --state PRIVATE_RECORDS
# Native Codex applies the reviewed diff and runs appropriate verification.
groundline improve record --capture CAPTURE_SHA --target PROJECT/AGENTS.md --owner-project-root PROJECT --state PRIVATE_RECORDS
```

For common scope omit `--owner-project-root`. Core binds the exact existing
file, owning root and before/after bytes, and never edits guidance or settings.
Native Codex preserves recoverable changes through the authorized task's normal
diff/backup mechanism; capture stores a digest, not a backup or a rollback plan.
Record means observed changed bytes, not independent proof of semantic correctness.
Identical retries reuse the existing record; a conflicting target or later user
edit is rejected. Unchanged content produces `NO_CHANGE`.

## Observe actual later work

Only record an independent natural work item whose applicable criterion and
actual behavior were reviewed. Local fixtures, release tests, acceptance from
another task, user silence and a successful installation are not that work.

`followup.json` contains `source_work_sha256`, `evidence_sha256`,
`target_after_sha256`, `observed_at_utc` (RFC3339), `exposure` (`observed` or
`unknown`), `opportunity` (`present`, `absent` or `unknown`) and `behavior`
(`met`, `violated` or `unknown`). Observation time must follow the change.
Absent/unknown opportunity requires unknown behavior. Check exact guidance
exposure separately from judging the action; never infer it from installation.

```console
groundline improve observe --change CHANGE_SHA --input followup.json --state PRIVATE_RECORDS
groundline improve status --state PRIVATE_RECORDS
```

Content assessments require observed exposure, a present opportunity and known
behavior. Missing model/tier/tool comparison fields do not block this qualitative
judgment. Unknowns stay visible; no observed trigger is not successful behavior.
The readout always leaves causal effect and efficiency improvement unverified.
No automatic model routing or cost experiment is part of this path.

Report the actual change, applied target, relevant verification and later met,
violated or unassessed result. A missing follow-up ends this pass and waits for
new work. Do not repeatedly scan unchanged history to fill it.

## Preserve existing state

`learning status --state PRIVATE_RECORDS` remains read-only access to existing
conversation records. Their validation and historical comparison results remain
unchanged; compact observations never retroactively qualify them. New records
share the existing store and lock, without migrating or rewriting old bytes.
Unsupported managed-environment records or layouts fail explicitly and remain
untouched. Do not reset state, edit native memory, change collection/hooks or
upload content to make a review pass.
