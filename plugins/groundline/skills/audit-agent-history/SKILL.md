---
name: audit-agent-history
description: Inspect bounded Codex usage history and prepare private, source-labeled observations for workflow analysis.
---

# Inspect usage evidence

Start from the requested time, runtime and task scope. Search metadata before
opening relevant content. Preserve original histories; ordinary audits are
read-only. Do not emit prompts, credentials, commands, patches, account IDs,
private paths or long transcript excerpts.

Resolve [the installed command](../../references/platform-commands.md). For
weekly analysis, read [weekly usage audit](../../references/weekly-usage-audit.md)
and run `groundline audit weekly --days 7 --review --json` once. Reuse a fresh
saved audit with `groundline audit review --input <snapshot.json> --json`;
it recalculates the local recommendation without reading native history again.

Weekly evidence covers the selected indexed window, not every conversation on
the Mac. Missing rows and ambiguous owners in that sample remain visible.
For an explicit whole-store integrity question use `groundline audit store --json`.
Its diagnosis is separate from the sample and never repairs or deletes history.

Keep provider-reported tokens, completed turns, verified tool calls, and actual
deliveries distinct. Storage bytes are not tokens. Counts cannot prove quality,
model efficiency or account membership. For a requested outcome comparison use
[delivery evidence](../../references/delivery-evidence.md) and
[evidence routing](../../references/evidence-routing.md). Aggregate observations
may inform native task judgment but do not supply missing delivery outcomes.
Report observed patterns, source/coverage limits, and the next relevant check.
