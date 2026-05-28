# Security

GroundLine is designed as a read-only control plane by default. It should guide
agent work without collecting credentials, raw transcripts, prompt logs, or
provider runtime state.

## Supported Versions

Security fixes target the current release line. Older tags are historical
release points and should be updated before use.

## Reporting

Do not open a public issue that contains secrets, tokens, private hostnames,
raw transcripts, or provider auth files.

Preferred reporting path:

1. Use GitHub Security Advisories for this repository when available.
2. If advisories are unavailable, contact the maintainer through a private
   channel and include a minimal reproduction.

Reports should include:

- affected version or commit
- affected script, skill, or document path
- exact command needed to reproduce
- whether the issue can expose secrets, mutate external services, or rewrite
  local provider state

## Security Boundaries

- Scripts must not print secret values.
- Scripts must not copy provider auth files, sessions, logs, caches, or shell
  snapshots into source control.
- Network and command-source checks must be explicit opt-ins.
- External commands must be argument arrays, not shell strings.
- Default home paths should be displayed with `~` unless the operator passes an
  explicit test home.
