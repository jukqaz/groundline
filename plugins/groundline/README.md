# GroundLine

GroundLine uses local Codex activity and observed delivery outcomes to inform
GPT-6 Astra, Sol, and Luna model, effort, and subagent choices. Codex owns
execution, settings, permissions, agents, worktrees, review, and upgrades.
Historical records remain intact.

The workflow is **audit → delivery → route**: activity describes a sample;
matched direct outcomes support comparison. A proposal or passing synthetic test
does not prove better quality or savings. See the
[optimization loop](references/codex-optimization-loop.md).

## Skills

| Skill | Scope |
| --- | --- |
| `align-agent-home` | Requested installation, configuration, and guidance alignment |
| `audit-agent-history` | Explicit history inspection and redacted usage evidence |
| `optimize-codex-workflow` | Task-scoped GPT-6 selection and workflow assessment |

Alignment and optimization are implicitly invocable; history inspection requires
an explicit request. Planning, Goals, handoffs, and reviewed edits stay native.

## Install and upgrade

Use `install.sh` from the reviewed binary-bearing `stable` distribution for
installation and application. Native package-only installation is also available:

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
```

Core does not add or activate Insights. Package delivery alone does not repair
personal settings. For requested application, use `$groundline:align-agent-home`
and [installation alignment](references/installation-alignment.md). Setup preserves
existing choices and native defaults; explicit settings, backups, previews, and
context restoration follow [configuration review](references/codex-configuration.md).

Follow [native upgrade](references/native-upgrade.md) for candidate API checks,
commit pinning, and recovery. Then verify the installed package separately from
live behavior:

```console
groundline provider-smoke --plugin-root /path/to/installed/groundline --require-installed --json
groundline doctor --plugin-root /path/to/installed/groundline --json
```

Packages support macOS/Linux on ARM64 and x86_64. Resolve the installed executable
from `bin/<target>` using [platform commands](references/platform-commands.md);
installation does not promise a shell `PATH` entry. Source tags contain no binaries.

## Usage evidence and delivery comparison

Run these examples in an owner-private directory outside the public repository:

```console
groundline audit weekly --days 7 --review --json > weekly.json
groundline audit review --input weekly.json --json
groundline efficiency record-delivery --input manifest.json --output receipts/delivery.json --json
groundline efficiency delivery-summary --deliveries receipts --json
groundline efficiency route --input routing.json --catalog native-models.json --audit weekly.json --deliveries receipts --json
```

[Weekly audit](references/weekly-usage-audit.md) covers the selected task-window
sample, with full-population coverage unknown. Saved review scans no history and
recomputes one recommendation. `groundline audit store --json` is a separate
whole-store diagnostic.

The [delivery contract](references/delivery-evidence.md) records effective
selections, quality, rework, resources, failures, and unknowns. For
[routing](references/evidence-routing.md), use a dedicated receipt directory and
an empty packet `outcomes` array. Aggregate reports are optional context; routing
changes no settings and does not establish automatically learned improvement.
Existing `personal status`/`rollback` are [recovery only](references/personal-recovery.md).

## Privacy boundary

Core has no hooks, background process, scheduler, collector identity, or network
client. Audit is bounded and read-only; delivery receipts are private local files.
Neither exports raw prompts, transcripts, paths, or configuration values.
Provider smoke rejects owner hook manifests. Optional Insights has separate
installation and consent. See [Security](SECURITY.md) and
[Privacy](https://github.com/jukqaz/groundline/blob/main/docs/privacy.md).

## Development

Use [CONTRIBUTING](https://github.com/jukqaz/groundline/blob/main/CONTRIBUTING.md)
for validation commands and CI lanes. See also
[architecture](https://github.com/jukqaz/groundline/blob/main/docs/architecture.md),
[integration profiles](https://github.com/jukqaz/groundline/blob/main/docs/integrations.md),
and the [release checklist](https://github.com/jukqaz/groundline/blob/main/docs/release-checklist.md).
