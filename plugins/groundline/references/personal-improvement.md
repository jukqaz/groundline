# Personal improvement contract

`groundline personal review|evaluate|rollback` is offline. Insights wire schemas
and consent are unchanged. Native Codex fetches reports/docs, classifies direct
outcomes, and performs authorized orchestration. Core never routes models,
changes native settings, grants permissions, or starts background work.

## Read-only review

```console
groundline personal review --report insights-7.json --audit weekly.json --model-evidence model-evidence.json --catalog native-models.json --json
```

The report must pass the exact schema-3 Insights contract; the audit must pass
the weekly native audit contract. Both must be generated within 24 hours.
Partial history remains visible but cannot authorize a trial. Server root
observations sum collection windows and are not unique completed tasks. Mixed
versions, missing usage, and historical gaps must not be hidden. Model/effort
context counts cannot attribute tokens or establish model efficiency.

When stored history cannot satisfy the strict current report contract, the API
returns HTTP 422 report_contract_rejected. This is separate from a storage
outage. Preserve that history and report the unavailable window explicitly;
never fabricate a valid report, erase records, or relabel unsupported runtimes.

Model evidence has the following strict shape. Replace example values with
actual evidence; example dates and hashes are deliberately not ready to apply.

```json
{
  "kind": "groundline-model-evidence",
  "schema": 1,
  "checked_at_utc": "2026-09-08T00:00:00Z",
  "runtime_version": "0.153.4",
  "runtime_family": "codex_app",
  "selected_model": "gpt-6-astra",
  "selected_effort": "xhigh",
  "latest_reference_model": "gpt-6-astra",
  "catalog_sha256": "REPLACE_WITH_SHA256_OF_EXACT_NATIVE_CATALOG_BYTES",
  "official_sources": [{
    "applies_to_model": "gpt-6-astra",
    "url": "https://developers.openai.com/api/docs/guides/latest-model",
    "sha256": "REPLACE_WITH_SHA256_OF_FETCHED_DOCUMENT",
    "checked_at_utc": "2026-09-08T00:00:00Z"
  }],
  "behavior_focus": ["approval_continuity", "diagnose_before_retry"]
}
```

Model evidence expires after 24 hours, official sources after seven days. Only
HTTPS developers.openai.com, platform.openai.com, and learn.chatgpt.com URLs
without credentials, queries, or custom ports are accepted. Sources must cover
the selected model as well as the latest reference model. Future model IDs and
efforts are accepted when present in the supplied native catalog. Empty
behavior_focus selects no candidate. No model/effort setting is rewritten.

The CLI checks contracts, hashes, timestamps, and catalog availability. It does
not independently certify document retrieval or semantic freshness. Its output
therefore says evidence_origin=operator_supplied and
latest_model_independently_verified=false. Codex must actually fetch and review
the official pages; changing timestamps is not a refresh. Catalogs may contain
private instructions and must never be printed or published.

The review returns model_context_sha256, which identifies the selected model,
effort, runtime family, and client version. Refreshed timestamps and unrelated
catalog entries do not change that cohort.

## Private direct outcomes

Add --outcomes only when directly observed work supports the following input:

```json
{
  "kind": "groundline-outcome-sample",
  "schema": 1,
  "period_start_utc": "2026-09-01T00:00:00Z",
  "period_end_utc": "2026-09-07T00:00:00Z",
  "model_context_sha256": "REPLACE_WITH_REVIEW_CONTEXT_SHA256",
  "guidance_sha256": "REPLACE_WITH_EXACT_GENERATED_GUIDANCE_SHA256",
  "comparison_context_sha256": "REPLACE_WITH_STABLE_NONTRIAL_SETTINGS_FINGERPRINT",
  "task_kind": "implementation",
  "scope_size": "medium",
  "activation_verified": false,
  "units": [{
    "unit_hash": "REPLACE_WITH_OWNER_LOCAL_HASH_OF_STABLE_DELIVERY_IDENTITY",
    "started_at_utc": "2026-09-02T00:00:00Z",
    "completed_at_utc": "2026-09-02T00:05:00Z",
    "outcome": "verified",
    "evidence": "runtime_check",
    "rework": false,
    "redundant_approval_count": 0,
    "continuation_prompt_count": 0,
    "repeated_call_count": 0,
    "tool_call_count": 5,
    "total_tokens": null
  }]
}
```

A unit is a stable bounded requested delivery, not a collection event or arbitrary
tool success. Associate runtime checks with its acceptance criteria, or use
explicit user acceptance. Count every eligible delivery, including failed and
unknown outcomes. Never cherry-pick easy/successful work. Reuse the same local
identity hash across observations; hashes remain private and are never emitted
in summaries or uploaded to Insights.

