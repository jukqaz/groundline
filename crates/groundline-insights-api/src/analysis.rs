//! SQL projections for optional, explicitly sourced analysis observations.
use groundline_contracts::insights::analysis::TOKEN_FIELDS;
use groundline_contracts::model::{EFFORTS, MAX_MODEL_CONTEXTS, MODEL_FAMILIES};

fn keys(expression: &str, fields: &[&str]) -> String {
    let mut fields = fields.to_vec();
    fields.sort_unstable();
    format!(
        "arraySort(JSONExtractKeys({expression})) = [{}]",
        fields
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn tokens(expression: &str) -> String {
    format!(
        "({} AND {})",
        keys(expression, TOKEN_FIELDS),
        TOKEN_FIELDS
            .iter()
            .map(|field| format!("(JSONType({expression},'{field}') IN ('Int64','UInt64') AND NOT startsWith(JSONExtractRaw({expression},'{field}'),'-'))"))
            .collect::<Vec<_>>()
            .join(" AND ")
    )
}

fn predicate_for_catalog(models: &[&str], max_buckets: usize) -> String {
    let valid = ["root", "delegated"].map(|component| {
        let at = format!("JSONExtractRaw(payload_json,'analysis','{component}')");
        let buckets = format!("JSONExtractArrayRaw({at},'buckets')");
        let component_keys=keys(&at,&["basis","buckets","unattributed"]);
        let bucket_keys=keys("b",&["model_family","effort","tokens"]);
        let known_tokens=tokens("JSONExtractRaw(b,'tokens')");
        let unknown_tokens=tokens(&format!("JSONExtractRaw({at},'unattributed')"));
        let models=models.iter().filter(|v|**v!="unknown").map(|v|format!("'{v}'")).collect::<Vec<_>>().join(",");
        let efforts=EFFORTS.iter().filter(|v|**v!="unknown").map(|v|format!("'{v}'")).collect::<Vec<_>>().join(",");
        let fields = TOKEN_FIELDS.iter().map(|field| format!(
            "arraySum(arrayMap(b -> toUInt128(JSONExtractUInt(b,'tokens','{field}')), {buckets})) + toUInt128(JSONExtractUInt({at},'unattributed','{field}')) = toUInt128(JSONExtractUInt(payload_json,'metrics','{component}','usage','{field}'))"
        )).collect::<Vec<_>>().join(" AND ");
        format!("{component_keys} AND {unknown_tokens} AND JSONType({at},'buckets') = 'Array' AND arrayAll(b -> {bucket_keys} AND {known_tokens} AND JSONExtractString(b,'model_family') IN ({models}) AND JSONExtractString(b,'effort') IN ({efforts}), {buckets}) AND JSONExtractString({at},'basis') = 'owned_response_turn_link' AND length({buckets}) <= {max_buckets} AND length(arrayDistinct(arrayMap(b -> tuple(JSONExtractString(b,'model_family'),JSONExtractString(b,'effort')), {buckets}))) = length({buckets}) AND ({fields})")
    }).join(" AND ");
    let shape = keys(
        "JSONExtractRaw(payload_json,'analysis')",
        &["purpose", "root", "delegated"],
    );
    format!(
        "(NOT JSONHas(payload_json,'analysis') OR ({shape} AND JSONExtractString(payload_json,'analysis','purpose') IN ('production','verification','unclassified') AND {valid}))"
    )
}

pub(super) fn predicate() -> String {
    predicate_for_catalog(MODEL_FAMILIES, MAX_MODEL_CONTEXTS)
}

// Frozen revision-8 catalog from v2026.929.1. Only used to recognize and migrate
// the last released storage definition, including its original bucket bound.
// It is not an ingest adapter or a supported older wire contract.
pub(super) fn previous_storage_predicate() -> String {
    const PREVIOUS_MODELS: &[&str] = &[
        "astra",
        "gpt-5",
        "gpt-6",
        "gpt-6-astra",
        "gpt-6-luna",
        "gpt-6-sol",
        "luna",
        "other",
        "sol",
        "terra",
        "unknown",
    ];
    predicate_for_catalog(PREVIOUS_MODELS, PREVIOUS_MODELS.len() * EFFORTS.len())
}

pub(super) fn usage_view() -> String {
    let dimensions = "event_id, collector_id, groundline_version, os_family, runtime_family, execution_mode, period_start, period_end, generated_at, purpose";
    let branches = ["root","delegated"].into_iter().flat_map(|component| {
        let fields = TOKEN_FIELDS.iter().map(|field| format!("JSONExtractUInt(bucket,'tokens','{field}') AS {field}")).collect::<Vec<_>>().join(",");
        let residual = TOKEN_FIELDS.iter().map(|field| format!("if(JSONHas(payload_json,'analysis'),JSONExtractUInt(payload_json,'analysis','{component}','unattributed','{field}'),JSONExtractUInt(payload_json,'metrics','{component}','usage','{field}')) AS {field}")).collect::<Vec<_>>().join(",");
        [
            format!("SELECT {dimensions}, '{component}' AS component, JSONExtractString(bucket,'model_family') AS model_family, JSONExtractString(bucket,'effort') AS effort, toUInt8(1) AS attributed, {fields} FROM groundline.basic_active ARRAY JOIN JSONExtractArrayRaw(payload_json,'analysis','{component}','buckets') AS bucket"),
            format!("SELECT {dimensions}, '{component}' AS component, 'unknown' AS model_family, 'unknown' AS effort, toUInt8(0) AS attributed, {residual} FROM groundline.basic_active"),
        ]
    }).collect::<Vec<_>>().join(" UNION ALL ");
    format!("CREATE OR REPLACE VIEW groundline.model_usage AS {branches}")
}
