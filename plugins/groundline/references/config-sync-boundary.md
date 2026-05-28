# Config Sync Boundary

GroundLine shares source-level capability shape, not runtime state.

## Include Candidates

- AGENTS.md
- skills/
- rules/
- reviewed hook policy notes
- tool profiles
- provider manifests
- capability blueprint
- source registry
- setup profile notes

## Exclude Always

- auth.json
- sessions/
- archived_sessions/
- shell_snapshots/
- plugin caches
- OAuth state
- local databases
- sqlite
- logs
- secret values

GroundLine may inventory excluded paths by name during read-only checks, but it
must not print secret values or copy runtime state into source control.
