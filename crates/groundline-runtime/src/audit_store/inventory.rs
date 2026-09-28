//! Bounded, read-only whole-store diagnostics; separate from window analysis.
use super::{
    AuditStoreError, MAX_ROLLOUT_PATH_BYTES, MAX_THREAD_ROWS, ThreadRow, rollout_roots,
    state_database, thread_rows,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MAX_INVENTORY_ENTRIES: usize = MAX_THREAD_ROWS * 4;
const MAX_INVENTORY_DEPTH: usize = 32;

pub fn inspect_store(codex_home: &Path) -> Result<Value, AuditStoreError> {
    let rows = thread_rows(&state_database(codex_home)?)?;
    let roots = rollout_roots(codex_home)?;
    let mut scanned = 0;
    let mut retained = 0;
    let inventory = store_inventory(
        &rows,
        &roots,
        &mut scanned,
        &mut retained,
        MAX_INVENTORY_ENTRIES,
    );
    Ok(json!({
        "kind":"groundline-codex-store-diagnostic", "schema":1,
        "status":if inventory.complete {"PASS"} else {"PARTIAL"},
        "store_integrity":inventory.report, "source_read_bytes":scanned,
        "scope":"all_local_history_metadata_not_requested_window",
        "window_eligibility_assessed":false, "mutation_performed":false,
        "raw_content_emitted":false, "private_paths_emitted":false,
        "thread_ids_emitted":false, "secret_value_printed":false
    }))
}

// Explicit storage diagnostics do not determine task-window sample eligibility.
pub(super) struct StoreInventory {
    pub(super) report: Value,
    pub(super) complete: bool,
}

pub(super) fn store_inventory(
    rows: &[ThreadRow],
    roots: &[PathBuf],
    scanned: &mut u64,
    retained: &mut u64,
    entry_limit: usize,
) -> StoreInventory {
    let mut pending = roots
        .iter()
        .map(|path| (path.clone(), 0))
        .collect::<Vec<_>>();
    let mut files = BTreeMap::new();
    let mut visited = 0;
    let mut rejected = 0_u64;
    let mut traversal_complete = true;
    while let Some((directory, depth)) = pending.pop() {
        // Never descend through a discovered symlink, including a directory
        // replaced since it was queued. Reader containment is checked separately.
        if !std::fs::symlink_metadata(&directory)
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        {
            rejected += 1;
            traversal_complete = false;
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&directory) else {
            rejected += 1;
            traversal_complete = false;
            continue;
        };
        for entry in entries {
            if visited >= entry_limit {
                traversal_complete = false;
                break;
            }
            visited += 1;
            let Ok(entry) = entry else {
                rejected += 1;
                traversal_complete = false;
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                rejected += 1;
                traversal_complete = false;
                continue;
            };
            let path = entry.path();
            if kind.is_symlink() {
                rejected += 1;
                traversal_complete = false;
            } else if kind.is_dir() {
                if depth >= MAX_INVENTORY_DEPTH {
                    rejected += 1;
                    traversal_complete = false;
                } else {
                    pending.push((path, depth + 1));
                }
            } else if kind.is_file()
                && let Ok(logical) = crate::rollout::logical_path(&path)
            {
                let archived = roots.iter().any(|root| {
                    root.file_name()
                        .is_some_and(|name| name == "archived_sessions")
                        && path.starts_with(root)
                });
                files.insert(logical, archived);
            }
        }
        if visited >= entry_limit {
            if !pending.is_empty() {
                traversal_complete = false;
            }
            break;
        }
    }
    let referenced = rows
        .iter()
        .filter_map(|row| crate::rollout::logical_path(&row.rollout).ok())
        .collect::<BTreeSet<_>>();
    let mut identities = BTreeMap::<String, Vec<PathBuf>>::new();
    let mut unidentified = 0_u64;
    for path in files.keys() {
        let mut identity = None;
        let read = crate::rollout::read_audit_rollout(path, roots, scanned, retained, |metadata| {
            identity = metadata
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty() && id.len() <= MAX_ROLLOUT_PATH_BYTES)
                .map(str::to_owned);
            false // Stop at bounded native metadata; share the existing read budgets.
        });
        if read.is_err() || identity.is_none() {
            unidentified += 1;
        } else if let Some(identity) = identity {
            identities.entry(identity).or_default().push(path.clone());
        }
    }
    let duplicate_groups = identities.values().filter(|paths| paths.len() > 1).count();
    let missing = files
        .keys()
        .filter(|path| !referenced.contains(*path))
        .count();
    let stale = rows
        .iter()
        .filter(|row| {
            crate::rollout::logical_path(&row.rollout)
                .map_or(true, |path| !files.contains_key(&path))
        })
        .count();
    let duplicate_references = rows
        .iter()
        .filter(|row| crate::rollout::logical_path(&row.rollout).is_ok())
        .count()
        .saturating_sub(referenced.len());
    let identity_counts_complete = traversal_complete && unidentified == 0;
    let archived_state_unknown = rows.iter().filter(|row| row.archived.is_none()).count();
    let complete = identity_counts_complete
        && archived_state_unknown == 0
        && missing == 0
        && stale == 0
        && duplicate_groups == 0
        && duplicate_references == 0;
    let partition = |archived: bool| {
        json!({
            "logical_rollout_file_count":files.values().filter(|value| **value == archived).count(),
            "database_row_count":rows.iter().filter(|row| row.archived == Some(archived)).count(),
            "unindexed_rollout_file_count":files.iter().filter(|(path, value)| **value == archived && !referenced.contains(*path)).count(),
            "stale_rollout_path_row_count":if traversal_complete {Some(rows.iter().filter(|row| row.archived == Some(archived) && crate::rollout::logical_path(&row.rollout).map_or(true, |path| !files.contains_key(&path))).count())} else {None},
        })
    };
    StoreInventory {
        complete,
        report: json!({
            "scope":"all_local_history_metadata_not_requested_window",
            "snapshot_atomic":false,
            "traversal_complete":traversal_complete,
            "counts_are_lower_bounds":!identity_counts_complete,
            "identity_counts_complete":identity_counts_complete,
            "entries_visited":visited,"entry_limit":entry_limit,
            "rejected_entry_count":rejected,
            "logical_rollout_file_count":files.len(),"database_row_count":rows.len(),
            "database_archived_state_unknown_row_count":archived_state_unknown,
            "unindexed_rollout_file_count":missing,
            "stale_rollout_path_row_count":if traversal_complete {Some(stale)} else {None},
            "duplicate_database_reference_count":duplicate_references,
            "duplicate_thread_id_count":duplicate_groups,
            "duplicate_thread_id_rollout_count":identities.values().filter(|paths| paths.len() > 1).map(Vec::len).sum::<usize>(),
            "thread_identity_unavailable_file_count":unidentified,
            "active":partition(false),"archived":partition(true),
            "complete":complete,
        }),
    }
}
