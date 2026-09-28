# Examples

## Installation

Choose Core, Insights, or both using the [installation guide](installation.md).
Package-only Core installation uses native Codex:

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
```

## Audit a task-window sample

Keep audit files, manifests, and receipts in an owner-private working directory
outside the public repository.

Capture one bounded audit and recommendation:

```console
groundline audit weekly --days 7 --review --json > weekly.json
```

When that saved window is still relevant and fresh, reuse it instead of scanning
history again:

```console
groundline audit review --input weekly.json --json
```

Saved review recomputes one recommendation under the current code. It does not
reuse the saved recommendation as current evidence. Weekly sample completeness
is distinct from full-population coverage, which remains unknown. For a separate
whole-store metadata diagnostic, use `groundline audit store --json`.
See [weekly audit](../plugins/groundline/references/weekly-usage-audit.md).

## Record actual deliveries and compare

Prepare the private manifest and local evidence using the
[delivery contract](../plugins/groundline/references/delivery-evidence.md). Record
verified, failed, and incomplete outcomes without inventing missing observations:

```console
groundline efficiency record-delivery --input manifest.json --output receipts/delivery.json --json
groundline efficiency delivery-summary --deliveries receipts --json
```

Prepare a current [routing packet](../plugins/groundline/references/evidence-routing.md)
and native model catalog. With receipt input, keep the packet's `outcomes` array
empty so direct observations are not counted twice:

```console
groundline efficiency route --input routing.json --catalog native-models.json --audit weekly.json --deliveries receipts --json
```

Audit and Insights reports are optional descriptive context. A measured model or
effort comparison needs matching direct outcomes, protected quality, and complete
owned resources. The CLI does not apply settings or establish automatic gains.
See the [architecture](architecture.md) for the product boundary and
[personal recovery](../plugins/groundline/references/personal-recovery.md) for
existing private trial state.
