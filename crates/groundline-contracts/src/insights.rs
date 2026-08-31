use std::collections::BTreeSet;

use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::ContractError;

pub const MAX_WEEKLY_REPORT_BYTES: usize = 128 * 1024;
pub const MAX_BASIC_EVENT_BYTES: usize = 64 * 1024;
const VALID_DAYS: &[u16] = &[7, 30, 90];
const OS_FAMILIES: &[&str] = &["linux", "macos", "unknown", "windows"];
const RUNTIME_FAMILIES: &[&str] = &["codex_app", "codex_cli", "unknown"];
const EXECUTION_MODES: &[&str] = &["desktop", "local_headless", "remote_headless", "unknown"];
const MODEL_FAMILIES: &[&str] = &["gpt-5", "luna", "other", "sol", "terra", "unknown"];
const EFFORTS: &[&str] = &[
    "high", "low", "max", "medium", "minimal", "none", "ultra", "unknown", "unset", "xhigh",
];
const BASIC_TOP_LEVEL_KEYS: &[&str] = &[
    "capabilities",
    "collector",
    "consent",
    "event_id",
    "idempotency_key",
    "kind",
    "metrics",
    "period",
    "privacy",
    "quality_contract",
    "sample",
    "schema_version",
    "source",
];
const FORBIDDEN_BASIC_KEYS: &[&str] = &[
    "command",
    "cwd",
    "hostname",
    "path",
    "prompt",
    "response",
    "thread_id",
    "transcript",
    "user_message",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeeklyReport {
    pub schema_version: u8,
    pub kind: String,
    pub status: String,
    pub reason_code: String,
    pub generated_at_utc: String,
    pub requested_days: u16,
    pub source_contract: SourceContract,
    pub collection_health: CollectionHealth,
    pub coverage: Coverage,
    pub weekly_metrics: WeeklyMetrics,
    pub cohorts: Cohorts,
    pub data_quality: DataQuality,
    pub comparison_readiness: ComparisonReadiness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceContract {
    pub dataset: String,
    pub time_basis: String,
    pub metric_time_field: String,
    pub freshness_time_field: String,
    pub roster_source: String,
    pub analysis_mode: String,
    pub query_set_version: u8,
    pub basic_aggregate_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionHealth {
    pub enrolled_installation_count: u64,
    pub metadata_known_installation_count: u64,
    pub metadata_unknown_installation_count: u64,
    pub observed_installation_count: u64,
    pub reporting_installation_count: u64,
    pub recent_installation_count: u64,
    pub never_reported_installation_count: u64,
    pub pending_initial_report_installation_count: u64,
    pub overdue_never_reported_installation_count: u64,
    pub stale_observed_installation_count: u64,
    pub current_package_claim_installation_count: u64,
    pub current_package_claim_unobserved_installation_count: u64,
    pub current_observed_installation_count: u64,
    pub current_reporting_installation_count: u64,
    pub current_recent_installation_count: u64,
    pub policy_latest_version: String,
    pub roster_status: String,
    pub latest_received_at_utc: Option<String>,
    pub freshness_status: String,
    pub freshness_threshold_hours: u64,
    pub initial_report_grace_hours: u64,
    pub stored_event_row_count: u64,
    pub deduplicated_event_count: u64,
    pub duplicate_event_row_count: u64,
    pub ttl_expired_event_row_count: u64,
    pub delayed_delivery_event_count: u64,
    pub overdue_delivery_event_count: u64,
    pub clock_skew_event_count: u64,
    pub delivery_delay_threshold_hours: u64,
    pub delivery_overdue_threshold_hours: u64,
    pub clock_skew_tolerance_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub event_count: u64,
    pub eligible_root_count: u64,
    pub selected_root_count: u64,
    pub observed_root_count: u64,
    pub completed_turn_count: u64,
    pub unreadable_root_count: u64,
    pub root_truncated_count: u64,
    pub non_root_truncated_count: u64,
    pub originator_unclassified_count: u64,
    pub originator_source_fallback_count: u64,
    pub root_usage_applicable_event_count: u64,
    pub root_usage_missing_event_count: u64,
    pub root_usage_fallback_event_count: u64,
    pub delegated_usage_applicable_event_count: u64,
    pub delegated_usage_missing_event_count: u64,
    pub delegated_usage_fallback_event_count: u64,
    pub guardian_usage_applicable_event_count: u64,
    pub guardian_usage_missing_event_count: u64,
    pub guardian_usage_fallback_event_count: u64,
    pub guardian_incomplete_excluded_count: u64,
    pub completed_root_coverage_applicable_event_count: u64,
    pub completed_root_coverage_capable_event_count: u64,
    pub completed_root_selection_coverage: Option<f64>,
    pub latency_capable_event_count: u64,
    pub boundary_count_capable_event_count: u64,
    pub guardian_attribution_applicable_event_count: u64,
    pub guardian_attribution_capable_event_count: u64,
    pub component_nonpass_event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeeklyMetrics {
    pub tokens: TokenMetrics,
    pub workflow: WorkflowMetrics,
    pub verification: VerificationMetrics,
    pub guardian: GuardianMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenMetrics {
    pub input: u64,
    pub cached_input: u64,
    pub non_cached_input: u64,
    pub output: u64,
    pub reasoning_output: u64,
    pub total: u64,
    pub delegated_total: u64,
    pub guardian_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowMetrics {
    pub compactions: u64,
    pub compactions_per_observed_root: Option<f64>,
    pub long_turn_count: u64,
    pub long_turn_rate: Option<f64>,
    pub exact_repeated_call_groups: u64,
    pub calls_in_exact_repeated_groups: u64,
    pub repeated_call_rate: Option<f64>,
    pub failure_signal_count: u64,
    pub failure_signal_rate: Option<f64>,
    pub tool_call_count: u64,
    pub user_messages_with_text: u64,
    pub short_message_count: u64,
    pub short_message_rate: Option<f64>,
    pub broad_scope_message_count: u64,
    pub broad_scope_message_rate: Option<f64>,
    pub boundary_review_root_count: u64,
    pub long_lived_root_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationMetrics {
    pub tool_call_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub unresolved_count: u64,
    pub outcome_coverage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardianMetrics {
    pub review_count: u64,
    pub workspace_attributed_review_count: u64,
    pub workspace_attribution_coverage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cohorts {
    pub event_distributions: EventDistributions,
    pub installation_distributions: InstallationDistributions,
    pub model_effort_context_distribution: Vec<ModelEffortContext>,
    pub model_effort_token_efficiency: ModelEffortTokenEfficiency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDistributions {
    pub schema_version: Vec<EventDistribution>,
    pub groundline_version: Vec<EventDistribution>,
    pub os_family: Vec<EventDistribution>,
    pub runtime_family: Vec<EventDistribution>,
    pub execution_mode: Vec<EventDistribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDistribution {
    pub value: String,
    pub event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationDistributions {
    pub groundline_version: Vec<InstallationDistribution>,
    pub os_family: Vec<InstallationDistribution>,
    pub runtime_family: Vec<InstallationDistribution>,
    pub execution_mode: Vec<InstallationDistribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationDistribution {
    pub value: String,
    pub installation_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelEffortContext {
    pub model_family: String,
    pub effort: String,
    pub context_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelEffortTokenEfficiency {
    pub status: String,
    pub reason_code: String,
    pub context_distribution_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataQuality {
    pub status: String,
    pub reason_codes: Vec<String>,
    pub sample_sufficient_event_count: u64,
    pub sample_insufficient_event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonReadiness {
    pub status: String,
    pub reason_codes: Vec<String>,
    pub minimum_event_count: u64,
    pub minimum_observed_root_count: u64,
}

fn invalid<T>() -> Result<T, ContractError> {
    Err(ContractError("invalid_weekly_report".to_owned()))
}

fn normalized_timestamp(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok_and(|parsed| {
        let utc = parsed.with_timezone(&Utc);
        let format = if utc.nanosecond() == 0 {
            SecondsFormat::Secs
        } else {
            SecondsFormat::Micros
        };
        utc.to_rfc3339_opts(format, true) == value
    })
}

fn round_ratio(numerator: u64, denominator: u64) -> Option<f64> {
    (denominator != 0)
        .then(|| ((numerator as f64 / denominator as f64) * 10_000.0).round_ties_even() / 10_000.0)
}

fn ratio_matches(value: Option<f64>, numerator: u64, denominator: u64) -> bool {
    numerator <= denominator && value == round_ratio(numerator, denominator)
}

fn optional_non_negative(value: Option<f64>) -> bool {
    value.is_none_or(|value| value.is_finite() && value >= 0.0)
}

fn valid_ratio(value: Option<f64>) -> bool {
    optional_non_negative(value) && value.is_none_or(|value| value <= 1.0)
}

fn sorted_unique(values: &[String]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

fn sums_to(values: impl IntoIterator<Item = u64>, expected: u64) -> bool {
    values
        .into_iter()
        .try_fold(0_u64, |total, value| total.checked_add(value))
        == Some(expected)
}

fn valid_event_distribution(
    distribution: &[EventDistribution],
    total: u64,
    allowed: Option<&[&str]>,
) -> bool {
    distribution.len() <= 128
        && distribution.iter().all(|item| {
            !item.value.is_empty()
                && item.value.len() <= 64
                && item.event_count > 0
                && allowed.is_none_or(|values| values.contains(&item.value.as_str()))
        })
        && distribution
            .windows(2)
            .all(|window| window[0].value < window[1].value)
        && sums_to(distribution.iter().map(|item| item.event_count), total)
}

fn valid_installation_distribution(
    distribution: &[InstallationDistribution],
    total: u64,
    allowed: Option<&[&str]>,
) -> bool {
    distribution.len() <= 128
        && distribution.iter().all(|item| {
            !item.value.is_empty()
                && item.value.len() <= 64
                && item.installation_count > 0
                && allowed.is_none_or(|values| values.contains(&item.value.as_str()))
        })
        && distribution
            .windows(2)
            .all(|window| window[0].value < window[1].value)
        && sums_to(
            distribution.iter().map(|item| item.installation_count),
            total,
        )
}

impl WeeklyReport {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ContractError> {
        if bytes.len() > MAX_WEEKLY_REPORT_BYTES {
            return invalid();
        }
        let report: Self = serde_json::from_slice(bytes)
            .map_err(|_| ContractError("invalid_weekly_report".to_owned()))?;
        report.validate()?;
        if serde_json::to_vec(&report)
            .map_err(|_| ContractError("invalid_weekly_report".to_owned()))?
            .len()
            > MAX_WEEKLY_REPORT_BYTES
        {
            return invalid();
        }
        Ok(report)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != 3
            || self.kind != "groundline-insights-weekly-report"
            || self.status != "PASS"
            || self.reason_code != "accepted"
            || !VALID_DAYS.contains(&self.requested_days)
            || !normalized_timestamp(&self.generated_at_utc)
        {
            return invalid();
        }
        let source = &self.source_contract;
        if source.dataset != "basic_active"
            || source.time_basis != "utc"
            || source.metric_time_field != "period_end_or_generated_at"
            || source.freshness_time_field != "received_at"
            || source.roster_source != "enrolled_installation_registry"
            || source.analysis_mode != "descriptive_single_period"
            || source.query_set_version != 3
            || !source.basic_aggregate_only
        {
            return invalid();
        }

        let health = &self.collection_health;
        if health.roster_status != "AVAILABLE"
            || health.freshness_threshold_hours != 48
            || !matches!(
                health.freshness_status.as_str(),
                "FRESH" | "STALE" | "NO_DATA"
            )
            || health
                .latest_received_at_utc
                .as_deref()
                .is_some_and(|value| !normalized_timestamp(value))
            || health.latest_received_at_utc.is_none() != (health.freshness_status == "NO_DATA")
            || health.reporting_installation_count > health.observed_installation_count
            || health.recent_installation_count > health.reporting_installation_count
            || !sums_to(
                [
                    health.observed_installation_count,
                    health.never_reported_installation_count,
                ],
                health.enrolled_installation_count,
            )
            || !sums_to(
                [
                    health.metadata_known_installation_count,
                    health.metadata_unknown_installation_count,
                ],
                health.enrolled_installation_count,
            )
            || crate::version::strict_version(&health.policy_latest_version).is_err()
            || health.current_package_claim_installation_count > health.enrolled_installation_count
            || health.current_package_claim_unobserved_installation_count
                > health.current_package_claim_installation_count
            || health.current_observed_installation_count > health.enrolled_installation_count
            || health.current_reporting_installation_count
                > health.current_observed_installation_count
            || health.current_recent_installation_count
                > health.current_reporting_installation_count
            || health.initial_report_grace_hours != 24
            || !sums_to(
                [
                    health.pending_initial_report_installation_count,
                    health.overdue_never_reported_installation_count,
                ],
                health.never_reported_installation_count,
            )
            || !sums_to(
                [
                    health.stale_observed_installation_count,
                    health.recent_installation_count,
                ],
                health.observed_installation_count,
            )
            || health.delivery_delay_threshold_hours != 6
            || health.delivery_overdue_threshold_hours != 24
            || health.clock_skew_tolerance_minutes != 5
            || health.stored_event_row_count < health.deduplicated_event_count
            || health.duplicate_event_row_count
                != health.stored_event_row_count - health.deduplicated_event_count
            || health.delayed_delivery_event_count > health.deduplicated_event_count
            || health.overdue_delivery_event_count > health.delayed_delivery_event_count
            || health.clock_skew_event_count > health.deduplicated_event_count
        {
            return invalid();
        }

        let coverage = &self.coverage;
        if coverage.selected_root_count > coverage.eligible_root_count
            || coverage.completed_root_selection_coverage
                != round_ratio(coverage.selected_root_count, coverage.eligible_root_count)
            || coverage.event_count != health.deduplicated_event_count
        {
            return invalid();
        }
        let event_bounded = [
            coverage.root_usage_applicable_event_count,
            coverage.root_usage_missing_event_count,
            coverage.root_usage_fallback_event_count,
            coverage.delegated_usage_applicable_event_count,
            coverage.delegated_usage_missing_event_count,
            coverage.delegated_usage_fallback_event_count,
            coverage.guardian_usage_applicable_event_count,
            coverage.guardian_usage_missing_event_count,
            coverage.guardian_usage_fallback_event_count,
            coverage.completed_root_coverage_applicable_event_count,
            coverage.completed_root_coverage_capable_event_count,
            coverage.latency_capable_event_count,
            coverage.boundary_count_capable_event_count,
            coverage.guardian_attribution_applicable_event_count,
            coverage.guardian_attribution_capable_event_count,
            coverage.component_nonpass_event_count,
        ];
        if event_bounded
            .iter()
            .any(|count| *count > coverage.event_count)
            || coverage.completed_root_coverage_capable_event_count
                > coverage.completed_root_coverage_applicable_event_count
            || coverage.guardian_attribution_capable_event_count
                > coverage.guardian_attribution_applicable_event_count
            || coverage.root_usage_missing_event_count > coverage.root_usage_applicable_event_count
            || coverage.root_usage_fallback_event_count > coverage.root_usage_applicable_event_count
            || coverage.delegated_usage_missing_event_count
                > coverage.delegated_usage_applicable_event_count
            || coverage.delegated_usage_fallback_event_count
                > coverage.delegated_usage_applicable_event_count
            || coverage.guardian_usage_missing_event_count
                > coverage.guardian_usage_applicable_event_count
            || coverage.guardian_usage_fallback_event_count
                > coverage.guardian_usage_applicable_event_count
        {
            return invalid();
        }

        let workflow = &self.weekly_metrics.workflow;
        if !optional_non_negative(workflow.compactions_per_observed_root)
            || workflow.compactions_per_observed_root
                != round_ratio(workflow.compactions, coverage.observed_root_count)
            || !ratio_matches(
                workflow.long_turn_rate,
                workflow.long_turn_count,
                coverage.completed_turn_count,
            )
            || !ratio_matches(
                workflow.repeated_call_rate,
                workflow.calls_in_exact_repeated_groups,
                workflow.tool_call_count,
            )
            || !ratio_matches(
                workflow.failure_signal_rate,
                workflow.failure_signal_count,
                workflow.tool_call_count,
            )
            || !ratio_matches(
                workflow.short_message_rate,
                workflow.short_message_count,
                workflow.user_messages_with_text,
            )
            || !ratio_matches(
                workflow.broad_scope_message_rate,
                workflow.broad_scope_message_count,
                workflow.user_messages_with_text,
            )
        {
            return invalid();
        }
        let verification = &self.weekly_metrics.verification;
        if !valid_ratio(verification.outcome_coverage)
            || !sums_to(
                [
                    verification.success_count,
                    verification.failure_count,
                    verification.unresolved_count,
                ],
                verification.tool_call_count,
            )
            || !ratio_matches(
                verification.outcome_coverage,
                match verification
                    .success_count
                    .checked_add(verification.failure_count)
                {
                    Some(value) => value,
                    None => return invalid(),
                },
                verification.tool_call_count,
            )
        {
            return invalid();
        }
        let guardian = &self.weekly_metrics.guardian;
        if !valid_ratio(guardian.workspace_attribution_coverage)
            || !ratio_matches(
                guardian.workspace_attribution_coverage,
                guardian.workspace_attributed_review_count,
                guardian.review_count,
            )
        {
            return invalid();
        }

        let events = &self.cohorts.event_distributions;
        if !valid_event_distribution(&events.schema_version, coverage.event_count, None)
            || events
                .schema_version
                .iter()
                .any(|item| !matches!(item.value.as_str(), "1" | "2" | "3" | "4" | "5"))
            || !valid_event_distribution(&events.groundline_version, coverage.event_count, None)
            || events
                .groundline_version
                .iter()
                .any(|item| crate::version::strict_version(&item.value).is_err())
            || !valid_event_distribution(&events.os_family, coverage.event_count, Some(OS_FAMILIES))
            || !valid_event_distribution(
                &events.runtime_family,
                coverage.event_count,
                Some(RUNTIME_FAMILIES),
            )
            || !valid_event_distribution(
                &events.execution_mode,
                coverage.event_count,
                Some(EXECUTION_MODES),
            )
        {
            return invalid();
        }
        let installations = &self.cohorts.installation_distributions;
        if !valid_installation_distribution(
            &installations.groundline_version,
            health.enrolled_installation_count,
            None,
        ) || installations.groundline_version.iter().any(|item| {
            item.value != "unknown" && crate::version::strict_version(&item.value).is_err()
        }) || !valid_installation_distribution(
            &installations.os_family,
            health.enrolled_installation_count,
            Some(OS_FAMILIES),
        ) || !valid_installation_distribution(
            &installations.runtime_family,
            health.enrolled_installation_count,
            Some(RUNTIME_FAMILIES),
        ) || !valid_installation_distribution(
            &installations.execution_mode,
            health.enrolled_installation_count,
            Some(EXECUTION_MODES),
        ) {
            return invalid();
        }
        let contexts = &self.cohorts.model_effort_context_distribution;
        if contexts.len() > 256
            || contexts.iter().any(|item| {
                !MODEL_FAMILIES.contains(&item.model_family.as_str())
                    || !EFFORTS.contains(&item.effort.as_str())
                    || item.context_count == 0
            })
            || contexts.windows(2).any(|window| {
                (&window[0].model_family, &window[0].effort)
                    >= (&window[1].model_family, &window[1].effort)
            })
        {
            return invalid();
        }
        let efficiency = &self.cohorts.model_effort_token_efficiency;
        if efficiency.status != "UNAVAILABLE"
            || efficiency.reason_code != "token_usage_not_attributed_to_model_effort"
            || !efficiency.context_distribution_only
        {
            return invalid();
        }

        let mut quality_reasons = BTreeSet::new();
        if coverage.event_count == 0 {
            quality_reasons.insert("no_events");
        } else {
            if coverage.observed_root_count < 5 {
                quality_reasons.insert("insufficient_root_sample");
            }
            if coverage.completed_root_coverage_capable_event_count
                < coverage.completed_root_coverage_applicable_event_count
            {
                quality_reasons.insert("completed_root_coverage_unavailable");
            }
            if coverage.root_truncated_count > 0 {
                quality_reasons.insert("completed_root_selection_truncated");
            }
            if coverage.non_root_truncated_count > 0 {
                quality_reasons.insert("non_root_selection_truncated");
            }
            if coverage.unreadable_root_count > 0 {
                quality_reasons.insert("unreadable_roots");
            }
            if coverage.originator_unclassified_count > 0
                || coverage.originator_source_fallback_count > 0
            {
                quality_reasons.insert("originator_gaps");
            }
            if coverage.root_usage_fallback_event_count > 0
                || coverage.delegated_usage_fallback_event_count > 0
                || coverage.guardian_usage_fallback_event_count > 0
            {
                quality_reasons.insert("usage_fallback_present");
            }
            if coverage.root_usage_missing_event_count > 0
                || coverage.delegated_usage_missing_event_count > 0
                || coverage.guardian_usage_missing_event_count > 0
            {
                quality_reasons.insert("usage_missing");
            }
            if coverage.guardian_incomplete_excluded_count > 0 {
                quality_reasons.insert("guardian_incomplete_excluded");
            }
            if coverage.component_nonpass_event_count > 0 {
                quality_reasons.insert("component_nonpass_present");
            }
            if coverage.latency_capable_event_count < coverage.event_count
                || coverage.completed_turn_count == 0
            {
                quality_reasons.insert("latency_denominator_unavailable");
            }
            if coverage.boundary_count_capable_event_count < coverage.event_count {
                quality_reasons.insert("boundary_counts_unavailable");
            }
            if coverage.guardian_attribution_capable_event_count
                < coverage.guardian_attribution_applicable_event_count
            {
                quality_reasons.insert("guardian_attribution_unavailable");
            }
            if verification.unresolved_count > 0 {
                quality_reasons.insert("verification_outcome_incomplete");
            }
        }
        if health.metadata_unknown_installation_count > 0 {
            quality_reasons.insert("enrollment_metadata_incomplete");
        }
        if health.current_package_claim_unobserved_installation_count > 0 {
            quality_reasons.insert("current_package_observation_incomplete");
        }
        if health.overdue_never_reported_installation_count > 0 {
            quality_reasons.insert("fleet_reporting_incomplete");
            quality_reasons.insert("initial_report_overdue");
        }
        if health.stale_observed_installation_count > 0 {
            quality_reasons.insert("stale_installation_reporting");
        }
        if health.duplicate_event_row_count > 0 {
            quality_reasons.insert("physical_event_duplicates_detected");
        }
        if health.ttl_expired_event_row_count > 0 {
            quality_reasons.insert("retention_cleanup_pending");
        }
        if health.overdue_delivery_event_count > 0 {
            quality_reasons.insert("event_delivery_overdue");
        } else if health.delayed_delivery_event_count > 0 {
            quality_reasons.insert("event_delivery_delayed");
        }
        if health.clock_skew_event_count > 0 {
            quality_reasons.insert("event_clock_skew_detected");
        }
        let expected_quality = quality_reasons
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let quality = &self.data_quality;
        if !sorted_unique(&quality.reason_codes)
            || quality.reason_codes != expected_quality
            || !sums_to(
                [
                    quality.sample_sufficient_event_count,
                    quality.sample_insufficient_event_count,
                ],
                coverage.event_count,
            )
            || (quality.status == "PASS") != quality.reason_codes.is_empty()
            || (quality.status == "FAIL")
                != quality.reason_codes.iter().any(|code| code == "no_events")
            || !matches!(quality.status.as_str(), "PASS" | "PARTIAL" | "FAIL")
        {
            return invalid();
        }

        let mut comparison_reasons = BTreeSet::from(["comparison_baseline_not_included"]);
        if quality.status != "PASS" {
            comparison_reasons.insert("data_quality_not_pass");
        }
        if coverage.event_count < 2 {
            comparison_reasons.insert("too_few_events");
        }
        if coverage.observed_root_count < 5 {
            comparison_reasons.insert("too_few_observed_roots");
        }
        if events.schema_version.len() > 1 {
            comparison_reasons.insert("mixed_schema_versions");
        }
        if events.groundline_version.len() > 1 {
            comparison_reasons.insert("mixed_groundline_versions");
        }
        let expected_comparison = comparison_reasons
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let readiness = &self.comparison_readiness;
        if readiness.status != "INSUFFICIENT"
            || readiness.minimum_event_count != 2
            || readiness.minimum_observed_root_count != 5
            || !sorted_unique(&readiness.reason_codes)
            || readiness.reason_codes != expected_comparison
        {
            return invalid();
        }
        Ok(())
    }
}

fn exact_object_keys(value: &Value, expected: &[&str]) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
}

fn no_forbidden_basic_keys(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object.keys().all(|key| {
                !FORBIDDEN_BASIC_KEYS
                    .iter()
                    .any(|forbidden| key.eq_ignore_ascii_case(forbidden))
            }) && object.values().all(no_forbidden_basic_keys)
        }
        Value::Array(values) => values.iter().all(no_forbidden_basic_keys),
        Value::String(value) => value.len() <= 4096,
        _ => true,
    }
}

fn allowed_string(value: &Value, allowed: &[&str]) -> bool {
    value.as_str().is_some_and(|value| allowed.contains(&value))
}

fn optional_non_negative_number(value: &Value) -> bool {
    value.is_null()
        || value
            .as_f64()
            .is_some_and(|value| value.is_finite() && value >= 0.0)
}

fn non_negative_u64(value: &Value) -> bool {
    value.as_u64().is_some()
}

fn non_negative_u32(value: &Value) -> bool {
    value
        .as_u64()
        .is_some_and(|value| value <= u64::from(u32::MAX))
}

fn allowed_u32_counts(value: &Value, allowed_keys: &[&str]) -> bool {
    value.as_object().is_some_and(|counts| {
        counts.len() <= allowed_keys.len()
            && counts
                .iter()
                .all(|(key, value)| allowed_keys.contains(&key.as_str()) && non_negative_u32(value))
    })
}

fn exact_boolean_contract(value: &Value, expected: &[(&str, bool)]) -> bool {
    exact_object_keys(
        value,
        &expected.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
    ) && expected
        .iter()
        .all(|(key, expected)| value.get(*key).and_then(Value::as_bool) == Some(*expected))
}

fn validate_sample(value: &Value) -> bool {
    const KEYS: &[&str] = &[
        "delegated_count",
        "delegated_truncated_count",
        "eligible_root_count",
        "guardian_count",
        "guardian_incomplete_excluded_count",
        "guardian_truncated_count",
        "minimum_root_count",
        "observed_root_count",
        "originator_source_fallback_root_count",
        "originator_unclassified_excluded_root_count",
        "requested_days",
        "root_count",
        "root_truncated_count",
        "sample_sufficient",
        "selected_recency_end_utc",
        "selected_recency_start_utc",
        "selected_root_count",
        "selection_coverage",
        "selection_mode",
        "unreadable_completed_root_count",
    ];
    if !exact_object_keys(value, KEYS) {
        return false;
    }
    let sample = value.as_object().expect("validated object");
    let counters = [
        "delegated_count",
        "delegated_truncated_count",
        "eligible_root_count",
        "guardian_count",
        "guardian_incomplete_excluded_count",
        "guardian_truncated_count",
        "minimum_root_count",
        "observed_root_count",
        "originator_source_fallback_root_count",
        "originator_unclassified_excluded_root_count",
        "requested_days",
        "root_count",
        "root_truncated_count",
        "selected_root_count",
        "unreadable_completed_root_count",
    ];
    if !counters
        .iter()
        .all(|key| sample.get(*key).is_some_and(non_negative_u32))
        || !sample
            .get("sample_sufficient")
            .is_some_and(Value::is_boolean)
        || !allowed_string(
            sample.get("selection_mode").unwrap_or(&Value::Null),
            &[
                "last_7_days",
                "latest_completed_fallback",
                "requested_window",
                "activity_window",
            ],
        )
        || !sample
            .get("selection_coverage")
            .is_some_and(optional_non_negative_number)
        || sample
            .get("selection_coverage")
            .and_then(Value::as_f64)
            .is_some_and(|value| value > 1.0)
    {
        return false;
    }
    let eligible = sample["eligible_root_count"].as_u64().unwrap();
    let selected = sample["selected_root_count"].as_u64().unwrap();
    let observed = sample["observed_root_count"].as_u64().unwrap();
    let root = sample["root_count"].as_u64().unwrap();
    let minimum = sample["minimum_root_count"].as_u64().unwrap();
    let expected_coverage = round_ratio(selected, eligible);
    let selection_coverage = sample["selection_coverage"].as_f64();
    let recency = [
        sample.get("selected_recency_start_utc"),
        sample.get("selected_recency_end_utc"),
    ];
    selected <= eligible
        && root == selected
        && observed >= root
        && sample["sample_sufficient"].as_bool() == Some(observed >= minimum)
        && selection_coverage == expected_coverage
        && recency.iter().all(|value| {
            value.is_some_and(|value| {
                value.is_null() || value.as_str().is_some_and(normalized_timestamp)
            })
        })
        && match (
            recency[0].and_then(|value| value.as_str()),
            recency[1].and_then(|value| value.as_str()),
        ) {
            (None, None) => true,
            (Some(start), Some(end)) => start <= end,
            _ => false,
        }
}

fn validate_usage(value: &Value, include_non_cached: bool) -> bool {
    let mut keys = vec![
        "cache_write_input_tokens",
        "cached_input_ratio",
        "cached_input_tokens",
        "cumulative_rollout_count",
        "fallback_rollout_count",
        "input_tokens",
        "output_tokens",
        "reasoning_output_tokens",
        "rollout_count_with_usage",
        "source",
        "total_tokens",
    ];
    if include_non_cached {
        keys.push("non_cached_input_tokens");
    }
    if !exact_object_keys(value, &keys) {
        return false;
    }
    let object = value.as_object().expect("validated object");
    let source = object.get("source").and_then(Value::as_str);
    let valid_source = matches!(
        source,
        Some(
            "codex-cumulative-total-snapshots"
                | "codex-cumulative-and-last-usage-fallback"
                | "codex-last-usage-events-summed-fallback"
                | "codex-cumulative-window-delta"
                | "codex-window-delta-and-last-usage-fallback"
                | "codex-last-usage-events-summed-window"
                | "unavailable"
                | "unknown"
        )
    );
    valid_source
        && keys
            .iter()
            .filter(|key| !matches!(**key, "source" | "cached_input_ratio"))
            .all(|key| {
                object.get(*key).is_some_and(|value| {
                    if matches!(
                        *key,
                        "cumulative_rollout_count"
                            | "fallback_rollout_count"
                            | "rollout_count_with_usage"
                    ) {
                        non_negative_u32(value)
                    } else {
                        non_negative_u64(value)
                    }
                })
            })
        && object
            .get("cached_input_ratio")
            .is_some_and(optional_non_negative_number)
        && object
            .get("cached_input_ratio")
            .and_then(Value::as_f64)
            .is_none_or(|value| value <= 1.0)
        && (!include_non_cached
            || object
                .get("input_tokens")
                .and_then(Value::as_u64)
                .zip(object.get("cached_input_tokens").and_then(Value::as_u64))
                .zip(
                    object
                        .get("non_cached_input_tokens")
                        .and_then(Value::as_u64),
                )
                .is_some_and(|((input, cached), non_cached)| {
                    input.saturating_sub(cached) == non_cached
                }))
}

fn validate_session_metrics(value: &Value) -> bool {
    if !exact_object_keys(
        value,
        &[
            "activity",
            "latency",
            "model_effort",
            "quality_proxies",
            "status",
            "tool_categories",
            "usage",
        ],
    ) {
        return false;
    }
    let object = value.as_object().expect("validated object");
    if !allowed_string(
        object.get("status").unwrap_or(&Value::Null),
        &[
            "PASS",
            "PARTIAL",
            "INSUFFICIENT_EVIDENCE",
            "FAIL",
            "UNKNOWN",
        ],
    ) || !validate_usage(object.get("usage").unwrap_or(&Value::Null), true)
        || !exact_object_keys(
            object.get("activity").unwrap_or(&Value::Null),
            &[
                "compactions",
                "task_completed",
                "task_started",
                "turn_contexts",
                "user_messages_with_text",
            ],
        )
        || !object
            .get("activity")
            .and_then(Value::as_object)
            .is_some_and(|activity| activity.values().all(non_negative_u32))
        || !exact_object_keys(
            object.get("latency").unwrap_or(&Value::Null),
            &[
                "completed_count",
                "long_turn_count",
                "max_ms",
                "median_ms",
                "p90_ms",
            ],
        )
        || !object
            .get("latency")
            .and_then(Value::as_object)
            .is_some_and(|latency| {
                latency.iter().all(|(key, value)| match key.as_str() {
                    "completed_count" | "long_turn_count" => non_negative_u32(value),
                    _ => optional_non_negative_number(value),
                })
            })
    {
        return false;
    }
    let Some(model_effort) = object.get("model_effort").and_then(Value::as_array) else {
        return false;
    };
    if model_effort.len() > 16
        || !model_effort.iter().all(|item| {
            exact_object_keys(item, &["count", "effort", "model_family"])
                && item.get("count").is_some_and(non_negative_u32)
                && allowed_string(
                    item.get("model_family").unwrap_or(&Value::Null),
                    MODEL_FAMILIES,
                )
                && allowed_string(item.get("effort").unwrap_or(&Value::Null), EFFORTS)
        })
    {
        return false;
    }
    let quality = object.get("quality_proxies").and_then(Value::as_object);
    let quality_keys = [
        "boundary_review_root_count",
        "broad_scope_message_count",
        "calls_in_exact_repeated_groups",
        "exact_repeated_call_groups",
        "failure_signals",
        "long_lived_root_count",
        "long_lived_root_session",
        "short_message_count",
        "task_boundary_review_recommended",
        "tool_call_count",
        "verification_failure_count",
        "verification_success_count",
        "verification_tool_calls",
        "verification_unresolved_count",
    ];
    let valid_quality = quality.is_some_and(|quality| {
        quality.len() == quality_keys.len()
            && quality_keys.iter().all(|key| quality.contains_key(*key))
            && quality.iter().all(|(key, value)| match key.as_str() {
                "task_boundary_review_recommended" | "long_lived_root_session" => {
                    value.is_boolean()
                }
                "failure_signals" => allowed_u32_counts(
                    value,
                    &[
                        "invalid_arguments",
                        "nonzero_exit",
                        "rejected",
                        "timeout",
                        "yielded_for_wait",
                    ],
                ),
                _ => non_negative_u32(value),
            })
            && allowed_u32_counts(
                object.get("tool_categories").unwrap_or(&Value::Null),
                &[
                    "codex_runtime",
                    "coordination",
                    "git_or_github",
                    "inspection",
                    "mutation",
                    "other_command",
                    "other_tool",
                    "research",
                    "verification",
                    "wait_or_poll",
                ],
            )
            && quality
                .get("verification_tool_calls")
                .and_then(Value::as_u64)
                .zip(
                    quality
                        .get("verification_success_count")
                        .and_then(Value::as_u64),
                )
                .zip(
                    quality
                        .get("verification_failure_count")
                        .and_then(Value::as_u64),
                )
                .zip(
                    quality
                        .get("verification_unresolved_count")
                        .and_then(Value::as_u64),
                )
                .is_some_and(|(((total, success), failure), unresolved)| {
                    success
                        .checked_add(failure)
                        .and_then(|value| value.checked_add(unresolved))
                        == Some(total)
                })
    });
    if !valid_quality {
        return false;
    }

    let activity = object
        .get("activity")
        .and_then(Value::as_object)
        .expect("validated activity");
    let latency = object
        .get("latency")
        .and_then(Value::as_object)
        .expect("validated latency");
    let quality = quality.expect("validated quality proxies");
    let count = |values: &Map<String, Value>, key: &str| {
        values.get(key).and_then(Value::as_u64).unwrap_or(u64::MAX)
    };
    let tool_calls = count(quality, "tool_call_count");
    let repeated_calls = count(quality, "calls_in_exact_repeated_groups");
    let repeated_groups = count(quality, "exact_repeated_call_groups");
    let user_messages = count(activity, "user_messages_with_text");
    let completed_turns = count(activity, "task_completed");
    let latency_completed = count(latency, "completed_count");
    let long_turns = count(latency, "long_turn_count");
    let failure_signals = quality
        .get("failure_signals")
        .and_then(Value::as_object)
        .and_then(|signals| {
            signals
                .values()
                .try_fold(0_u64, |total, value| total.checked_add(value.as_u64()?))
        });

    latency_completed <= completed_turns
        && long_turns <= latency_completed
        && repeated_groups <= repeated_calls
        && repeated_calls <= tool_calls
        && failure_signals.is_some_and(|total| total <= tool_calls)
        && count(quality, "verification_tool_calls") <= tool_calls
        && count(quality, "short_message_count") <= user_messages
        && count(quality, "broad_scope_message_count") <= user_messages
}

fn validate_guardian_metrics(value: &Value) -> bool {
    if !exact_object_keys(
        value,
        &[
            "outcomes",
            "review_count",
            "risk_levels",
            "rollout_count",
            "signals",
            "status",
            "usage",
        ],
    ) {
        return false;
    }
    let object = value.as_object().expect("validated object");
    let structurally_valid =
        allowed_string(
            object.get("status").unwrap_or(&Value::Null),
            &[
                "PASS",
                "PARTIAL",
                "INSUFFICIENT_EVIDENCE",
                "FAIL",
                "UNKNOWN",
            ],
        ) && validate_usage(object.get("usage").unwrap_or(&Value::Null), false)
            && object.get("review_count").is_some_and(non_negative_u32)
            && object.get("rollout_count").is_some_and(non_negative_u32)
            && object.get("outcomes").is_some_and(|value| {
                allowed_u32_counts(
                    value,
                    &["approved", "cancelled", "error", "rejected", "unknown"],
                )
            })
            && object.get("risk_levels").is_some_and(|value| {
                allowed_u32_counts(value, &["critical", "high", "low", "medium", "unknown"])
            })
            && object
                .get("signals")
                .and_then(Value::as_object)
                .is_some_and(|signals| {
                    exact_object_keys(
                        &Value::Object(signals.clone()),
                        &[
                            "outside_workspace_action_rate",
                            "reviewer_already_low_effort",
                            "temporary_workspace_action_rate",
                            "workspace_attributed_review_count",
                            "workspace_attribution_coverage",
                        ],
                    ) && signals
                        .get("reviewer_already_low_effort")
                        .is_some_and(Value::is_boolean)
                        && signals
                            .get("workspace_attributed_review_count")
                            .is_some_and(non_negative_u32)
                        && [
                            "outside_workspace_action_rate",
                            "temporary_workspace_action_rate",
                            "workspace_attribution_coverage",
                        ]
                        .iter()
                        .all(|key| {
                            signals.get(*key).is_some_and(optional_non_negative_number)
                                && signals
                                    .get(*key)
                                    .and_then(Value::as_f64)
                                    .is_none_or(|value| value <= 1.0)
                        })
                });
    if !structurally_valid {
        return false;
    }
    object
        .get("signals")
        .and_then(Value::as_object)
        .and_then(|signals| {
            signals
                .get("workspace_attributed_review_count")
                .and_then(Value::as_u64)
        })
        .zip(object.get("review_count").and_then(Value::as_u64))
        .is_some_and(|(attributed, reviews)| attributed <= reviews)
}

fn validate_basic_semantics(event: &Value) -> bool {
    if !exact_object_keys(event, BASIC_TOP_LEVEL_KEYS) || !no_forbidden_basic_keys(event) {
        return false;
    }
    let object = event.as_object().expect("validated object");
    if object.get("schema_version").and_then(Value::as_u64) != Some(5)
        || object.get("kind").and_then(Value::as_str) != Some("groundline-insights-basic-weekly")
        || !exact_object_keys(
            object.get("collector").unwrap_or(&Value::Null),
            &[
                "execution_mode",
                "instance_id",
                "os_family",
                "runtime_family",
            ],
        )
        || !exact_object_keys(
            object.get("source").unwrap_or(&Value::Null),
            &[
                "audit_kind",
                "audit_schema",
                "collection_generation",
                "collection_trigger",
                "groundline_version",
            ],
        )
        || !exact_object_keys(
            object.get("metrics").unwrap_or(&Value::Null),
            &["delegated", "guardian", "root"],
        )
        || !exact_boolean_contract(
            object.get("capabilities").unwrap_or(&Value::Null),
            &[
                (
                    "completed_root_coverage",
                    object
                        .get("source")
                        .and_then(|value| value.get("audit_kind"))
                        .and_then(Value::as_str)
                        == Some("groundline-codex-weekly-audit"),
                ),
                ("guardian_workspace_attribution", false),
                ("latency_completed_count", true),
                ("root_boundary_counts", true),
            ],
        )
        || !exact_boolean_contract(
            object.get("quality_contract").unwrap_or(&Value::Null),
            &[
                ("billing_inference_performed", false),
                ("correlation_is_not_causation", true),
                ("provider_usage_only", true),
                ("rework_not_observed", true),
                ("verification_is_a_tool_call_proxy", true),
                ("verification_outcome_is_a_tool_result_proxy", true),
            ],
        )
        || !validate_sample(object.get("sample").unwrap_or(&Value::Null))
    {
        return false;
    }
    let collector = object.get("collector").and_then(Value::as_object).unwrap();
    let source = object.get("source").and_then(Value::as_object).unwrap();
    let metrics = object.get("metrics").and_then(Value::as_object).unwrap();
    if collector
        .get("instance_id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .is_none()
        || !allowed_string(
            collector.get("os_family").unwrap_or(&Value::Null),
            OS_FAMILIES,
        )
        || !allowed_string(
            collector.get("runtime_family").unwrap_or(&Value::Null),
            RUNTIME_FAMILIES,
        )
        || !allowed_string(
            collector.get("execution_mode").unwrap_or(&Value::Null),
            EXECUTION_MODES,
        )
        || source.get("audit_schema").and_then(Value::as_u64) != Some(1)
        || !matches!(
            source.get("audit_kind").and_then(Value::as_str),
            Some("groundline-codex-weekly-audit" | "groundline-codex-activity-audit")
        )
        || source
            .get("groundline_version")
            .and_then(Value::as_str)
            .is_none_or(|value| crate::version::strict_version(value).is_err())
        || source
            .get("collection_generation")
            .and_then(Value::as_u64)
            .is_none_or(|value| value > u32::MAX as u64)
        || !matches!(
            source.get("collection_trigger").and_then(Value::as_str),
            Some(
                "manual"
                    | "history_sync"
                    | "session_start_hook"
                    | "stop_hook"
                    | "post_compact_hook"
                    | "session_end_hook"
            )
        )
        || !validate_session_metrics(metrics.get("root").unwrap_or(&Value::Null))
        || !validate_session_metrics(metrics.get("delegated").unwrap_or(&Value::Null))
        || !validate_guardian_metrics(metrics.get("guardian").unwrap_or(&Value::Null))
    {
        return false;
    }
    let valid_period = object
        .get("period")
        .and_then(Value::as_object)
        .is_some_and(|period| {
            if !exact_object_keys(
                &Value::Object(period.clone()),
                &["end_utc", "generated_at_utc", "start_utc"],
            ) || !period
                .get("generated_at_utc")
                .and_then(Value::as_str)
                .is_some_and(normalized_timestamp)
            {
                return false;
            }
            match (
                period.get("start_utc").and_then(Value::as_str),
                period.get("end_utc").and_then(Value::as_str),
            ) {
                (None, None) => {
                    period.get("start_utc").is_some_and(Value::is_null)
                        && period.get("end_utc").is_some_and(Value::is_null)
                }
                (Some(start), Some(end)) => {
                    normalized_timestamp(start) && normalized_timestamp(end) && start < end
                }
                _ => false,
            }
        });
    exact_object_keys(
        object.get("privacy").unwrap_or(&Value::Null),
        &["basic_aggregate_only"],
    ) && object["privacy"]["basic_aggregate_only"] == Value::Bool(true)
        && object
            .get("consent")
            .and_then(Value::as_object)
            .is_some_and(|consent| {
                exact_object_keys(
                    &Value::Object(consent.clone()),
                    &["accepted_at_utc", "receipt_id", "scope"],
                ) && consent.get("scope").and_then(Value::as_str) == Some("basic_weekly")
                    && consent
                        .get("receipt_id")
                        .and_then(Value::as_str)
                        .and_then(|value| Uuid::parse_str(value).ok())
                        .is_some()
                    && consent
                        .get("accepted_at_utc")
                        .and_then(Value::as_str)
                        .is_some_and(normalized_timestamp)
            })
        && valid_period
}

/// Validate the only accepted 0.18 Basic upload contract. Legacy schemas are
/// deliberately rejected at this boundary.
pub fn validate_basic_event_bytes(bytes: &[u8]) -> Result<Value, ContractError> {
    if bytes.is_empty() || bytes.len() > MAX_BASIC_EVENT_BYTES {
        return Err(ContractError("invalid_basic_event".to_owned()));
    }
    let event: Value = serde_json::from_slice(bytes)
        .map_err(|_| ContractError("invalid_basic_event".to_owned()))?;
    if !validate_basic_semantics(&event) {
        return Err(ContractError("invalid_basic_event".to_owned()));
    }
    let mut canonical = event.clone();
    let Some(object) = canonical.as_object_mut() else {
        return Err(ContractError("invalid_basic_event".to_owned()));
    };
    let event_id = object
        .remove("event_id")
        .and_then(|value| value.as_str().map(str::to_owned));
    let idempotency_key = object
        .remove("idempotency_key")
        .and_then(|value| value.as_str().map(str::to_owned));
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|_| ContractError("invalid_basic_event".to_owned()))?;
    let digest = format!("{:x}", Sha256::digest(encoded));
    let expected_event_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, digest.as_bytes()).to_string();
    if event_id.as_deref() != Some(expected_event_id.as_str())
        || idempotency_key.as_deref() != Some(format!("sha256:{digest}").as_str())
    {
        return Err(ContractError("invalid_basic_event".to_owned()));
    }
    Ok(event)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{WeeklyReport, validate_guardian_metrics, validate_session_metrics};

    fn session_metrics() -> Value {
        json!({
            "status":"PASS",
            "activity":{"task_started":4,"task_completed":4,"turn_contexts":4,"compactions":0,"user_messages_with_text":4},
            "latency":{"completed_count":4,"long_turn_count":1,"median_ms":1.0,"p90_ms":2.0,"max_ms":3.0},
            "model_effort":[],
            "tool_categories":{},
            "usage":{"source":"unavailable","input_tokens":0,"cached_input_tokens":0,"non_cached_input_tokens":0,
                "cache_write_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0,"total_tokens":0,
                "cached_input_ratio":null,"cumulative_rollout_count":0,"fallback_rollout_count":0,"rollout_count_with_usage":0},
            "quality_proxies":{"tool_call_count":4,"verification_tool_calls":1,"verification_success_count":1,
                "verification_failure_count":0,"verification_unresolved_count":0,"failure_signals":{"nonzero_exit":1},
                "exact_repeated_call_groups":1,"calls_in_exact_repeated_groups":2,"short_message_count":1,
                "broad_scope_message_count":1,"task_boundary_review_recommended":false,"long_lived_root_session":false,
                "boundary_review_root_count":0,"long_lived_root_count":0}
        })
    }

    fn guardian_metrics() -> Value {
        json!({
            "status":"PASS","rollout_count":2,"review_count":2,
            "usage":{"source":"unavailable","input_tokens":0,"cached_input_tokens":0,"cache_write_input_tokens":0,
                "output_tokens":0,"reasoning_output_tokens":0,"total_tokens":0,"cached_input_ratio":null,
                "cumulative_rollout_count":0,"fallback_rollout_count":0,"rollout_count_with_usage":0},
            "outcomes":{},"risk_levels":{},
            "signals":{"outside_workspace_action_rate":null,"temporary_workspace_action_rate":null,
                "reviewer_already_low_effort":false,"workspace_attributed_review_count":1,
                "workspace_attribution_coverage":0.5}
        })
    }

    #[test]
    fn event_metric_relationships_reject_report_poisoning_inputs() {
        assert!(validate_session_metrics(&session_metrics()));
        for (pointer, value) in [
            ("/latency/completed_count", json!(5)),
            ("/latency/long_turn_count", json!(5)),
            ("/quality_proxies/calls_in_exact_repeated_groups", json!(5)),
            ("/quality_proxies/exact_repeated_call_groups", json!(3)),
            ("/quality_proxies/verification_tool_calls", json!(5)),
            ("/quality_proxies/short_message_count", json!(5)),
            ("/quality_proxies/broad_scope_message_count", json!(5)),
            (
                "/quality_proxies/tool_call_count",
                json!(u64::from(u32::MAX) + 1),
            ),
        ] {
            let mut metrics = session_metrics();
            *metrics.pointer_mut(pointer).expect("fixture pointer") = value;
            assert!(!validate_session_metrics(&metrics), "accepted {pointer}");
        }
        let mut metrics = session_metrics();
        metrics["quality_proxies"]["failure_signals"] = json!({"nonzero_exit":3,"timeout":2});
        assert!(!validate_session_metrics(&metrics));
        let mut metrics = session_metrics();
        metrics["quality_proxies"]["failure_signals"] = json!({"private_path":1});
        assert!(!validate_session_metrics(&metrics));
        let mut metrics = session_metrics();
        metrics["tool_categories"] = json!({"repository_name":1});
        assert!(!validate_session_metrics(&metrics));

        assert!(validate_guardian_metrics(&guardian_metrics()));
        let mut guardian = guardian_metrics();
        guardian["signals"]["workspace_attributed_review_count"] = json!(3);
        assert!(!validate_guardian_metrics(&guardian));
        let mut guardian = guardian_metrics();
        guardian["outcomes"] = json!({"private_identifier":1});
        assert!(!validate_guardian_metrics(&guardian));
    }

    fn report() -> Value {
        json!({
            "schema_version": 3,
            "kind": "groundline-insights-weekly-report",
            "status": "PASS",
            "reason_code": "accepted",
            "generated_at_utc": "2026-08-27T00:00:00Z",
            "requested_days": 7,
            "source_contract": {
                "dataset": "basic_active",
                "time_basis": "utc",
                "metric_time_field": "period_end_or_generated_at",
                "freshness_time_field": "received_at",
                "roster_source": "enrolled_installation_registry",
                "analysis_mode": "descriptive_single_period",
                "query_set_version": 3,
                "basic_aggregate_only": true
            },
            "collection_health": {
                "enrolled_installation_count": 1,
                "metadata_known_installation_count": 1,
                "metadata_unknown_installation_count": 0,
                "observed_installation_count": 1,
                "reporting_installation_count": 1,
                "recent_installation_count": 1,
                "never_reported_installation_count": 0,
                "pending_initial_report_installation_count": 0,
                "overdue_never_reported_installation_count": 0,
                "stale_observed_installation_count": 0,
                "current_package_claim_installation_count": 1,
                "current_package_claim_unobserved_installation_count": 0,
                "current_observed_installation_count": 1,
                "current_reporting_installation_count": 1,
                "current_recent_installation_count": 1,
                "policy_latest_version": "0.18.0",
                "roster_status": "AVAILABLE",
                "latest_received_at_utc": "2026-08-27T00:00:00Z",
                "freshness_status": "FRESH",
                "freshness_threshold_hours": 48,
                "initial_report_grace_hours": 24,
                "stored_event_row_count": 2,
                "deduplicated_event_count": 2,
                "duplicate_event_row_count": 0,
                "ttl_expired_event_row_count": 0,
                "delayed_delivery_event_count": 0,
                "overdue_delivery_event_count": 0,
                "clock_skew_event_count": 0,
                "delivery_delay_threshold_hours": 6,
                "delivery_overdue_threshold_hours": 24,
                "clock_skew_tolerance_minutes": 5
            },
            "coverage": {
                "event_count": 2,
                "eligible_root_count": 5,
                "selected_root_count": 5,
                "observed_root_count": 5,
                "completed_turn_count": 5,
                "unreadable_root_count": 0,
                "root_truncated_count": 0,
                "non_root_truncated_count": 0,
                "originator_unclassified_count": 0,
                "originator_source_fallback_count": 0,
                "root_usage_applicable_event_count": 2,
                "root_usage_missing_event_count": 0,
                "root_usage_fallback_event_count": 0,
                "delegated_usage_applicable_event_count": 0,
                "delegated_usage_missing_event_count": 0,
                "delegated_usage_fallback_event_count": 0,
                "guardian_usage_applicable_event_count": 0,
                "guardian_usage_missing_event_count": 0,
                "guardian_usage_fallback_event_count": 0,
                "guardian_incomplete_excluded_count": 0,
                "completed_root_coverage_applicable_event_count": 2,
                "completed_root_coverage_capable_event_count": 2,
                "completed_root_selection_coverage": 1.0,
                "latency_capable_event_count": 2,
                "boundary_count_capable_event_count": 2,
                "guardian_attribution_applicable_event_count": 0,
                "guardian_attribution_capable_event_count": 0,
                "component_nonpass_event_count": 0
            },
            "weekly_metrics": {
                "tokens": {
                    "input": 100,
                    "cached_input": 80,
                    "non_cached_input": 20,
                    "output": 10,
                    "reasoning_output": 2,
                    "total": 110,
                    "delegated_total": 0,
                    "guardian_total": 0
                },
                "workflow": {
                    "compactions": 0,
                    "compactions_per_observed_root": 0.0,
                    "long_turn_count": 0,
                    "long_turn_rate": 0.0,
                    "exact_repeated_call_groups": 0,
                    "calls_in_exact_repeated_groups": 0,
                    "repeated_call_rate": null,
                    "failure_signal_count": 0,
                    "failure_signal_rate": null,
                    "tool_call_count": 0,
                    "user_messages_with_text": 0,
                    "short_message_count": 0,
                    "short_message_rate": null,
                    "broad_scope_message_count": 0,
                    "broad_scope_message_rate": null,
                    "boundary_review_root_count": 0,
                    "long_lived_root_count": 0
                },
                "verification": {
                    "tool_call_count": 0,
                    "success_count": 0,
                    "failure_count": 0,
                    "unresolved_count": 0,
                    "outcome_coverage": null
                },
                "guardian": {
                    "review_count": 0,
                    "workspace_attributed_review_count": 0,
                    "workspace_attribution_coverage": null
                }
            },
            "cohorts": {
                "event_distributions": {
                    "schema_version": [{"value": "5", "event_count": 2}],
                    "groundline_version": [{"value": "0.18.0", "event_count": 2}],
                    "os_family": [{"value": "macos", "event_count": 2}],
                    "runtime_family": [{"value": "codex_app", "event_count": 2}],
                    "execution_mode": [{"value": "desktop", "event_count": 2}]
                },
                "installation_distributions": {
                    "groundline_version": [{"value": "0.18.0", "installation_count": 1}],
                    "os_family": [{"value": "macos", "installation_count": 1}],
                    "runtime_family": [{"value": "codex_app", "installation_count": 1}],
                    "execution_mode": [{"value": "desktop", "installation_count": 1}]
                },
                "model_effort_context_distribution": [],
                "model_effort_token_efficiency": {
                    "status": "UNAVAILABLE",
                    "reason_code": "token_usage_not_attributed_to_model_effort",
                    "context_distribution_only": true
                }
            },
            "data_quality": {
                "status": "PASS",
                "reason_codes": [],
                "sample_sufficient_event_count": 2,
                "sample_insufficient_event_count": 0
            },
            "comparison_readiness": {
                "status": "INSUFFICIENT",
                "reason_codes": ["comparison_baseline_not_included"],
                "minimum_event_count": 2,
                "minimum_observed_root_count": 5
            }
        })
    }

    #[test]
    fn accepts_the_exact_schema_three_quality_contract() {
        let bytes = serde_json::to_vec(&report()).unwrap();
        let parsed = WeeklyReport::from_slice(&bytes).expect("valid report");
        assert_eq!(parsed.collection_health.enrolled_installation_count, 1);
    }

    #[test]
    fn rejects_legacy_unknown_and_internally_inconsistent_reports() {
        let mut legacy = report();
        legacy["schema_version"] = json!(2);
        assert!(WeeklyReport::from_slice(&serde_json::to_vec(&legacy).unwrap()).is_err());

        let mut unknown = report();
        unknown["private_endpoint"] = json!("not-allowed");
        assert!(WeeklyReport::from_slice(&serde_json::to_vec(&unknown).unwrap()).is_err());

        let mut inconsistent = report();
        inconsistent["collection_health"]["duplicate_event_row_count"] = json!(1);
        assert!(WeeklyReport::from_slice(&serde_json::to_vec(&inconsistent).unwrap()).is_err());
    }

    #[test]
    fn oversized_report_counters_fail_without_panicking() {
        let mut overflowing = report();
        overflowing["collection_health"]["enrolled_installation_count"] = json!(u64::MAX);
        overflowing["collection_health"]["observed_installation_count"] = json!(u64::MAX);
        overflowing["collection_health"]["never_reported_installation_count"] = json!(1);

        assert!(WeeklyReport::from_slice(&serde_json::to_vec(&overflowing).unwrap()).is_err());
    }
}
