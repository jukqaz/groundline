# Native GroundLine Insights Upgrade

Core and Insights share Codex's Git marketplace but install independently.
Refreshing it can update both installed products, including Insights during a
Core-only setup. Source tags contain no binaries; use a reviewed, complete
binary-bearing `stable` distribution.

## Before mutation

Keep Git, `gh`, browser, and connector authentication separate. Inspect the
registered source without printing credentials, then run the installed binary:

```bash
groundline-insights provider-smoke --require-installed --json
groundline-insights worker status
```

Record version, target, checksum, hook count, and lifecycle receipt separately.
A cache directory proves no dispatch. Upgrade the owner API first: the current
collector requires Basic schema 5 and ingest contract revision 9 or newer.

Use that reviewed distribution's `install.sh`. Before native writes it checks
the candidate artifacts and runs the candidate's read-only `worker check-server`
when Insights is installed or has local state. Missing profiles make no request;
malformed profiles, transport failures, and incompatible APIs block the update.
The installer preserves identity, consent, pending windows, disabled flags, and
the registered official HTTPS/SSH transport; it pins the exact reviewed commit.
Only newly selected plugins use `plugin add`, which would enable an existing
disabled plugin if repeated.

## Upgrade

These native commands recheck/refresh the registered commit; they do not select a
new release after the installer has pinned it and do not run its preflight:

```bash
codex plugin marketplace upgrade groundline --json
codex plugin list --json
```

For a new release, use its reviewed distribution and installer. Preserve deliberate pins.

## Adoption proof

Verify independently: native source/result, installed listing, new binary
checksum, exactly four effective hooks, user review of changed trust hashes,
private schema-7 profile/credential readiness, current-version lifecycle receipt,
and accepted upload. Missing lanes remain `UNVERIFIED`; restart the App only if
fresh-task/hook evidence stays stale after refresh.

Caught source-transition failures attempt recovery to the previous actual SHA,
verify old artifact bytes and installed flags, and report `previous_commit_pinned`.
The old symbolic ref is unavailable in native JSON. Unsupported sources fail
before writes; interrupted transitions or failed rollback need review, not reset.
Upgrades never convert unsupported local state or skip incomplete windows.
Enrollment reuses the existing identity/token and requires the active generation.
See [operations troubleshooting](operations-troubleshooting.md) and the
[contract](insights-contract.md).
