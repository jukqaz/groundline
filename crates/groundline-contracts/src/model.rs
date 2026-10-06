//! Observed model identities, separate from availability and selection policy.
//! Public version/snapshot IDs stay distinct; private IDs use opaque stable keys.

use regex::Regex;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;

pub const MODEL_FAMILIES: &[&str] = &[
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
/// GroundLine's optimization scope, not an availability catalog. Native model
/// and effort support must still be checked on the execution host.
pub const OPTIMIZATION_MODELS: &[&str] = &["gpt-6.1-sol", "gpt-6-astra", "gpt-6-sol", "gpt-6-luna"];

pub fn optimization_model(value: &str) -> bool {
    OPTIMIZATION_MODELS.contains(&value)
}
/// Syntax of a bounded local model ID, not proof of native availability or
/// permission to emit it as an Insights dimension. Never strip snapshot suffixes.
pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'/'))
}
pub const EFFORTS: &[&str] = &[
    "high", "low", "max", "medium", "minimal", "none", "ultra", "unknown", "unset", "xhigh",
];
/// Per component bound, independent of the number of named models.
pub const MAX_MODEL_CONTEXTS: usize = 128;
pub const MAX_MODEL_LABEL_BYTES: usize = 96;
/// Public naming grammar, not a list of models available to an account.
/// Keep the same literal in Rust and ClickHouse validation. Unknown naming
/// shapes are still distinct opaque IDs rather than a shared `other` bucket.
pub const PUBLIC_MODEL_PATTERN: &str = "^((gpt|chatgpt)-([0-9]+([.][0-9]+)*|[0-9]+o)(-(sol|astra|luna|terra|codex|mini|nano|pro|chat|preview|turbo|instruct|thinking|search|max|latest))*|o[0-9]+(-(mini|pro|preview|deep-research))*|gpt-oss-[0-9]+b|gpt-daybreak-blue-latest|codex-(mini|max)(-latest)?)(-[0-9]{4}(-[0-9]{2}-[0-9]{2})?)?$";
pub const PRIVATE_MODEL_PATTERN: &str = "^private-[0-9a-f]{64}$";

static PUBLIC_MODEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(PUBLIC_MODEL_PATTERN).expect("fixed public model grammar"));
static PRIVATE_MODEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(PRIVATE_MODEL_PATTERN).expect("fixed private model key grammar"));

fn label_bytes(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_MODEL_LABEL_BYTES
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'.'))
}

/// A publishable label. Historical family labels are readable, but are never
/// reconstructed into an exact model ID. `overflow` is an explicit residual.
pub fn valid_label(value: &str) -> bool {
    label_bytes(value)
        && (MODEL_FAMILIES.contains(&value)
            || value == "overflow"
            || PUBLIC_MODEL.is_match(value)
            || PRIVATE_MODEL.is_match(value))
}

pub fn identity_kind(value: &str) -> &'static str {
    if value == "unknown" {
        "unknown"
    } else if value == "overflow" {
        "overflow"
    } else if ["astra", "luna", "sol", "terra", "other"].contains(&value) {
        "historical_family"
    } else if label_bytes(value) && PUBLIC_MODEL.is_match(value) {
        "public_model_id"
    } else if label_bytes(value) && PRIVATE_MODEL.is_match(value) {
        "opaque_model_id"
    } else {
        "historical_family"
    }
}

/// Observe native metadata without grouping different models into one family.
/// No registry, model availability inference or raw private ID export is needed.
pub fn observed_label(value: &str) -> String {
    let value = value.trim();
    if matches!(
        value.to_ascii_lowercase().as_str(),
        "" | "unknown" | "unset"
    ) || value.len() > 256
        || value.chars().any(char::is_control)
    {
        return "unknown".to_owned();
    }
    let public = value.to_ascii_lowercase();
    if label_bytes(&public) && PUBLIC_MODEL.is_match(&public) {
        return public;
    }
    let mut digest = Sha256::new();
    digest.update(b"groundline-model-id-v1\0");
    digest.update(value.as_bytes());
    format!("private-{:x}", digest.finalize())
}

pub fn effort(value: &str) -> &'static str {
    EFFORTS
        .iter()
        .copied()
        .find(|effort| value.trim().eq_ignore_ascii_case(effort))
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observations_preserve_versions_snapshots_and_safe_future_models() {
        for model in [
            "gpt-5",
            "gpt-6",
            "gpt-5.6-sol",
            "gpt-6-sol",
            "gpt-6.1-sol",
            "gpt-6.2-sol",
            "gpt-7-sol",
            "gpt-6-sol-2026-09-22",
            "gpt-6.1-sol-2026-10-06",
            "o4-mini",
            "gpt-4o",
            "gpt-oss-120b",
            "codex-mini-latest",
            "gpt-daybreak-blue-latest",
        ] {
            assert_eq!(observed_label(model), model);
            assert_eq!(observed_label(&model.to_ascii_uppercase()), model);
            assert!(valid_label(model));
            assert_eq!(identity_kind(model), "public_model_id");
        }
        for historical in ["sol", "astra", "other"] {
            assert!(valid_label(historical));
            assert_eq!(identity_kind(historical), "historical_family");
        }
        // Observing a current metadata string never applies the old family mapper.
        assert_ne!(observed_label("sol"), "sol");
        assert_eq!(identity_kind("unknown"), "unknown");
        assert_eq!(identity_kind("overflow"), "overflow");
    }

    #[test]
    fn private_observations_are_stable_distinct_and_publishable_without_raw_names() {
        let first = observed_label("provider/private-console");
        assert_eq!(first, observed_label(" provider/private-console "));
        assert_ne!(first, observed_label("provider/private-console-2"));
        assert_ne!(first, observed_label("provider/Private-console"));
        assert_ne!(
            observed_label("개인 모델 하나"),
            observed_label("개인 모델 둘")
        );
        assert!(valid_label(&observed_label("개인 모델 하나")));
        assert_eq!(
            identity_kind(&observed_label("개인 모델 하나")),
            "opaque_model_id"
        );
        assert!(valid_label(&first));
        assert_eq!(identity_kind(&first), "opaque_model_id");
        assert_eq!(first.len(), 72);
        assert!(!first.contains("provider"));
        for invalid in [
            "private-console",
            "provider/private-console",
            "gpt-6-secret-sol",
            "private-abc",
            "gpt-6-sol\n",
        ] {
            assert!(!valid_label(invalid), "{invalid:?}");
        }
        for invalid in ["", "unset", "UNKNOWN", "private\0model", "private\nmodel"] {
            assert_eq!(observed_label(invalid), "unknown");
        }
        assert_eq!(observed_label(&"a".repeat(257)), "unknown");
    }

    #[test]
    fn optimization_scope_excludes_older_and_ambiguous_cohorts() {
        assert!(optimization_model("gpt-6.1-sol"));
        for model in OPTIMIZATION_MODELS {
            assert!(optimization_model(model));
        }
        for model in [
            "gpt-5.6-sol",
            "gpt-5.6-luna",
            "gpt-5.5",
            "sol",
            "luna",
            "astra",
            "gpt-6",
            "gpt-6-private-sol",
            "gpt-6.1-sol-unconfirmed-snapshot",
            "gpt-6.2-sol",
            "gpt-7-sol",
            "future-model",
            "unknown",
        ] {
            assert!(!optimization_model(model), "{model}");
        }
    }
}
