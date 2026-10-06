//! SQL projections for optional, explicitly sourced analysis observations.
use groundline_contracts::insights::analysis::TOKEN_FIELDS;
use groundline_contracts::model::{
    EFFORTS, MAX_MODEL_CONTEXTS, MODEL_FAMILIES, PRIVATE_MODEL_PATTERN, PUBLIC_MODEL_PATTERN,
};

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

fn previous_tokens(expression: &str) -> String {
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
        let known_tokens=previous_tokens("JSONExtractRaw(b,'tokens')");
        let unknown_tokens=previous_tokens(&format!("JSONExtractRaw({at},'unattributed')"));
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

// Frozen revision-9 storage predicate, including its exact catalog and bound.
// This recognizes the last storage definition; it never rewrites historical IDs.
pub(super) fn previous_storage_predicate() -> String {
    const PREVIOUS_MODELS: &[&str] = &[
        "astra",
        "gpt-5",
        "gpt-6",
        "gpt-6-astra",
        "gpt-6-luna",
        "gpt-6-sol",
        "gpt-6.1-sol",
        "luna",
        "other",
        "sol",
        "terra",
        "unknown",
    ];
    predicate_for_catalog(PREVIOUS_MODELS, 120)
}

fn unsigned(expression: &str, field: &str, max: &str) -> String {
    let raw = format!("JSONExtractRaw({expression},'{field}')");
    format!(
        "(JSONType({expression},'{field}') IN ('Int64','UInt64') AND NOT startsWith({raw},'-') AND (length({raw}) < {} OR (length({raw}) = {} AND {raw} <= '{max}')))",
        max.len(),
        max.len()
    )
}

fn tokens(expression: &str) -> String {
    let numeric = TOKEN_FIELDS
        .iter()
        .map(|field| unsigned(expression, field, "18446744073709551615"))
        .collect::<Vec<_>>()
        .join(" AND ");
    format!(
        "({} AND {numeric} AND JSONExtractUInt({expression},'cached_input_tokens') <= JSONExtractUInt({expression},'input_tokens') AND JSONExtractUInt({expression},'reasoning_output_tokens') <= JSONExtractUInt({expression},'output_tokens') AND toUInt128(JSONExtractUInt({expression},'input_tokens')) + toUInt128(JSONExtractUInt({expression},'output_tokens')) <= toUInt128(JSONExtractUInt({expression},'total_tokens')))",
        keys(expression, TOKEN_FIELDS)
    )
}

pub(super) fn label(expression: &str) -> String {
    let historical = MODEL_FAMILIES
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "(length({expression}) <= 96 AND ({expression} IN ({historical},'overflow') OR match({expression},'{PUBLIC_MODEL_PATTERN}') OR match({expression},'{PRIVATE_MODEL_PATTERN}')))"
    )
}

pub(super) fn identity(expression: &str) -> String {
    // These IDs were emitted as aggregate families before exact observations.
    let historical = "'astra','luna','sol','terra','other'";
    format!(
        "multiIf({expression}='unknown','unknown',{expression}='overflow','overflow',{expression} IN ({historical}),'historical_family',match({expression},'{PUBLIC_MODEL_PATTERN}'),'public_model_id',match({expression},'{PRIVATE_MODEL_PATTERN}'),'opaque_model_id','unknown')"
    )
}