A sample contains one task_kind (implementation, review, deployment,
documentation) and one scope_size (small, medium, large), at most 1,000 unique
units, and a period no longer than 90 days. Unit timestamps must lie inside the
period. Preserve cohort classifications across comparisons. comparison_context_sha256
fingerprints the unchanged non-trial instruction surfaces, tool availability,
permissions, and service tier. Build it from a stable private canonical manifest;
exclude the generated trial guidance. If these inputs are unavailable or mixed,
do not supply an eligible sample. Disclose excluded
mixed-model work instead of attributing it to one model.

outcome is verified, failed, or unknown; evidence is runtime_check,
user_acceptance, or unobserved. Unknown requires unobserved evidence. An assistant
saying done is not evidence. Count redundant approvals only when scope and
existing authority were unchanged; count continuation prompts only when needed
to recover unnecessarily stopped work. New requirements and normal steering do
not count. Legitimate polling or changed-condition verification are not repeated
calls. total_tokens is nullable and must be exact usage attributed once to that
unit. Never assign inherited totals to a child. Wall duration includes waiting
and is not model latency. Raw text, paths, commands, and unknown fields fail.

## Trial and recovery

Only approval_continuity and diagnose_before_retry are initially supported.
Instructions come from reviewed source, never remote text. Short/broad messages
and high effort alone do not select a rule. A trial requires fresh evidence,
PASS report quality/native audit, ten directly evidenced outcomes, and a
matching intervention/retry problem inside those outcomes. Aggregated repetition
alone cannot authorize application. Existing baseline guidance must have verified
activation. Ten is a
minimum gate, not statistical significance. Missing evidence yields OBSERVE and
never writes guidance, even with --apply.

Create an owner-private state directory outside Git (0700 on Unix, equivalent
owner-only ACL on Windows), then use the user's authorized dedicated scope:

```console
groundline personal review --report insights-7.json --audit weekly.json --model-evidence model-evidence.json --catalog native-models.json --outcomes before.json --state-dir /owner-private/personal --apply --json
```

Only generated personal-guidance.md, a lock, trial.json, and content-addressed
trial/evaluation records are written there. No AGENTS.md or config.toml changes.
Initial guidance is empty, so its initial fingerprint is SHA256 of empty bytes.
Files are bounded/private, links are rejected, mutations are serialized, and a
prepared journal precedes instruction writes. Preserve user edits. A repeated
candidate cannot reuse baseline units from any prior trial, including archived
trials separated by another candidate. Archived trials must pass the same
contract and match their content-addressed names; invalid history is preserved
and blocks application. The directory has a 128-entry bound. New writes must fit
before a trial or evaluation changes guidance, so reaching the bound does not
prevent rollback. Retention is an explicit owner operation, never automatic
deletion.

Native activation is separate: connect the file through the authorized native
instruction surface and verify loading plus behavior before setting
activation_verified=true. File creation or installation does not prove it.

```console
groundline personal evaluate --state-dir /owner-private/personal --outcomes after.json --model-evidence model-evidence.json --catalog native-models.json --json
groundline personal rollback --state-dir /owner-private/personal --json
```

Evaluation requires ten directly evidenced units per disjoint period, no reused
identities, the same model/effort/runtime/client/task kind/scope size and non-trial settings, and exact
activated trial guidance. Changed cohorts, missing activation, or insufficient
samples are INCONCLUSIVE. Retain only an observed primary-metric benefit without
regression in verified outcomes, rework, user intervention, wall duration, or
known token use. Otherwise restore prior generated guidance. These are
conservative observational gates, not a causal or statistical confidence claim.
Unknown token usage is never a savings estimate. User edits prevent replacement
and rollback. Interrupted prepared trials can be rolled back before or after the
guidance write. Mutation-path errors report an unknown mutation state; inspect
the journal instead of assuming nothing changed.

## Native recurring operation

For a user-requested weekly automation, use the explicit skill and native Codex
scheduler. Refresh actual evidence, detect model/client changes, and stay quiet
on unchanged results. Apply only an already-authorized personal scope. Do not
create extra tasks, paid API evals, Chronicle experiments, or per-hook model
calls. If the installed personal command is missing, report install drift;
never patch a cache or treat an unpublished source binary as a release.

Sources reviewed 2026-09-08:
- [Current model guidance](https://developers.openai.com/api/docs/guides/latest-model)
- [Native model guidance](https://learn.chatgpt.com/docs/models)
- [Evaluation design](https://developers.openai.com/api/docs/guides/evaluation-best-practices)
