# GroundLine

GroundLine uses local Codex activity and observed delivery outcomes to inform
GPT-6 Astra, Sol, and Luna model, effort, and subagent choices. Historical
records remain intact. Codex owns execution, settings, permissions, agents,
worktrees, review, and upgrades.

The workflow is **audit → delivery → route**. Aggregate activity describes a
sample; matched direct outcomes support an empirical comparison. Neither a
proposal nor a passing synthetic test establishes improved quality or savings.
See [the optimization loop](references/codex-optimization-loop.md).

## Skills

| Skill | Scope |
| --- | --- |
| `align-agent-home` | Requested installation, configuration, and guidance alignment |
| `audit-agent-history` | Explicit history inspection and redacted usage evidence |
| `optimize-codex-workflow` | Task-scoped GPT-6 selection and workflow assessment |

Alignment and optimization are implicitly invocable; history inspection requires
an explicit request. Native Codex handles planning, Goals, handoffs, and reviewed
file edits. GroundLine does not provide a second orchestration or imported-skill
management layer.

## Privacy boundary

Core has no lifecycle hooks, background process, scheduler, collector identity,
or network client. Audit commands open bounded local state read-only and return
aggregates without raw prompts, transcripts, paths, or configuration values.
Explicit delivery recording creates private local receipts; nothing is uploaded.

`groundline provider-smoke --plugin-root <path> --json` rejects an owner hook
manifest. Optional Insights is a separate plugin with its own consent contract.

## Install and upgrade

Use the repository's reviewed binary-bearing `stable` distribution and its
`install.sh` or `install.ps1`, or install the package through native Codex:

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
```

Core installation does not install or activate Insights. Package-only
installation does not run a personal-setting repair hook. For requested
application or repair, use `$groundline:align-agent-home` and
[installation alignment](references/installation-alignment.md).

Setup preserves existing choices and native defaults. Explicit `--model`,
`--effort`, and `--service-tier` options use the supplied native catalog; there is
no fixed model preset. Context restoration requires `--restore-native-context`.
Changed settings receive private backups, and omitting `--apply` previews without
writes. See [configuration review](references/codex-configuration.md).

Use [native upgrade](references/native-upgrade.md), then verify the installed
package and checksum separately from live behavior:

```console
groundline provider-smoke --plugin-root /path/to/installed/groundline --require-installed --json
groundline doctor --plugin-root /path/to/installed/groundline --json
```

Native packages cover Apple Silicon/Intel macOS and ARM64/x86_64 Linux/Windows.
Resolve the installed executable from `bin/<target>`; installation does not
promise a shell `PATH` entry. Source tags do not contain installable binaries.

## Usage evidence and delivery comparison

Run these examples in an owner-private working directory outside the public repository.

```console
groundline audit weekly --days 7 --review --json > weekly.json
groundline audit review --input weekly.json --json
groundline efficiency record-delivery --input manifest.json --output receipts/delivery.json --json
groundline efficiency delivery-summary --deliveries receipts --json
groundline efficiency route --input routing.json --catalog native-models.json --audit weekly.json --deliveries receipts --json
```

Use one fresh audit or reuse a saved one as appropriate. Saved review scans no
history and recomputes one recommendation under the current code. Weekly audits
cover the selected task-window sample; full-population coverage remains unknown.
`groundline audit store --json` separately inspects whole-store metadata. See
[weekly audit](references/weekly-usage-audit.md) for limits and evidence scope.

The [delivery contract](references/delivery-evidence.md) defines manifests,
private receipts, observed effective selections, quality, rework, and owned
resources. Failed work and unknown measurements remain visible. Pass a dedicated
receipt directory to `route --deliveries` with an empty packet `outcomes` array.
The [routing contract](references/evidence-routing.md) defines matched comparison
and catalog requirements; aggregate reports are optional context. Routing does
not change settings or establish an automatically learned improvement.

`personal status` and `personal rollback` only inspect or recover existing private
trial state. They do not start new trials. See [personal recovery](references/personal-recovery.md).

## Development

```console
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo run --locked -p xtask -- verify-source --root . --json
```

Pull requests run the fast lane. Full qualification and six-platform release
artifacts run for release tags or explicit manual requests.

See [architecture](https://github.com/jukqaz/groundline/blob/main/docs/architecture.md),
[integration profiles](https://github.com/jukqaz/groundline/blob/main/docs/integrations.md),
[Privacy](https://github.com/jukqaz/groundline/blob/main/docs/privacy.md),
[Security](SECURITY.md), and the
[release checklist](https://github.com/jukqaz/groundline/blob/main/docs/release-checklist.md).
