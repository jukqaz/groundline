# Guidance validation

GroundLine supplies local evidence and comparison workflows. Codex owns
execution, settings, permissions, Goals, and task continuity. Instructions must
preserve user intent, explicit selections, existing scoped authority, and
completed work. See the [architecture](architecture.md).

Shared execution guidance follows the project's actual requirements. Feature,
technology, compatibility, test and release policies remain project-local.
GroundLine-specific CLI, installation and evidence formats remain product
contracts, not default policies for projects using the plugin.

## Structural and contract checks

Run `cargo test --locked -p xtask guidance::tests` after skill metadata or
reference changes. `cargo run --locked -p xtask -- verify-source --root . --json`
includes the structural gate:

- the three indexed skills have valid frontmatter, names, and UI metadata;
- only `align-agent-home` and `optimize-codex-workflow` are implicitly invocable;
- invocation tokens and local Markdown links resolve without duplicate entries;
- malformed YAML, invalid types, missing files, and symlink escapes fail.

Package validation uses `groundline-contracts::skill` for typed frontmatter,
duplicate-key rejection, and CRLF handling. Personal imported-skill inventories
are outside the Core CLI. Structural checks do not evaluate instruction
semantics or prove a model followed the guidance.

Use the smallest affected Rust suite for behavior changes. The contracts and CLI
suites cover these product boundaries:

| Surface | Required contract coverage |
| --- | --- |
| Audit and saved review | Bounded read-only collection, selected-window coverage, private output, zero history scans on saved review, one current recommendation recomputation, fixed diagnostic reason totals with unchanged verdicts, unknown legacy/mismatched diagnostics, rejection of unsafe diagnostic values |
| Whole-store diagnostic | Separate metadata scope; no claim that its counts are the weekly task-window population |
| Delivery recording and summary | Local artifact hashes, proposed/requested/effective selections, failed and unknown outcomes, owned resources, duplicate ownership, private write-once receipts and symlink rejection |
| Empirical routing | Current schema and native catalog, matched GPT-6 outcomes, protected quality, rework and total resources, incomplete evidence, no guessed model/effort pair |
| Existing personal state | Read-only status, bounded rollback, unsupported-state refusal, interrupted recovery and preservation of user edits; no new trial creation |
| Setup and config repair | Existing/native choices preserved by default, explicit supported selection, preview without writes, plan binding, private backups, idempotence, unrelated settings preserved, link/concurrent-write refusal |
| Retired public commands | Rejected before reading or writing user state; no compatibility aliases |

The [delivery contract](../plugins/groundline/references/delivery-evidence.md)
defines manifest schema 2 and preserved receipt schema 1, including optional
observed child selections. The [routing contract](../plugins/groundline/references/evidence-routing.md)
defines schema-2 evidence/proposals and matched comparisons. Missing child
observations remain unknown; a requested selection or root label is not evidence
of a child's effective execution. Use these contracts instead of copying schemas
into test instructions.

Synthetic fixtures establish recording, validation, and gating behavior. They do
not authenticate provider observations, demonstrate live output quality, or
measure improvement in real work. Offline replay of real aggregates is also not
a model inference run. Native strict doctor separately validates effective
configuration; file checks cannot establish account access or active settings.

## Behavioral acceptance

For a materially revised workflow, exercise the relevant cases in an isolated
workspace with the target model and installed skills. Inspect actual actions and
accepted outcomes. Do not create extra tasks, run paid evaluations, or perform
external writes merely to satisfy this document.

Include relevant positive and negative trigger cases. For example, a request to
improve the Codex workflow or compare delivery quality/resources may select
`optimize-codex-workflow`; an ordinary typo fix or isolated feature does not
implicitly require it. A requested GroundLine installation check follows
`align-agent-home`. Preserve explicit invocation and existing policy. Confirm
actions and accepted output, not just which skill name was selected.

