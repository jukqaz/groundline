---
name: audit-agent-history
description: Use when explicitly auditing agent histories, recovering context, or preparing a redacted evidence packet.
---

# Audit Agent History

## Purpose

Inspect Codex histories as storage and continuity surfaces, not prompt dumps.
Inventory metadata and search narrowly before opening content. For assessment
use `audit-agent-history -> evaluate-ai-usage-maturity`: this skill prepares a
redacted Codex Evidence Packet or Usage Evidence Packet; the next skill scores
it.

## Safety

- Default read-only; delete, move, archive, or compress only on explicit request.
- Never print secrets, credentials, raw prompts, long excerpts, commands, patches, or private paths.
- Revalidate summary-derived claims against current files or live systems.

## Workflow

Resolve the installed GroundLine binary and references from this skill's plugin
root. Never assume the user's current repository contains them.

1. Identify Codex/project storage roots and the requested time/task scope.
2. Inventory counts, sizes, time range, layout, and trustworthy model/effort/compaction/verification metadata.
3. Search paths and keywords with indexes, `rg`, `find`, or `jq`; open only the minimum matches.
4. Report reusable patterns, duplication/retention candidates, capability candidates, and facts needing live proof.
5. For cost or efficiency, separate exact Codex-reported usage from
   activity/storage proxies; never convert storage bytes into tokens.
6. For general usage evidence, run `groundline audit weekly --days 7 --json`
   or a bounded `groundline audit activity --start <RFC3339> --json`. Both
   include root, delegated-agent, and canonical Guardian aggregates without raw
   content.
8. For a scheduled weekly audit, read
   [the weekly usage audit contract](../../references/weekly-usage-audit.md)
   fully and run its bounded aggregator. Pass the redacted result to
   `groundline efficiency recommend --audit <weekly.json> --json`. Present
   its single candidate for review; never apply it without the user's decision.
9. When the user explicitly permits Chronicle evidence, verify Chronicle is
   running through its native skill and read only the minimum recent surface.
   Create the numeric aggregate defined in
   `references/chronicle-evidence-contract.md`, then fuse it with the Codex
   audit using `groundline efficiency fuse --audit <weekly.json> --chronicle
   <chronicle.json> --json`. Never use
   Chronicle observation counts as tokens or change Chronicle state or its
   experiment ledger.
10. For an efficiency counterfactual, use
   `groundline efficiency simulate --audit <weekly.json> --json`. Label the
   result as a simulation, not measured savings or billing.
11. GroundLine does not maintain a mutable experiment ledger. Record an
   accepted experiment only in a user-selected repository artifact.

Repeated allowed reviews for temporary/external paths are workspace-boundary signals. Keep Auto-review for remote or destructive mutations and ordinary work inside the active workspace.

## Candidate Test

Rate repeatability, risk reduction, portability, non-obviousness, and whether the right form is a skill, script, hook, agent, or documentation.

## Output Contract

```text
Codex inventory:
- ...
High-signal patterns:
- ...
Candidates:
- ...
Retention notes:
- ...
Revalidation needed:
- ...
Usage Evidence Packet, when requested:
- Codex time window and coverage
- exact usage source: Codex-reported|unavailable
- context, retry, handoff, verification signals
- storage/activity proxies
- optional Chronicle behavior-boundary counts
- raw content excluded: true
```

Omit unavailable behavior signals rather than guessing.
