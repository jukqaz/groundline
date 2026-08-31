# Native GroundLine Insights Upgrade

GroundLine Insights uses the same Codex Git marketplace as GroundLine Core.
Register the public monorepo over HTTPS with `--ref stable` for a moving upgrade
channel. An exact release tag is an immutable freeze or rollback channel.
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
```

If the marketplace was deliberately pinned to a tag, do not silently replace
it. With explicit authorization, remove that registration and add the same
public monorepo at `stable`, then install GroundLine Insights again.

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
