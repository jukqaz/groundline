# Installation and existing-setting repair

Use this workflow for requested installation **and application**, environment
alignment, or old-setting repair. Complete authorized repair and verification
in the same task. Package-only installation authorizes no unrelated policy edit;
a review remains read-only. Core has no hooks, and hook trust is not config consent.

Choose the affected path before checking unrelated surfaces:

- **Package installation or verification:** resolve the package/native artifact,
  use `provider-smoke --require-installed`, and reuse distribution setup results.
- **Explicit Codex settings repair:** inspect relevant active configuration and
  guidance, preview only the evidenced repair, and verify affected settings with
  App-bundled and PATH CLIs using relevant scoped diagnostics such as `config/read`
  or `skills/list`, where supported. Use strict doctor only when full installation
  or runtime health diagnosis is needed.
- **Requested Insights connection or collection verification:** use actual
  Insights `worker status` and the requested live path. Do not check or enable
  Insights merely because a package or Codex setting changed.

## Establish the active surface

Resolve only the relevant host, Codex home, App executable, package, selections,
profile, and project with [platform commands](platform-commands.md).
Inspect only relevant user/profile/trusted-project layers and active
`AGENTS.override.md`/`AGENTS.md`, skills, agents, rules, and hooks. Let native Codex
resolve precedence; an inactive file proves no loaded behavior. Privately record
each finding's source, owner, reason, proposed change, and verification.

Use [configuration review](codex-configuration.md) for catalog and applicable native
checks and current official guidance for the selected model. Stale catalogs,
terminal/network warnings, or task-store failures alone prove no config defect.
Never publish raw config, catalogs, prompts, or personal files.

## Decide and repair

The reviewed distribution's `install.sh` joins checked native installation,
`setup --apply`, and strict doctor on macOS/Linux. Follow
[native upgrade](native-upgrade.md) for commit pinning, API preflight, and recovery.
GUI installation or bare `plugin add` delivers the package only.

When requested installation/application includes setup, obtain successful
`codex debug models` output before
passing its JSON to `groundline setup --catalog - --apply`. Valid partial output
from a failed catalog command is insufficient. Omit `--apply` for a write-free
preview. Default setup preserves existing/native choices; there is no model
preset. Supply `--model`, `--effort`, and `--service-tier` only as requested.

`--restore-native-context` explicitly removes root context overrides. Setup can
also remove only the four recognized retired Core hook trust records; Insights
and other trust remain intact. Unknown state is not migrated. Other semantic
settings remain unchanged, with formatting/comments retained where TOML editing
supports them.

Setup resolves `CODEX_HOME`, then the OS home; `--codex-home` selects an existing
home for testing. It exclusively creates absent config or first saves an
owner-private `config.toml.groundline-backup-<uuid>` sibling (only the basename is
reported). Unchanged repeats write nothing. Symlinks, hardlinks, malformed inputs,
unsupported profile/provider/catalog state, and unavailable model/effort fail
without replacement. Its lock serializes GroundLine writers, not other editors;
bytes are rechecked before replacement. Preserve backups on uncertain outcomes.
Native higher-priority layers still require separate verification.

## Existing settings and migration

When unsupported selections, types, or formats need review, inspect a setup
preview. Choose a scoped native diagnostic that matches the finding.

| Evidence | Action within the requested repair |
| --- | --- |
| Model/effort omitted or an intentional model, tier, delegation, permission, or experiment choice | Preserve native defaults or the explicit choice unless the request covers a change |
| Nonpositive context limit or compaction above an explicit window | Preview `config-repair`; apply that exact plan with a new private backup |
| Old positive context overrides without a remaining requirement | Establish origin and intended defaults; use `--restore-native-context` for the approved restoration |
| Unsupported model/effort in fresh active-host evidence | Fix only an established typo; otherwise obtain the missing intended choice |
| Unknown/deprecated native setting | Verify the native schema or relevant scoped diagnostic and official replacement; patch its owning layer without old aliases |
| Conflicting guidance, hooks, or command paths | Trace active ownership and equivalent behavior; revise only the obsolete rule and preserve trust/consent boundaries |
| Stale imported skill/provider cache | Review source and local changes; use its updater, never patch a cache |
| Recognized retired Core hook approval | Remove only the validated record |
| Unparseable input, unknown owner, managed policy, or unresolved layer | Preserve bytes, report missing evidence/authority, and continue independent repairs |

For other personal-file edits, use native editing tools, an exact reviewed diff,
and a new owner-only backup outside repositories. Reread before writing and
preserve intervening edits. Setup/config-repair implement only documented rules;
they are not general config upgraders or instruction rewriters.

For guidance review, check overbroad triggers, repeated approval, forced task
creation/delegation, suppressed verification, exhaustive tests for trivial edits,
and skill rules that override explicit user instructions. Revise only conflicts
with the current request/policy. Keep [model selection and adaptive delegation](model-effort-routing.md)
in that reference; installation does not override no-delegation choices or
optimize the current model/effort. Ordinary delegation inherits current settings.

## Finish and recover

Verify changed settings with applicable scoped native diagnostics. When full
installation or runtime health diagnosis is needed, use strict doctor and separate
its configuration row from other failures. For guidance, validate metadata/links;
exercise affected behavior
only when needed and authorized, using an isolated workspace or existing task.
Do not create a task without a request merely for testing; label unavailable
behavioral proof.

If this change causes failure, restore its private backup only after the current
file still matches the post-edit bytes. Merge intervening edits or obtain the
new decision. Preserve failed-write backups and state uncertainty explicitly.
Never reset unsupported state, delete sessions, weaken approvals, or enable
Insights to pass a check. Native tasks, identity, consent, cursors, and outbox
remain untouched by configuration migration.

Report affected package, repaired files, retained choices, native diagnostics,
behavior, and requested Insights outcomes separately; omit unrelated checks.
Exit 2 means review remains: resolve it and
rerun the same installer. Repeat passed checks only for changed settings,
instructions, runtime, scope, or unresolved evidence. Detailed parser, plan,
locking, and repair guarantees are in [configuration review](codex-configuration.md).
