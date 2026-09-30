# Conditional Codex capability routing

Use this matrix only for a requested workflow or a directly observed eligible
opportunity. It is a decision aid for native Codex, not a feature checklist,
activation policy, or runtime router. Select the smallest useful surface and
verify the requested outcome on that surface.

Distinguish use from a verified result. If an eligible capability was invoked
but its requested outcome is still unverified, inspect that existing execution
before repeating it. Independent authorized work may continue while it runs.
Keep the result pending until its actual handle reports completion and the
requested outcome is checked. An unresolved operation is not a new-use suggestion
and does not prove success or failure.

For every candidate, keep five facts separate: provider documentation; presence
in the active host's tools or bundled CLI; availability to this account and
task (including authentication); authority for this operation; and observed
success. A missing history record, unavailable signal, or absent denominator
is UNKNOWN, never evidence of non-use. Do not export raw conversation text to
fill a missing aggregate; inspect only relevant local evidence when authorized.

| Observed eligible opportunity | Native surface to consider | Evidence before use and outcome check |
| --- | --- | --- |
| Independent, bounded reads can proceed without shared state | Parallel native tool calls; combine per-call results | Confirm the calls are independent and the active interface supports concurrent calls. Preserve each result/error and verify the combined answer against all required inputs. |
| Work has independent lanes whose outputs can be integrated | Native GPT-6 subagents, with a compact handoff when a child needs bounded context | Confirm user or applicable project authority, current model/effort controls, concurrency, and lane independence. Preserve no-delegation restrictions. Select a supported pair per lane; inspect returned evidence and perform root integration checks. Do not claim the running root switched model or effort. |
| A native operation is long-running and returns a waitable handle | Supported asynchronous execution and native wait/poll tools | Confirm the actual handle and wait mechanism. Wait on the matching operation, correlate its terminal result, and inspect completion/failure; a process start or orchestration response is not success. |
| A repeatable specialist workflow matches the task | Installed, relevant Codex skill | Confirm discovery, enabled state and invocation policy separately; read required instructions only when the trigger applies. An explicit-only audit skill is not missing. Verify the requested behavior or deliverable, not just discovery or invocation. |
| Needed context or an authorized action belongs to a connected service | Authenticated MCP or app connector | Confirm the exact connected tool and identity are available, required auth is valid, and the operation is authorized. For writes, verify the specific saved state by rereading it. Do not infer connector access from a plugin listing. |
| Work depends on a website or local app state | Built-in Browser, the user-selected Chrome/browser session, or Computer Use on the specified app | Choose the surface that matches the user's session and target. Confirm actual page/app access and any required sign-in or approval; inspect the resulting live state. A URL, screenshot, or tool start alone does not prove a transaction or save. |
| Repository work needs isolation, parallel checkouts, or an app-managed environment | Native worktree creation and host-supported setup | Confirm a Git repository, active host support, starting state, and local dependency needs. Preserve current WIP; verify returned path, branch/commit, and repository root before editing or building. |
| A code change or generated deliverable needs a reviewable handoff | Native diff/review panel or file/artifact preview | Confirm a diff or deliverable exists and the relevant viewer is available. Inspect the rendered file or exact diff and run the requested acceptance check; attachment or panel creation alone is not review. |
| A recurring user-owned decision or long-lived objective needs continuity | Native Goal for a user-requested objective; native heartbeat/automation for an explicitly requested follow-up | Create a Goal only on explicit request. Create or change recurrence only when requested; use the native heartbeat for a thread follow-up, and a standalone scheduled task only when its separate project/task behavior is requested. Verify the saved objective/prompt, schedule, and status. |
| A later turn benefits from relevant saved preference or prior decision | Available memory/context reference | Use only relevant, permitted saved information and distinguish it from current evidence. Recheck drift-prone facts before acting; never use memory presence as proof of current runtime or account state. |
| A permission prompt or sandbox boundary is affecting a concrete operation | Existing sandbox and approval controls, including auto-review only where supported and already authorized | Diagnose the exact denied operation and active host policy first. Keep least-privilege boundaries; do not broaden filesystem/network access, enable auto-review, or weaken sandboxing to increase utilization. Verify the original operation after an authorized narrow change. |
| Steering or compaction arrives during existing work | Current task state and existing execution handles | Retain the goal, corrections, authority, accepted checks and unfinished work. Inspect ongoing operations, resume the same task and avoid duplicate starts. A follow-up verification request does not by itself prove rework. |
| Repeated omissions justify a mechanical repository check | A narrowly scoped native hook on a supported local runtime | Verify event support, enabled state and trusted hash independently. Bound input, duration and repeat behavior; inspect actual execution and result. Core remains hook-free. Insights' detached capture/worker path is not a generic review hook or a cloud collector. |

Goals and recurrence require explicit requests. App/service writes and permission
changes must stay within their authorized scope; existing scoped authority does
not need repeated confirmation. A request to assess or
recommend remains advisory. Respect opt-in experiments and current host policy;
do not enable features, plugins, hooks, flags, accounts, or broad permissions
merely to improve a utilization count. If no directly evidenced candidate has a
useful acceptance check, keep the current workflow and report the evidence gap.

The offline `groundline efficiency route --input ... --catalog ...
[--audit ...] [--report ...] --json` accepts the strict schema-2 model/effort
comparison packet only. It has no capability-input fields, feature suggestions,
or operation-verification output. Use this matrix through native task judgment
without an evidence packet, weekly scan or minimum sample. Treat model comparison
output under the [evidence routing contract](evidence-routing.md). The native executor makes any
authorized task/lane selection using host controls; aggregate counts cannot
change a running root, choose settings automatically, or prove feature eligibility.

Provider documentation reviewed 2026-09-30 (recheck the relevant page when the
host or product changes):
- [Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents)
- [Long-running work](https://learn.chatgpt.com/docs/long-running-work)
- [Git worktrees](https://learn.chatgpt.com/docs/environments/git-worktrees)
- [Automations](https://learn.chatgpt.com/docs/automations)
- [Approvals and sandboxing](https://learn.chatgpt.com/docs/sandboxing)
- [Auto-review](https://learn.chatgpt.com/docs/sandboxing/auto-review)
- [Skills and plugins](https://learn.chatgpt.com/docs/skills-and-plugins)
- [Skill discovery and invocation policy](https://learn.chatgpt.com/docs/build-skills)
- [Hook support, trust and termination](https://learn.chatgpt.com/docs/hooks)
- [MCP](https://learn.chatgpt.com/docs/extend/mcp)
- [Browser](https://learn.chatgpt.com/docs/browser)
- [Computer Use](https://learn.chatgpt.com/docs/computer-use)
- [Code review](https://learn.chatgpt.com/docs/code-review)
- [Working with files and artifacts](https://learn.chatgpt.com/docs/artifacts-viewer)
