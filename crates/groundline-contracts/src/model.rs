//! Bounded audit labels, not a model catalog or a routing policy.
//! New model IDs remain readable without admitting arbitrary wire dimensions.

pub const MODEL_FAMILIES: &[&str] = &[
    "astra", "gpt-5", "gpt-6", "luna", "other", "sol", "terra", "unknown",
];
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
    // Match provider model components, never substrings such as `console`.
    if value.starts_with("gpt-") {
        for part in value.split('-').skip(2) {
            if let Some(family) = ["astra", "luna", "sol", "terra"]
                .into_iter()
                .find(|family| *family == part)
            {
                return family;
            }
        }
        let generation = value.split('-').nth(1).unwrap_or_default();
        if generation == "5" || generation.starts_with("5.") {
            return "gpt-5";
        }
        if generation == "6" || generation.starts_with("6.") {
            return "gpt-6";
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
            ("gpt-6-astra", "astra"),
            ("GPT-6.1-ASTRA-2026-09-05", "astra"),
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
}