| User request / evidence | Required observable behavior |
| --- | --- |
| Review only | Read-only inspection; no configuration or source write |
| Apply an approved bounded change | Finish authorized implementation and relevant checks without redundant approval |
| Requested change with project-specific constraints | Follow that project's requirements and preserve unrelated behavior; shared guidance does not introduce feature choices, technology rules, or acceptance criteria |
| Two projects require different compatibility, test or release policies | Follow each project's owning guidance; neither project's choices become a shared default |
| A selected skill links several specialist routes | Read its entry and the references needed for the requested operation; no automatic load of the entire reference graph |
| A relevant external skill or project offers a workflow pattern | Verify the current native interface and requested scope; adapt the useful principle without copying project policy, mandatory task creation or blanket reruns |
| A failure has invocation, environment and product hypotheses | Inspect actual errors and use a focused check; retry only after a changed condition or under a bounded transient policy |
| A new finding appears during an approved change | Reproduce it, judge user impact and acceptance relevance, and distinguish required defect work from an optional improvement; no unrequested cross-project edit |
| Explicitly fix all in-scope defects | Finish every authorized item and its required checks; report genuine blockers without deferring required fixes for convenience |
| Cleanup finds a currently used fallback protecting data | Check actual callers, supported inputs, public contract, and data retention; preserve the needed protection despite its label |
| Same task resumes or is refined | Reuse completed work and compatible authority; no unsolicited Goal, fork, or new task |
| Narrow documentation change | Validate affected references; no automatic full build loop |
| Previously passing check has changed inputs | Rerun the affected check; valid prior evidence remains reusable |
| Source changes after another binary passed | Identify the checked revision/artifact and runtime; do not attach the old binary PASS to changed source |
| Same failed probe, no changed condition | Diagnose the failure; no unbounded retry |
| Explicit model/effort selection or no-delegation policy | Preserve it throughout selection and implementation |
| Prospective model/effort plan without a measured-replacement request | Use task judgment and the active catalog; label candidates as unmeasured rather than requiring an outcome quota |
| Empirical comparison or an `efficiency route` packet | Apply the routing contract and protected quality/resource gates |
| No matched outcomes | CLI comparison stays INCONCLUSIVE; ordinary authorized native work can continue |
| Requested workflow improvement with no routing packet | Select relevant skills, tools and native capabilities through task judgment; verify actual results without a weekly scan, quota or automatic delegation |
| Skill listed or plugin installed | Verify invocation policy, callable access, authentication and actual execution separately; an explicit-only audit skill remains discoverable |
| Long-running operation already started | Resume its real handle, correlate the terminal result and inspect the requested output before repeating it |
| Valid model outside optimization scope or stale native catalog | Distinct INCONCLUSIVE reasons; preserve an explicit pin and never guess a replacement alias |
| Complete direct outcomes with absent/partial aggregate context | Compare direct outcomes and report aggregate limits separately |
| Weekly sample is complete but whole-population coverage is unknown | Keep that distinction; do not generalize to all history or rerun whole-store inventory as a prerequisite |
| Saved audit is reused | Scan no history; validate freshness and recompute one recommendation with current code |
| Requested adaptive delegation | Assign useful independent native lanes, observe effective child selections where available, include integration and failed-work cost, and validate the combined result |
| Independent lane with selectable history | Use only context controls exposed by the active interface; retain its goal, files, constraints and acceptance check. A small prompt alone does not establish a smaller fork |
| Compaction or interruption with a handoff | Retain base revision or dirty fingerprint, owned files, check results and scope, remaining required work, and pending native handles; reconcile them before continuing |
| Large filtered or truncated output | Preserve status, essential errors, exact scope and available original-result reference; a shorter output is not a successful check or whole-task savings |
| Small sequential task | Continue directly when delegation adds no useful independent work |
| Earlier or mixed model generation in history | Preserve records and selected settings; withhold unsupported GPT-6 optimization claims |
| Package-only installation | No automatic personal-setting rewrite or repair hook |
| Default setup or installer | Preserve existing/native settings; back up actual authorized changes and verify once |
| Explicit supported model/effort change | Validate the native catalog; no fixed preset, silent fallback, or unrequested context reset |
| Historical model, effort or team-size examples conflict with current work | Use current supported controls and the user's choices; no fixed model-by-role or agent-count rule |
| Config/catalog changes after repair preview | Reject the stale repair plan before replacing settings |
| Existing personal-trial state or user-edited generated guidance | Use status/rollback only; preserve user edits and unsupported state |
| Core and consented Insights coexist | Apply each plugin's own hook and consent boundary |
| Source differs from installed package | Report the difference; do not claim an installed update or fresh-task activation |
| Local worker markers or historical acknowledgements exist | Report local scope and per-phase freshness; do not infer native trust/dispatch, current collection, cloud or account-wide coverage |
| Secret-like input or private history | Exclude it from public reports and artifacts |
| Live verification requested | Verify the requested runtime outcome separately from source tests and package checks |
| Automated checks pass but device, deployment, approval, or publication proof is missing | Connect the delivered result to the exact proven scope and remaining required checks; no inferred completion of those stages |
| Required acceptance passes and an optional improvement appears | Close the accepted scope without reopening completed reviews; retain the optional finding without adding it to the delivery |

Model and effort selection details belong in
[model guidance](../plugins/groundline/references/model-effort-routing.md).
Coverage and source limits belong in
[weekly audit](../plugins/groundline/references/weekly-usage-audit.md).
Existing private trial recovery follows
[personal recovery](../plugins/groundline/references/personal-recovery.md).

Record source revision, installed fingerprint, model/effort, requested scope,
actions, outcome, and remaining gaps without raw transcripts, credentials, or
private paths. A manual reading of these cases is not a model-run pass. Repeat
checks for relevant changes or concrete unresolved risk.

### Native evaluation evidence

Before an authorized comparison, verify the installed app-server protocol and
effective permissions. A write-restricted workspace does not establish restricted
reads. Probe the allowed and excluded paths using task-owned fixture files without
starting a model turn. Reject an unsupported policy or a failed isolation probe;
do not silently fall back to unrestricted reads, scan personal history, or change
the user's default profile. Documentation and generated protocol schemas may
describe different runtime versions.

Bind token updates and terminal events to the requested thread and turn. Treat
cumulative updates as snapshots, retain cached input and reasoning output as
subsets, and preserve missing, invalid, or reset counters as unknown. Drain queued
events after termination and retain foreign-thread observations separately; they
cannot replace the root total or establish child ownership. Root usage alone does
not establish complete child, Guardian, or retry accounting.

For continuity, correlate the actual command launch, subsequent interaction, and
successful completion using the same native item and process handle. Reading a
script name or writing a completion claim is insufficient. Match the `turn/steer`
acknowledgement to the active turn and inspect the accepted input and resulting
artifact. An interrupted turn remains incomplete even when partial output exists.

Reserve a full per-run allowance before starting another inference and include
late usage in the total. Enforce the authorized call count and wall/token limits;
an exhausted or exceeded budget requires a new explicit numerical authorization.
Replay existing traces and test the harness without spending another inference.
Neither a successful harness test nor an inconclusive pilot qualifies a workflow
for default promotion.

## Release and installation

Local source edits do not update installed plugins. Qualify and publish through
the existing release workflow, use the native upgrade path, then verify the
installed artifact and requested fresh-task behavior. Do not patch provider
caches or label unpublished WIP as stable. User-owned agents, rules, and existing
private recovery state remain intact. Follow
[installation alignment](../plugins/groundline/references/installation-alignment.md)
and the [release checklist](release-checklist.md).
