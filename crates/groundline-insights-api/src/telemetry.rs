//! Bounded operational counters. No request content or identity is retained.
use super::*;

#[derive(Default)]
struct Counter {
    requests: u64,
    duration_us: u64,
    max_duration_us: u64,
}
#[derive(Default)]
pub(super) struct Metrics {
    values: BTreeMap<(&'static str, &'static str), Counter>,
}

fn route(path: &str) -> &'static str {
    match path {
        "/healthz" => "health",
        "/v1/enroll" => "enroll",
        "/v1/events" => "ingest",
        "/v3/reports/weekly" => "report",
        v if v.starts_with("/v1/collectors/") => "collector_admin",
        _ => "other",
    }
}

pub(super) async fn observe(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let route = route(request.uri().path());
    let started = Instant::now();
    let response = next.run(request).await;
    let outcome = match response.status().as_u16() {
        200..=299 => "success",
        401 | 403 => "unauthorized",
        429 => "limited",
        400..=499 => "rejected",
        500..=599 => "server_error",
        _ => "other",
    };
    let elapsed = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    if let Ok(mut metrics) = state.metrics.lock() {
        let c = metrics.values.entry((route, outcome)).or_default();
        c.requests = c.requests.saturating_add(1);
        c.duration_us = c.duration_us.saturating_add(elapsed);
        c.max_duration_us = c.max_duration_us.max(elapsed);
    }
    response
}

pub(super) async fn ensure(db: &ClickHouse) -> Result<(), ApiError> {
    for sql in [
        "CREATE TABLE IF NOT EXISTS groundline.collector_diagnostics (collector_id UUID, received_at DateTime64(3,'UTC'), retry_attempts UInt8, operator_required UInt8, previous_cycle_pending UInt64) ENGINE = ReplacingMergeTree(received_at) ORDER BY collector_id TTL received_at + INTERVAL 30 DAY",
        "CREATE TABLE IF NOT EXISTS groundline.api_observations (sample_id UUID, period_start DateTime64(3,'UTC'), period_end DateTime64(3,'UTC'), received_at DateTime64(3,'UTC'), route LowCardinality(String), outcome LowCardinality(String), requests UInt64, duration_us UInt64, max_duration_us UInt64, dropped_requests UInt64) ENGINE = ReplacingMergeTree(received_at) PARTITION BY toYYYYMM(period_end) ORDER BY (sample_id,route,outcome) TTL period_end + INTERVAL 30 DAY",
        "CREATE TABLE IF NOT EXISTS groundline.lifecycle (entry_id UUID, occurred_at DateTime64(3,'UTC'), kind LowCardinality(String), collector_id Nullable(UUID), version String) ENGINE = ReplacingMergeTree(occurred_at) ORDER BY entry_id TTL occurred_at + INTERVAL 365 DAY",
    ] {
        db.request(sql, &[], None).await?;
    }
    Ok(())
}

pub(super) async fn lifecycle(
    db: &ClickHouse,
    kind: &str,
    collector: Option<Uuid>,
    version: &str,
    id: Uuid,
    at: &str,
) -> Result<(), ApiError> {
    let mut body=serde_json::to_vec(&json!({"entry_id":id,"occurred_at":at,"kind":kind,"collector_id":collector,"version":version})).map_err(|_|ApiError::storage())?;
    body.push(b'\n');
    db.request(
        "INSERT INTO groundline.lifecycle FORMAT JSONEachRow",
        &[],
        Some(body),
    )
    .await?;
    Ok(())
}

pub(super) async fn run(state: AppState) {
    let mut start = Utc::now();
    let mut dropped = 0_u64;
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let end = Utc::now();
        let values = match state.metrics.lock() {
            Ok(mut m) => std::mem::take(&mut m.values),
            Err(_) => return,
        };
        let count = values.values().map(|c| c.requests).sum::<u64>();
        let sample = Uuid::new_v4();
        let mut rows = Vec::new();
        for ((route, outcome), c) in values.into_iter().chain(std::iter::once((
            ("heartbeat", "success"),
            Counter::default(),
        ))) {
            rows.push(json!({"sample_id":sample,"period_start":start.to_rfc3339_opts(SecondsFormat::Millis,true),
                "period_end":end.to_rfc3339_opts(SecondsFormat::Millis,true),"received_at":end.to_rfc3339_opts(SecondsFormat::Millis,true),
                "route":route,"outcome":outcome,"requests":c.requests,"duration_us":c.duration_us,"max_duration_us":c.max_duration_us,
                "dropped_requests":if route=="heartbeat" {dropped} else {0}}));
        }
        let mut body = Vec::new();
        for row in rows {
            let Ok(encoded) = serde_json::to_vec(&row) else {
                return;
            };
            body.extend_from_slice(&encoded);
            body.push(b'\n');
        }
        let mut delivered = false;
        for attempt in 0..3 {
            if state
                .clickhouse
                .request(
                    "INSERT INTO groundline.api_observations FORMAT JSONEachRow",
                    &[],
                    Some(body.clone()),
                )
                .await
                .is_ok()
            {
                delivered = true;
                break;
            }
            if attempt < 2 {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
        dropped = if delivered {
            0
        } else {
            dropped.saturating_add(count)
        };
        start = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_routes_never_become_metric_labels() {
        assert_eq!(route("/v1/collectors/private-id"), "collector_admin");
        assert_eq!(route("/private-token"), "other");
        assert_eq!(route("/v1/events"), "ingest");
    }
}
