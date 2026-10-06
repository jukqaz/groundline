//! Bounded descriptive model observations; no task or outcome attribution.
use groundline_contracts::insights::MAX_MODEL_REPORT_PAIRS;
use groundline_contracts::insights::analysis::TOKEN_FIELDS;
use serde_json::{Value, json};

use super::{analysis, count, ratio};

const PERIOD: &str = "ifNull(period_end,generated_at)>parseDateTimeBestEffort({start:String}) AND ifNull(period_end,generated_at)<=parseDateTimeBestEffort({end:String})";

fn sums(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|f| format!("sum(toUInt128(observations.{f})) AS {f}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn pattern_view() -> String {
    let zero_tokens = TOKEN_FIELDS
        .iter()
        .map(|f| format!("toUInt64(0) AS {f}"))
        .collect::<Vec<_>>()
        .join(",");
    let branches=["root","delegated"].map(|component| {
        let observed=analysis::response_observed(component);
        let identity=analysis::observed_identity("JSONExtractString(item,'model_family')", &observed);
        format!("SELECT {}, '{component}' AS component, JSONExtractString(item,'model_family') AS model_family, {identity} AS model_identity, JSONExtractString(item,'effort') AS effort, JSONExtractUInt(item,'count') AS context_count, toUInt8({observed}) AS response_observed, if({observed},toNullable(toUInt64(0)),NULL) AS observed_response_count, {zero_tokens} FROM groundline.basic_active ARRAY JOIN JSONExtractArrayRaw(payload_json,'metrics','{component}','model_effort') AS item",analysis::DIMENSIONS)
    });
    let source = format!(
        "{} UNION ALL SELECT {},component,model_family,model_identity,effort,toUInt64(0) AS context_count,response_observed,observed_response_count,{} FROM groundline.model_usage",
        branches.join(" UNION ALL "),
        analysis::DIMENSIONS,
        TOKEN_FIELDS.join(",")
    );
    let positive = TOKEN_FIELDS
        .iter()
        .map(|f| format!("{f}>0"))
        .collect::<Vec<_>>()
        .join(" OR ");
    format!(
        "CREATE OR REPLACE VIEW groundline.model_usage_patterns AS SELECT {},component,model_family,model_identity,effort,sum(toUInt128(observations.context_count)) AS context_count,toUInt8(max(observations.response_observed)) AS response_observed,if(max(observations.response_observed)=1,toNullable(sum(toUInt128(ifNull(observations.observed_response_count,0)))),NULL) AS observed_response_count,{} FROM ({source}) AS observations GROUP BY {},component,model_family,model_identity,effort HAVING context_count>0 OR ifNull(observed_response_count,0)>0 OR {positive}",
        analysis::DIMENSIONS,
        sums(TOKEN_FIELDS),
        analysis::DIMENSIONS
    )
}

pub(super) fn coverage_view() -> String {
    let branches=["root","delegated"].map(|component| {
        let at=format!("JSONExtractRaw(payload_json,'analysis','{component}')");
        let contexts=format!("JSONExtractArrayRaw(payload_json,'metrics','{component}','model_effort')");
        let observed=analysis::response_observed(component);
        let responses=["observed_response_count","unattributed_response_count","overflow_response_count"].map(|f| format!("if({observed},toNullable(JSONExtractUInt({at},'{f}')),NULL) AS {f}")).join(",");
        let tokens=TOKEN_FIELDS.iter().map(|f| format!("JSONExtractUInt(payload_json,'metrics','{component}','usage','{f}') AS {f}")).collect::<Vec<_>>().join(",");
        format!("SELECT {},'{component}' AS component,arraySum(arrayMap(m -> JSONExtractUInt(m,'count'),{contexts})) AS context_count,arraySum(arrayMap(m -> if(JSONExtractString(m,'model_family')='overflow',JSONExtractUInt(m,'count'),0),{contexts})) AS overflow_context_count,arraySum(arrayMap(m -> if(JSONExtractString(m,'model_family')='unknown',JSONExtractUInt(m,'count'),0),{contexts})) AS unknown_context_count,toUInt8({observed}) AS response_observed,{responses},arraySum(arrayMap(b -> JSONExtractUInt(b,'tokens','total_tokens'),JSONExtractArrayRaw({at},'buckets'))) AS attributed_total_tokens,total_tokens-attributed_total_tokens AS unattributed_total_tokens,{tokens} FROM groundline.basic_active",analysis::DIMENSIONS)
    });
    format!(
        "CREATE OR REPLACE VIEW groundline.model_usage_coverage AS {}",
        branches.join(" UNION ALL ")
    )
}

// Rank dimensions before aggregating, then merge overflow for each event. This
// preserves both every counter and distinct contributing-event counts.
pub(super) fn pattern_query() -> String {
    let counters = TOKEN_FIELDS
        .iter()
        .copied()
        .chain(["context_count"])
        .collect::<Vec<_>>();
    let aggregate = sums(&counters);
    format!(
        "WITH source AS (SELECT * FROM groundline.model_usage_patterns AS observations WHERE {PERIOD}), dimensions AS (SELECT component,purpose,model_family,model_identity,effort,row_number() OVER (PARTITION BY component,purpose ORDER BY if(model_family='overflow',1,0),model_family,model_identity,effort) AS model_rank FROM source GROUP BY component,purpose,model_family,model_identity,effort), events AS (SELECT event_id,component,purpose,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'overflow',model_family) AS bounded_model,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'overflow',model_identity) AS bounded_identity,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'unknown',effort) AS bounded_effort,max(observations.response_observed) AS response_observed,if(max(observations.response_observed)=1,toNullable(sum(toUInt128(ifNull(observations.observed_response_count,0)))),NULL) AS observed_response_count,{aggregate} FROM source AS observations INNER JOIN dimensions USING (component,purpose,model_family,model_identity,effort) GROUP BY event_id,component,purpose,bounded_model,bounded_identity,bounded_effort) SELECT component,purpose,bounded_model AS model_family,bounded_identity AS model_identity,bounded_effort AS effort,count() AS event_count,countIf(observations.response_observed=1) AS response_observed_event_count,countIf(observations.response_observed=0) AS response_unobserved_event_count,if(countIf(observations.response_observed=1)>0,toNullable(sum(toUInt128(ifNull(observations.observed_response_count,0)))),NULL) AS observed_response_count,{aggregate} FROM events AS observations GROUP BY component,purpose,bounded_model,bounded_identity,bounded_effort ORDER BY component,purpose,model_family,model_identity,effort FORMAT JSONEachRow"
    )
}

pub(super) fn coverage_query() -> String {
    let counters = TOKEN_FIELDS
        .iter()
        .copied()
        .chain([
            "context_count",
            "overflow_context_count",
            "unknown_context_count",
            "attributed_total_tokens",
            "unattributed_total_tokens",
        ])
        .collect::<Vec<_>>();
    let responses = [
        "observed_response_count",
        "unattributed_response_count",
        "overflow_response_count",
    ]
    .map(|f| {
        format!("if(countIf(observations.response_observed=1)>0,toNullable(sum(toUInt128(ifNull(observations.{f},0)))),NULL) AS {f}")
    })
    .join(",");
    format!(
        "SELECT component,purpose,count() AS event_count,countIf(observations.response_observed=1) AS response_observed_event_count,countIf(observations.response_observed=0) AS response_unobserved_event_count,{responses},{} FROM groundline.model_usage_coverage AS observations WHERE {PERIOD} GROUP BY component,purpose ORDER BY component,purpose FORMAT JSONEachRow",
        sums(&counters)
    )
}

pub(super) fn context_query() -> String {
    format!(
        "WITH dimensions AS (SELECT model_family,model_identity,effort,sum(toUInt128(observations.context_count)) AS context_count,row_number() OVER (ORDER BY if(model_family='overflow',1,0),model_family,model_identity,effort) AS model_rank FROM groundline.model_usage_patterns AS observations WHERE {PERIOD} AND component='root' AND observations.context_count>0 GROUP BY model_family,model_identity,effort), bounded AS (SELECT *,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'overflow',model_family) AS bounded_model,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'overflow',model_identity) AS bounded_identity,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'unknown',effort) AS bounded_effort FROM dimensions) SELECT bounded_model AS model_family,bounded_identity AS model_identity,bounded_effort AS effort,sum(toUInt128(observations.context_count)) AS context_count FROM bounded AS observations GROUP BY bounded_model,bounded_identity,bounded_effort ORDER BY model_family,model_identity,effort FORMAT JSONEachRow"
    )
}

pub(super) fn token_query() -> String {
    let aggregate = sums(TOKEN_FIELDS);
    format!(
        "WITH dimensions AS (SELECT component,model_family,model_identity,effort,{aggregate},row_number() OVER (PARTITION BY component ORDER BY if(model_family='overflow',1,0),model_family,model_identity,effort) AS model_rank FROM groundline.model_usage AS observations WHERE {PERIOD} GROUP BY component,model_family,model_identity,effort), bounded AS (SELECT *,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'overflow',model_family) AS bounded_model,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'overflow',model_identity) AS bounded_identity,if(model_family='overflow' OR model_rank>{MAX_MODEL_REPORT_PAIRS},'unknown',effort) AS bounded_effort FROM dimensions) SELECT component,bounded_model AS model_family,bounded_identity AS model_identity,bounded_effort AS effort,{aggregate} FROM bounded AS observations GROUP BY component,bounded_model,bounded_identity,bounded_effort ORDER BY component,model_family,model_identity,effort FORMAT JSONEachRow"
    )
}

pub(super) fn context_coverage(rows: &[Value]) -> Value {
    let total = rows
        .iter()
        .map(|r| count(r, "context_count"))
        .fold(0_u64, u64::saturating_add);
    let overflow = rows
        .iter()
        .filter(|r| r["model_family"] == "overflow")
        .map(|r| count(r, "context_count"))
        .fold(0_u64, u64::saturating_add);
    let unknown = rows
        .iter()
        .filter(|r| r["model_family"] == "unknown")
        .map(|r| count(r, "context_count"))
        .fold(0_u64, u64::saturating_add);
    json!({"row_limit":MAX_MODEL_REPORT_PAIRS,"context_count":total,"overflow_context_count":overflow,"unknown_context_count":unknown,"dimension_coverage":ratio(total.saturating_sub(overflow).saturating_sub(unknown),total)})
}

pub(super) fn token_coverage(rows: &[Value]) -> Vec<Value> {
    ["root","delegated"].map(|component| {
        let selected=rows.iter().filter(|r| r["component"]==component).collect::<Vec<_>>();
        let total=selected.iter().map(|r| count(r,"total_tokens")).fold(0_u64, u64::saturating_add);
        let overflow=selected.iter().filter(|r| r["model_family"]=="overflow").map(|r| count(r,"total_tokens")).fold(0_u64, u64::saturating_add);
        let unknown=selected.iter().filter(|r| r["model_family"]=="unknown").map(|r| count(r,"total_tokens")).fold(0_u64, u64::saturating_add);
        json!({"component":component,"row_limit":MAX_MODEL_REPORT_PAIRS,"total_tokens":total,"overflow_total_tokens":overflow,"unknown_total_tokens":unknown,"dimension_coverage":ratio(total.saturating_sub(overflow).saturating_sub(unknown),total)})
    }).to_vec()
}

pub(super) fn report(rows: &[Value], coverage: &[Value]) -> Value {
    let coverage = coverage
        .iter()
        .map(|c| {
            let mut c = c.clone();
            let cohort = rows
                .iter()
                .filter(|r| r["component"] == c["component"] && r["purpose"] == c["purpose"])
                .collect::<Vec<_>>();
            let overflow = cohort
                .iter()
                .filter(|r| r["model_family"] == "overflow")
                .copied()
                .collect::<Vec<_>>();
            let overflow_contexts = overflow
                .iter()
                .map(|r| count(r, "context_count"))
                .fold(0_u64, u64::saturating_add);
            c["overflow_context_count"] = json!(overflow_contexts);
            c["unknown_context_count"] = json!(
                cohort
                    .iter()
                    .filter(|r| r["model_family"] == "unknown")
                    .map(|r| count(r, "context_count"))
                    .fold(0_u64, u64::saturating_add)
            );
            c["context_dimension_coverage"] = json!(ratio(
                count(&c, "context_count")
                    .saturating_sub(overflow_contexts)
                    .saturating_sub(count(&c, "unknown_context_count")),
                count(&c, "context_count")
            ));
            let observed = c["observed_response_count"].as_u64();
            let unattributed = c["unattributed_response_count"].as_u64();
            c["attributed_response_count"] = json!(
                observed
                    .zip(unattributed)
                    .and_then(|(o, u)| o.checked_sub(u))
            );
            c["response_attribution_coverage"] = json!(
                observed
                    .zip(unattributed)
                    .and_then(|(o, u)| o.checked_sub(u).and_then(|a| ratio(a, o).as_f64()))
            );
            c["token_attribution_coverage"] = json!(ratio(
                count(&c, "attributed_total_tokens"),
                count(&c, "total_tokens")
            ));
            c["report_overflow_response_count"] = json!(observed.map(|_| {
                overflow
                    .iter()
                    .map(|r| count(r, "observed_response_count"))
                    .fold(0_u64, u64::saturating_add)
            }));
            c["report_overflow_total_tokens"] = json!(
                overflow
                    .iter()
                    .map(|r| count(r, "total_tokens"))
                    .fold(0_u64, u64::saturating_add)
            );
            c
        })
        .collect::<Vec<_>>();
    json!({"basis":"owned_response_turn_link","context_count_basis":"turn_context_observations","response_count_basis":"owned_usage_response_observations","task_count_attributed":false,"row_limit_per_cohort":MAX_MODEL_REPORT_PAIRS,"rows":rows,"coverage":coverage})
}
