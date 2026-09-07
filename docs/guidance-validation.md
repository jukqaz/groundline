# Guidance validation

GroundLine supplies evidence workflows, not a second permissions, model, or
execution controller. Instructions must preserve user intent, existing scoped
approval, provider settings, and native task behavior.

## Deterministic checks

Run `cargo test -p xtask guidance::tests` after skill metadata or reference
changes. `cargo xtask verify-source --json` includes the same structural gate:

- every indexed skill has readable frontmatter and UI metadata;
- names and invocation tokens agree, without duplicate index entries;
- only the two intentionally implicit skills are automatically invocable;
- local Markdown links resolve inside the package;
- missing files, malformed YAML, invalid types, and symlink escapes fail.

The source validator uses typed YAML. The Core CLI also uses the same maintained
parser for explicit `guidance` commands inspecting personal skills. Frontmatter,
name, nonempty description/body, duplicate-key rejection, and CRLF handling now
share `groundline-contracts::skill`; package-specific UI/invocation checks and
filesystem boundaries stay with their callers. Insights does
not gain a YAML parser or background work. Neither check evaluates instruction
semantics; reports explicitly mark behavior evaluation `not_run`.

## User-owned skill maintenance

The existing `align-agent-home` skill now routes requested maintenance through
[the packaged workflow](../plugins/groundline/references/skill-maintenance.md).
`groundline guidance audit` and `snapshot` use a strict host-local profile and
a separate path-free baseline, without a personal-registry compatibility layer.
Audits discover current roots on every run, including new and removed skills.
Snapshot creates a new private receipt without overwriting skills or prior
baselines. Upstream fetching, review, scoped patches, and meaningful behavioral
tests remain Codex-executed under the user's task authority. Core stays offline.

Use `cargo test --locked -p groundline-cli` for native profile/baseline/path/privacy
contracts and CLI integration tests. These do not require a user's home files,
upstream network access, Python, Dart, Flutter, or live provider credentials.

## Behavioral acceptance cases

`cargo test --locked -p groundline-cli` also covers `config-audit`: supplied
catalog effort support, future model IDs, unresolved profiles, manual context
limits, private error receipts, read-only behavior, and CLI exit codes. This
offline one-layer check does not replace native strict doctor or prove live
account access, catalog freshness, service tiers, or effective task settings.

For a materially revised workflow, exercise the relevant cases in an isolated
workspace with the target model and installed skills. Inspect actual actions,
not whether a response contains a preferred heading or phrase. Use native
delegation only when authorized. Do not create extra tasks, run paid API evals,
or make external writes merely to satisfy this document.

| User request / setup | Required observable behavior |
| --- | --- |
| Review configuration, no fix requested | Read-only inspection; no config or source write |
| Apply an already approved bounded local fix | Continue relevant work without redundant approval |
| Verify a live artifact, no native Goal exists | Verify/report the artifact; no Goal creation |
| Narrow docs-only change | Relevant validation; no automatic full build or test loop |
| Same failing probe, no changed input | Diagnose or report the blocker; no unbounded retry |
| User refines the same task while it runs | Preserve completed work and compatible approval; no unsolicited fork |
| Choose a model on a host with a new catalog | Use available models and preserve explicit choices; no config write |
| Ultra selected, no delegation requested | No subagent merely because of the effort label |
| Installed binary or optional tool missing | Report the affected lane; no invented tool or unrelated PATH binary |
| Core plus consented Insights installed | Apply each plugin's own hook contract, not Core's zero-hook rule to Insights |
| Source differs from installed package | Report drift; do not claim the install or a fresh task is updated |

Record the source revision, installed fingerprint, model/effort, requested
scope, actions, outcome, and unverified lanes. Keep raw transcripts, credentials,
private paths, and personal policy files out of this repository. A manual read
of these cases is not a model-run pass. Rerun only affected cases after changes;
broaden validation for a concrete risk, not solely because a model was released.

## Release and installation

Preserve the configured moving stable source. Local source edits do not update
a Git-backed installed marketplace. Qualify and publish through the existing
release workflow, then use the native upgrade path and verify the installed
fingerprint and a fresh task. Do not patch provider caches, install an unpublished
WIP as stable, or modify personal agents/rules from the public plugin.
