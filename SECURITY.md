# Security policy

## Supported version

Security fixes target the latest stable GroundLine release.

## Public security boundary

GroundLine is local-first. The public plugin installs no lifecycle hook, starts
no background process, performs no network request, keeps no collector identity,
and contains no remote endpoint or credential field. Its CLI accepts explicit
paths, rejects symlinks for bounded inputs, opens the local Codex state store
read-only, and emits aggregates or reason codes instead of raw records and paths.

The source qualification gate checks that the installable package is synchronized,
the six native targets remain explicit, external CI actions are pinned, and private
or personal markers are absent. These checks reduce accidental exposure but do not
replace review.

## Reporting

Please use GitHub's private vulnerability reporting for security issues. Do not
include credentials, private paths, raw prompts, transcripts, or personal data in
an issue. Include the GroundLine version, platform target, minimal reproduction,
and redacted output when possible.
