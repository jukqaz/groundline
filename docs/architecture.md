# GroundLine architecture

Core owns one narrow workflow: contextual review, a real scoped guidance change,
and later natural observation. Native Codex owns semantic judgment and editing.
AGENTS and skills carry the result into work. Optional Insights owns consented
operational collection and server analytics independently.

```mermaid
flowchart LR
  Feedback[Relevant conversation feedback] --> Review[Context and counterexample]
  Review --> Decision[No change or small scoped change]
  Decision --> Native[Native guidance edit and verification]
  Native --> Record[Actual before and after digests]
  Record --> Followup[Natural work: met, violated or unassessed]
```

| Boundary | Responsibility |
| --- | --- |
| groundline-contracts::improvement | Strict compact records, independent follow-up and qualitative readouts |
| Core CLI | Bounded private I/O, actual target binding, no-clobber publication and manual diagnostics |
| Historical conversation contract | Read and validate immutable conversation records without requalifying old comparisons |
| Native Codex | Interpret feedback, choose scope, edit and verify owning files, observe actual later work |
| Insights | Separately consented collection, HTTPS delivery, ClickHouse and Grafana |
| xtask | Source/package/native validation and explicit publication |

There is no Core model router, environment baseline writer, device synchronizer
or ongoing learning profile. Configuration setup/repair remain explicit manual
maintenance and preserve unrelated choices. Ordinary work does not invoke Core.

The existing private record directory holds compact captures, changes and
observations beside unchanged historical conversation records. No new scheduler
ledger, transcript copy or automatic migration is introduced. The current reader
rejects unsupported managed records/layouts without changing them.

Content assessments require exact changed guidance exposure, an actual later
opportunity and known behavior. Model/tool/tier equality is not required for that
qualitative judgment. Causal benefit and efficiency improvement stay unverified.
Old comparison gates remain historical validation, never retroactive claims.

[Development](development.md) · [Examples](examples.md) ·
[Guidance validation](guidance-validation.md) · [Release checks](release-checklist.md)
