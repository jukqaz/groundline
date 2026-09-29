# Installed command resolution

Read this reference when a skill needs a GroundLine or Codex CLI command.
Do not run every example as a routine preflight.

Resolve `GROUNDLINE_ROOT` from the installed skill file: the directory two
levels above its containing skill directory. Do not use the current repository
or another plugin's root. Resolve `GROUNDLINE_BIN` under that root using the
verified execution platform:

| System | Architecture | Relative executable |
| --- | --- | --- |
| macOS | ARM64 | `bin/aarch64-apple-darwin/groundline` |
| macOS | x86_64 | `bin/x86_64-apple-darwin/groundline` |
| Linux | ARM64 | `bin/aarch64-unknown-linux-musl/groundline` |
| Linux | x86_64 | `bin/x86_64-unknown-linux-musl/groundline` |

Use the native host architecture, not an emulated shell's architecture. Check
that the file exists and is executable before invoking it. If the installed
artifact is missing, report that lane unavailable; do not silently run a source
build or an unrelated binary on `PATH`.

In skill examples, replace the bare `groundline` name with this absolute path.
Quote paths. POSIX shells can use `"$GROUNDLINE_BIN" <arguments>`. The examples
describe arguments; no additional language runtime is required.

For Codex, inspect the actual App installation or active execution host to
resolve `CODEX_BIN`. Use that same absolute executable for version, features,
catalog, and doctor checks. An App installation path is OS-specific; do not
assume a macOS path exists on Linux. On a CLI-only host, use the
resolved CLI and label App evidence unavailable. Inspect command help before
using version-dependent options. Treat PATH CLI evidence separately when it
differs from the App's bundled executable.

On macOS, use the actual application bundle metadata and its bundled executable;
the display name does not determine the `.app` directory name. Inspect the
standard system/user Applications directories narrowly and match the application's
bundle identifier before resolving `Contents/Resources/codex-cli/bin/codex`
(or `Contents/Resources/codex` in an older verified bundle). Do not declare the
App absent after checking one guessed directory. Capture resolved paths privately;
report the executable's version and whether it is App-bundled or PATH-resolved.

Use only checks needed for the decision. Do not dump prompt input, environment
values, auth, or private provider state. Read selected safe fields when needed.
