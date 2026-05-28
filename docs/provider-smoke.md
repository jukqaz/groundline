# Provider Smoke

Provider smoke checks prove that GroundLine has the manifests and path plan
needed for Codex, Claude Code, and Antigravity.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json
```

Expected safety fields:

- `mutation_performed=false`
- `real_home_touched=false`
- `source_package.skill_index_consistent=true`
- each provider reports `manifest_present=true`
- each provider includes a read-only `runtime_probe`
- default home paths are displayed with `~`

Use `--home` with a temporary directory when testing install plans:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --home /tmp/groundline-home --json
```

The smoke command does not install, copy, link, or rewrite runtime state.

When a fake or real provider target already contains a GroundLine checkout, the
runtime probe reports target manifest presence, target skills presence, and
whether the target skill count matches the source package. It still performs no
mutation.

Use `docs/provider-dogfood.md` and `scripts/groundline_dogfood.py` when a staged
package plus shared scenario contract check is required.
