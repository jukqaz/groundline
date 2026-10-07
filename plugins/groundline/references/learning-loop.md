# Connect actual work to the next improvement

Use this route for requested usage-driven adaptation or environment alignment.
Native Codex acquires evidence, authors the candidate and performs the task.
GroundLine validates private records offline. Run only the affected part of the
loop; ordinary work does not need a per-task audit or review gate.

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
```

For an existing device, replace `--new-device` with its explicit
`--expected-revision` and `--expected-exception-revision`. Divergent lineage and
stale revisions fail without merging or overwriting. Import registers a baseline;
only apply writes managed targets. Existing backups, recovery and rollback keep
later user edits. Verify App/PATH loading separately on hosts where Codex exists.
