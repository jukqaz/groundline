//! Parse a bounded native catalog once; never retain ignored native instructions.
use std::collections::BTreeSet;

use groundline_contracts::ContractError;
use serde::Deserialize;
use serde_path_to_error::{Path, Segment};

pub const MAX_CATALOG_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Deserialize)]
struct RawCatalog {
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

/// Validated models sorted for allocation-free repeated lookups.
pub struct Catalog {
    models: Vec<Model>,
}

fn error(field: &str) -> ContractError {
    ContractError(format!("config_audit_invalid_catalog{field}"))
}

// Error paths may contain arbitrary map keys. Match only the public schema and
// report fixed labels, without formatting the path, input, or original error.
fn field(path: &Path) -> &'static str {
    let segments: Vec<_> = path.iter().collect();
    match segments.as_slice() {
        [Segment::Map { key }] if key == "models" => "_models",
        [Segment::Map { key }, Segment::Seq { .. }] if key == "models" => "_model",
        [
            Segment::Map { key },
            Segment::Seq { .. },
            Segment::Map { key: item },
        ] if key == "models" => match item.as_str() {
            "slug" => "_model_slug",
            "default_reasoning_level" => "_model_default_effort",
            "supported_reasoning_levels" => "_model_efforts",
            "support_verbosity" => "_model_verbosity",
            _ => "",
        },
        [
            Segment::Map { key },
            Segment::Seq { .. },
            Segment::Map { key: levels },
            Segment::Seq { .. },
            rest @ ..,
        ] if key == "models"
            && levels == "supported_reasoning_levels"
            && (rest.is_empty() || matches!(rest, [Segment::Map { key }] if key == "effort")) =>
        {
            "_model_effort"
        }
        _ => "",
    }
}

