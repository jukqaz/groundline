# Changelog

## 0.20.1

- Clarify that agents must resolve the packaged native executable from the
  installed plugin root instead of assuming Codex adds it to the user shell's
  `PATH`.

## 0.20.0

- Make `plugins/groundline` the only canonical Core package in the monorepo.
- Keep Core offline and zero-hook while sharing versioned Rust contracts and a
  single marketplace/release channel with the optional Insights plugin.
- Remove root package synchronization and verify the canonical package directly.

## 0.19.0

- Establish a clean public, local-first GroundLine core with no lifecycle hook,
  network client, background worker, remote destination, or collector identity.
- Keep bounded local Codex audits, project configuration inventory, deterministic
  efficiency contracts, and six-target native packaging.
- Add a zero-hook provider smoke contract and a public-readiness gate that rejects
  private infrastructure markers, personal paths, and package drift.
- Keep GitHub Actions cost-bounded: pull requests run fast checks, while full
  qualification and release artifacts require explicit manual dispatch.
