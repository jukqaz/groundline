# Release checklist

1. Freeze the release scope and update the manifest, workspace, and changelog
   versions together.
2. Run formatting, workspace tests, Clippy, package synchronization, and
   `verify-source` on the final source.
3. Validate `.codex-plugin/plugin.json` with the Codex plugin validator.
4. Confirm the repository has no hook directory, network dependency, private
   endpoint, personal path, credential material, or unsynchronized package file.
5. Run the manual qualification workflow once.
6. Build all six artifacts only from the final release commit; verify the exact
   target set, manifests, sizes, and SHA-256 checksums.
7. Create the immutable release tag, then advance `stable` only after the tag and
   artifacts are proven.
8. Refresh the marketplace in Codex, upgrade in a new task, and compare source,
   packaged, installed, and runtime fingerprints independently.
