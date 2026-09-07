//! Offline posture checks against an explicit native catalog, not a Codex schema.
use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

use groundline_contracts::ContractError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAX_CONFIG_BYTES: u64 = 512 * 1024;
const MAX_CATALOG_BYTES: u64 = 8 * 1024 * 1024;

// Native configuration is additive. Read only these relevant fields and let
// native strict doctor validate the complete effective configuration/schema.
#[derive(Default, Deserialize)]
struct Settings {
    model: Option<String>,
    model_reasoning_effort: Option<String>,
    review_model: Option<String>,
    model_provider: Option<String>,
    service_tier: Option<String>,
    model_context_window: Option<i64>,
    model_auto_compact_token_limit: Option<i64>,
    model_verbosity: Option<String>,
    model_catalog_json: Option<String>,
    profile: Option<String>,
}

#[derive(Deserialize)]
struct Catalog {
    models: Vec<Model>,
}

#[derive(Deserialize)]
struct Model {
    slug: String,
    default_reasoning_level: String,
    supported_reasoning_levels: Vec<Effort>,
    support_verbosity: Option<bool>,
}

#[derive(Deserialize)]
struct Effort {
    effort: String,
}

#[derive(Serialize)]
struct Finding {
    code: &'static str,
    severity: &'static str,
}

fn error(code: &str) -> ContractError {
    ContractError(format!("config_audit_{code}"))
}

