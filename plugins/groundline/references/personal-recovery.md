# Recover existing personal guidance

The fixed-rule personal trial engine is retired. New guidance trials, automatic
evaluation and a parallel improvement ledger are no longer created. Current
work uses [delivery evidence](delivery-evidence.md) and
[evidence comparison](evidence-routing.md).

Existing private state is not deleted or silently migrated. Use the explicit
state directory with `groundline personal status --state-dir <directory> --json`.
For a requested rollback use `groundline personal rollback --state-dir <directory> --json`.
Inspect the installed command's help before use. Recovery reads the recorded
schema and restores only the tool-owned guidance when it still matches the
recorded trial. User edits, unsupported state and concurrent operations are
preserved or rejected, never overwritten to make recovery appear successful.

Restoring a private guidance file is not proof that a native task stopped using
its previously loaded instructions. Verify any user-managed integration separately.
No cleanup of state directories, archives, native tasks or credentials is implied.
