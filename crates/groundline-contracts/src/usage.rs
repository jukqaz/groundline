/// Canonical four-decimal ratio. A zero denominator means unobserved, not zero.
pub fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    (denominator != 0)
        .then(|| ((numerator as f64 / denominator as f64) * 10_000.0).round_ties_even() / 10_000.0)
}

/// Bounded usage provenance shared by the producer and ingest validator.
#[cfg(feature = "insights")]
pub const SOURCES: &[&str] = &[
    "codex-cumulative-total-snapshots",
    "codex-cumulative-and-last-usage-fallback",
    "codex-last-usage-events-summed-fallback",
    "codex-cumulative-window-delta",
    "codex-window-delta-and-last-usage-fallback",
    "codex-last-usage-events-summed-window",
    "codex-response-usage-records",
    "codex-mixed-usage-sources",
    "unavailable",
    "unknown",
];
