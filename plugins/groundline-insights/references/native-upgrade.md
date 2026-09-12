# Native GroundLine Insights Upgrade

GroundLine Insights uses the same Codex Git marketplace as GroundLine Core.
Register the public monorepo over HTTPS with `--ref stable` for a moving upgrade
channel. Version tags contain source, while `stable` contains the generated
native binaries. Do not pin a collector installation to a source-only tag.
Core and Insights remain separate install records: refreshing the shared
marketplace never opts the user into an uninstalled sibling plugin.

## Before mutation

Keep Git transport, `gh`, browser login, and connector access separate. Verify
the registered source and ref without printing credentials. Run the currently
installed target binary:

```bash
groundline-insights provider-smoke --require-installed --json
groundline-insights worker status
```

Record installed version, target, artifact checksum status, hook event count,
and lifecycle receipt status. A cached directory is not hook dispatch proof.

## Upgrade

Use Codex App **Refresh** or:

```bash
codex plugin marketplace upgrade groundline --json
codex plugin list --json
```

If the installed Insights version remains unchanged after refresh, use
`codex plugin add groundline-insights@groundline --json` for that same ID and
verify the installed checksum. Preserve any deliberately pinned source; a
source-only tag needs an explicitly selected, verified binary distribution.

Before collection, upgrade the owner API and check Basic schema 5 plus ingest
contract revision 6 or newer. Enrollment must return the current generation;
reuse the existing collector identity and token. A package upgrade does not
convert unsupported local state or skip an incomplete collection window.

## Adoption proof

1. Retain the Refresh or CLI upgrade result.
2. Verify the installed listing with `codex plugin list --json`.
3. Run the new binary's provider smoke and require checksum verification.
4. Inspect exactly four effective hook entries and let the user review changed
   trust hashes.
5. Confirm the owner-local schema-7 profile and enrollment credential are
   configured without printing either value.
6. Observe a current-version lifecycle receipt.
7. Observe an accepted upload separately.

An unavailable lane remains `UNVERIFIED`. Restart the App only if the fresh
task or hook evidence remains stale after Refresh.
