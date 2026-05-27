## Summary

- 

## Verification

- [ ] `PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json`
- [ ] `PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint`
- [ ] `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v`

## Safety

- [ ] No secrets, auth files, raw transcripts, provider sessions, logs, or shell snapshots added.
- [ ] Any mutation boundary is documented.
- [ ] Public docs and LLM guidance remain aligned.
