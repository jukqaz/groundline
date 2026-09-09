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

## Applying a bounded repair

For installation/application requests, follow
[installation alignment](installation-alignment.md), including active guidance
review and the declared `setup` defaults. `setup --catalog <native-models.json>
--apply` resolves the current home and applies the installation policy; omitting
`--apply` previews it. The separate `config-repair` command always emits JSON and previews
without writes by default:

```console
groundline config-repair --config /private/config.toml --catalog /private/models.json
```

It removes nonpositive root context limits. If an explicit compaction limit
exceeds the explicit context window, it removes both conflicting overrides so
Codex can resolve defaults. Positive overrides otherwise stay for review. Add
`--restore-native-context` only when restoring native context defaults is the
reviewed intent. No model, effort, service tier, permissions, or other keys are
changed. Comments and bytes outside removed assignment lines are preserved.

Use the returned `plan_sha256` for the exact target, config bytes, catalog bytes,
repair policy, and options. Apply with a new backup path in an existing private
location outside the repository:

```console
groundline config-repair --config /private/config.toml --catalog /private/models.json --apply --expect-plan PLAN_SHA256 --backup /private/backups/config-before.toml
```

Repeat any preview options on apply. A hash is an input-consistency check, not
authorization; the user's request provides authority. Existing scoped approval
does not require another confirmation between preview and apply.

`READY` means a candidate exists; it is not a runtime pass. Malformed TOML,
unresolved profile/provider/catalog overrides, remaining catalog errors,
symlinked inputs, and changed plans prevent application. Resolve the evidence
or use a reviewed native patch rather than force a rewrite. A completed write
returns the candidate audit status, `configuration_changed`, `backup_written`,
and `file_verified`; any remaining review findings remain visible.

The command uses a private backup, a sibling advisory lock, a fresh byte check,
and atomic replacement with owner-only permissions. It never replaces an existing
backup. The lock serializes GroundLine repairs; Codex and other editors do not
share it. Pause other writers during application. A narrow external-writer race
remains; `external_writers_locked` is false. A failed final sync can leave an
uncertain write result, reported as `configuration_changed: null`. Inspect the
file and backup before recovery. A no-change rerun writes no backup or lock.

Only supplied files are checked. Effective layer resolution, full native schema,
and fresh-task behavior still require native verification. No state is reset,
no retired format is migrated, and no package or hook is installed by this command.

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
