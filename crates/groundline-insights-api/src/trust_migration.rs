//! One explicit storage transition from the last released v5 trust predicate.
//! The staged column is a crash marker as well as a fresh materialized result:
//! existing parts evaluate it from their raw columns, never from stored trust.

use super::{ApiError, ClickHouse, TRUSTED_EVENT_PREDICATE, analysis, projection};
use serde_json::Value;
use sha2::{Digest, Sha256};

// Exact revision-8 expression from the v2026.929.1 source tree, not the older
// migration's previous expression. The frozen catalog is checked below.
pub(super) const PREVIOUS_FINGERPRINT: &str =
    "a230de5bd9799dbb85fc1b1806df95986e032d5cfd93511f6795246bfaa80092";
const TRUST_COLUMN: &str = "trusted_event_v5";
const STAGED_COLUMN: &str = "trusted_event_v5_revalidated";
// A scan that does not finish promptly leaves the guarded views in place and
// requires a separately planned maintenance window; startup never launches a
// background full-table materialization.
const VERIFICATION_SECONDS: u8 = 20;

pub(super) fn expression(analysis_predicate: &str) -> String {
    format!(
        "ifNull(({TRUSTED_EVENT_PREDICATE}) AND ({}) AND ({analysis_predicate}), 0)",
        projection::consistency_sql()
    )
}

pub(super) fn expiry(trusted: &str, retention_days: u64) -> String {
    format!(
        "toDateTime(received_at) + toIntervalDay(if({trusted}, {retention_days}, {}))",
        super::QUARANTINE_RETENTION_DAYS
    )
}

fn fingerprint(expression: &str) -> String {
    format!("{:x}", Sha256::digest(expression.as_bytes()))
}

fn marker(fingerprint: &str) -> String {
    format!("migration:{PREVIOUS_FINGERPRINT}:{fingerprint}:pending")
}

fn matches(column: &Value, expression: &str, comment: &str) -> bool {
    column["type"] == "UInt8"
        && column["default_kind"] == "MATERIALIZED"
        && column["default_expression"] == expression
        && column["comment"] == comment
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Fresh,
    Current,
    Previous { staged: bool },
    PendingSwap,
}

fn classify(
    rows: &[Value],
    current_expression: &str,
    current_fingerprint: &str,
    previous_expression: &str,
) -> Result<State, ApiError> {
    let mut current = None;
    let mut staged = None;
    for row in rows {
        match row["name"].as_str() {
            Some(TRUST_COLUMN) if current.replace(row).is_none() => {}
            Some(STAGED_COLUMN) if staged.replace(row).is_none() => {}
            _ => return Err(ApiError::storage()),
        }
    }
    let pending = marker(current_fingerprint);
    match (current, staged) {
        (None, None) => Ok(State::Fresh),
        (Some(column), None) if matches(column, current_expression, current_fingerprint) => {
            Ok(State::Current)
        }
        (Some(column), None) if matches(column, current_expression, &pending) => {
            Ok(State::PendingSwap)
        }
        (Some(column), None) if matches(column, previous_expression, PREVIOUS_FINGERPRINT) => {
            Ok(State::Previous { staged: false })
        }
        (Some(column), Some(next))
            if matches(column, previous_expression, PREVIOUS_FINGERPRINT)
                && matches(next, current_expression, &pending) =>
        {
            Ok(State::Previous { staged: true })
        }
        _ => Err(ApiError::storage()),
    }
}

async fn columns(db: &ClickHouse) -> Result<Vec<Value>, ApiError> {
    db.json_rows(
        "SELECT name, type, default_kind, default_expression, comment FROM system.columns WHERE database = 'groundline' AND table = 'basic_weekly' AND name IN ('trusted_event_v5', 'trusted_event_v5_revalidated') FORMAT JSONEachRow",
        &[],
    )
    .await
}

async fn expression_ast(db: &ClickHouse, expression: &str) -> Result<Vec<u8>, ApiError> {
    db.request(
        &format!(
            "EXPLAIN AST SELECT {expression} FROM groundline.basic_weekly FORMAT TabSeparatedRaw"
        ),
        &[],
        None,
    )
    .await
}

