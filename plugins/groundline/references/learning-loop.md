# Connect actual work to the next improvement

Use this route for requested usage-driven adaptation or environment alignment.
Native Codex acquires evidence, authors the candidate and performs the task.
GroundLine validates private records offline. Run only the affected part of the
loop; ordinary work does not need a per-task audit or review gate.

## Opt in to ongoing observations on one device

After registering the owner's managed targets, configure the private profile
with the installed, versioned Core executable. Configuration starts disabled:

```console
groundline learning configure --environment-state environment-state --target workflow --device-id owner-device
groundline learning enable
```

The owner-private profile lives at
`$CODEX_HOME/groundline/learning/profile.json`. Default records and delivery
directories are beside it; explicit `--state`, `--deliveries`, and
`--codex-home` select private device-local storage. Configuration pins the
Core executable and its SHA. Reconfigure after a native plugin upgrade, then
enable the reviewed profile. Do not edit provider cache files. Disable private
learning independently with `groundline learning disable`; this does not revoke
or grant Insights upload consent.

With Insights installed and the five hook definitions trusted through native
Codex, hooks capture bounded immutable boundary metadata and target digests.
They retain no prompt, cwd, transcript path, or tool-output text and emit no
developer context. The background worker verifies the Core pin and runs the
offline consumer; a failure or lock conflict does not block ordinary work.
Core-only users keep the explicit capture path below. Insights-only users keep
their collection behavior without a Core dependency.

For a substantive unit that the owner wants to learn from, native Codex authors
the agreed completion criterion, cohort, phase and task category before work.
Read only boundaries matched to the current owner-private native artifact:

```console
groundline learning boundaries --native-artifact native-rollout.jsonl
groundline learning task-start --input task-start.json --native-artifact native-rollout.jsonl
groundline learning finalize --input task-finalize.json --receipt receipts/delivery.json --native-artifact native-rollout.jsonl
```

When multiple native turns match, select the explicit `--turn-hash`; never take
the latest boundary from another chat. A boundary UUID is a received-record ID,
not proof of event ownership. Missing or unmatched native thread/turn evidence
remains unknown. Native Codex prepares these private inputs; do not ask the
owner to maintain JSON or fill out a rating form.

`task-start.json` has kind `groundline-learning-task-start-input`, schema `1`,
`boundary_id`, `scope` (`unit_hash`, `cohort_sha256`, `phase`, `task_category`,
`criterion` with `sha256` and `version`, `target_id`, nullable `runtime`),
`criterion_change_kind`, and nullable `criterion_change_evidence_sha256`.
`task-finalize.json` has kind `groundline-learning-task-finalize-input`, schema
`1`, the returned `task_sha256`, explicit `boundary_id`, `correction_kind`, and
nullable `correction_evidence_sha256`. Requirement or direction changes do not
become assistant errors. Completion, acceptance and verification still require
the existing direct delivery evidence. Unknown costs and native activation stay
unknown. `--manifest` can replace `--receipt` for the existing delivery builder;
it does not infer an outcome from hook events.

The consumer is deterministic and makes no model or network calls:

```console
groundline learning consume
groundline learning reconcile
groundline learning patterns
```

Consumption links submitted outcomes, selects comparable natural follow-up
evidence for applied candidates, and journals the evaluation-input digest.
Retries reuse saved results. No follow-up stays pending; mismatched cohorts,
criteria, runtime, ownership or incomplete costs do not prove improvement.
Unscoped hook boundaries remain unlinked. Readouts identify their bounded
active coverage rather than reporting it as complete historical coverage.

Known criterion changes need `--criterion-evidence`; known corrections need
`--correction-evidence`, each matching the supplied evidence SHA. Keep unknown
when direct evidence is unavailable. Consumed boundaries move to immutable
private shards with exact-ID references. Under active-state pressure, older
unreferenced finalized work moves to cold storage; recent finalized work and
the full closure of active candidate/evaluation evidence remain available.
Archive maintenance preserves raw receipt and response-ownership indexes so
old responses cannot be charged to another work unit. Pattern readouts describe
the bounded active scope, not a full historical analysis. No evidence is deleted
to make an incomplete comparison pass.

Native Codex may review new patterns in an owner-requested native automation.
Skip analysis when the evidence digest has not changed. Include analysis costs,
keep meaningful official changes scoped to their affected targets, and create
small candidates within the owner's authorized managed area. Model, effort,
permissions and native memory remain native-owned.

Before applying a learning candidate, bind an explicit trial authorization to
its exact proposal, plan, scope, authority and rollback evidence:

```console
groundline learning authorize-trial --input trial.json --evidence authority.txt --state learning-state
groundline environment apply --state-dir environment-state --proposal-id candidate --learning-state learning-state --intent trial --json
```

`trial.json` has kind `groundline-learning-trial-authorization`, schema `1`,
`proposal_sha256`, `plan_sha256`, `scope_sha256`, `rollback_sha256`,
`authority_ref` and `evidence_sha256`. Hold or reject blocks application. An
adoption requires the latest explicit adopt decision and a same-plan
`NO_REGRESSION_OBSERVED` evaluation with complete analysis costs. An
inconclusive result cannot authorize adoption. Unrelated environment repairs
and rollback keep their existing authority and recovery paths.

