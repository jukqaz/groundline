# Capability Blueprint

GroundLine stores a capability blueprint, not complete runtime config.

The blueprint describes the desired shape of the agent environment:

- setup profile
- agent guidance files
- skill expectations
- rules and hook policy
- provider manifests
- recommended tool profiles
- source registry entries
- doctor requirements
- radar requirements

The blueprint is used by doctor and radar to produce recommendations and
upgrade packets. It does not apply changes by itself.

## Profiles

- `standalone-groundline`: minimal GroundLine loop.
- `companion-superpowers`: use Superpowers for planning, TDD, debugging, and review.
- `external-stack`: emit handoff packets for another workflow tool.
- `strict-local`: avoid external calls and secret access.
- `standard-tools`: prefer GitHub, Context7, and Exa when available.

## Boundary

This is not complete runtime config. Secrets, sessions, logs, caches, OAuth
material, local databases, and provider runtime state stay outside the
blueprint.
