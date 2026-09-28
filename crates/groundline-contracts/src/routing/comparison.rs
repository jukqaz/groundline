//! Conservative comparison of directly observed delivery cohorts.
//! A five-percent margin prevents tiny descriptive deltas from churning routing;
//! it is not a confidence interval or proof of causality.
use super::{MIN_UNITS, Outcome};

pub(super) const MIN_IMPROVEMENT_PERCENT: u8 = 5;

pub(super) struct Metrics {
    pub(super) count: usize,
    pub(super) verified: usize,
    pub(super) rework: usize,
    pub(super) tokens: u128,
    pub(super) wall: u128,
    pub(super) complete: bool,
    pub(super) unknown: bool,
    /// Twice the median per-delivery value, preserving half-unit medians exactly.
    pub(super) median_tokens_twice: Option<u128>,
    pub(super) median_wall_twice: Option<u128>,
}

fn doubled_median(values: &mut [u64]) -> Option<u128> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some(u128::from(values[mid - 1]) + u128::from(values[mid]))
    } else {
        Some(2 * u128::from(values[mid]))
    }
}

pub(super) fn metrics(rows: &[&Outcome]) -> Metrics {
    let mut tokens = Vec::with_capacity(rows.len());
    let mut wall = Vec::with_capacity(rows.len());
    let mut result = Metrics {
        count: 0,
        verified: 0,
        rework: 0,
        tokens: 0,
        wall: 0,
        complete: true,
        unknown: false,
        median_tokens_twice: None,
        median_wall_twice: None,
    };
    for row in rows {
        result.count += 1;
        result.verified += usize::from(row.outcome == "verified");
        result.rework += usize::from(row.rework);
        result.unknown |= row.outcome == "unknown";
        result.complete &= row.owned_resources_complete
            && row.owned_total_tokens.is_some()
            && row.wall_duration_ms.is_some();
        if let (Some(owned_tokens), Some(wall_ms)) = (row.owned_total_tokens, row.wall_duration_ms)
        {
            result.tokens += u128::from(owned_tokens);
            result.wall += u128::from(wall_ms);
            tokens.push(owned_tokens);
            wall.push(wall_ms);
        }
    }
    if result.complete {
        result.median_tokens_twice = doubled_median(&mut tokens);
        result.median_wall_twice = doubled_median(&mut wall);
    }
    result
}

pub(super) fn eligibility_reasons(m: &Metrics) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if m.count < MIN_UNITS {
        reasons.push("sample_below_minimum");
    }
    if m.verified == 0 {
        reasons.push("no_verified_delivery");
    }
    if m.unknown {
        reasons.push("unknown_outcome");
    }
    if !m.complete {
        reasons.push("missing_owned_resources");
    }
    reasons
}

pub(super) fn eligible(m: &Metrics) -> bool {
    eligibility_reasons(m).is_empty()
}

fn no_worse(
    candidate: u128,
    candidate_denominator: usize,
    baseline: u128,
    baseline_denominator: usize,
) -> bool {
    candidate * (baseline_denominator as u128) <= baseline * (candidate_denominator as u128)
}

fn margin(
    candidate: u128,
    candidate_denominator: usize,
    baseline: u128,
    baseline_denominator: usize,
) -> bool {
    // At least five percent smaller per verified delivery; includes failed
    // deliveries in the numerator. The bounded 1000-row input fits u128.
    candidate * (baseline_denominator as u128) * 100
        <= baseline * (candidate_denominator as u128) * (100 - u128::from(MIN_IMPROVEMENT_PERCENT))
        && baseline > 0
}

pub(super) fn comparison_reasons(
    candidate: &Metrics,
    baseline: &Metrics,
    objective: &str,
) -> Vec<&'static str> {
    let mut reasons = eligibility_reasons(candidate);
    if !eligible(baseline) {
        reasons.push("baseline_ineligible");
    }
    if !reasons.is_empty() {
        return reasons;
    }
    if candidate.verified * baseline.count < baseline.verified * candidate.count {
        reasons.push("success_rate_regression");
    }
    if candidate.rework * baseline.count > baseline.rework * candidate.count {
        reasons.push("rework_regression");
    }
    let token_no_worse = no_worse(
        candidate.tokens,
        candidate.verified,
        baseline.tokens,
        baseline.verified,
    );
    let wall_no_worse = no_worse(
        candidate.wall,
        candidate.verified,
        baseline.wall,
        baseline.verified,
    );
    if !token_no_worse {
        reasons.push("token_regression");
    }
    if !wall_no_worse {
        reasons.push("latency_regression");
    }
    if candidate.median_tokens_twice > baseline.median_tokens_twice
        || candidate.median_wall_twice > baseline.median_wall_twice
    {
        reasons.push("typical_resource_regression");
    }
    let token_margin = margin(
        candidate.tokens,
        candidate.verified,
        baseline.tokens,
        baseline.verified,
    );
    let wall_margin = margin(
        candidate.wall,
        candidate.verified,
        baseline.wall,
        baseline.verified,
    );
    if !(match objective {
        "tokens" => token_margin,
        "latency" => wall_margin,
        "balanced" => token_margin || wall_margin,
        _ => false,
    }) {
        reasons.push("improvement_below_margin");
    }
    reasons
}

