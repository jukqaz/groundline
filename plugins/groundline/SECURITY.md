# Security policy

## Supported version

Security fixes target the latest stable GroundLine release.

## Public security boundary

GroundLine is local-first. The public plugin installs no lifecycle hook, starts
no background process, performs no network request, keeps no collector identity,
and contains no remote endpoint or credential field. Its CLI accepts explicit
paths, rejects symlinks for bounded inputs, opens the local Codex state store
read-only with SQLite `NOFOLLOW`, owner, 8 GiB, and 100,000-row ceilings, and
accepts rollout files only from canonical non-symlinked Codex session roots. It
emits aggregates or reason codes instead of raw records and paths.

The source qualification gate checks both canonical plugin packages directly,
the Core zero-hook invariant, the Insights four-hook invariant, all six native
targets, pinned external CI actions, and the absence of private or personal
markers. These checks reduce accidental exposure but do not replace review.

## Reporting

Please use GitHub's private vulnerability reporting for security issues. Do not
include credentials, private paths, raw prompts, transcripts, or personal data in
an issue. Include the GroundLine version, platform target, minimal reproduction,
and redacted output when possible.
