/// Bounded usage provenance shared by the producer and ingest validator.
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