fn valid_label(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn inspect(config: &str, catalog: &[u8]) -> Result<Value, ContractError> {
    let settings: Settings = toml::from_str(config).map_err(|_| error("invalid_config"))?;
    let catalog: Catalog = serde_json::from_slice(catalog).map_err(|_| error("invalid_catalog"))?;
    if catalog.models.is_empty() || catalog.models.len() > 512 {
        return Err(error("invalid_catalog"));
    }
    let mut models = BTreeSet::new();
    for model in &catalog.models {
        let efforts = model
            .supported_reasoning_levels
            .iter()
            .map(|level| level.effort.as_str())
            .collect::<BTreeSet<_>>();
        if !valid_label(&model.slug)
            || !models.insert(model.slug.as_str())
            || efforts.is_empty()
            || efforts.len() > 16
            || efforts.len() != model.supported_reasoning_levels.len()
            || efforts.iter().any(|e| !valid_label(e))
            || !efforts.contains(model.default_reasoning_level.as_str())
        {
            return Err(error("invalid_catalog"));
        }
    }
    let model = settings
        .model
        .as_ref()
        .and_then(|slug| catalog.models.iter().find(|m| &m.slug == slug));
    let mut findings = Vec::new();
    let mut add = |code, severity| findings.push(Finding { code, severity });
    if settings.model.is_some() && model.is_none() {
        add("model_not_in_supplied_catalog", "error");
    }
    if let Some(effort) = &settings.model_reasoning_effort {
        match model {
            Some(model)
                if !model
                    .supported_reasoning_levels
                    .iter()
                    .any(|e| &e.effort == effort) =>
            {
                add("unsupported_reasoning_effort", "error")
            }
            None => add("effort_requires_resolved_model", "review"),
            _ => (),
        }
    }
    if settings
        .review_model
        .as_ref()
        .is_some_and(|m| !models.contains(m.as_str()))
    {
        add("review_model_not_in_supplied_catalog", "error");
    }
    if settings
        .model_provider
        .as_deref()
        .is_some_and(|p| p != "openai")
    {
        add("custom_provider_runtime_unverified", "review");
    }
    if settings.profile.is_some() {
        add("profile_layer_not_resolved", "review");
    }
    if settings.model_catalog_json.is_some() {
        add("catalog_override_requires_verification", "review");
    }
    for (value, reason) in [
        (settings.model_context_window, "manual_context_window"),
        (
            settings.model_auto_compact_token_limit,
            "manual_compaction_threshold",
        ),
    ] {
        if let Some(value) = value {
            if value <= 0 {
                add("nonpositive_context_limit", "error");
            } else {
                add(reason, "review");
            }
        }
    }
    if let (Some(window), Some(threshold)) = (
        settings.model_context_window,
        settings.model_auto_compact_token_limit,
    ) && threshold > window
    {
        add("compaction_threshold_exceeds_context_window", "error");
    }
    if settings.model_verbosity.is_some() && model.and_then(|m| m.support_verbosity) != Some(true) {
        add("verbosity_support_unverified", "review");
    }
    let status = if findings.iter().any(|f| f.severity == "error") {
        "FAIL"
    } else if findings.is_empty() {
        "PASS"
    } else {
        "REVIEW_REQUIRED"
    };
    Ok(json!({
        "kind":"groundline-config-audit", "schema":1, "status":status,
        "scope":"single_config_layer", "findings":findings,
        "model_explicit":settings.model.is_some(),
        "selected_model_in_catalog":settings.model.as_ref().map(|_| model.is_some()),
        "effort_explicit":settings.model_reasoning_effort.is_some(),
        "service_tier_explicit":settings.service_tier.is_some(),
        "catalog_model_count":catalog.models.len(),
        "native_schema_validation":"not_run", "effective_runtime_verified":false,
        "catalog_freshness_verified":false, "account_access_verified":false,
        "service_tier_verified":false,
        "network_performed":false, "mutation_performed":false,
        "configuration_values_emitted":false, "private_paths_emitted":false,
        "raw_content_emitted":false, "scripts_executed":false
    }))
}

pub fn audit(config: &Path, catalog: &Path) -> Result<Value, ContractError> {
    let config = crate::load_bounded_range(config, 0, MAX_CONFIG_BYTES)?;
    let config = std::str::from_utf8(&config).map_err(|_| error("invalid_config"))?;
    let catalog = if catalog == Path::new("-") {
        let mut bytes = Vec::new();
        std::io::stdin()
            .lock()
            .take(MAX_CATALOG_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| error("invalid_catalog"))?;
        if bytes.len() as u64 > MAX_CATALOG_BYTES {
            return Err(error("invalid_catalog"));
        }
        bytes
    } else {
        crate::load_bounded(catalog, MAX_CATALOG_BYTES)?
    };
    inspect(config, &catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Value {
        json!({"models":[{
            "slug":"gpt-6-astra", "default_reasoning_level":"low",
            "supported_reasoning_levels":[{"effort":"low"},{"effort":"medium"},{"effort":"ultra"}],
            "support_verbosity":true, "base_instructions":"PRIVATE_SENTINEL"
        }]})
    }
    fn run(config: &str) -> Value {
        inspect(config, &serde_json::to_vec(&catalog()).unwrap()).unwrap()
    }

    #[test]
    fn accepts_current_astra_settings_without_changing_user_posture() {
        let result = run(
            "model='gpt-6-astra'\nmodel_reasoning_effort='medium'\nservice_tier='fast'\n[unrelated]\nsecret='PRIVATE_SENTINEL'",
        );
        assert_eq!(result["status"], "PASS");
        assert_eq!(result["mutation_performed"], false);
        assert_eq!(result["service_tier_explicit"], true);
        for private in ["PRIVATE_SENTINEL", "gpt-6-astra", "medium", "fast"] {
            assert!(!result.to_string().contains(private));
        }
        assert_eq!(result["effective_runtime_verified"], false);
        assert_eq!(result["service_tier_verified"], false);
        assert_eq!(run("# Native defaults")["status"], "PASS");
        assert!(run("# Native defaults")["selected_model_in_catalog"].is_null());
    }

    #[test]
    fn effort_support_comes_from_supplied_catalog_not_a_family_table() {
        for effort in ["none", "minimal", "future-effort"] {
            assert_eq!(
                run(&format!(
                    "model='gpt-6-astra'\nmodel_reasoning_effort='{effort}'"
                ))["status"],
                "FAIL"
            );
        }
        assert_eq!(
            run("model_reasoning_effort='medium'")["status"],
            "REVIEW_REQUIRED"
        );
        let mut future = catalog();
        future["models"][0]["slug"] = json!("future-private-model");
        future["models"][0]["default_reasoning_level"] = json!("future-effort");
        future["models"][0]["supported_reasoning_levels"] = json!([{"effort":"future-effort"}]);
        let result = inspect(
            "model='future-private-model'\nmodel_reasoning_effort='future-effort'",
            &serde_json::to_vec(&future).unwrap(),
        )
        .unwrap();
        assert_eq!(result["status"], "PASS");
        assert!(!result.to_string().contains("future-private"));
    }

    #[test]
    fn unresolved_layers_and_overrides_request_review_not_automatic_rewrites() {
        for config in [
            "profile='private'",
            "model_provider='private'",
            "model_catalog_json='/private/catalog.json'",
            "model_context_window=272000",
            "model_auto_compact_token_limit=200000",
            "model_verbosity='low'",
        ] {
            assert_eq!(run(config)["status"], "REVIEW_REQUIRED", "{config}");
        }
        for config in [
            "model='missing'",
            "review_model='missing'",
            "model_context_window=0",
            "model_auto_compact_token_limit=-1",
            "model_context_window=100\nmodel_auto_compact_token_limit=200",
        ] {
            assert_eq!(run(config)["status"], "FAIL", "{config}");
        }
    }

    #[test]
    fn malformed_inputs_and_duplicate_catalog_keys_fail_privately() {
        let bytes = serde_json::to_vec(&catalog()).unwrap();
        for config in [
            "model=",
            "model=[]",
            "model='one'\nmodel='two'",
            "model_context_window='PRIVATE_SENTINEL'",
        ] {
            assert_eq!(
                inspect(config, &bytes).unwrap_err().to_string(),
                "config_audit_invalid_config"
            );
        }
        for value in [
            json!({}),
            json!({"models":[]}),
            json!({"models":[catalog()["models"][0],catalog()["models"][0]]}),
        ] {
            assert_eq!(
                inspect("", &serde_json::to_vec(&value).unwrap())
                    .unwrap_err()
                    .to_string(),
                "config_audit_invalid_catalog"
            );
        }
        let mut value = catalog();
        value["models"][0]["supported_reasoning_levels"] =
            json!([{"effort":"low"},{"effort":"low"}]);
        assert!(inspect("", &serde_json::to_vec(&value).unwrap()).is_err());
    }
}
