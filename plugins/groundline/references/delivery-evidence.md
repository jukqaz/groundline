# Record a completed delivery and its actual selection

Use this route after an authorized adaptive-selection lane reaches its agreed
acceptance check, including failed or unresolved work. A delivery is the agreed
work unit, not a message, turn, collector window, or arbitrary time slice.
Native Codex observes and classifies the evidence; Core checks and records it
offline. No account switch, new telemetry upload, scheduler, or global model
setting is involved.

## Observe once

Carry the acceptance criteria and current phase into the lane handoff. Phases are
`implementation`, `runtime_verification`, `visual_acceptance`, `deployment`,
`research`, `review`, and `documentation`. For visual or subjective work, check a
representative result against the agreed quality criteria before scaling out.
Pass forward decisions, evidence locations, completed checks and remaining work;
do not replay the full parent conversation or repeat completed tests to fill a
comparison quota.

After completion, distinguish the proposed pair, the requested pair, and the
observed effective pair. Use the actual runtime's selection evidence; a spawn
argument, catalog entry, or stored default cannot establish effective execution.
If unavailable, set `effective` to null. A mismatch is retained and attributed
to the observed pair. The receipt does not authenticate a provider or prove
backend activation: `activation_verified` remains false.

## Write one private receipt

Prepare a private manifest and bounded local evidence artifacts, then run:

```sh
groundline efficiency record-delivery --input manifest.json --output receipts/delivery.json --json
```

The selected output directory must already exist. Output is a new owner-private
file (0600 on Unix); existing files and links are not replaced. The command reads
at most 2 MiB per input/artifact, checks SHA-256 against the exact bytes, checks
the declared observation against the artifact, and removes artifact paths from
the saved receipt. Keep both the manifest and evidence artifacts in a private
workspace outside public Git. Never include transcripts, credentials or account
identifiers. Do not reconstruct account membership from a shared Mac's history.

The strict manifest fields are:

| Field | Value |
| --- | --- |
| `kind`, `schema` | `groundline-delivery-manifest`, `1` |
| `unit_hash` | SHA-256 identifying one agreed delivery, shared with its root resource entries |
| `cohort_sha256` | Same phase, acceptance criteria, difficulty, runtime, tools, permissions, service tier, delegation policy and non-routing guidance; exclude the pair being compared |
| `phase` | One of the phases above |
| `completed_at_utc` | Observed RFC3339 completion/assessment time |
| `recommendation`, `requested`, `effective` | Null or `{model, effort, evidence_sha256, artifact_path}`; exact GPT-6 models only |
| `verification` | `{status, evidence_kind, evidence_sha256, artifact_path, rework}` |
| `resources` | `{complete, wall_duration_ms, entries}` |

Artifact paths are absolute or relative to the manifest. The recommendation
artifact is the actual schema-1 `groundline-routing-proposal` with a matching
`suggestion.model` and `suggestion.effort`. Do not turn a null suggestion into a
recommendation. The other artifacts are compact operator observations:

```json
{"kind":"groundline-selection-request","schema":1,"model":"gpt-6-sol","effort":"high","source":"operator_supplied"}
```

The effective observation uses the same fields with
`kind: "groundline-native-selection-observation"`. The verification artifact
uses `kind: "groundline-delivery-verification-observation"`, `schema: 1`,
`status`, `evidence_kind`, and `source: "operator_supplied"`. These normalized
records must describe evidence actually inspected for this delivery. Hash checks
bind local bytes; they cannot certify semantic classification or provenance.

Verification status is `verified`, `failed`, or `unknown`; evidence kind is
`runtime_check`, `user_acceptance`, or `unobserved`. Unknown pairs with
unobserved. Mark `rework` only for a repair of the same agreed outcome, not a new
request or a necessary verification step. Missing resource measurements do not
erase an observed success or failure.

Each resource entry has `owner` (`root`, `child`, `approval`, or `retry`),
`unit_hash`, `response_hash`, and nullable integer `input_tokens`,
`cached_input_tokens`, `output_tokens`, `reasoning_output_tokens`, and
`total_tokens`. Use stable response identifiers hashed locally; count every
owned response exactly once, including failed attempts and delegated work.
Use response increments, not repeated cumulative counters. Cached input and
reasoning output are subsets, not additional tokens. Known input plus output
must equal total. Missing values stay null. `complete: true` requires all
components, a root entry, and the delivery's wall duration; do not add parallel
child wall times. Complete coverage is an operator observation, not something
the CLI can prove from the rows alone. Do not convert these counters into
subscription quota or money.

The saved receipt adds `observed_selection_matches_requested` (true/false/null),
`activation_verified: false`, and verification `authenticity_verified: false`.
The CLI response is a compact write result; the receipt contains the detailed
path-free observations. A failed write can leave an incomplete output if cleanup
also fails; inspect that file instead of treating failure as proof of no change.

## Feed the next eligible comparison

Use a dedicated directory of receipts with an empty `outcomes` array and the
current `task.phase` in the ordinary routing packet:

```sh
groundline efficiency route --input routing.json --catalog native-models.json --audit weekly.json --report insights-7.json --deliveries receipts --json
```

The reader is nonrecursive, limited to 1,000 directory entries and 16 MiB of JSON.
It rejects duplicate delivery IDs and overlapping response ownership, including
across receipts. Manual aggregate outcomes cannot be mixed with receipts because
their overlap cannot be established. Only the same cohort in the preceding 30
days enters comparison; other cohorts and older receipts remain on disk and are
counted separately. A matched cohort with a different phase is rejected. Keep
each phase in its appropriate cohort. For model/effort-only comparison, match
observed child model/effort, roles and work allocation as well as delegation
policy. Owner/resource rows do not establish that composition. If it is unknown
or changed, retain the receipt outside that claimed comparable cohort; do not
attribute the difference to the root pair. Cohort classification is operator-
supplied private evidence, not a property authenticated by the CLI.

Missing effective selection in a matched delivery blocks an empirical
replacement; no task heuristic bypasses this gap. Unknown verification or incomplete
resources stays in the comparison and prevents eligibility. Explicit selections
remain pinned. Receipt hashes were checked when recorded; routing validates the
receipt but does not reread private evidence artifacts or authenticate its
author. A proposal still needs current catalog evidence and the existing protected-
outcome and resource gates. Audit/report inputs are optional aggregate context;
their absence does not weaken complete matched direct outcomes. This is an observational
comparison, not a causal improvement claim.
