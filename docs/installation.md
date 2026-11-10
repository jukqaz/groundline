# Install, configure, and verify

Core and Insights have independent release tags, channels and native marketplaces.
Install each reviewed distribution separately when both products are wanted.
Core is offline and has no hooks. Insights owns optional connection and collection.

| Product | Release tag | Binary channel | Native ID |
| --- | --- | --- | --- |
| Core | `core-vYEAR.MMDD.N` | `core-stable` | `groundline@groundline-core` |
| Insights | `insights-vYEAR.MMDD.N` | `stable` | `groundline-insights@groundline` |

The shared workspace version identifies a source build. Only the product selected
by the tag is published. Core tags publish three binary archives and `SHA256SUMS`;
Insights tags additionally publish the API image. Source tags and `main` contain
no installable native binaries. Historical combined `v...` releases remain intact.

## Start

With Git, Codex, Bash and `jq`, review the complete Core distribution:

```console
git clone --branch core-stable --single-branch https://github.com/jukqaz/groundline.git groundline-core-install
bash groundline-core-install/install.sh
```

For Insights use a separate clone of `stable` and its installer. An optional
`--profile core` or `--profile insights` must match the selected distribution;
combined installation is not a profile. Supported hosts are Apple Silicon macOS
and Linux ARM64/x86_64. The installer verifies the macOS App bundle identity and
prefers its CLI, including both native bundle layouts. `--codex /absolute/path`
selects a runtime explicitly. Keep the intended `CODEX_HOME`.

For a new package-only Core installation:

```console
CODEX="/absolute/path/to/codex"
"$CODEX" plugin marketplace add https://github.com/jukqaz/groundline.git --ref core-stable --json
"$CODEX" plugin add groundline@groundline-core --json
```

Insights uses `--ref stable` and `groundline-insights@groundline`. Package-only
commands do not run installer preflight or connection setup. Existing installations
should use the reviewed installer rather than re-add disabled plugins manually.
Resolve each executable from its installed native `bin/<target>` directory;
shell PATH registration is not promised.

## Update and preserve

Review a new complete product distribution and run its installer. It checks the
descriptor, source-parent binding, exact clean commit, selected catalog and package
bytes before native writes. Official HTTPS/SSH transport is retained. It pins
the selected native marketplace to the reviewed commit; App Refresh cannot follow
a channel that moved after review. No separate updater runs in the background.

An enabled legacy `groundline@groundline` migrates to `groundline@groundline-core`.
Its original `groundline` source stays pinned for existing Insights. A disabled
legacy Core, enabled historical learning profile, changed cache or unsupported
source stops before writes and preserves that state. Insights updates require
legacy Core migration first. A disabled plugin already in its independent
namespace remains disabled. All other products and preferences are checked intact.

Only Insights updates run the candidate's read-only `worker check-server` before
native writes. Missing profiles make no request; malformed profiles, transport
failures or incompatible APIs stop the update. Upgrade the owner API first when
the collector contract requires it. Core does not inspect, enroll or update the
collector. Neither installer resets identity, consent, cursors, pending events,
history or learning records. See [native upgrade](../plugins/groundline/references/native-upgrade.md).

## Explicit settings and collection

Default installation does not load models or invoke setup. Existing model, effort,
service tier, permissions, context, experiments and collection choices are preserved.
Core accepts explicit `--model`, `--effort`, `--service-tier default|fast` and
`--restore-native-context`. Only then does it require a successfully completed native
model catalog before setup. Unsupported choices stop without substitution. See
[configuration maintenance](../plugins/groundline/references/codex-configuration.md).

For explicitly requested Insights connection/consent, use its own distribution:

```console
bash groundline-insights-install/install.sh --insights-endpoint https://insights.example.com --enrollment-token-file /private/enrollment-token --enable-insights
```

Alternatively use `--insights-profile /private/profile.json` with a schema-7 owner
profile. Do not combine profile and endpoint/token inputs. Files must be private
to their owner; no credential belongs in arguments or source. Matching inputs are
reused; conflicting connections require review. `--enable-insights` is explicit
collection consent. Without it, current consent stays and a new installation is
disabled. Native hook trust remains the user's decision; setup never approves hooks.
The App CLI selects the App source for explicit setup unless a runtime override
already exists. See [integration](integrations.md) for runtime scopes.

Setup is also available through installed `groundline-insights setup` with
`--endpoint`, `--enrollment-token-file`, `--enable`, `--verify` or `--input`.
First activity and a fresh accepted upload must be observed separately; historical
acknowledgements and synthetic tests prove no new delivery.

## Verify and resume

The installer backs up actual config privately before native writes, then emits
`groundline-installation` JSON after diagnostics. Exit 0 is `PASS`, 1 is `FAIL`,
and 2 is `ACTION_REQUIRED` (for example a native doctor failure after installation).
Receipts retain original child exit codes and completed stages. Default setup
stages say `PRESERVED`. Every retry rechecks current state.

Caught source-transition or artifact failures attempt scoped native recovery.
Only the selected product/source is restored; the old config snapshot is never
written over concurrent user edits. `previous_product_registration_restored`
means previous versions and flags were checked. Backups remain available for
owner review. Failed recovery, process kill or power loss requires inspection
before retrying; these native commands are not one atomic transaction.

Run native strict doctor and verify installed checksums separately from a fresh
skill/hook observation. Source, package, installation, private server delivery and
natural improvement are distinct evidence. See the [release checklist](release-checklist.md).