pub(super) fn observed_identity(model: &str, observed: &str) -> String {
    let historical = MODEL_FAMILIES
        .iter()
        .filter(|m| **m != "unknown")
        .map(|m| format!("'{m}'"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "if(NOT ({observed}) AND {model} IN ({historical}),'historical_family',{})",
        identity(model)
    )
}

pub(super) fn predicate() -> String {
    let efforts = EFFORTS
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    let bucket_efforts = EFFORTS
        .iter()
        .filter(|s| **s != "unknown")
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    let metrics = ["root", "delegated"].map(|component| {
        let at = format!("JSONExtractRaw(payload_json,'metrics','{component}')");
        let observations = format!("JSONExtractArrayRaw({at},'model_effort')");
        let model = "JSONExtractString(m,'model_family')";
        let shape = keys("m", &["model_family","effort","count"]);
        let count = unsigned("m", "count", "4294967295");
        let valid_model = label(model);
        format!("(JSONType({at},'model_effort')='Array' AND length({observations}) <= {MAX_MODEL_CONTEXTS} AND arrayAll(m -> {shape} AND {count} AND {valid_model} AND JSONExtractString(m,'effort') IN ({efforts}) AND ({model}!='overflow' OR JSONExtractString(m,'effort')='unknown'), {observations}) AND length(arrayDistinct(arrayMap(m -> tuple(JSONExtractString(m,'model_family'),JSONExtractString(m,'effort')), {observations})))=length({observations}))")
    }).join(" AND ");
    let valid = ["root", "delegated"].map(|component| {
        let at = format!("JSONExtractRaw(payload_json,'analysis','{component}')");
        let buckets = format!("JSONExtractArrayRaw({at},'buckets')");
        let old_component = keys(&at,&["basis","buckets","unattributed"]);
        let new_component = keys(&at,&["basis","buckets","unattributed","observed_response_count","unattributed_response_count","overflow_response_count"]);
        let old_bucket = keys("b",&["model_family","effort","tokens"]);
        let new_bucket = keys("b",&["model_family","effort","tokens","response_count"]);
        let response_fields = ["observed_response_count","unattributed_response_count","overflow_response_count"]
            .map(|field| unsigned(&at, field, "18446744073709551615")).join(" AND ");
        let response = unsigned("b", "response_count", "18446744073709551615");
        let shape = format!("(({old_component} AND arrayAll(b -> {old_bucket}, {buckets})) OR ({new_component} AND {response_fields} AND arrayAll(b -> {new_bucket} AND {response} AND JSONExtractUInt(b,'response_count')>0, {buckets}) AND arraySum(arrayMap(b -> toUInt128(JSONExtractUInt(b,'response_count')), {buckets})) + toUInt128(JSONExtractUInt({at},'unattributed_response_count')) = toUInt128(JSONExtractUInt({at},'observed_response_count')) AND JSONExtractUInt({at},'overflow_response_count') <= JSONExtractUInt({at},'unattributed_response_count')))");
        let known_tokens=tokens("JSONExtractRaw(b,'tokens')");
        let unknown_tokens=tokens(&format!("JSONExtractRaw({at},'unattributed')"));
        let model="JSONExtractString(b,'model_family')";
        let valid_model=label(model);
        let fields=TOKEN_FIELDS.iter().map(|field| format!("arraySum(arrayMap(b -> toUInt128(JSONExtractUInt(b,'tokens','{field}')), {buckets})) + toUInt128(JSONExtractUInt({at},'unattributed','{field}')) = toUInt128(JSONExtractUInt(payload_json,'metrics','{component}','usage','{field}'))")).collect::<Vec<_>>().join(" AND ");
        format!("({shape} AND {unknown_tokens} AND JSONType({at},'buckets')='Array' AND arrayAll(b -> {known_tokens} AND {valid_model} AND {model} NOT IN ('unknown','overflow') AND JSONExtractString(b,'effort') IN ({bucket_efforts}), {buckets}) AND JSONExtractString({at},'basis')='owned_response_turn_link' AND length({buckets}) <= {MAX_MODEL_CONTEXTS} AND length(arrayDistinct(arrayMap(b -> tuple(JSONExtractString(b,'model_family'),JSONExtractString(b,'effort')), {buckets}))) = length({buckets}) AND ({fields}))")
    }).join(" AND ");
    let shape = keys(
        "JSONExtractRaw(payload_json,'analysis')",
        &["purpose", "root", "delegated"],
    );
    format!(
        "({metrics}) AND (NOT JSONHas(payload_json,'analysis') OR ({shape} AND JSONExtractString(payload_json,'analysis','purpose') IN ('production','verification','unclassified') AND {valid}))"
    )
}

pub(super) fn response_observed(component: &str) -> String {
    let at = format!("JSONExtractRaw(payload_json,'analysis','{component}')");
    format!(
        "(JSONHas({at},'observed_response_count') AND JSONHas({at},'unattributed_response_count') AND JSONHas({at},'overflow_response_count') AND arrayAll(b -> JSONHas(b,'response_count'),JSONExtractArrayRaw({at},'buckets')))"
    )
}

pub(super) const DIMENSIONS: &str = "event_id, collector_id, groundline_version, os_family, runtime_family, execution_mode, period_start, period_end, generated_at, purpose";

pub(super) fn usage_view() -> String {
    let branches = ["root","delegated"].into_iter().flat_map(|component| {
        let fields = TOKEN_FIELDS.iter().map(|field| format!("JSONExtractUInt(bucket,'tokens','{field}') AS {field}")).collect::<Vec<_>>().join(",");
        let residual = TOKEN_FIELDS.iter().map(|field| format!("if(JSONHas(payload_json,'analysis'),JSONExtractUInt(payload_json,'analysis','{component}','unattributed','{field}'),JSONExtractUInt(payload_json,'metrics','{component}','usage','{field}')) AS {field}")).collect::<Vec<_>>().join(",");
        let observed=response_observed(component);
        let model_identity=observed_identity("JSONExtractString(bucket,'model_family')", &observed);
        [
            format!("SELECT {DIMENSIONS}, '{component}' AS component, JSONExtractString(bucket,'model_family') AS model_family, {model_identity} AS model_identity, JSONExtractString(bucket,'effort') AS effort, toUInt8(1) AS attributed, toUInt8({observed}) AS response_observed, if({observed},toNullable(JSONExtractUInt(bucket,'response_count')),NULL) AS observed_response_count, {fields} FROM groundline.basic_active ARRAY JOIN JSONExtractArrayRaw(payload_json,'analysis','{component}','buckets') AS bucket"),
            format!("SELECT {DIMENSIONS}, '{component}' AS component, 'unknown' AS model_family, 'unknown' AS model_identity, 'unknown' AS effort, toUInt8(0) AS attributed, toUInt8({observed}) AS response_observed, if({observed},toNullable(JSONExtractUInt(payload_json,'analysis','{component}','unattributed_response_count')),NULL) AS observed_response_count, {residual} FROM groundline.basic_active"),
        ]
    }).collect::<Vec<_>>().join(" UNION ALL ");
    format!("CREATE OR REPLACE VIEW groundline.model_usage AS {branches}")
}
