# Native upgrade boundary

Codex owns marketplace and plugin changes. Source tags contain no binaries;
qualified `stable` commits add both checksummed, attested native binary trees.
Core and Insights install separately, but refreshing their shared marketplace
can update both **already installed** products. `--profile core` does not isolate
an installed Insights collector.

## Checked upgrade

Use the latest reviewed, complete `stable` distribution's `install.sh` for each
release. It validates candidate artifacts and, when Insights is installed or
has local state, runs the candidate's read-only `worker check-server` before
native writes. No profile means no request; malformed profiles, incompatible
APIs, and transport failures block the update. Upgrade the owner API before
collectors whose accepted event dimensions change.

The installer uses native marketplace remove/add at the reviewed commit, then
native upgrade. It preserves the official HTTPS/SSH transport, installed products,
and enabled flags. Only newly selected products use `plugin add`; re-adding an
existing disabled plugin would enable it. It neither edits caches nor resolves
versions through a separate updater.

App Refresh and `codex plugin marketplace upgrade groundline --json` stay on
that immutable commit. A new release requires its reviewed distribution and
installer. Direct native commands bypass the candidate preflight. Numeric
versions identify manifests and cache directories; date-and-letter titles are
display names. Repeating an unchanged version rechecks artifacts and setup
without resetting owner state.

## Evidence and recovery

Verify separately: source revision/tag, packaged manifest/fingerprint, installed
manifest/checksum, and a fresh-task runtime result. One lane does not prove another.
For requested settings or guidance repair, continue with
[installation alignment](installation-alignment.md); refresh alone proves no repair.

Caught source-transition or verification failures attempt native recovery and
check the previous artifact bytes and installed flags. `previous_commit_pinned`
means recovery pinned the previous actual SHA: native JSON does not expose the
old symbolic ref. Unsupported sources or inconsistent snapshots fail before
writes. Termination or power loss can interrupt this multi-command transition;
failed recovery requires review, not a state reset.

The isolated native regression checks an older manifest, stable moving beyond
the reviewed commit, exact-commit installation, source-add/upgrade failure
recovery, repeated installation, and preserved settings, disabled flags, and
consent. Both fixture versions use the current binary. Old-runtime behavior,
authenticated tasks, and owner-API delivery remain unverified by this fixture.
