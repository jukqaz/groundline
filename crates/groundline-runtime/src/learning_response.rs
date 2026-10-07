//! Response-level evidence for one explicitly scoped native task. This reads
//! owner-bound JSONL snapshots; it does not infer acceptance, prices, unseen
//! requests, or ownership from hook/session correlation or a nearby context.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::{DateTime, Utc};
use groundline_contracts::ContractError;
use groundline_contracts::delivery::{
    MAX_RESOURCE_ENTRIES, ResourceEntry, Resources, SelectionObservation,
};
use groundline_contracts::learning::continuous::bytes_sha256;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::rollout::{
    NATIVE_READ_BUDGET, NATIVE_SOURCE_BYTES, NativeSnapshot, read_native_metadata,
    read_owned_native,
};

const MAX_NATIVE_REFS: usize = 64;
const MAX_TURNS: usize = 4096;
const MAX_SEEN_RESPONSES: usize = 16_384;

/// Explicit paths are inputs only. They are never serialized into the proof.
pub struct NativeRef {
    pub path: PathBuf,
    pub session_hash: String,
    pub turn_hash: String,
}

pub struct TaskObservation {
    pub unit_hash: String,
    pub root: NativeRef,
    pub children: Vec<NativeRef>,
    /// Same-session followups require an explicit task ownership declaration.
    /// A later timestamp or the same session alone never adds an owned turn.
    pub owned_root_turn_hashes: Vec<String>,
    pub start_utc: String,
    pub end_utc: String,
}

#[derive(Debug, Serialize)]
pub struct NativeIdentity {
    pub session_hash: Option<String>,
    pub bytes_read: u64,
    pub expanded_bytes: u64,
    pub records_scanned: u64,
    /// Prefix discovery proves neither the full source digest nor its turns.
    pub coverage_complete: bool,
}

