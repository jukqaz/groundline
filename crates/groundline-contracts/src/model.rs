//! Bounded audit labels, not a model catalog or a routing policy.
//! New model IDs remain readable without admitting arbitrary wire dimensions.

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
pub const MAX_MODEL_CONTEXTS: usize = MODEL_FAMILIES.len() * EFFORTS.len();

pub fn family(value: &str) -> &'static str {
    let value = value.trim().to_ascii_lowercase();
    if let Some(family) = MODEL_FAMILIES
        .iter()
        .copied()
        .find(|family| *family == value)
    {
        return family;
    }
    if matches!(value.as_str(), "unknown" | "unset" | "") {
        return "unknown";
    }
    // Keep newly observed GPT-6 tiers separate from historical unversioned
    // labels. Those old aggregates cannot be assigned a generation retroactively.
    if let Some(rest) = value.strip_prefix("gpt-6-") {
        return match rest.split('-').next() {
            Some("astra") => "gpt-6-astra",
            Some("sol") => "gpt-6-sol",
            Some("luna") => "gpt-6-luna",
            _ => "gpt-6",
        };
    }
    // Resolve generation before historical tier names. Unknown 6.x and future
    // generations must not fall into a legacy Sol/Astra/Luna cohort.
    if value.starts_with("gpt-") {
        let generation = value.split('-').nth(1).unwrap_or_default();
        if generation == "6" || generation.starts_with("6.") {
            return "gpt-6";
        }
        if generation == "5" || generation.starts_with("5.") {
            for part in value.split('-').skip(2) {
                if let Some(family) = ["astra", "luna", "sol", "terra"]
                    .into_iter()
                    .find(|family| *family == part)
                {
                    return family;
                }
            }
            return "gpt-5";
        }
    }
    "other"
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
    fn bounded_labels_accept_new_versions_without_leaking_custom_names() {
        for (model, expected) in [
            ("gpt-6-astra", "gpt-6-astra"),
            ("gpt-6-sol", "gpt-6-sol"),
            ("gpt-6-luna", "gpt-6-luna"),
            ("gpt-6.1-sol", "gpt-6.1-sol"),
            ("GPT-6-SOL-2026-09-22", "gpt-6-sol"),
            ("gpt-6-private-sol", "gpt-6"),
            ("GPT-6.1-ASTRA-2026-09-05", "gpt-6"),
            ("gpt-6.1-sol-unconfirmed-snapshot", "gpt-6"),
            ("gpt-6.1-private-sol", "gpt-6"),
            ("gpt-6.2-sol", "gpt-6"),
            ("gpt-7-sol", "other"),
            ("gpt-5.6-sol", "sol"),
            ("gpt-5.6-terra", "terra"),
            ("gpt-5.6-luna", "luna"),
            ("gpt-6.2", "gpt-6"),
            ("gpt-5.5", "gpt-5"),
            ("gpt-50", "other"),
            ("gpt-7-future", "other"),
            ("private-console", "other"),
            ("private-gpt-6-astra", "other"),
            ("unset", "unknown"),
            ("", "unknown"),
        ] {
            assert_eq!(family(model), expected, "{model}");
            assert!(MODEL_FAMILIES.contains(&family(model)));
        }
        assert_eq!(effort(" HIGH "), "high");
        assert_eq!(effort("future-private-effort"), "unknown");
    }

    #[test]
    fn optimization_scope_excludes_older_and_ambiguous_cohorts() {
        assert!(optimization_model("gpt-6.1-sol"));
        for model in OPTIMIZATION_MODELS {
            assert!(optimization_model(model));
            assert_eq!(family(model), *model);
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
            assert!(!optimization_model(family(model)), "{model}");
        }
    }
}
