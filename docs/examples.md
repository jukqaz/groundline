# Examples

Inspect a repository without emitting configuration values:

```console
groundline project-audit --repo . --json
```

Assess whether a frozen batch is ready for implementation:

```json
{
  "kind": "groundline-batch-input",
  "schema": 1,
  "phase": "freeze",
  "goal": {
    "status": "none",
    "objective_present": true,
    "user_requested": false
  },
  "signals": {
    "scope_locked": true,
    "new_observations": 2
  }
}
```

```console
groundline efficiency batch --input batch.json --json
```
