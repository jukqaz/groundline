## Summary

-

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] Fast PR checks: xtask/CLI contracts and actionlint
- [ ] Full qualification dispatched once after the change was frozen, if release-bound
- [ ] Six-target artifact matrix dispatched once, if release-bound
- [ ] Skipped platform or live lanes are marked `UNVERIFIED` with the exact
  missing command or evidence.

## Safety

- [ ] No secrets, auth files, raw transcripts, provider sessions, logs, or shell snapshots added.
- [ ] Any mutation boundary is documented.
- [ ] Public docs and LLM guidance remain aligned.
