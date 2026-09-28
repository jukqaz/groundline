# Install, configure, and verify

The public installer is one flow with three product profiles: `core` (default),
`insights`, or `both`. Core remains offline and independently installable. The
installer uses Codex's marketplace commands; only the Insights executable owns
service connection and collection. There is no additional GUI or background daemon.

## Start

Install Git and Codex first; the Bash installer also requires `jq`. Review and clone the binary-bearing `stable`
distribution. Source tags and `main` do not contain installable native binaries.

```console
git clone --branch stable --single-branch https://github.com/jukqaz/groundline.git groundline-install
bash groundline-install/install.sh
```

Windows:

```powershell
powershell -File groundline-install/install.ps1
```

macOS prefers the App-bundled executable when present. Windows inspects installed
OpenAI App packages and then PATH. Use `--codex /absolute/path/to/codex` or
`-Codex C:\path\to\codex.exe` for an explicit runtime. Preflight checks required
native commands before package changes. Keep the same intended `CODEX_HOME`.

Installing through Codex's plugin UI or `plugin add` delivers the package only.
Finish through this installer, or invoke the installed setup commands with the
documented inputs. Do not assume a plugin post-install callback ran.

## Update an existing installation

Obtain the current complete `stable` distribution and run the same installer with
the same intended Codex executable and `CODEX_HOME`. It refreshes the native
marketplace at that distribution's exact clean Git commit, verifies installed artifacts,
then rechecks setup and strict doctor. Existing installed/enabled states are retained;
only newly selected products use `plugin add`. Disabled plugins stay disabled.
No separate updater runs in the background.

The installer changes an existing official HTTPS/SSH registration through native
remove/add commands, retaining its transport. It accepts only an owned, clean
native Git snapshot aligned with the installed versions; Codex's validated
`.codex-marketplace-install.json` bookkeeping file is the sole untracked exception.
Unsupported sources or partial state stop before changes. The reviewed distribution
itself must be clean and contain committed binaries, checksums, and manifests.
App Refresh now stays on the pinned commit. To adopt another release, obtain and
review its complete current `stable` distribution and rerun its installer.

Profiles select explicit installation and setup steps. Native marketplace refresh
can also update another already-installed product from this shared channel:
`--profile core` does not keep an installed Insights collector on its old version.
Check the owner API's compatibility before updating a home with enabled Insights.
See the [native upgrade boundary](../plugins/groundline/references/native-upgrade.md).

When Insights state exists, the installer first verifies the candidate Insights
artifact and runs `worker check-server` before any native marketplace write.
This reads the existing profile and checks the API's ingest capabilities. A
malformed profile or incompatible/unreachable API stops the update without
changing collection state. No profile means no network request; a fresh Core
installation remains offline apart from native package delivery. This check
does not enroll, grant consent, run collection, or prove successful delivery.

## Settings policy

Default setup preserves existing model, effort, Fast, permissions, experiments,
and context choices. A fresh config uses native Codex defaults. Known retired
Core hook approval records can be removed with a private backup. Native strict
doctor checks effective configuration separately from the bounded file review.

| Explicit choice | Shell option | PowerShell option |
| --- | --- | --- |
| A supported model | `--model <id>` | `-Model <id>` |
| Supported effort | `--effort <level>` | `-Effort <level>` |
| Service tier | `--service-tier default` or `fast` | `-ServiceTier default` or `fast` |
| Restore native context sizing | `--restore-native-context` | `-RestoreNativeContext` |

There is no fixed model preset. An unsupported explicit selection stops settings
changes without substitution. Resolve
profile/provider/catalog overrides in their owning layer. See
[existing settings and migration](../plugins/groundline/references/installation-alignment.md#existing-settings-and-migration).
Personal guidance repair remains an explicit `groundline:align-agent-home` task.

## Add Insights in the same flow

Choose an owner-operated service; no maintainer endpoint or credential is bundled.
Have the owner provide an enrollment token in an owner-private file. Do not put
the token itself in command arguments, shell history, or the repository.

```console
bash groundline-install/install.sh --profile both --insights-endpoint https://insights.example.com --enrollment-token-file /private/enrollment-token --enable-insights
```

```powershell
powershell -File groundline-install/install.ps1 -Profile both -InsightsEndpoint https://insights.example.com -EnrollmentTokenFile C:\private\enrollment-token -EnableInsights
```

Alternatively supply `--insights-profile /private/profile.json` or
`-InsightsProfile C:\private\profile.json` using the existing schema-7 owner
profile. Input files must have owner-private permissions. Endpoint/token inputs
cannot be combined with a profile file.

`--enable-insights` / `-EnableInsights` is explicit consent to aggregate uploads.
Without it, an existing active consent is retained and a fresh installation stays
disabled. Connection verification runs only with active consent. Existing
matching profiles are reused without replacement; a different endpoint or token
requires separate connection review. Identity, cursors, consent, pending events,
and history are never reset by the installer.

The same settings can be completed after package-only installation through the
installed `groundline-insights setup` command with `--endpoint`,
`--enrollment-token-file`, `--enable`, and `--verify`, or `--input` for a private
profile. Calling it with no options reports pending actions without activating
collection. Connection state is shared in the Codex home; collector state and
consent remain scoped to the selected native runtime/execution mode. Inspect the
scope with `worker status`; use documented runtime environment selection in the
[integration guide](integrations.md) when operating another native source.
When the installer selects the App's bundled CLI, Insights setup selects the App
source too. Explicit runtime/originator environment settings take precedence.
The setup receipt names `runtime_family` and `execution_mode`; check these when
passing a custom executable or configuring remote automation.

Codex must review and trust the installed Insights hooks through its native hook
workflow. GroundLine does not approve itself. Complete a real Codex task after
review, then rerun verification. A fresh home with no native activity is
`ACTION_REQUIRED` / first-activity pending, never a fake successful upload or an
incomplete-history reset. Connection authentication, recent hook observation,
server delivery acknowledgement, and current collection health are separate
fields. A historical acknowledgement does not prove a fresh upload in this run.

## Finish and resume

Both installers print a final `groundline-installation` JSON receipt after command
diagnostics. Stage names and exit semantics are identical on all supported OSes:

- Exit 0 / `PASS`: selected installation stages passed.
- Exit 2 / `ACTION_REQUIRED`: package/settings may be complete; review or runtime
  evidence is still required. Resolve the named action and rerun.
- Exit 1 / `FAIL`: a required step failed. Its stage retains the original child
  exit code and already completed stages remain visible.

Native doctor failure is reported separately from settings application. It does
not erase completed steps or trigger automatic rollback. The same installer
rechecks current state on every retry; it never trusts an old completion flag.
Repeated unchanged setup creates no additional config backup. Moving `stable`
after review cannot change the installer's selected commit.

If source replacement or artifact verification fails, the installer attempts
native recovery before setup. It removes only newly added products, reinstates
the previous actual commit, and compares prior versions, enabled states, and
artifact bytes. The receipt reports `source_commit`, `previous_commit`, and
`rollback`; `previous_commit_pinned` means recovery to that immutable revision,
not restoration of the old symbolic ref. A failed recovery requires review before
retrying. These native commands are not atomic across process kill or power loss.

For release qualification, verify all six native targets plus real Codex package
installation, preservation of existing model choices, explicit supported
model/effort selection, partial-failure recovery, and a real hook-to-server receipt. Synthetic provider tests prove the
installer contract, not authenticated Codex behavior or private server delivery.
