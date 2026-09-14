//! Optional observations in the current v5 envelope. Absence means unobserved;
//! it never authorizes reconstructing historical model usage or work purpose.
use super::exact_object_keys;
use serde_json::{Value, json};

pub const TOKEN_FIELDS: &[&str] = &[
    "input_tokens",
    "cached_input_tokens",
    "cache_write_input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
    "total_tokens",
];
pub const PURPOSES: &[&str] = &["production", "verification", "unclassified"];

pub fn from_audit(audit: &Value) -> Value {
    let component =
        |name: &str| {
            audit[name].get("model_attributed_usage").cloned().unwrap_or_else(|| json!({
            "basis":"owned_response_turn_link", "buckets":[],
            "unattributed": TOKEN_FIELDS.iter().map(|key| ((*key).to_owned(),
                json!(audit[name]["provider_reported_usage"][key].as_u64().unwrap_or(0))))
                .collect::<serde_json::Map<_,_>>()
        }))
        };
    json!({"purpose":audit.pointer("/scope/explicit_purpose").and_then(Value::as_str).unwrap_or("unclassified"),
        "root":component("root"),"delegated":component("delegated")})
}

fn tokens(value: &Value) -> bool {
    exact_object_keys(value, TOKEN_FIELDS) && TOKEN_FIELDS.iter().all(|key| value[key].is_u64())
}

pub fn valid(analysis: &Value, metrics: &Value) -> bool {
    if !exact_object_keys(analysis, &["purpose", "root", "delegated"])
        || !analysis["purpose"]
            .as_str()
            .is_some_and(|v| PURPOSES.contains(&v))
    {
        return false;
    }
    for name in ["root", "delegated"] {
        let c = &analysis[name];
        let Some(buckets) = c["buckets"].as_array() else {
            return false;
        };
        if !exact_object_keys(c, &["basis", "buckets", "unattributed"])
            || c["basis"] != "owned_response_turn_link"
            || !tokens(&c["unattributed"])
            || buckets.len() > crate::model::MAX_MODEL_CONTEXTS
        {
            return false;
        }
        let mut labels = std::collections::BTreeSet::new();
        for b in buckets {
            if !exact_object_keys(b, &["model_family", "effort", "tokens"]) || !tokens(&b["tokens"])
            {
                return false;
            }
            let (Some(model), Some(effort)) = (b["model_family"].as_str(), b["effort"].as_str())
            else {
                return false;
            };
            if !crate::model::MODEL_FAMILIES.contains(&model)
                || model == "unknown"
                || !crate::model::EFFORTS.contains(&effort)
                || effort == "unknown"
                || !labels.insert((model, effort))
            {
                return false;
            }
        }
        for key in TOKEN_FIELDS {
            let sum = buckets
                .iter()
                .try_fold(c["unattributed"][key].as_u64().unwrap(), |sum, b| {
                    sum.checked_add(b["tokens"][key].as_u64().unwrap())
                });
            if sum != metrics[name]["usage"][key].as_u64() {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_duplicate_labels_wrong_sum_and_private_fields() {
        let audit = json!({"root":{"provider_reported_usage":{"total_tokens":9}}});
        let a = from_audit(&audit);
        let metrics = json!({"root":{"usage":a["root"]["unattributed"]},"delegated":{"usage":a["delegated"]["unattributed"]}});
        assert!(valid(&a, &metrics));
        for field in ["total_tokens", "input_tokens"] {
            let mut bad = a.clone();
            bad["root"]["unattributed"][field] = json!(10);
            assert!(!valid(&bad, &metrics));
        }
        let mut bad = a.clone();
        bad["purpose"] = json!("private task name");
        assert!(!valid(&bad, &metrics));
        bad = a.clone();
        bad["root"]["prompt"] = json!("private");
        assert!(!valid(&bad, &metrics));
        let mut bucket =
            json!({"model_family":"astra","effort":"high","tokens":a["delegated"]["unattributed"]});
        bucket["tokens"]["total_tokens"] = json!(0);
        bad = a;
        bad["root"]["buckets"] = json!([bucket, bucket]);
        assert!(!valid(&bad, &metrics));
    }
}