async fn classify_storage(
    db: &ClickHouse,
    current_expression: &str,
    current_fingerprint: &str,
    previous_expression: &str,
) -> Result<State, ApiError> {
    let mut rows = columns(db).await?;
    let pending = marker(current_fingerprint);
    for column in &mut rows {
        let expected = match (column["name"].as_str(), column["comment"].as_str()) {
            (Some(TRUST_COLUMN), Some(comment)) if comment == PREVIOUS_FINGERPRINT => {
                previous_expression
            }
            (Some(TRUST_COLUMN | STAGED_COLUMN), Some(comment))
                if comment == current_fingerprint || comment == pending =>
            {
                current_expression
            }
            _ => continue,
        };
        let stored = column["default_expression"]
            .as_str()
            .ok_or_else(ApiError::storage)?;
        if stored != expected {
            // ClickHouse prints its parsed expression in system.columns. That
            // spelling differs from the source SQL for complex predicates;
            // compare the parser AST, never just the editable comment.
            if expression_ast(db, stored).await? != expression_ast(db, expected).await? {
                return Err(ApiError::storage());
            }
            column["default_expression"] = Value::from(expected);
        }
    }
    classify(
        &rows,
        current_expression,
        current_fingerprint,
        previous_expression,
    )
}

async fn guard_reads(db: &ClickHouse) -> Result<(), ApiError> {
    // Every published event view depends on basic_current. A stopped migration
    // therefore cannot expose a mix of decisions from the two predicates.
    db.request(
        "CREATE OR REPLACE VIEW groundline.basic_current AS SELECT *, trusted_event_v5 FROM groundline.basic_weekly WHERE 0",
        &[],
        None,
    )
    .await?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Scan {
    row_count: u64,
    source_hash: Value,
}

async fn scan(db: &ClickHouse, column: &str, expression: &str) -> Result<Scan, ApiError> {
    let sql = format!(
        "SELECT count() AS row_count, sum(toUInt128(cityHash64(toString(event_id), toString(collector_id), toString(collection_generation), toString(received_at), idempotency_key, payload_json))) AS source_hash, countIf({column} != ({expression})) AS mismatches FROM groundline.basic_weekly SETTINGS max_execution_time={VERIFICATION_SECONDS}, max_threads=2 FORMAT JSONEachRow"
    );
    let row = db
        .json_row(&sql, &[])
        .await?
        .ok_or_else(ApiError::storage)?;
    if row["mismatches"].as_u64() != Some(0) {
        return Err(ApiError::storage());
    }
    Ok(Scan {
        row_count: row["row_count"].as_u64().ok_or_else(ApiError::storage)?,
        source_hash: row
            .get("source_hash")
            .cloned()
            .ok_or_else(ApiError::storage)?,
    })
}

pub(super) async fn ensure(
    db: &ClickHouse,
    retention_days: u64,
    current_expression: &str,
) -> Result<(), ApiError> {
    let current_fingerprint = fingerprint(current_expression);
    let previous_expression = expression(&analysis::previous_storage_predicate());
    // If a future source change alters this historic expression, require a new
    // reviewed migration instead of accidentally recognizing a different schema.
    if fingerprint(&previous_expression) != PREVIOUS_FINGERPRINT {
        return Err(ApiError::storage());
    }
    let mut state = classify_storage(
        db,
        current_expression,
        &current_fingerprint,
        &previous_expression,
    )
    .await?;
    if state == State::Fresh {
        db.request(
            &format!("ALTER TABLE groundline.basic_weekly ADD COLUMN IF NOT EXISTS {TRUST_COLUMN} UInt8 MATERIALIZED {current_expression} COMMENT '{current_fingerprint}'"),
            &[], None,
        ).await?;
        state = classify_storage(
            db,
            current_expression,
            &current_fingerprint,
            &previous_expression,
        )
        .await?;
    }
    match state {
        State::Current => Ok(()),
        State::Fresh => Err(ApiError::storage()),
        State::Previous { .. } | State::PendingSwap => {
            guard_reads(db).await?;
            // Do not let a pre-migration stored trust bit drive quarantine TTL
            // while old parts are being re-evaluated.
            let direct_expiry = expiry(&format!("({current_expression})"), retention_days);
            db.request(
                &format!("ALTER TABLE groundline.basic_weekly MODIFY TTL {direct_expiry}"),
                &[],
                None,
            )
            .await?;

            if let State::Previous { staged } = state {
                if !staged {
                    db.request(
                        &format!("ALTER TABLE groundline.basic_weekly ADD COLUMN IF NOT EXISTS {STAGED_COLUMN} UInt8 MATERIALIZED {current_expression} COMMENT '{}'", marker(&current_fingerprint)),
                        &[], None,
                    ).await?;
                }
                if classify_storage(
                    db,
                    current_expression,
                    &current_fingerprint,
                    &previous_expression,
                )
                .await?
                    != (State::Previous { staged: true })
                {
                    return Err(ApiError::storage());
                }
                let before = scan(db, STAGED_COLUMN, current_expression).await?;
                // This is one atomic metadata ALTER. The new column has a
                // distinct physical name, so old stored trust cannot reappear.
                db.request(
                    &format!("ALTER TABLE groundline.basic_weekly DROP COLUMN {TRUST_COLUMN}, RENAME COLUMN {STAGED_COLUMN} TO {TRUST_COLUMN}"),
                    &[], None,
                ).await?;
                if classify_storage(
                    db,
                    current_expression,
                    &current_fingerprint,
                    &previous_expression,
                )
                .await?
                    != State::PendingSwap
                    || scan(db, TRUST_COLUMN, current_expression).await? != before
                {
                    return Err(ApiError::storage());
                }
            } else if classify_storage(
                db,
                current_expression,
                &current_fingerprint,
                &previous_expression,
            )
            .await?
                != State::PendingSwap
            {
                return Err(ApiError::storage());
            } else {
                scan(db, TRUST_COLUMN, current_expression).await?;
            }
            db.request(
                &format!("ALTER TABLE groundline.basic_weekly COMMENT COLUMN {TRUST_COLUMN} '{current_fingerprint}'"),
                &[], None,
            ).await?;
            if classify_storage(
                db,
                current_expression,
                &current_fingerprint,
                &previous_expression,
            )
            .await?
                != State::Current
            {
                return Err(ApiError::storage());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn column(name: &str, expression: &str, comment: &str) -> Value {
        json!({"name":name,"type":"UInt8","default_kind":"MATERIALIZED",
            "default_expression":expression,"comment":comment})
    }

    #[test]
    fn only_canonical_previous_and_exact_pending_states_can_migrate() {
        let old = expression(&analysis::previous_storage_predicate());
        assert_eq!(fingerprint(&old), PREVIOUS_FINGERPRINT);
        let new = expression(&analysis::predicate());
        let hash = fingerprint(&new);
        assert_ne!(hash, PREVIOUS_FINGERPRINT);
        let prior = column(TRUST_COLUMN, &old, PREVIOUS_FINGERPRINT);
        let next = column(STAGED_COLUMN, &new, &marker(&hash));
        assert_eq!(classify(&[], &new, &hash, &old).unwrap(), State::Fresh);
        assert_eq!(
            classify(std::slice::from_ref(&prior), &new, &hash, &old).unwrap(),
            State::Previous { staged: false }
        );
        assert_eq!(
            classify(&[prior.clone(), next.clone()], &new, &hash, &old).unwrap(),
            State::Previous { staged: true }
        );
        assert_eq!(
            classify(
                &[column(TRUST_COLUMN, &new, &marker(&hash))],
                &new,
                &hash,
                &old
            )
            .unwrap(),
            State::PendingSwap
        );
        assert_eq!(
            classify(&[column(TRUST_COLUMN, &new, &hash)], &new, &hash, &old).unwrap(),
            State::Current
        );
        for bad in [
            column(TRUST_COLUMN, &new, PREVIOUS_FINGERPRINT),
            column(TRUST_COLUMN, "toUInt8(1)", &hash),
            column(TRUST_COLUMN, &old, "unknown-schema"),
            column(STAGED_COLUMN, &new, &marker(&hash)),
        ] {
            assert!(classify(&[bad], &new, &hash, &old).is_err());
        }
        let mut wrong_kind = prior.clone();
        wrong_kind["default_kind"] = json!("DEFAULT");
        assert!(classify(&[wrong_kind], &new, &hash, &old).is_err());
        let mut wrong_type = prior;
        wrong_type["type"] = json!("String");
        assert!(classify(&[wrong_type], &new, &hash, &old).is_err());
    }
}
