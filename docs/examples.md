# Examples

## Installation profiles

Register the marketplace once. Install Core, Insights, or both according to the
chosen profile; neither plugin is an implicit dependency of the other.

```console
codex plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
codex plugin add groundline@groundline --json
codex plugin add groundline-insights@groundline --json
```

See [integrations and installation profiles](integrations.md) before enabling
Insights or connecting owner-operated infrastructure.

## Core CLI

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

This command is advisory and performs no orchestration. A changed outcome,
repository, or permission boundary returns `reconcile` for the current task.
An alternative approach returns `stay`; a side question returns `side` so the
agent can answer and resume. Only explicit `new_task_requested` or
`fork_requested` signals recommend `new_task` or `fork`. These signals record
the user's request; they do not authorize unrelated writes. Reconciled scope
must be verified before an existing Goal can be marked complete.