#[derive(Debug, Serialize)]
pub struct NativeArtifactReadout {
    pub session_hash: String,
    pub source_sha256: String,
    pub bytes_read: u64,
    pub expanded_bytes: u64,
    pub records_scanned: u64,
    pub owned_response_count: usize,
    pub source_mutation_performed: bool,
    pub source_authenticity_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct SelectionEvidence {
    pub sha256: String,
    /// Canonical object made only from safe labels, hashes and fixed fields.
    /// Persist `serde_json::to_vec_pretty(&artifact)` plus one newline to use
    /// the exact bytes as delivery::record's checked selection artifact.
    pub artifact: Value,
}

#[derive(Debug, Serialize)]
pub struct ResponseProof {
    pub resources: Resources,
    pub root_effective: Option<SelectionObservation>,
    pub closure_observed: bool,
    /// Native root closure, never the supplied cutoff or a hook timestamp.
    pub root_closed_at_utc: Option<String>,
    pub native_artifacts: Vec<NativeArtifactReadout>,
    pub selection_evidence: Vec<SelectionEvidence>,
    /// Fixed codes, without raw IDs, dynamic field names or native error text.
    pub missing: Vec<String>,
    /// A closed, reconciled local observation cannot price requests for which
    /// the provider left no usage record. Unknown is never converted to zero.
    pub unobserved_failure_retry_tokens: Option<u64>,
}

fn error(code: &str) -> ContractError {
    ContractError(format!("learning_response_{code}"))
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn identifier(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.is_empty() && v.len() <= 256 && !v.chars().any(char::is_control))
}

fn scoped_hash(domain: &str, id: &str) -> String {
    bytes_sha256(format!("{domain}\0{id}").as_bytes())
}

fn session(value: Option<&str>) -> Option<String> {
    identifier(value).map(|id| scoped_hash("groundline-hook-session", id))
}

fn turn(value: Option<&str>) -> Option<String> {
    identifier(value).map(|id| scoped_hash("groundline-hook-turn", id))
}

/// Bounded metadata-prefix discovery for sessions and archived sessions. It
/// does not use a filename as native identity or read unrelated full history.
pub fn identify(path: &Path, maximum_bytes: u64) -> Result<NativeIdentity, ContractError> {
    let (metadata, snapshot) = read_native_metadata(path, maximum_bytes)?;
    Ok(NativeIdentity {
        session_hash: metadata
            .as_ref()
            .and_then(|v| session(v["payload"]["id"].as_str())),
        bytes_read: snapshot.bytes_read,
        expanded_bytes: snapshot.expanded_bytes,
        records_scanned: snapshot.records_scanned,
        coverage_complete: false,
    })
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Tokens {
    input_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    reasoning_output_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

impl Tokens {
    fn values(&self) -> [Option<u64>; 5] {
        [
            self.input_tokens,
            self.cached_input_tokens,
            self.output_tokens,
            self.reasoning_output_tokens,
            self.total_tokens,
        ]
    }

    fn complete(&self) -> bool {
        !self.values().contains(&None)
    }

    fn check(&self) -> Result<(), ContractError> {
        for (subset, superset) in [
            (self.cached_input_tokens, self.input_tokens),
            (self.reasoning_output_tokens, self.output_tokens),
            (self.input_tokens, self.total_tokens),
            (self.output_tokens, self.total_tokens),
            (self.cached_input_tokens, self.total_tokens),
            (self.reasoning_output_tokens, self.total_tokens),
        ] {
            if subset.zip(superset).is_some_and(|(a, b)| a > b) {
                return Err(error("invalid_token_arithmetic"));
            }
        }
        if let (Some(input), Some(output)) = (
            self.input_tokens.or(self.cached_input_tokens),
            self.output_tokens.or(self.reasoning_output_tokens),
        ) {
            let minimum = input
                .checked_add(output)
                .ok_or_else(|| error("token_overflow"))?;
            if self.total_tokens.is_some_and(|total| total < minimum)
                || (self.input_tokens.is_some()
                    && self.output_tokens.is_some()
                    && self.total_tokens.is_some_and(|total| total != minimum))
            {
                return Err(error("invalid_token_arithmetic"));
            }
        }
        Ok(())
    }
}

fn tokens(value: &Value) -> Result<Option<Tokens>, ContractError> {
    if value.is_null() {
        return Ok(None);
    }
    let tokens: Tokens =
        serde_json::from_value(value.clone()).map_err(|_| error("invalid_usage_counter"))?;
    tokens.check()?;
    Ok(Some(tokens))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pair {
    model: String,
    effort: String,
}

fn pair(payload: &Value) -> Option<Pair> {
    let model = payload["model"].as_str()?;
    let effort = payload["effort"]
        .as_str()
        .or_else(|| payload["reasoning_effort"].as_str())?;
    if !groundline_contracts::model::optimization_model(model)
        || !["low", "medium", "high", "xhigh", "max", "ultra"].contains(&effort)
        || payload["effort"]
            .as_str()
            .zip(payload["reasoning_effort"].as_str())
            .is_some_and(|(a, b)| a != b)
    {
        return None;
    }
    Some(Pair {
        model: model.to_owned(),
        effort: effort.to_owned(),
    })
}

#[derive(Default)]
struct TurnData {
    context_seen: bool,
    pair: Option<Pair>,
    context_conflict: bool,
    started_at: Option<DateTime<Utc>>,
    closed_at: Option<DateTime<Utc>>,
    aborted: bool,
    response_count: usize,
    sum: [Option<u64>; 5],
    last_total: Option<Tokens>,
    reconciliation_seen: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct Metadata {
    session_hash: String,
    parent_hash: Option<String>,
    inherited: bool,
    owned_from: Option<u64>,
}

#[derive(Clone, PartialEq, Eq)]
struct ResponseSignature {
    turn_hash: Option<String>,
    root_turn_hash: Option<String>,
    usage: Tokens,
    thread_total: Option<Tokens>,
    turn_total: Option<Tokens>,
}

struct Response {
    response_hash: String,
    scope_turn: String,
    exact_turn: Option<String>,
    usage: Tokens,
    at: DateTime<Utc>,
}

struct Notification {
    turn_hash: String,
    last: Option<Tokens>,
    total: Option<Tokens>,
}

struct Parsed {
    session_hash: String,
    selected: BTreeSet<String>,
    is_child: bool,
    root_session: String,
    root_turns: BTreeSet<String>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    metadata: Option<Metadata>,
    turns: BTreeMap<String, TurnData>,
    seen: BTreeMap<String, ResponseSignature>,
    responses: Vec<Response>,
    notifications: Vec<Notification>,
    active_turn: Option<String>,
    previous_ordinal: Option<u64>,
    child_links: BTreeSet<String>,
    missing: BTreeSet<String>,
    unlinked_children: bool,
    lineage_observed: bool,
}

impl Parsed {
    fn missing(&mut self, code: &str) {
        self.missing.insert(code.to_owned());
    }

    fn in_window(&self, at: Option<DateTime<Utc>>) -> bool {
        at.is_some_and(|at| at >= self.start && at <= self.end)
    }

    fn metadata(&mut self, payload: &Value) -> Result<(), ContractError> {
        let hash =
            session(payload["id"].as_str()).ok_or_else(|| error("native_session_missing"))?;
        if hash != self.session_hash {
            return Err(error("native_session_mismatch"));
        }
        let direct = session(payload["parent_thread_id"].as_str());
        let nested =
            session(payload["source"]["subagent"]["thread_spawn"]["parent_thread_id"].as_str());
        if direct
            .as_ref()
            .zip(nested.as_ref())
            .is_some_and(|(a, b)| a != b)
        {
            return Err(error("native_lineage_conflict"));
        }
        let parent_hash = direct.or(nested);
        if self.is_child
            && parent_hash
                .as_ref()
                .is_some_and(|p| p != &self.root_session)
        {
            return Err(error("native_child_parent_mismatch"));
        }
        let inherited = payload.get("history_base").is_some()
            || payload.get("forked_from_id").is_some()
            || payload.get("subagent_history_start_ordinal").is_some();
        let owned_from = [
            payload["history_base"]["end_ordinal_exclusive"].as_u64(),
            payload["forked_from_ordinal_exclusive"].as_u64(),
            payload["subagent_history_start_ordinal"].as_u64(),
        ]
        .into_iter()
        .flatten()
        .max();
        let metadata = Metadata {
            session_hash: hash,
            parent_hash,
            inherited,
            owned_from: if inherited && payload["history_mode"] != "paginated" {
                None
            } else {
                owned_from
            },
        };
        if self.metadata.as_ref().is_some_and(|old| old != &metadata) {
            return Err(error("native_metadata_conflict"));
        }
        if inherited && metadata.owned_from.is_none() {
            self.missing("native_ownership_boundary_missing");
        }
        self.metadata = Some(metadata);
        Ok(())
    }

    fn accept(&mut self, record: Value) -> Result<(), ContractError> {
        let kind = record["type"].as_str().unwrap_or_default();
        let payload = &record["payload"];
        if kind == "session_meta" {
            return self.metadata(payload);
        }
        let Some(metadata) = self.metadata.as_ref() else {
            return Err(error("native_metadata_missing"));
        };
        let ordinal = record["ordinal"].as_u64();
        if metadata.inherited {
            let Some(owned_from) = metadata.owned_from else {
                return Ok(());
            };
            let Some(ordinal) = ordinal else {
                self.missing("native_ownership_ordinal_missing");
                return Ok(());
            };
            if ordinal < owned_from {
                return Ok(());
            }
        }
        let at = record["timestamp"]
            .as_str()
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.with_timezone(&Utc));
        let turn_hash = turn(payload["turn_id"].as_str());
        // Identical response replay is allowed even if its replay ordinal is
        // not increasing. Every conflicting stable response record fails.
        if kind == "token_usage_record" {
            return self.response(payload, at, ordinal);
        }
        if metadata.inherited {
            if ordinal
                .zip(self.previous_ordinal)
                .is_some_and(|(a, b)| a <= b)
            {
                self.missing("native_ownership_ordinal_conflict");
                return Ok(());
            }
            self.previous_ordinal = ordinal;
        }
        match kind {
            "turn_context" => {
                if let Some(turn_hash) = turn_hash {
                    if !self.turns.contains_key(&turn_hash) && self.turns.len() >= MAX_TURNS {
                        return Err(error("native_turn_limit_exceeded"));
                    }
                    let candidate = pair(payload);
                    if self.is_child
                        && self.selected.contains(&turn_hash)
                        && turn(payload["root_turn_id"].as_str())
                            .is_some_and(|t| self.root_turns.contains(&t))
                        && self.metadata.as_ref().and_then(|m| m.parent_hash.as_ref())
                            == Some(&self.root_session)
                    {
                        self.lineage_observed = true;
                    }
                    let observed = self.turns.entry(turn_hash).or_default();
                    if observed.context_seen && observed.pair != candidate {
                        observed.context_conflict = true;
                        observed.pair = None;
                    } else if !observed.context_conflict {
                        observed.pair = candidate;
                    }
                    observed.context_seen = true;
                }
            }
            "compacted" if self.in_window(at) => {
                // A marker can represent a billed request without a matching
                // delta. A real delta remains usable, but is never given a
                // nearby/root model when its exact context is absent.
                self.missing("native_compaction_coverage_unknown");
            }
            "response_item" if self.in_window(at) => {
                let namespace = payload["namespace"].as_str().filter(|v| !v.is_empty());
                if matches!(
                    payload["type"].as_str(),
                    Some("function_call" | "custom_tool_call")
                ) && (namespace == Some("collaboration")
                    || payload["name"].as_str().is_some_and(|name| {
                        // A qualified agent name is explicit. Bare native
                        // names are conservative only without a namespace;
                        // another declared namespace owns its same-name tools.
                        let agent_name = name
                            .strip_prefix("collaboration.")
                            .or_else(|| namespace.is_none().then_some(name));
                        matches!(
                            agent_name,
                            Some(
                                "spawn_agent"
                                    | "followup_task"
                                    | "send_message"
                                    | "send_input"
                                    | "resume_agent"
                                    | "wait_agent"
                                    | "wait"
                                    | "interrupt_agent"
                                    | "list_agents"
                                    | "close_agent"
                            )
                        )
                    }))
                {
                    // Current App rollouts can omit collab event_msg inventory.
                    // The invocation signals potentially unobserved child cost;
                    // arguments/results never become inventory or ownership.
                    self.missing("native_subagent_inventory_unknown");
                }
            }
            "event_msg" => self.event(payload, at, turn_hash)?,
            _ => {}
        }
        Ok(())
    }

    fn response(
        &mut self,
        payload: &Value,
        at: Option<DateTime<Utc>>,
        ordinal: Option<u64>,
    ) -> Result<(), ContractError> {
        let turn_hash = turn(payload["turn_id"].as_str());
        let root_turn_hash = turn(payload["root_turn_id"].as_str());
        let scope = turn_hash
            .as_ref()
            .filter(|t| self.selected.contains(*t))
            .cloned()
            .or_else(|| {
                (!self.is_child)
                    .then(|| {
                        root_turn_hash
                            .as_ref()
                            .filter(|t| self.selected.contains(*t))
                            .cloned()
                    })
                    .flatten()
            });
        let relevant = scope.is_some() && self.in_window(at);
        let owner = session(payload["thread_id"].as_str());
        if owner.as_ref() != Some(&self.session_hash) {
            if relevant {
                self.missing("native_response_owner_unmatched");
            }
            return Ok(());
        }
        let Some(id) = identifier(payload["response_id"].as_str()) else {
            if self.in_window(at) || scope.is_some() {
                self.missing("native_response_identity_missing");
            }
            return Ok(());
        };
        let response_hash = scoped_hash("groundline-native-response", id);
        let usage = tokens(&payload["usage"])?.unwrap_or_default();
        let signature = ResponseSignature {
            turn_hash: turn_hash.clone(),
            root_turn_hash: root_turn_hash.clone(),
            usage: usage.clone(),
            thread_total: tokens(&payload["thread_token_usage"])?,
            turn_total: tokens(&payload["turn_token_usage"])?,
        };
        if let Some(previous) = self.seen.get(&response_hash) {
            if previous != &signature {
                return Err(error("native_response_conflict"));
            }
            return Ok(());
        }
        if self.seen.len() >= MAX_SEEN_RESPONSES {
            return Err(error("native_response_limit_exceeded"));
        }
        self.seen.insert(response_hash.clone(), signature.clone());
        if self.metadata.as_ref().is_some_and(|v| v.inherited) {
            if ordinal
                .zip(self.previous_ordinal)
                .is_some_and(|(a, b)| a <= b)
            {
                self.missing("native_ownership_ordinal_conflict");
                return Ok(());
            }
            self.previous_ordinal = ordinal;
        }
        let Some(scope_turn) = scope else {
            if self.in_window(at) {
                self.missing("native_unlinked_turn_usage");
            }
            return Ok(());
        };
        if self.is_child {
            if self.metadata.as_ref().and_then(|m| m.parent_hash.as_ref())
                != Some(&self.root_session)
            {
                self.missing("native_child_parent_unknown");
                return Ok(());
            }
            let Some(root_turn_hash) = root_turn_hash else {
                self.missing("native_child_root_turn_unknown");
                return Ok(());
            };
            if !self.root_turns.contains(&root_turn_hash) {
                return Err(error("native_child_root_turn_mismatch"));
            }
            self.lineage_observed = true;
        }
        let Some(at) = at else {
            self.missing("native_response_timestamp_missing");
            return Ok(());
        };
        if at < self.start {
            self.missing("native_owned_usage_before_start");
            return Ok(());
        }
        if at > self.end {
            self.missing("native_owned_usage_after_cutoff");
            return Ok(());
        }
        if self.responses.len() >= MAX_RESOURCE_ENTRIES {
            return Err(error("resource_entry_limit_exceeded"));
        }
        if !usage.complete() {
            self.missing("native_usage_counters_missing");
        }
        let observed = self.turns.entry(scope_turn.clone()).or_default();
        if observed.response_count == 0 {
            observed.sum = [Some(0); 5];
        }
        for (sum, increment) in observed.sum.iter_mut().zip(usage.values()) {
            *sum = match (*sum, increment) {
                (Some(a), Some(b)) => {
                    Some(a.checked_add(b).ok_or_else(|| error("token_overflow"))?)
                }
                _ => None,
            };
        }
        observed.response_count += 1;
        observed.reconciliation_seen = false;
        if let Some(total) = signature.turn_total {
            for (sum, counter) in observed.sum.iter().zip(total.values()) {
                if sum.zip(counter).is_some_and(|(a, b)| a != b) {
                    self.missing
                        .insert("native_usage_reconciliation_unmatched".to_owned());
                }
            }
            observed.reconciliation_seen =
                total.complete() && observed.sum.iter().all(Option::is_some);
            observed.last_total = Some(total);
        }
        self.responses.push(Response {
            response_hash,
            scope_turn,
            exact_turn: turn_hash,
            usage,
            at,
        });
        Ok(())
    }

    fn event(
        &mut self,
        payload: &Value,
        at: Option<DateTime<Utc>>,
        turn_hash: Option<String>,
    ) -> Result<(), ContractError> {
        let kind = payload["type"].as_str().unwrap_or_default();
        match kind {
            "task_started" => {
                self.active_turn = turn_hash.clone();
                if let Some(turn_hash) = turn_hash.filter(|t| self.selected.contains(t)) {
                    let observed = self.turns.entry(turn_hash).or_default();
                    if let Some(at) = at.filter(|at| *at <= self.end) {
                        if observed.started_at.is_some_and(|old| old != at) {
                            self.missing("native_turn_start_conflict");
                        } else {
                            observed.started_at = Some(at);
                        }
                    }
                }
            }
            "task_complete" | "turn_aborted" => {
                if self.active_turn.as_ref() == turn_hash.as_ref() {
                    self.active_turn = None;
                }
                if let Some(turn_hash) = turn_hash.filter(|t| self.selected.contains(t)) {
                    let observed = self.turns.entry(turn_hash).or_default();
                    if let Some(at) = at.filter(|at| *at >= self.start && *at <= self.end) {
                        observed.closed_at = Some(observed.closed_at.map_or(at, |old| old.max(at)));
                        observed.aborted |= kind == "turn_aborted";
                    }
                }
            }
            "token_count" if self.in_window(at) => {
                let owned = turn_hash.or_else(|| self.active_turn.clone());
                if let Some(owned) = owned.as_ref().filter(|t| self.selected.contains(*t)) {
                    // Notifications are an independent coverage check, never
                    // another resource row or a synthetic response identity.
                    if self.notifications.len() >= MAX_SEEN_RESPONSES {
                        return Err(error("native_notification_limit_exceeded"));
                    }
                    self.notifications.push(Notification {
                        turn_hash: owned.clone(),
                        last: tokens(&payload["info"]["last_token_usage"])?,
                        total: tokens(&payload["info"]["total_token_usage"])?,
                    });
                } else if owned.is_none() {
                    self.missing("native_unscoped_token_notification");
                }
            }
            "collab_agent_spawn_end" | "collab_agent_interaction_end" if self.in_window(at) => {
                if turn_hash
                    .as_ref()
                    .is_some_and(|t| self.selected.contains(t))
                    && !self.is_child
                {
                    if session(payload["sender_thread_id"].as_str()).as_ref()
                        != Some(&self.session_hash)
                    {
                        self.missing("native_child_inventory_unknown");
                    } else if let Some(child) = session(payload["receiver_thread_id"].as_str()) {
                        self.child_links.insert(child);
                    } else {
                        self.missing("native_child_inventory_unknown");
                    }
                } else if self.is_child && self.in_window(at) {
                    self.unlinked_children = true;
                    self.missing("native_descendant_coverage_unknown");
                }
            }
            "error" | "stream_error" | "request_retry" | "turn_failed" if self.in_window(at) => {
                self.missing("native_failed_or_retry_cost_unknown");
            }
            _ => {}
        }
        Ok(())
    }

    fn finish(&mut self) {
        let mut notification_missing = false;
        let mut notification_unmatched = false;
        for notification in &self.notifications {
            if let Some(last) = &notification.last {
                notification_unmatched |= !self
                    .responses
                    .iter()
                    .any(|r| r.scope_turn == notification.turn_hash && r.usage == *last);
            } else {
                notification_missing = true;
            }
            if let Some(total) = &notification.total {
                notification_unmatched |= !self.seen.values().any(|r| {
                    r.turn_hash.as_ref() == Some(&notification.turn_hash)
                        && r.thread_total.as_ref() == Some(total)
                });
            }
        }
        if notification_missing {
            self.missing("native_token_notification_missing");
        }
        if notification_unmatched {
            self.missing("native_token_notification_unmatched");
        }
        for selected in self.selected.clone() {
            let Some(observed) = self.turns.get(&selected) else {
                self.missing("native_owned_turn_unobserved");
                continue;
            };
            let context_missing = !observed.context_seen || observed.pair.is_none();
            let context_conflict = observed.context_conflict;
            let start_missing = observed.started_at.is_none();
            let closure_missing = observed.closed_at.is_none();
            let aborted = observed.aborted;
            let no_usage = observed.response_count == 0;
            let coverage_missing = !observed.reconciliation_seen;
            let bad_chronology = observed
                .started_at
                .zip(observed.closed_at)
                .is_some_and(|(s, e)| s > e)
                || self.responses.iter().any(|r| {
                    r.scope_turn == selected
                        && (observed.started_at.is_some_and(|s| r.at < s)
                            || observed.closed_at.is_some_and(|e| r.at > e))
                });
            if context_missing {
                self.missing("native_context_unmatched");
            }
            if context_conflict {
                self.missing("native_context_conflict");
            }
            if start_missing {
                self.missing("native_turn_start_missing");
            }
            if closure_missing {
                self.missing(if self.is_child {
                    "native_child_closure_missing"
                } else {
                    "native_owned_turn_closure_missing"
                });
            }
            if aborted {
                self.missing("native_failed_or_retry_cost_unknown");
            }
            if no_usage {
                self.missing("native_usage_missing");
            }
            if coverage_missing {
                self.missing("native_usage_coverage_unknown");
            }
            if bad_chronology {
                self.missing("native_response_chronology_unmatched");
            }
        }
    }

    fn closed(&self) -> bool {
        self.selected
            .iter()
            .all(|t| self.turns.get(t).is_some_and(|v| v.closed_at.is_some()))
    }
}

fn selection_evidence(
    session_hash: &str,
    turn_hash: &str,
    pair: &Pair,
    native_source_sha256: &str,
) -> Result<SelectionEvidence, ContractError> {
    let artifact = json!({
        "kind":"groundline-native-selection-evidence", "schema":1,
        "session_hash":session_hash, "turn_hash":turn_hash,
        "model":pair.model, "effort":pair.effort,
        "native_source_sha256":native_source_sha256,
        "source_authenticity_verified":false, "native_activation":"UNVERIFIED"
    });
    let mut bytes =
        serde_json::to_vec_pretty(&artifact).map_err(|_| error("selection_projection_failed"))?;
    bytes.push(b'\n');
    Ok(SelectionEvidence {
        sha256: bytes_sha256(&bytes),
        artifact,
    })
}

/// Observe only declared root turns and explicitly supplied child turns. The
/// response ID digest is stable across task/time windows and source snapshots
/// so delivery collection rejects reuse across independently claimed tasks.
pub fn observe(request: &TaskObservation) -> Result<ResponseProof, ContractError> {
    observe_with_recheck(request, || {})
}

fn observe_with_recheck(
    request: &TaskObservation,
    before_recheck: impl FnOnce(),
) -> Result<ResponseProof, ContractError> {
    let started = Instant::now();
    let start = DateTime::parse_from_rfc3339(&request.start_utc)
        .map_err(|_| error("invalid_timestamp"))?
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339(&request.end_utc)
        .map_err(|_| error("invalid_timestamp"))?
        .with_timezone(&Utc);
    if !digest(&request.unit_hash)
        || end < start
        || end > Utc::now() + chrono::Duration::minutes(5)
        || request.children.len() + 1 > MAX_NATIVE_REFS
        || request.owned_root_turn_hashes.len() >= MAX_TURNS
    {
        return Err(error("invalid_task_scope"));
    }
    let mut root_turns = BTreeSet::from([request.root.turn_hash.clone()]);
    for owned in &request.owned_root_turn_hashes {
        if !digest(owned) || !root_turns.insert(owned.clone()) {
            return Err(error("invalid_owned_turns"));
        }
    }
    let mut paths = BTreeMap::new();
    let mut session_paths = BTreeMap::new();
    let mut refs = BTreeSet::new();
    let mut child_groups = BTreeMap::<String, (&NativeRef, BTreeSet<String>)>::new();
    for native in std::iter::once(&request.root).chain(&request.children) {
        if !digest(&native.session_hash)
            || !digest(&native.turn_hash)
            || !refs.insert((&native.session_hash, &native.turn_hash))
            || paths
                .get(&native.path)
                .is_some_and(|s| *s != &native.session_hash)
            || session_paths
                .get(&native.session_hash)
                .is_some_and(|p| *p != &native.path)
        {
            return Err(error("invalid_native_refs"));
        }
        paths.insert(&native.path, &native.session_hash);
        session_paths.insert(&native.session_hash, &native.path);
    }
    for native in &request.children {
        if native.session_hash == request.root.session_hash {
            return Err(error("invalid_native_refs"));
        }
        child_groups
            .entry(native.session_hash.clone())
            .or_insert_with(|| (native, BTreeSet::new()))
            .1
            .insert(native.turn_hash.clone());
    }
    let mut parsed = Vec::<Parsed>::new();
    let mut sources = Vec::<NativeSnapshot>::new();
    let mut remaining = NATIVE_SOURCE_BYTES;
    for (index, (native, selected)) in std::iter::once((&request.root, root_turns.clone()))
        .chain(child_groups.into_values())
        .enumerate()
    {
        let mut state = Parsed {
            session_hash: native.session_hash.clone(),
            selected,
            is_child: index != 0,
            root_session: request.root.session_hash.clone(),
            root_turns: root_turns.clone(),
            start,
            end,
            metadata: None,
            turns: BTreeMap::new(),
            seen: BTreeMap::new(),
            responses: Vec::new(),
            notifications: Vec::new(),
            active_turn: None,
            previous_ordinal: None,
            child_links: BTreeSet::new(),
            missing: BTreeSet::new(),
            unlinked_children: false,
            lineage_observed: index == 0,
        };
        let snapshot = read_owned_native(&native.path, remaining, started, |record| {
            state.accept(record)
        })?;
        remaining = remaining.saturating_sub(snapshot.bytes_read.max(snapshot.expanded_bytes));
        if state.metadata.is_none() {
            return Err(error("native_session_missing"));
        }
        if !state.turns.contains_key(&native.turn_hash)
            && !state.missing.contains("native_ownership_boundary_missing")
            && !state.missing.contains("native_ownership_ordinal_missing")
        {
            return Err(error("native_turn_mismatch"));
        }
        state.finish();
        parsed.push(state);
        sources.push(snapshot);
    }
    before_recheck();
    for source in &sources {
        source.recheck()?;
    }
    if started.elapsed() > NATIVE_READ_BUDGET {
        return Err(error("processing_budget_exceeded"));
    }
    let supplied: BTreeSet<String> = request
        .children
        .iter()
        .map(|r| r.session_hash.clone())
        .collect();
    let mut missing = BTreeSet::new();
    for state in &parsed {
        missing.extend(state.missing.iter().cloned());
    }
    let child_coverage = parsed[0].child_links == supplied;
    if !child_coverage {
        missing.insert("native_child_coverage_missing".to_owned());
    }
    if !parsed[0]
        .turns
        .get(&request.root.turn_hash)
        .is_some_and(|v| v.closed_at.is_some())
    {
        missing.insert("native_root_closure_missing".to_owned());
    }
    let root_closed = parsed[0]
        .closed()
        .then(|| {
            parsed[0]
                .selected
                .iter()
                .filter_map(|t| parsed[0].turns.get(t)?.closed_at)
                .max()
        })
        .flatten();
    let closure_observed = parsed.iter().all(Parsed::closed)
        && parsed.iter().all(|p| !p.unlinked_children)
        && parsed[0].child_links.is_subset(&supplied)
        && parsed.iter().all(|p| p.lineage_observed);
    let latest_closure = closure_observed
        .then(|| {
            parsed
                .iter()
                .flat_map(|p| {
                    p.selected
                        .iter()
                        .filter_map(|t| p.turns.get(t).and_then(|v| v.closed_at))
                })
                .max()
        })
        .flatten();
    let wall_duration_ms =
        latest_closure.and_then(|at| u64::try_from((at - start).num_milliseconds()).ok());
    let mut evidence = BTreeMap::<String, SelectionEvidence>::new();
    let mut root_effective = None;
    let mut entries = Vec::new();
    let mut response_owners = BTreeMap::<String, String>::new();
    for (state, source) in parsed.iter().zip(&sources) {
        let mut selections = BTreeMap::new();
        let needed_turns: BTreeSet<&String> = state
            .selected
            .iter()
            .chain(state.responses.iter().filter_map(|r| r.exact_turn.as_ref()))
            .collect();
        for (turn_hash, observed) in &state.turns {
            if needed_turns.contains(turn_hash)
                && let Some(pair) = &observed.pair
            {
                let projection = selection_evidence(
                    &state.session_hash,
                    turn_hash,
                    pair,
                    &source.source_sha256,
                )?;
                let selection = SelectionObservation {
                    model: pair.model.clone(),
                    effort: pair.effort.clone(),
                    evidence_sha256: projection.sha256.clone(),
                };
                evidence.insert(projection.sha256.clone(), projection);
                if !state.is_child && *turn_hash == request.root.turn_hash {
                    root_effective = Some(selection.clone());
                }
                selections.insert(turn_hash.clone(), selection);
            }
        }
        for response in &state.responses {
            if response_owners
                .insert(response.response_hash.clone(), state.session_hash.clone())
                .is_some()
            {
                return Err(error("native_response_ownership_conflict"));
            }
            let effective = response
                .exact_turn
                .as_ref()
                .and_then(|t| selections.get(t).cloned());
            if effective.is_none() {
                missing.insert("native_response_context_unmatched".to_owned());
            }
            entries.push(ResourceEntry {
                owner: if state.is_child { "child" } else { "root" }.to_owned(),
                unit_hash: if state.is_child {
                    state.session_hash.clone()
                } else {
                    request.unit_hash.clone()
                },
                response_hash: response.response_hash.clone(),
                effective,
                input_tokens: response.usage.input_tokens,
                cached_input_tokens: response.usage.cached_input_tokens,
                output_tokens: response.usage.output_tokens,
                reasoning_output_tokens: response.usage.reasoning_output_tokens,
                total_tokens: response.usage.total_tokens,
            });
            if entries.len() > MAX_RESOURCE_ENTRIES {
                return Err(error("resource_entry_limit_exceeded"));
            }
        }
    }
    let mut total = 0_u64;
    for entry in &entries {
        if let Some(tokens) = entry.total_tokens {
            total = total
                .checked_add(tokens)
                .ok_or_else(|| error("token_overflow"))?;
        }
    }
    let _ = total;
    let complete = closure_observed
        && child_coverage
        && missing.is_empty()
        && wall_duration_ms.is_some()
        && entries.iter().any(|r| r.owner == "root");
    Ok(ResponseProof {
        resources: Resources {
            complete,
            wall_duration_ms,
            entries,
        },
        root_effective,
        closure_observed,
        root_closed_at_utc: root_closed.map(|v| v.to_rfc3339()),
        native_artifacts: parsed
            .iter()
            .zip(&sources)
            .map(|(state, source)| NativeArtifactReadout {
                session_hash: state.session_hash.clone(),
                source_sha256: source.source_sha256.clone(),
                bytes_read: source.bytes_read,
                expanded_bytes: source.expanded_bytes,
                records_scanned: source.records_scanned,
                owned_response_count: state.responses.len(),
                source_mutation_performed: false,
                source_authenticity_verified: false,
            })
            .collect(),
        selection_evidence: evidence.into_values().collect(),
        missing: missing.into_iter().collect(),
        unobserved_failure_retry_tokens: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::{PermissionsExt, symlink};

    const ROOT: &str = "private-native-root";
    const TURN: &str = "private-native-turn";
    const CHILD: &str = "private-native-child";
    const CHILD_TURN: &str = "private-child-turn";

    fn at(second: u32) -> String {
        format!("2026-10-01T00:00:{second:02}Z")
    }

    fn record(second: u32, kind: &str, payload: Value) -> Value {
        json!({"timestamp":at(second),"type":kind,"payload":payload})
    }

    fn count(input: u64, cached: u64, output: u64, reasoning: u64) -> Value {
        json!({"input_tokens":input,"cached_input_tokens":cached,
            "output_tokens":output,"reasoning_output_tokens":reasoning,
            "total_tokens":input+output,"cache_write_input_tokens":0})
    }

    fn usage(
        second: u32,
        owner: &str,
        owned_turn: &str,
        id: &str,
        delta: Value,
        total: Value,
    ) -> Value {
        record(
            second,
            "token_usage_record",
            json!({
                "session_id":owner,"thread_id":owner,"turn_id":owned_turn,
                "root_turn_id":TURN,"response_id":id,"usage":delta,
                "thread_token_usage":total,"turn_token_usage":total
            }),
        )
    }

    fn root_rows() -> Vec<Value> {
        vec![
            record(
                0,
                "session_meta",
                json!({"id":ROOT,"base_instructions":"private instruction"}),
            ),
            record(
                1,
                "event_msg",
                json!({"type":"task_started","turn_id":TURN}),
            ),
            record(
                1,
                "turn_context",
                json!({"turn_id":TURN,"model":"gpt-6-astra","effort":"high","cwd":"private directory"}),
            ),
            record(
                1,
                "event_msg",
                json!({"type":"user_message","message":"private request"}),
            ),
            usage(
                2,
                ROOT,
                TURN,
                "private-response-1",
                count(10, 3, 4, 1),
                count(10, 3, 4, 1),
            ),
            usage(
                3,
                ROOT,
                TURN,
                "private-response-2",
                count(6, 1, 2, 1),
                count(16, 4, 6, 2),
            ),
            record(
                5,
                "event_msg",
                json!({"type":"task_complete","turn_id":TURN,"last_agent_message":"private result"}),
            ),
        ]
    }

    fn bytes(rows: &[Value]) -> Vec<u8> {
        let mut bytes = rows
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes();
        bytes.push(b'\n');
        bytes
    }

    struct Fixture {
        _directory: tempfile::TempDir,
        path: PathBuf,
        request: TaskObservation,
    }

    fn fixture(rows: &[Value]) -> Fixture {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .canonicalize()
            .unwrap()
            .join("native.jsonl");
        fs::write(&path, bytes(rows)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        Fixture {
            _directory: directory,
            request: TaskObservation {
                unit_hash: bytes_sha256(b"task scope"),
                root: NativeRef {
                    path: path.clone(),
                    session_hash: session(Some(ROOT)).unwrap(),
                    turn_hash: turn(Some(TURN)).unwrap(),
                },
                children: Vec::new(),
                owned_root_turn_hashes: Vec::new(),
                start_utc: at(1),
                end_utc: at(7),
            },
            path,
        }
    }

    fn add_child(f: &mut Fixture, rows: &[Value]) {
        let path = f.path.with_file_name("child.jsonl");
        fs::write(&path, bytes(rows)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        f.request.children.push(NativeRef {
            path,
            session_hash: session(Some(CHILD)).unwrap(),
            turn_hash: turn(Some(CHILD_TURN)).unwrap(),
        });
    }

    fn child_rows() -> Vec<Value> {
        vec![
            record(
                1,
                "session_meta",
                json!({"id":CHILD,"parent_thread_id":ROOT,
                "source":{"subagent":{"thread_spawn":{"parent_thread_id":ROOT,"depth":1,
                    "agent_nickname":"private nickname","agent_path":"private path"}}}}),
            ),
            record(
                2,
                "event_msg",
                json!({"type":"task_started","turn_id":CHILD_TURN}),
            ),
            record(
                2,
                "turn_context",
                json!({"turn_id":CHILD_TURN,"model":"gpt-6-luna","reasoning_effort":"medium"}),
            ),
            usage(
                3,
                CHILD,
                CHILD_TURN,
                "private-child-response",
                count(5, 2, 2, 1),
                count(5, 2, 2, 1),
            ),
            record(
                4,
                "event_msg",
                json!({"type":"task_complete","turn_id":CHILD_TURN}),
            ),
        ]
    }

    fn code(request: &TaskObservation) -> String {
        observe(request).unwrap_err().0
    }

    #[test]
    fn real_response_increments_have_exact_context_and_closed_complete_readout() {
        let rows = root_rows();
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert!(proof.resources.complete);
        assert!(proof.closure_observed);
        assert_eq!(
            proof.root_closed_at_utc,
            Some("2026-10-01T00:00:05+00:00".to_owned())
        );
        assert_eq!(proof.resources.wall_duration_ms, Some(4000));
        assert_eq!(proof.resources.entries.len(), 2);
        assert_eq!(
            proof
                .resources
                .entries
                .iter()
                .map(|r| r.total_tokens.unwrap())
                .sum::<u64>(),
            22
        );
        assert_eq!(proof.resources.entries[0].cached_input_tokens, Some(3));
        assert_eq!(proof.resources.entries[0].reasoning_output_tokens, Some(1));
        assert!(proof.unobserved_failure_retry_tokens.is_none());
        assert_eq!(proof.root_effective.as_ref().unwrap().model, "gpt-6-astra");
        assert_eq!(
            proof.resources.entries[0]
                .effective
                .as_ref()
                .unwrap()
                .effort,
            "high"
        );
        assert_eq!(
            proof.native_artifacts[0].source_sha256,
            bytes_sha256(&bytes(&rows))
        );
        for evidence in &proof.selection_evidence {
            let mut bytes = serde_json::to_vec_pretty(&evidence.artifact).unwrap();
            bytes.push(b'\n');
            assert_eq!(evidence.sha256, bytes_sha256(&bytes));
        }
        let serialized = serde_json::to_string(&proof).unwrap();
        for private in [
            ROOT,
            TURN,
            "private-response",
            "private instruction",
            "private request",
            "private result",
            "private directory",
        ] {
            assert!(!serialized.contains(private), "private source leaked");
        }
        assert!(!serialized.contains(f.path.to_str().unwrap()));
    }

    #[test]
    fn exact_replay_deduplicates_but_conflicting_response_record_fails() {
        let mut rows = root_rows();
        let mut replay = rows[4].clone();
        replay["timestamp"] = json!(at(3));
        rows.insert(5, replay.clone());
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert!(proof.resources.complete);
        assert_eq!(proof.resources.entries.len(), 2);
        replay["payload"]["usage"] = count(11, 3, 4, 1);
        rows[5] = replay;
        fs::write(&f.path, bytes(&rows)).unwrap();
        assert_eq!(
            code(&f.request),
            "learning_response_native_response_conflict"
        );
    }

    #[test]
    fn stable_response_hash_does_not_change_with_unit_or_cutoff() {
        let mut f = fixture(&root_rows());
        let first = observe(&f.request).unwrap();
        f.request.unit_hash = bytes_sha256(b"another claimed task");
        f.request.end_utc = at(8);
        let second = observe(&f.request).unwrap();
        assert_eq!(
            first.resources.entries[0].response_hash,
            second.resources.entries[0].response_hash
        );
        assert_ne!(
            first.resources.entries[0].unit_hash,
            second.resources.entries[0].unit_hash
        );
    }

    #[test]
    fn root_and_child_contamination_cannot_be_charged_to_the_task() {
        let mut rows = root_rows();
        rows.insert(
            5,
            usage(
                3,
                "foreign-thread",
                TURN,
                "foreign-response",
                count(999, 0, 1, 0),
                count(999, 0, 1, 0),
            ),
        );
        let mut f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert!(!proof.resources.complete);
        assert_eq!(proof.resources.entries.len(), 2);
        assert!(
            proof
                .missing
                .contains(&"native_response_owner_unmatched".to_owned())
        );
        let mut child = child_rows();
        child[0]["payload"]["parent_thread_id"] = json!("another-parent");
        child[0]["payload"]["source"]["subagent"]["thread_spawn"]["parent_thread_id"] =
            json!("another-parent");
        add_child(&mut f, &child);
        assert_eq!(
            code(&f.request),
            "learning_response_native_child_parent_mismatch"
        );
    }

    #[test]
    fn actual_child_root_turn_links_cost_but_absent_inventory_remains_partial() {
        let mut f = fixture(&root_rows());
        add_child(&mut f, &child_rows());
        let proof = observe(&f.request).unwrap();
        assert!(proof.closure_observed);
        assert!(!proof.resources.complete);
        assert!(
            proof
                .missing
                .contains(&"native_child_coverage_missing".to_owned())
        );
        let child = proof
            .resources
            .entries
            .iter()
            .find(|r| r.owner == "child")
            .unwrap();
        assert_eq!(child.total_tokens, Some(7));
        assert_eq!(child.effective.as_ref().unwrap().model, "gpt-6-luna");
        assert_eq!(child.effective.as_ref().unwrap().effort, "medium");
        let serialized = serde_json::to_string(&proof).unwrap();
        assert!(!serialized.contains(CHILD));
        assert!(!serialized.contains("private nickname"));
    }

    #[test]
    fn explicit_same_child_followup_turns_share_one_checked_source() {
        let mut rows = child_rows();
        rows.push(record(
            4,
            "event_msg",
            json!({"type":"task_started","turn_id":"child-followup"}),
        ));
        rows.push(record(
            4,
            "turn_context",
            json!({"turn_id":"child-followup","model":"gpt-6-sol","effort":"low"}),
        ));
        let mut followup = usage(
            4,
            CHILD,
            "child-followup",
            "child-followup-response",
            count(2, 0, 1, 0),
            count(2, 0, 1, 0),
        );
        followup["payload"]["thread_token_usage"] = count(7, 2, 3, 1);
        rows.push(followup);
        rows.push(record(
            5,
            "event_msg",
            json!({"type":"task_complete","turn_id":"child-followup"}),
        ));
        let mut f = fixture(&root_rows());
        add_child(&mut f, &rows);
        f.request.children.push(NativeRef {
            path: f.request.children[0].path.clone(),
            session_hash: session(Some(CHILD)).unwrap(),
            turn_hash: turn(Some("child-followup")).unwrap(),
        });
        let proof = observe(&f.request).unwrap();
        assert!(proof.closure_observed);
        assert_eq!(proof.native_artifacts.len(), 2);
        assert_eq!(
            proof
                .resources
                .entries
                .iter()
                .filter(|r| r.owner == "child")
                .count(),
            2
        );
        assert_eq!(
            proof
                .resources
                .entries
                .iter()
                .find(|r| r.effective.as_ref().is_some_and(|e| e.model == "gpt-6-sol"))
                .unwrap()
                .total_tokens,
            Some(3)
        );
        assert!(!proof.resources.complete);
    }

    #[test]
    fn child_parent_metadata_alone_and_wrong_root_turn_cannot_claim_cost() {
        let mut child = child_rows();
        child[0]["payload"]["root_turn_id"] = json!(TURN);
        child[3]["payload"]
            .as_object_mut()
            .unwrap()
            .remove("root_turn_id");
        let mut f = fixture(&root_rows());
        add_child(&mut f, &child);
        let proof = observe(&f.request).unwrap();
        assert!(!proof.closure_observed);
        assert_eq!(proof.resources.entries.len(), 2);
        assert!(
            proof
                .missing
                .contains(&"native_child_root_turn_unknown".to_owned())
        );
        child[3]["payload"]["root_turn_id"] = json!("previous-parent-task");
        fs::write(&f.request.children[0].path, bytes(&child)).unwrap();
        assert_eq!(
            code(&f.request),
            "learning_response_native_child_root_turn_mismatch"
        );
    }

    #[test]
    fn actual_agent_tool_invocations_block_false_zero_child_complete_without_blocking_closure() {
        for tool in ["spawn_agent", "followup_task", "send_message"] {
            let mut rows = root_rows();
            rows.insert(
                5,
                record(
                    3,
                    "response_item",
                    json!({"type":"function_call",
                "name":tool,"namespace":"collaboration","call_id":"private call",
                "arguments":"{\"message\":\"private child request\",\"id\":\"private child id\"}"}),
                ),
            );
            let f = fixture(&rows);
            let proof = observe(&f.request).unwrap();
            assert!(!proof.resources.complete, "{tool}");
            assert!(proof.closure_observed, "{tool}");
            assert!(
                proof
                    .missing
                    .contains(&"native_subagent_inventory_unknown".to_owned())
            );
            assert_eq!(proof.resources.entries.len(), 2);
            let serialized = serde_json::to_string(&proof).unwrap();
            for private in ["private child request", "private child id", "private call"] {
                assert!(!serialized.contains(private));
            }
        }
    }

    #[test]
    fn same_name_tools_respect_explicit_namespace_and_qualified_agent_identity() {
        for (namespace, name, agent) in [
            (Some("collaboration"), "send_message", true),
            (Some("collaboration"), "wait", true),
            (None, "send_message", true),
            (None, "wait", true),
            (None, "collaboration.send_message", true),
            (None, "collaboration.wait", true),
            (Some("mcp__slack"), "send_message", false),
            (Some("mcp__slack"), "wait", false),
            (None, "mcp__slack.send_message", false),
        ] {
            let mut payload = json!({"type":"function_call","name":name,
                "arguments":"private tool arguments"});
            if let Some(namespace) = namespace {
                payload["namespace"] = json!(namespace);
            }
            let mut rows = root_rows();
            rows.insert(5, record(3, "response_item", payload));
            let f = fixture(&rows);
            let proof = observe(&f.request).unwrap();
            assert_eq!(proof.resources.complete, !agent, "{namespace:?}/{name}");
            assert!(proof.closure_observed);
            assert_eq!(
                proof
                    .missing
                    .contains(&"native_subagent_inventory_unknown".to_owned()),
                agent
            );
            assert!(
                !serde_json::to_string(&proof)
                    .unwrap()
                    .contains("private tool arguments")
            );
        }
    }

    #[test]
    fn logged_retry_is_unknown_cost_and_does_not_export_native_failure_text() {
        let mut rows = root_rows();
        rows.insert(
            5,
            record(
                3,
                "event_msg",
                json!({"type":"request_retry",
            "message":"private provider failure", "endpoint":"private endpoint"}),
            ),
        );
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert!(proof.closure_observed);
        assert!(!proof.resources.complete);
        assert!(proof.unobserved_failure_retry_tokens.is_none());
        assert!(
            proof
                .missing
                .contains(&"native_failed_or_retry_cost_unknown".to_owned())
        );
        assert!(
            !serde_json::to_string(&proof)
                .unwrap()
                .contains("private provider failure")
        );
    }

    #[test]
    fn child_exact_response_id_reused_as_root_is_an_ownership_conflict() {
        let mut child = child_rows();
        child[3]["payload"]["response_id"] = json!("private-response-1");
        let mut f = fixture(&root_rows());
        add_child(&mut f, &child);
        assert_eq!(
            code(&f.request),
            "learning_response_native_response_ownership_conflict"
        );
    }

    #[test]
    fn unrelated_same_session_turn_is_not_owned_without_an_explicit_ref() {
        let mut rows = root_rows();
        rows.push(record(
            5,
            "event_msg",
            json!({"type":"task_started","turn_id":"followup"}),
        ));
        rows.push(record(
            5,
            "turn_context",
            json!({"turn_id":"followup","model":"gpt-6-sol","effort":"low"}),
        ));
        let mut followup = usage(
            6,
            ROOT,
            "followup",
            "followup-response",
            count(2, 0, 1, 0),
            count(2, 0, 1, 0),
        );
        followup["payload"]["root_turn_id"] = json!("followup");
        rows.push(followup);
        rows.push(record(
            7,
            "event_msg",
            json!({"type":"task_complete","turn_id":"followup"}),
        ));
        let mut f = fixture(&rows);
        let first = observe(&f.request).unwrap();
        assert_eq!(first.resources.entries.len(), 2);
        assert!(!first.resources.complete);
        assert!(
            first
                .missing
                .contains(&"native_unlinked_turn_usage".to_owned())
        );
        f.request
            .owned_root_turn_hashes
            .push(turn(Some("followup")).unwrap());
        let second = observe(&f.request).unwrap();
        assert_eq!(second.resources.entries.len(), 3);
        assert!(second.resources.complete);
        assert_eq!(second.root_effective.as_ref().unwrap().model, "gpt-6-astra");
        assert_eq!(
            second.root_closed_at_utc,
            Some("2026-10-01T00:00:07+00:00".to_owned())
        );
    }

    #[test]
    fn actual_context_requires_the_exact_turn_and_conflicts_are_nullable() {
        let mut rows = root_rows();
        rows[2]["payload"]["turn_id"] = json!("unrelated-context-turn");
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert!(proof.root_effective.is_none());
        assert!(
            proof
                .resources
                .entries
                .iter()
                .all(|r| r.effective.is_none())
        );
        assert!(!proof.resources.complete);
        rows[2]["payload"]["turn_id"] = json!(TURN);
        rows.insert(
            3,
            record(
                1,
                "turn_context",
                json!({"turn_id":TURN,"model":"gpt-6-sol","effort":"low"}),
            ),
        );
        fs::write(&f.path, bytes(&rows)).unwrap();
        let proof = observe(&f.request).unwrap();
        assert!(proof.root_effective.is_none());
        assert!(
            proof
                .missing
                .contains(&"native_context_conflict".to_owned())
        );
    }

    #[test]
    fn compaction_cost_keeps_a_null_pair_instead_of_inheriting_the_root() {
        let mut rows = root_rows();
        rows.insert(
            6,
            record(
                4,
                "compacted",
                json!({"root_turn_id":TURN,"replacement_history":"private compacted history"}),
            ),
        );
        let mut compact = usage(
            4,
            ROOT,
            "compaction-turn-without-context",
            "compaction-response",
            count(2, 0, 1, 0),
            count(19, 4, 6, 2),
        );
        compact["payload"]["turn_token_usage"] = Value::Null;
        rows.insert(7, compact);
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert_eq!(proof.resources.entries.len(), 3);
        assert_eq!(proof.resources.entries[2].total_tokens, Some(3));
        assert!(proof.resources.entries[2].effective.is_none());
        assert!(!proof.resources.complete);
        assert!(
            proof
                .missing
                .contains(&"native_response_context_unmatched".to_owned())
        );
        assert!(
            !serde_json::to_string(&proof)
                .unwrap()
                .contains("private compacted history")
        );
    }

    #[test]
    fn missing_counters_stay_null_and_a_missing_response_is_not_zero_cost() {
        let mut rows = root_rows();
        rows[4]["payload"]["usage"] = json!({"total_tokens":14});
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert_eq!(proof.resources.entries[0].input_tokens, None);
        assert_eq!(proof.resources.entries[0].cached_input_tokens, None);
        assert_eq!(proof.resources.entries[0].total_tokens, Some(14));
        assert!(!proof.resources.complete);
        assert!(proof.closure_observed);
        let rows = root_rows()
            .into_iter()
            .filter(|r| r["type"] != "token_usage_record")
            .collect::<Vec<_>>();
        fs::write(&f.path, bytes(&rows)).unwrap();
        let proof = observe(&f.request).unwrap();
        assert!(proof.closure_observed);
        assert!(proof.resources.entries.is_empty());
        assert!(proof.missing.contains(&"native_usage_missing".to_owned()));
    }

    #[test]
    fn counter_coverage_detects_an_unrecorded_request_and_invalid_subset() {
        let mut rows = root_rows();
        rows.remove(4);
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert_eq!(proof.resources.entries.len(), 1);
        assert!(!proof.resources.complete);
        assert!(
            proof
                .missing
                .contains(&"native_usage_reconciliation_unmatched".to_owned())
        );
        rows = root_rows();
        rows[4]["payload"]["usage"]["cached_input_tokens"] = json!(11);
        fs::write(&f.path, bytes(&rows)).unwrap();
        assert_eq!(
            code(&f.request),
            "learning_response_invalid_token_arithmetic"
        );
    }

    #[test]
    fn token_notifications_are_checks_not_extra_rows_and_can_precede_persistence() {
        let mut rows = root_rows();
        rows.insert(
            4,
            record(
                2,
                "event_msg",
                json!({"type":"token_count","info":{
            "last_token_usage":count(10,3,4,1),"total_token_usage":count(10,3,4,1)}}),
            ),
        );
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert!(proof.resources.complete);
        assert_eq!(proof.resources.entries.len(), 2);
        rows[4]["payload"]["info"]["last_token_usage"] = count(999, 0, 1, 0);
        fs::write(&f.path, bytes(&rows)).unwrap();
        let proof = observe(&f.request).unwrap();
        assert!(!proof.resources.complete);
        assert!(
            proof
                .missing
                .contains(&"native_token_notification_unmatched".to_owned())
        );
    }

    #[test]
    fn exact_native_closure_is_required_and_abort_has_unknown_unlogged_cost() {
        let mut rows = root_rows();
        rows.pop();
        let mut f = fixture(&rows);
        let open = observe(&f.request).unwrap();
        assert!(!open.closure_observed);
        assert!(open.root_closed_at_utc.is_none());
        rows.push(record(
            8,
            "event_msg",
            json!({"type":"task_complete","turn_id":TURN}),
        ));
        fs::write(&f.path, bytes(&rows)).unwrap();
        assert!(!observe(&f.request).unwrap().closure_observed);
        f.request.end_utc = at(9);
        assert!(observe(&f.request).unwrap().closure_observed);
        rows.pop();
        rows.push(record(
            8,
            "event_msg",
            json!({"type":"turn_aborted","turn_id":TURN}),
        ));
        fs::write(&f.path, bytes(&rows)).unwrap();
        let proof = observe(&f.request).unwrap();
        assert!(proof.closure_observed);
        assert!(!proof.resources.complete);
        assert!(
            proof
                .missing
                .contains(&"native_failed_or_retry_cost_unknown".to_owned())
        );
    }

    #[test]
    fn fork_prefix_and_exact_response_replay_are_not_new_child_or_root_cost() {
        let mut rows = root_rows();
        rows[0]["payload"]["history_mode"] = json!("paginated");
        rows[0]["payload"]["history_base"] =
            json!({"end_ordinal_exclusive":12,"path":"private ancestor"});
        rows[0]["payload"]["forked_from_id"] = json!("private ancestor");
        for (i, row) in rows.iter_mut().enumerate().skip(1) {
            row["ordinal"] = json!(i + 11);
        }
        let mut inherited = usage(
            1,
            ROOT,
            TURN,
            "inherited-response",
            count(1000, 0, 1, 0),
            count(1000, 0, 1, 0),
        );
        inherited["ordinal"] = json!(11);
        rows.insert(1, inherited);
        rows.insert(6, rows[5].clone());
        let f = fixture(&rows);
        let proof = observe(&f.request).unwrap();
        assert_eq!(proof.resources.entries.len(), 2);
        assert!(proof.resources.complete);
        rows[0]["payload"]["history_base"] = json!({"path":"unknown boundary"});
        fs::write(&f.path, bytes(&rows)).unwrap();
        let proof = observe(&f.request).unwrap();
        assert!(!proof.resources.complete);
        assert!(proof.resources.entries.is_empty());
        assert!(
            proof
                .missing
                .contains(&"native_ownership_boundary_missing".to_owned())
        );
    }

    #[test]
    fn expected_session_and_turn_are_checked_against_native_identity() {
        let mut f = fixture(&root_rows());
        f.request.root.session_hash = session(Some("other-session")).unwrap();
        assert_eq!(
            code(&f.request),
            "learning_response_native_session_mismatch"
        );
        f.request.root.session_hash = session(Some(ROOT)).unwrap();
        f.request.root.turn_hash = turn(Some("other-turn")).unwrap();
        assert_eq!(code(&f.request), "learning_response_native_turn_mismatch");
    }

    #[test]
    fn growth_same_length_mutation_and_path_rebinding_fail_snapshot_checks() {
        for change in ["grow", "same-length", "rebind"] {
            let f = fixture(&root_rows());
            let error = observe_with_recheck(&f.request, || match change {
                "grow" => {
                    let mut file = fs::OpenOptions::new().append(true).open(&f.path).unwrap();
                    file.write_all(b"{}\n").unwrap();
                }
                "same-length" => {
                    let mut source = fs::read(&f.path).unwrap();
                    source[0] = b' ';
                    fs::write(&f.path, source).unwrap();
                }
                "rebind" => {
                    let source = fs::read(&f.path).unwrap();
                    fs::rename(&f.path, f.path.with_extension("retained")).unwrap();
                    fs::write(&f.path, source).unwrap();
                }
                _ => unreachable!(),
            })
            .unwrap_err();
            assert_eq!(error.0, "learning_response_source_changed", "{change}");
        }
    }

    #[test]
    fn oversize_and_unsafe_sources_are_rejected_before_attribution() {
        let f = fixture(&root_rows());
        fs::OpenOptions::new()
            .write(true)
            .open(&f.path)
            .unwrap()
            .set_len(NATIVE_SOURCE_BYTES + 1)
            .unwrap();
        assert_eq!(
            code(&f.request),
            "learning_response_source_byte_limit_exceeded"
        );
        fs::write(&f.path, bytes(&root_rows())).unwrap();
        fs::set_permissions(&f.path, fs::Permissions::from_mode(0o666)).unwrap();
        assert_eq!(code(&f.request), "learning_response_source_owner_required");
        fs::set_permissions(&f.path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::hard_link(&f.path, f.path.with_extension("linked")).unwrap();
        assert_eq!(code(&f.request), "learning_response_source_owner_required");
        fs::remove_file(f.path.with_extension("linked")).unwrap();
        let source = f.path.with_extension("original");
        fs::rename(&f.path, &source).unwrap();
        symlink(&source, &f.path).unwrap();
        assert_eq!(code(&f.request), "learning_response_source_unavailable");
    }

    #[test]
    fn compressed_observation_digests_stored_bytes_and_bounds_expansion() {
        let mut f = fixture(&root_rows());
        let raw = fs::read(&f.path).unwrap();
        let compressed = zstd::stream::encode_all(raw.as_slice(), 1).unwrap();
        let path = f.path.with_extension("jsonl.zst");
        fs::write(&path, &compressed).unwrap();
        f.request.root.path = path.clone();
        let proof = observe(&f.request).unwrap();
        assert!(proof.resources.complete);
        assert_eq!(
            proof.native_artifacts[0].source_sha256,
            bytes_sha256(&compressed)
        );
        assert_eq!(proof.native_artifacts[0].expanded_bytes, raw.len() as u64);
        let budget = compressed.len() as u64 + 1;
        let error = read_owned_native(&path, budget, Instant::now(), |_| Ok(()))
            .err()
            .unwrap();
        assert_eq!(error.0, "learning_response_source_byte_limit_exceeded");
    }

    #[test]
    fn discovery_reads_only_metadata_prefix_and_does_not_claim_turn_or_full_digest() {
        let rows = vec![
            root_rows()[0].clone(),
            record(
                1,
                "response_item",
                json!({"type":"message","content":"x".repeat(1024*1024)}),
            ),
        ];
        let f = fixture(&rows);
        let identity = identify(&f.path, 2 * 1024 * 1024).unwrap();
        assert_eq!(identity.session_hash, session(Some(ROOT)));
        assert_eq!(identity.records_scanned, 1);
        assert!(identity.bytes_read < fs::metadata(&f.path).unwrap().len());
        assert!(!identity.coverage_complete);
        let output = serde_json::to_string(&identity).unwrap();
        assert!(!output.contains("source_sha256"));
        assert!(!output.contains("turn_hash"));
        assert!(!output.contains(ROOT));
    }
}
