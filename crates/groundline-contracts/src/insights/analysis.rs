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
const RESPONSE_FIELDS: &[&str] = &[
    "observed_response_count",
    "unattributed_response_count",
    "overflow_response_count",
];

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
    exact_object_keys(value, TOKEN_FIELDS)
        && TOKEN_FIELDS.iter().all(|key| value[key].is_u64())
        && super::valid_token_counters(std::array::from_fn(|i| {
            value[TOKEN_FIELDS[i]].as_u64().unwrap()
        }))
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
        let responses_observed = c.get("observed_response_count").is_some();
        let component_keys = if responses_observed {
            &[
                "basis",
                "buckets",
                "unattributed",
                "observed_response_count",
                "unattributed_response_count",
                "overflow_response_count",
            ][..]
        } else {
            &["basis", "buckets", "unattributed"][..]
        };
        if !exact_object_keys(c, component_keys)
            || c["basis"] != "owned_response_turn_link"
            || !tokens(&c["unattributed"])
            || buckets.len() > crate::model::MAX_MODEL_CONTEXTS
        {
            return false;
        }
        if responses_observed && !RESPONSE_FIELDS.iter().all(|key| c[key].is_u64()) {
            return false;
        }
        let mut labels = std::collections::BTreeSet::new();
        for b in buckets {
            let bucket_keys = if responses_observed {
                &["model_family", "effort", "tokens", "response_count"][..]
            } else {
                &["model_family", "effort", "tokens"][..]
            };
            if !exact_object_keys(b, bucket_keys)
                || !tokens(&b["tokens"])
                || (responses_observed && !b["response_count"].as_u64().is_some_and(|n| n > 0))
            {
                return false;
            }
            let (Some(model), Some(effort)) = (b["model_family"].as_str(), b["effort"].as_str())
            else {
                return false;
            };
            if !crate::model::valid_label(model)
                || matches!(model, "unknown" | "overflow")
                || !crate::model::EFFORTS.contains(&effort)
                || effort == "unknown"
                || !labels.insert((model, effort))
            {
                return false;
            }
        }
        if responses_observed {
            let response_sum = buckets.iter().try_fold(
                c["unattributed_response_count"].as_u64().unwrap(),
                |sum, b| sum.checked_add(b["response_count"].as_u64().unwrap()),
            );
            if response_sum != c["observed_response_count"].as_u64()
                || c["overflow_response_count"].as_u64().unwrap()
                    > c["unattributed_response_count"].as_u64().unwrap()
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
    fn reject_incoherent_buckets_and_residual_even_when_totals_match() {
        let audit =
            json!({"root":{"provider_reported_usage":{"total_tokens":10,"input_tokens":10}}});
        let a = from_audit(&audit);
        let metrics = json!({"root":{"usage":a["root"]["unattributed"]},
            "delegated":{"usage":a["delegated"]["unattributed"]}});
        let zero = a["delegated"]["unattributed"].clone();
        for bad_residual in [false, true] {
            let mut bad = a.clone();
            let mut input_only = zero.clone();
            input_only["input_tokens"] = json!(10);
            let mut total_only = zero.clone();
            total_only["total_tokens"] = json!(10);
            bad["root"]["unattributed"] = if bad_residual {
                input_only.clone()
            } else {
                zero.clone()
            };
            bad["root"]["buckets"] = if bad_residual {
                json!([{"model_family":"gpt-6.1-sol","effort":"high","tokens":total_only}])
            } else {
                json!([
                    {"model_family":"gpt-6.1-sol","effort":"high","tokens":input_only},
                    {"model_family":"gpt-6-astra","effort":"high","tokens":total_only}
                ])
            };
            assert!(!valid(&bad, &metrics));
        }
    }

    #[test]
    fn response_observations_are_coupled_and_conserve_counts() {
        let audit = json!({"root":{"provider_reported_usage":{"total_tokens":9}}});
        let mut a = from_audit(&audit);
        let metrics = json!({"root":{"usage":a["root"]["unattributed"]},
            "delegated":{"usage":a["delegated"]["unattributed"]}});
        let mut usage = a["delegated"]["unattributed"].clone();
        usage["total_tokens"] = json!(8);
        a["root"]["buckets"] = json!([{
            "model_family":"gpt-6.2-sol-2026-10-06","effort":"high","tokens":usage,"response_count":2
        }]);
        a["root"]["unattributed"]["total_tokens"] = json!(1);
        a["root"]["observed_response_count"] = json!(3);
        a["root"]["unattributed_response_count"] = json!(1);
        a["root"]["overflow_response_count"] = json!(1);
        assert!(valid(&a, &metrics));
        for key in RESPONSE_FIELDS {
            let mut bad = a.clone();
            bad["root"].as_object_mut().unwrap().remove(*key);
            assert!(!valid(&bad, &metrics));
            bad = a.clone();
            bad["root"][key] = json!("private count");
            assert!(!valid(&bad, &metrics));
        }
        for (key, n) in [
            ("observed_response_count", 4),
            ("unattributed_response_count", 2),
            ("overflow_response_count", 2),
        ] {
            let mut bad = a.clone();
            bad["root"][key] = json!(n);
            assert!(!valid(&bad, &metrics));
        }
        for n in [0, u64::MAX] {
            let mut bad = a.clone();
            bad["root"]["buckets"][0]["response_count"] = json!(n);
            assert!(!valid(&bad, &metrics));
        }
        let mut bad = a.clone();
        bad["root"]["buckets"][0]
            .as_object_mut()
            .unwrap()
            .remove("response_count");
        assert!(!valid(&bad, &metrics));
        for label in ["private/model-name", "overflow", "unknown"] {
            bad = a.clone();
            bad["root"]["buckets"][0]["model_family"] = json!(label);
            assert!(!valid(&bad, &metrics));
        }
        let opaque = crate::model::observed_label("private/model-name");
        a["root"]["buckets"][0]["model_family"] = json!(opaque);
        assert!(valid(&a, &metrics));
        assert!(!a.to_string().contains("private/model-name"));
        // Stored observations without counts remain unobserved, never fabricated zero.
        for key in RESPONSE_FIELDS {
            a["root"].as_object_mut().unwrap().remove(*key);
        }
        a["root"]["buckets"][0]
            .as_object_mut()
            .unwrap()
            .remove("response_count");
        assert!(valid(&a, &metrics));
        a["root"]["buckets"][0]["response_count"] = json!(2);
        assert!(!valid(&a, &metrics));
    }
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
