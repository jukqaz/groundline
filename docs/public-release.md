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
- Do not tag, push, create a GitHub Release, or change repository visibility
  without explicit user approval in the current session.
- If `docs/maturity-assessment.md` says the ship decision is `hold`, stop
  before publishing unless the user accepts the missing proof for this release.

## Release Evidence

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --plan --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_release_gate.py --json --keep-going --include-docker-execution
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)
PYTHONDONTWRITEBYTECODE=1 python3 scripts/lint.py --json --require-actionlint
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_runtime_layout.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_validate.py --json
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_doctor.py --json --offline --probe-tools
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_radar.py --json --offline --command-sources
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_safety_eval.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_privacy_scan.py --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json --require-installed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform macos --sandbox local --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run_scenarios.py --platform linux --sandbox docker --json
```

Keep the repository private until all required evidence is PASS or an explicit
partial result is documented.

Publishing commands belong in `docs/release-checklist.md` under
`Approval-required Publishing Commands`; they are not part of the read-only
release evidence block.

If git history contains personal author metadata or old file contents that
should not be public, follow `docs/git-history-privacy.md` and publish from a
sanitized history instead of making the existing repository public.
