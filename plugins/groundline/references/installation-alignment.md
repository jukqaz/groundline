# Installation and existing-setting repair

Use this workflow when the user asks to install **and apply** GroundLine, align
the existing environment, or fix old settings. Carry the authorized work through
repair and verification in the same task. A request only to install the package
does not authorize unrelated personal-policy edits. A review stays read-only.

## Establish the active surface

Resolve the actual host, Codex home, App executable, installed Core package,
selected model/effort, profile, and target project. Use
[platform commands](platform-commands.md) and native help. Upgrade through
[native upgrade](native-upgrade.md) when requested; never edit installed caches.

Inspect relevant user, selected-profile, and trusted project config layers.
Let Codex resolve precedence. Inspect active `AGENTS.override.md`/`AGENTS.md`,
applicable skills, custom agents, rules, and hooks only where they affect this
request. Reading an inactive base file is not evidence about loaded guidance.
Record source, owner, reason, proposed change, and verification privately for
each finding. Do not upload raw config, catalogs, prompts, or personal files.

Use [configuration review](codex-configuration.md) for bounded catalog checks
and native strict doctor. Fetch the selected model's current official guidance;
do not substitute a generic model preset. A stale catalog, terminal warning,
network failure, or task-store warning does not prove a configuration defect.

## Decide and repair

For the published install-and-apply flow, run the distribution's `install.sh`
(macOS/Linux) or `install.ps1` (Windows). They verify the native artifact, use
Codex's marketplace installer, run `setup --apply`, and finish with strict doctor.
They require no additional chat request. Native GUI or bare `plugin add` still
performs package delivery only; no post-install execution callback is assumed.

Only when the user also requests the declared configuration setup, use `setup`
for an already installed package. A guidance review or general alignment request
does not select the preset. If the user specifies a different model, effort, or
service tier, preserve it and use bounded repairs instead; `setup` has no
alternative-preset flag. Resolve the package executable and the active Codex
executable with [platform commands](platform-commands.md). Obtain successful
`codex debug models` output before invoking `groundline setup --catalog - --apply`
with that JSON on stdin. Do not apply after a failed catalog command, even if it
produced valid partial JSON. Omit `--apply` for a write-free preview.

`setup` reads the baseline embedded from `config/setup-defaults.toml`:
`gpt-6-astra`, `xhigh`, and `service_tier="default"` (Fast off). This is the
explicit installation policy, not a claim that OpenAI recommends a universal
preset. Restore Codex-owned context sizing by removing the two root context
overrides, and remove only the four recognized retired Core hook trust records.
Unknown state is never migrated. All other semantic settings remain unchanged;
comments and formatting are retained where the TOML editor supports them.

The home is resolved from `CODEX_HOME`, then the OS user home; `--codex-home`
selects a specific existing home for testing. An absent config is created
exclusively. An existing config gets a new owner-private sibling backup named
`config.toml.groundline-backup-<uuid>` before replacement; the JSON report gives
only this basename. Repeated unchanged application writes nothing. Symlinked,
hardlinked (Unix), malformed, unsupported profile/provider/catalog, or unavailable
model/effort inputs fail without replacing the config. GroundLine serializes its
own writers and rechecks bytes before replacement; unrelated editors do not share
its lock. Keep backups on uncertain write outcomes. Inspect higher-priority
project/profile/system settings with native Codex before claiming runtime proof.

| Evidence | Action within an installation-and-repair request |
| --- | --- |
| Nonpositive context limit, or compaction limit above an explicit window | Preview `config-repair`, then apply the same plan with a new private backup |
| Positive context overrides copied from an earlier setup without a remaining requirement | Establish their origin and intended native defaults; use `--restore-native-context` only for that approved choice |
| Unsupported model/effort in refreshed active-host evidence | Preserve any explicit choice; fix a typo only when the intended supported value is established; otherwise obtain that missing choice |
| Unknown/deprecated native setting | Verify native schema/doctor and official replacement; patch the specific owning layer with a backup, without maintaining old aliases |
| Duplicate or conflicting personal guidance | Trace the active instruction chain; revise the obsolete rule in its owning file, preserving the user's current policy |
| Stale imported skill or provider cache | Review source and local changes; use the owning updater, not direct cache patches |
| Duplicate hooks or obsolete command paths | Establish ownership and equivalent behavior; repair only the conflicting definition; leave trust and collection consent to their native workflow |
| Intentional model, effort, service tier, delegation, permissions, experiment | Preserve unless a concrete requested change covers it |
| Unparseable input, unknown ownership, managed policy, or unresolved layer | Preserve bytes and report the specific missing evidence/authority; continue independent repairs |

Back up each changed personal file outside repositories with owner-only access.
Never overwrite a previous backup. Prepare the smallest patch, reread the file
before writing, and preserve intervening user edits. For ordinary config/guidance
patches, use the native editing tools after inspecting the exact diff. The Rust
setup and repair commands handle only their documented rules; neither is a
general config upgrader or a natural-language instruction rewriter.

For Astra guidance review, look for unnecessary repeated approval, forced task
creation, unconditional delegation, mandatory exhaustive testing for every edit,
and skill text that silently overrides a user's explicit instruction. Revise
only a rule that actually conflicts with the current request or user policy.
The official delegation example is tunable guidance, not permission to spawn
agents or overwrite a user's no-delegation default. Avoid copying a long Astra
prompt into every skill. Keep one concise owning rule and link from references.

## Finish and recover

After changing config, run native strict doctor once and interpret the config
row separately from environment/network rows. After changing guidance, validate
metadata/links and exercise the affected behavior in an authorized isolated
workspace or fresh task. Do not create another task solely for validation
without the user's request. Report behavioral proof unavailable when necessary.

If verification implicates this change, compare the current file with the exact
post-edit bytes before restoring its private backup. Never overwrite subsequent
user edits; merge or ask for the new decision. Preserve failed-write backups and
report an uncertain write outcome explicitly. Do not reset unsupported state,
delete sessions, weaken approvals, or enable Insights to make a check pass.

Report installation, files repaired, intentional choices retained, and native
runtime/behavior evidence separately. Record an unresolved finding rather than
claiming a clean setup. An unchanged repeat application should write nothing;
rerun only after changed settings, instructions, runtime, or requested scope.

Core has no lifecycle hooks. The explicit installer or install/apply request
starts this workflow under the user's existing authority. Hook trust is never
used as configuration consent.
