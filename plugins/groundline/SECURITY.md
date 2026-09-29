# Security policy

## Supported version

Security fixes target the latest stable release.

## Public security boundary

Core installs no lifecycle hook or background process, makes no network request,
and stores no collector identity, endpoint, or credential. Bounded input readers
reject symlinks. Codex SQLite is read-only with `NOFOLLOW`, owner checks, an 8 GiB
limit, and a 100,000-row ceiling. Rollouts must belong to canonical non-symlinked
session roots. Output contains aggregates or reason codes, not raw records or paths.
See [audit scope](references/weekly-usage-audit.md) and
[private delivery receipts](references/delivery-evidence.md).

Source qualification checks package boundaries, Core's zero hooks, Insights'
four hooks, pinned CI actions, and private markers. Release qualification also
verifies all four native targets. Neither proves live dispatch.

## Reporting

Use GitHub private vulnerability reporting. Include the version, platform,
minimal reproduction, and redacted output; omit credentials, private paths,
raw prompts, transcripts, and personal data from public issues.