fn valid_label(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

#[cold]
fn decode_error(bytes: &[u8]) -> ContractError {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    match serde_path_to_error::deserialize::<_, RawCatalog>(&mut deserializer) {
        Err(failure) => error(field(failure.path())),
        Ok(_) => error(""),
    }
}

impl Catalog {
    /// Accept current native JSON, including one UTF-8 BOM; reject trailing data.
    pub fn parse(bytes: &[u8]) -> Result<Self, ContractError> {
        if bytes.len() as u64 > MAX_CATALOG_BYTES {
            return Err(error(""));
        }
        let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
        // Collect detailed paths only after a failure, so valid catalogs pay no
        // path-tracking allocations. from_slice also rejects trailing data.
        let mut raw: RawCatalog = serde_json::from_slice(bytes).map_err(|_| decode_error(bytes))?;
        if raw.models.is_empty() || raw.models.len() > 512 {
            return Err(error(""));
        }
        raw.models.sort_unstable_by(|a, b| a.slug.cmp(&b.slug));
        for (index, model) in raw.models.iter().enumerate() {
            let efforts = model
                .supported_reasoning_levels
                .iter()
                .map(|level| level.effort.as_str())
                .collect::<BTreeSet<_>>();
            if !valid_label(&model.slug)
                || index > 0 && raw.models[index - 1].slug == model.slug
                || efforts.is_empty()
                || efforts.len() > 16
                || efforts.len() != model.supported_reasoning_levels.len()
                || efforts.iter().any(|e| !valid_label(e))
                || !efforts.contains(model.default_reasoning_level.as_str())
            {
                return Err(error(""));
            }
        }
        Ok(Self { models: raw.models })
    }

    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    fn find(&self, slug: &str) -> Option<&Model> {
        self.models
            .binary_search_by(|model| model.slug.as_str().cmp(slug))
            .ok()
            .map(|index| &self.models[index])
    }

    pub fn supports_model(&self, slug: &str) -> bool {
        self.find(slug).is_some()
    }

    pub fn supports_effort(&self, slug: &str, effort: &str) -> Option<bool> {
        self.find(slug).map(|model| {
            model
                .supported_reasoning_levels
                .iter()
                .any(|level| level.effort == effort)
        })
    }

    pub fn supports_verbosity(&self, slug: &str) -> Option<bool> {
        self.find(slug).and_then(|model| model.support_verbosity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    fn model(slug: &str) -> serde_json::Value {
        json!({"slug":slug,"default_reasoning_level":"xhigh",
            "supported_reasoning_levels":[{"effort":"low"},{"effort":"xhigh"}],
            "support_verbosity":true,"base_instructions":"PRIVATE_SENTINEL"})
    }

    #[test]
    fn accepts_one_native_utf8_bom_but_rejects_other_encoding_prefixes() {
        let bytes = serde_json::to_vec(&json!({"models":[model("valid")]})).unwrap();
        for prefix in [b"".as_slice(), b"\xef\xbb\xbf"] {
            let catalog = Catalog::parse(&[prefix, &bytes].concat()).unwrap();
            assert_eq!(catalog.model_count(), 1);
            assert_eq!(catalog.supports_effort("valid", "xhigh"), Some(true));
        }
        for prefix in [
            b"\xff\xfe".as_slice(),
            b"\xfe\xff",
            b"\xef\xbb\xbf\xef\xbb\xbf",
        ] {
            assert!(Catalog::parse(&[prefix, &bytes].concat()).is_err());
        }
    }

    #[test]
    fn error_paths_are_fixed_schema_labels_without_private_values() {
        for (key, value, suffix) in [
            ("slug", json!({"PRIVATE_SENTINEL":true}), "_model_slug"),
            (
                "default_reasoning_level",
                json!(false),
                "_model_default_effort",
            ),
            ("supported_reasoning_levels", json!(42), "_model_efforts"),
            (
                "support_verbosity",
                json!("PRIVATE_SENTINEL"),
                "_model_verbosity",
            ),
            (
                "supported_reasoning_levels",
                json!([{"effort": {"PRIVATE_SENTINEL":1}}]),
                "_model_effort",
            ),
        ] {
            let mut record = model("private-model");
            record[key] = value;
            let result = Catalog::parse(&serde_json::to_vec(&json!({"models":[record]})).unwrap());
            let error = result.err().unwrap().to_string();
            assert_eq!(error, format!("config_audit_invalid_catalog{suffix}"));
            assert!(!error.contains("PRIVATE_SENTINEL"));
        }
    }

    #[test]
    fn trailing_data_duplicate_fields_and_deep_private_keys_are_rejected() {
        let valid = serde_json::to_vec(&json!({"models":[model("valid")]})).unwrap();
        for bytes in [
            [valid.as_slice(), b"{}"].concat(),
            [valid.as_slice(), b"PRIVATE_SENTINEL"].concat(),
            br#"{"models":[],"models":[{"PRIVATE_SENTINEL":1}]}"#.to_vec(),
            br#"{"models":[{"slug":{"PRIVATE_SENTINEL":{"other":1}}}]}"#.to_vec(),
        ] {
            let error = Catalog::parse(&bytes).err().unwrap().to_string();
            assert!(error.starts_with("config_audit_invalid_catalog"));
            assert!(!error.contains("PRIVATE_SENTINEL"));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn lookups_match_the_input_set_in_any_order(slugs in prop::collection::vec("[a-z]{1,12}", 1..80)) {
            let unique = slugs.iter().collect::<BTreeSet<_>>();
            let bytes = serde_json::to_vec(&json!({"models":slugs.iter().map(|s| model(s)).collect::<Vec<_>>()})).unwrap();
            let result = Catalog::parse(&bytes);
            if unique.len() != slugs.len() {
                prop_assert!(result.is_err());
            } else {
                let catalog = result.unwrap();
                prop_assert_eq!(catalog.model_count(), slugs.len());
                for slug in &slugs {
                    prop_assert!(catalog.supports_model(slug));
                    prop_assert_eq!(catalog.supports_effort(slug, "xhigh"), Some(true));
                    prop_assert_eq!(catalog.supports_effort(slug, "unsupported"), Some(false));
                    prop_assert_eq!(catalog.supports_verbosity(slug), Some(true));
                }
                prop_assert!(!catalog.supports_model("missing-model"));
                prop_assert_eq!(catalog.supports_effort("missing-model", "xhigh"), None);
            }
        }
    }
}