pub(super) fn score(m: &Metrics, objective: &str) -> f64 {
    debug_assert!(eligible(m));
    match objective {
        "tokens" => m.tokens as f64 / m.verified as f64,
        "latency" => m.wall as f64 / m.verified as f64,
        _ => (m.tokens as f64 * m.wall as f64).sqrt() / m.verified as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(tokens: Option<u64>, wall: Option<u64>) -> Outcome {
        Outcome {
            unit_hash: String::new(),
            evidence_sha256: String::new(),
            evidence_kind: "runtime_check".into(),
            cohort_sha256: String::new(),
            model: "gpt-6-sol".into(),
            effort: "medium".into(),
            outcome: "verified".into(),
            rework: false,
            owned_total_tokens: tokens,
            wall_duration_ms: wall,
            owned_resources_complete: true,
            completed_at_utc: String::new(),
        }
    }
    fn cohort(tokens: u64, wall: u64) -> Vec<Outcome> {
        (0..10).map(|_| unit(Some(tokens), Some(wall))).collect()
    }
    fn observe(rows: &[Outcome]) -> Metrics {
        metrics(&rows.iter().collect::<Vec<_>>())
    }

    #[test]
    fn exactly_five_percent_passes_and_one_percent_does_not() {
        let baseline = observe(&cohort(100, 1000));
        let five = observe(&cohort(95, 1000));
        let one = observe(&cohort(99, 1000));
        assert!(comparison_reasons(&five, &baseline, "tokens").is_empty());
        assert!(
            comparison_reasons(&one, &baseline, "tokens").contains(&"improvement_below_margin")
        );
    }
    #[test]
    fn typical_regression_blocks_outlier_driven_total_improvement() {
        let baseline = observe(&cohort(100, 1000));
        let mut candidate = cohort(101, 1000);
        candidate[0].owned_total_tokens = Some(1);
        let candidate = observe(&candidate);
        assert!(candidate.tokens < baseline.tokens);
        assert!(
            comparison_reasons(&candidate, &baseline, "tokens")
                .contains(&"typical_resource_regression")
        );
    }
    #[test]
    fn missing_unknown_and_failed_deliveries_remain_visible() {
        let mut rows = cohort(100, 1000);
        rows[0].owned_total_tokens = None;
        rows[1].outcome = "unknown".into();
        rows[1].evidence_kind = "unobserved".into();
        let m = observe(&rows);
        assert!(!m.complete);
        assert!(m.median_tokens_twice.is_none());
        assert!(eligibility_reasons(&m).contains(&"missing_owned_resources"));
        assert!(eligibility_reasons(&m).contains(&"unknown_outcome"));
        rows[0].owned_total_tokens = Some(100);
        rows[1].outcome = "failed".into();
        let m = observe(&rows);
        assert_eq!(m.tokens, 1000);
        assert_eq!(m.verified, 9);
    }
    #[test]
    fn large_values_are_compared_without_overflow() {
        let baseline = observe(&cohort(u64::MAX, u64::MAX));
        let candidate = observe(&cohort(u64::MAX - 1, u64::MAX - 1));
        assert!(!comparison_reasons(&candidate, &baseline, "balanced").is_empty());
    }
    #[test]
    fn protected_success_rework_and_other_resource_are_independent() {
        let baseline = observe(&cohort(100, 1000));
        let mut rows = cohort(90, 1100);
        rows[0].outcome = "failed".into();
        rows[1].rework = true;
        let candidate = observe(&rows);
        let reasons = comparison_reasons(&candidate, &baseline, "tokens");
        assert!(reasons.contains(&"success_rate_regression"));
        assert!(reasons.contains(&"rework_regression"));
        assert!(reasons.contains(&"latency_regression"));
    }
}