## Prepare the link from observed work

Before the relevant work, capture the registered target and explicit unit,
cohort and phase. Keep inputs and state owner-private outside public Git:

```console
groundline learning capture --observation capture.json --environment-state environment-state --target workflow --state learning-state
```

`capture.json` has kind `groundline-learning-capture-input`, schema `1`,
`unit_hash`, `cohort_sha256`, `phase`, and explicit `runtime` (null when unknown).
The CLI supplies the capture time and observes the local target. A supplied
runtime also requires `--native-evidence` matching its evidence SHA. Discovery
or a supplied hash does not authenticate runtime activation. A registered
baseline alone is separate from an observed applied generation.

After the agreed unit reaches its completion check, reuse its existing delivery
receipt and the returned snapshot SHA:

```console
groundline learning prepare --receipt receipts/delivery.json --snapshot SNAPSHOT_SHA --state learning-state --output link.json
groundline learning link-outcome --input link.json --receipt receipts/delivery.json --state learning-state
```

Preparation copies observed receipt identity, phase and completion time and
links only a matching capture from before completion. It does not reread today's
environment to describe older work. Correction defaults to `unknown`; explicit
acceptance, assistant error, new requirement or direction change needs evidence.
Receipt resources remain the source of owned costs; missing parent, child,
retry or analysis measurements are not filled with zero. Use
[delivery evidence](delivery-evidence.md) for the receipt contract.

## Follow the candidate through its next result

Keep the existing `learning propose`, `environment plan/apply`, and
`learning evaluate` paths for the scoped candidate and its natural follow-up.
Read its state with the actual apply/rollback operation records:

```console
groundline learning status --state learning-state --operation environment-state/operations/operation.json
groundline learning decide --input decision.json --state learning-state
```

The readout separates candidate, applied observation, evaluation and explicit
`adopt`, `hold` or `reject` decisions. A decision has a basis and evidence hashes;
subsequent decisions name the previous decision SHA. It does not apply files or
promote a baseline. A missing operation stays unobserved, a rollback is distinct,
and an inconclusive evaluation remains inconclusive after a decision.
Record repeated analysis and its owned cost even when no new candidate is saved.

## Review official changes only where they matter

Acquire the relevant official text once through native Codex web/browser tools.
Use a Markdown or text snapshot instead of HTML navigation. Then compare:

```console
groundline sources check --manifest official-manifest.json --snapshot official-snapshots.json --state-dir official-state
```

The manifest has kind `groundline-official-source-manifest`, schema `1`, and
`sources`: each has `source_id`, official HTTPS `url`, explicit `model` and
`runtime` (nullable), and `affected_target_ids`. The snapshot has kind
`groundline-official-source-snapshots`, schema `1`, and `snapshots`: each has
`source_id`, matching `url`, `observed_at_utc` and UTF-8 `content`.

The first observation creates a baseline. Unchanged content skips reanalysis;
a newer observation still advances freshness metadata. Repeating identical
evidence is a no-op. Older snapshots and unavailable content preserve the last
valid state. Changed content lists only mapped targets for review; it is not a
recommendation, account availability, or automatic guidance change. Snapshot
origin remains explicit input, not authenticated by the digest. Core creates no
network client, background watcher, model call or schedule.

## Carry the common intent to another device

Export the registered desired content, explicitly supplying a proposal only
when the desired bytes are not yet available locally:

```console
groundline environment export --state-dir environment-state --output common-bundle.json --json
```

Carry the private bundle and its returned SHA through an owner-selected private
Git checkout or private shared directory. Keep credentials, native memory,
sessions, caches, local roots, device bindings and exceptions off that transport.
The bundle contains managed guidance text and must not enter public source.
On the receiving device, provide its own private bindings and authority:

```console
groundline environment inspect-bundle --bundle common-bundle.json --bundle-sha256 BUNDLE_SHA --bindings device-bindings.json --json
groundline environment import --state-dir environment-state --bundle common-bundle.json --bundle-sha256 BUNDLE_SHA --bindings device-bindings.json --authority-ref AUTHORITY --new-device --json
groundline environment plan-bundle --state-dir environment-state --bundle common-bundle.json --bundle-sha256 BUNDLE_SHA --proposal-id common-update --json
groundline environment apply --state-dir environment-state --proposal-id common-update --json
groundline environment status --state-dir environment-state --learning-state learning-state --json
```

For an existing device, replace `--new-device` with its explicit
`--expected-revision` and `--expected-exception-revision`. Divergent lineage and
stale revisions fail without merging or overwriting. Import registers a baseline;
only apply writes managed targets. Existing backups, recovery and rollback keep
later user edits. Verify App/PATH loading separately on hosts where Codex exists.
The common baseline has one `common_change_ref`; each device retains its own
plan, operation and evaluation refs. Status validates historical saved bindings
even after a newer baseline is registered. An older plan still fails current
CAS checks when applied. Neither a shared bundle nor another device's successful
operation proves this device's native activation or quality improvement.
