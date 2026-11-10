# Bounded weekly follow-up

An authorized weekly improvement first follows [conversation improvement](conversation-improvement.md):
inspect relevant later work for an existing change, then consider a small new
friction only if actual feedback warrants it. No change is a complete result.
Reuse unchanged evidence. No new relevant work means stop, not expand collection.

## Optional usage diagnosis

Run an aggregate audit only for an explicit request or a concrete usage problem
that the aggregate can help locate. It is not a prerequisite for guidance edits,
content review, ordinary implementation or a weekly follow-up. An explicit user
audit request still takes precedence. Read [platform commands](platform-commands.md)
when resolving the installed executable.

```console
groundline audit weekly --days 7 --review --json
```

Run once, retain the redacted full result in the authorized private task area,
and reuse it. The combined command already performs one audit and recommendation.
If a current saved result is explicitly available, `audit review --input RESULT
--json` reads that observation without rescanning history. Its window must be
within 24 hours; an older result is dated evidence, not a current audit.

The native audit is bounded and read-only. Incomplete roots, unknown command
classification, interrupted work and exhausted budgets stay visible. A weekly
sample is not whole-history coverage. Do not enlarge a sample just to meet a
quota, infer errors from repeated messages, or claim savings from aggregate
ratios. Inspect necessary surrounding conversation context before any correction.
`audit store` is a separate explicit whole-store metadata diagnostic.

Local review works without Insights. Collection, transmission, server receipts
and local analysis are separate. A delivery failure does not require enabling
hooks, fabricating outcomes or retrying a successful local audit.

Keep scheduler instructions to purpose, authorized scope and reporting. Product
references own this procedure and the existing private store owns derived
records. Do not change model, effort, tier, native memory, consent or schedules
as a side effect. A short report states the real change or no-change reason,
applicable verification, contextual follow-up and remaining uncertainty.
