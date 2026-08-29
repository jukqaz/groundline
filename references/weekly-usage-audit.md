# Weekly local audit

Use `groundline audit weekly --days 7 --json` for aggregate local evidence.
Review counts, coverage, failure reason codes, and the proposed single workflow
change. Keep source validation, installed runtime validation, and user-visible
behavior as separate evidence lanes.

The command is read-only, performs no network request, and does not emit raw
task content or private paths.
