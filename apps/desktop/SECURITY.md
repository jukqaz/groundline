# Desktop dependency policy

This preview ships only `aarch64-apple-darwin`. The desktop dependency gate uses
`apps/desktop/deny.toml`; the root policy continues to check Core, Insights, and
API dependencies for all six native targets. Linux GTK and its build-only
`proc-macro-error`/`target-lexicon` dependencies are not part of this macOS
artifact. A future desktop platform requires its own qualification before release.

## Unshipped Linux dependency advisory

The cross-platform lockfile includes Linux GTK3's `glib 0.18.5`, affected by
[RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).
Its VariantStrIter can violate Rust's pointer requirements and crash. The
patched `glib >=0.20` is outside the GTK3 dependency range used by stable Tauri.
This is a known unresolved dependency issue, not a maintenance-only notice.

`cargo tree --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --target
aarch64-apple-darwin --all-features -i glib` has no matching dependency;
the Linux target shows the GTK3 path. No `glib` code is compiled into the shipped
macOS preview, Core, Insights CLI, or the API. The desktop build script rejects
every target other than macOS Apple Silicon until that platform is separately
qualified. Do not distribute a Linux desktop build from this lockfile. Keep the
repository alert open and require a compatible patched dependency before adding
Linux support; this advisory is not added to the ignore list.

## Reviewed maintenance notices

Checked 2026-09-10 against the registry's current stable Tauri 2.11.5,
tauri-utils 2.9.3, and the live RustSec database. Tauri still requires
`urlpattern 0.3`, which uses `unic-ucd-ident 0.9` and four companion Unicode
crates. urlpattern 0.6 exists but is outside Tauri's compatible dependency range;
no upstream fork or forced incompatible replacement is shipped here.

The five individually listed RustSec exceptions are **unmaintained notices**,
not findings of exploitation, unsoundness, or a patched security bug:

- [unic-char-range / RUSTSEC-2025-0075](https://rustsec.org/advisories/RUSTSEC-2025-0075.html)
- [unic-common / RUSTSEC-2025-0080](https://rustsec.org/advisories/RUSTSEC-2025-0080.html)
- [unic-char-property / RUSTSEC-2025-0081](https://rustsec.org/advisories/RUSTSEC-2025-0081.html)
- [unic-ucd-version / RUSTSEC-2025-0098](https://rustsec.org/advisories/RUSTSEC-2025-0098.html)
- [unic-ucd-ident / RUSTSEC-2025-0100](https://rustsec.org/advisories/RUSTSEC-2025-0100.html)

The reviewed consumer is tauri-utils `acl::RemoteUrlPattern`, whose parser uses
these Unicode tables for pattern variable names. GroundLine enables only the
bundled `main` capability with no remote URL patterns. The app's IPC cannot
accept a remote capability definition. A regression test enforces this boundary.
This limits exposure but does not establish that every dependency is defect-free.

No general `unmaintained = "none"`, vulnerability suppression, untrusted registry,
or Git dependency exception is allowed. Every new advisory still fails the gate.
Unused exception IDs also fail so a Tauri update cannot leave silent stale
exceptions. Reassess this disposition for each release and before adding remote
capabilities or a new desktop platform. Remove these exceptions when stable
Tauri adopts the maintained URL pattern dependency.
