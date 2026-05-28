# Public Release

Use this checklist before changing repository visibility or publishing a new
release.

## Identity

- `LICENSE` names `GroundLine contributors`.
- Provider manifests use `GroundLine` or `GroundLine contributors`.
- Documentation does not require a private GitHub account.
- Personal paths, local hostnames, and provider runtime state are absent from
  committed files.
- Commit author and committer metadata have been checked for personal or
  company identity exposure.

## Safety

- Secret scans show no committed tokens, auth files, or private keys.
- Default script output does not print full user home paths.
- External tool probes are read-only.
- Radar command sources are opt-in and reject shell strings.
- CI downloads pinned tools and verifies checksums.

## Release Evidence

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

Keep the repository private until all required evidence is PASS or an explicit
partial result is documented.

If git history contains personal author metadata or old file contents that
should not be public, follow `docs/git-history-privacy.md` and publish from a
sanitized history instead of making the existing repository public.
