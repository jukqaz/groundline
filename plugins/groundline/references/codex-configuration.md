# Codex configuration review

Use the actual execution host and the App-bundled CLI resolved by
[platform commands](platform-commands.md). A global file, selected profile,
project layer, App selection, and active task can have different effective
settings. Do not merge those layers with GroundLine or infer live state from a
saved file. Preserve model, effort, service tier, permissions, and experiments
unless a requested change has a concrete benefit.

## Offline checks

Pipe the resolved native catalog directly into the bounded stdin reader:

```console
CODEX_BIN debug models | groundline config-audit --config /private/config.toml --catalog - --json
```

Replace `CODEX_BIN` with the resolved native executable. For a reproducible review,
you may instead capture it to a new owner-private JSON file outside the repository
with restrictive permissions. The refreshed command and `--bundled` are different
evidence; record which was used. Never print the full catalog or config: catalogs
can contain instructions, and config can contain credentials and private paths.

```console
groundline config-audit --config /private/config.toml --catalog /private/models.json --json
```

The command reads only the explicit config and catalog file or stdin. It runs offline,
does not start Codex, write settings, install anything, or inspect other files.
It compares model/effort and review-model selections with the supplied catalog,
and surfaces manual context limits or unresolved layers for review. It does not
maintain a model-family allowlist or reject a future model merely for its name.

- `PASS`: checked fields agree, or they are omitted to use native defaults.
- `REVIEW_REQUIRED`: a profile/provider/catalog override, unresolved effort, or
  manual context choice needs native evidence. Intentional choices can remain.
- `FAIL`: invalid input or a model/effort conflict in the supplied evidence;
  the process exits nonzero. No settings are repaired automatically.

This is not a complete config-schema validator. Unknown native keys are left to
Codex; only selected fields are inspected. Catalog age, account access, Fast
availability/pricing, custom providers, and effective session settings remain
unverified. A stale catalog can produce a stale finding: refresh the evidence,
not an invented compatibility alias. Service-tier presence is reported, never
treated as a verified tier or rewritten.

Run `CODEX_BIN --strict-config doctor --summary --no-color --ascii` separately
when validating native settings. Distinguish its configuration row from
terminal, WebSocket, desktop, and task-store failures. Do not change permissions,
remove task state, or disable safety checks to make every row green.

## Astra and context posture

Follow [model and effort guidance](model-effort-routing.md) on demand. Use the
native catalog's supported efforts instead of translating effort names or
hard-coding context sizes. Leave context window and compaction thresholds unset
unless evidence justifies an override; unset compaction uses the model default.
Do not transfer API-only request parameters into Codex TOML.

For persistent instruction changes, prefer the closest repository guidance and
load specialist references only for the matching task. Avoid restating the same
approval, delegation, or verification rule in multiple loaded skills. Keep
commands and observable acceptance criteria concrete. Reuse passing tests until
a relevant change or unresolved risk warrants more work.

Experimental context management and asynchronous question tools are native
capabilities. Verify availability on the active account/runtime before use;
leave opt-in experiments unchanged unless requested. User steering refines the
same task when its outcome and authority remain compatible.

## Worktrees and private surfaces

For App-managed local worktrees, prefer checked-in local environments. Add root
`.worktreeinclude` only for demonstrated ignored-file dependencies and requested
changes. Keep patterns narrow, verify targets are ignored, exclude broad or
tracked directories, and smoke a new worktree when feasible. This file does not
configure Remote or ordinary CLI Git worktrees.

Keep auth, sessions, logs, OAuth material, shell snapshots, caches, databases,
MCP headers, and environment values out of source and diagnostic output. Trust
records alone do not prove activation. Core has no lifecycle hooks; optional
Insights uses its own reviewed hook and explicit collection-consent contract.

Sources checked 2026-09-07:
- [Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
- [Astra prompting best practices](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices)
- [Codex subagent configuration](https://learn.chatgpt.com/docs/agent-configuration/subagents)
