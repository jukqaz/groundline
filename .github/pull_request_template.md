## Summary

-

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] Fast PR checks: xtask/CLI contracts and actionlint
- [ ] Required source checks passed; documentation-only changes use scoped checks
- [ ] Release qualification and six-target packages are linked, or explicitly
  pending the release-tag workflow; avoid a duplicate manual release build
- [ ] Skipped platform or live lanes are marked `UNVERIFIED` with the exact
  missing command or evidence.

## Safety

- [ ] No secrets, auth files, raw transcripts, provider sessions, logs, or shell snapshots added.
- [ ] Any mutation boundary is documented.
- [ ] Public docs and LLM guidance remain aligned.
