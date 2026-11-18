# GroundLine architecture

Core provides one skill for contextual review, a scoped guidance change and
later natural observation. The same skill guides model and effort selection
when the user delegates that decision for the current task. Native Codex owns
judgment, editing and execution. AGENTS and skills carry guidance into work.
Optional Insights owns consented operational collection and server analytics
independently.

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
| Native Codex | Interpret feedback, edit and verify guidance, observe later work, and apply model and effort selections through native controls |
| Insights | Separately consented collection, HTTPS delivery, ClickHouse and Grafana |
| xtask | Source/package/native validation and explicit publication |

Model and effort guidance runs on request within the current task. When the
active native interface supports it, Codex can select a model and effort when
spawning a new subagent. Use the native client for a later turn. Configuration
setup and repair remain explicit manual maintenance. Core's offline CLI handles
records and diagnostics.

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
