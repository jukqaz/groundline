use super::{
    continuation,
    tool_outcome::{Outcome, State},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Check {
    outcome: Option<Outcome>,
    reason: Option<&'static str>,
    recovered: bool,
}

struct Poll {
    slot: Option<(usize, usize)>,
    key: String,
    waiting: BTreeSet<usize>,
}

#[derive(Default)]
pub(super) struct Verifications {
    checks: Vec<Check>,
    calls: BTreeMap<String, usize>,
    polls: BTreeMap<String, Vec<Poll>>,
    plans: BTreeMap<String, Vec<continuation::Slot>>,
    pending: BTreeMap<String, BTreeSet<usize>>,
    ambiguous_keys: BTreeSet<String>,
    literal_polls: u64,
    linked_polls: u64,
}

#[derive(Default)]
pub(super) struct Counts {
    pub success: u64,
    pub failure: u64,
    pub recovered: u64,
    pub reasons: BTreeMap<&'static str, u64>,
    pub literal_polls: u64,
    pub linked_polls: u64,
}

impl Verifications {
    pub(super) fn call(
        &mut self,
        id: Option<&str>,
        name: &str,
        arguments: &str,
        verification: bool,
    ) {
        if verification {
            let index = self.checks.len();
            self.checks.push(Check::default());
            if let Some(id) = id.filter(|id| !id.is_empty()) {
                if let Some(old) = self.calls.insert(id.to_owned(), index) {
                    self.checks[old].reason = Some("ambiguous_call_id");
                    self.checks[index].reason = Some("ambiguous_call_id");
                }
            } else {
                self.checks[index].reason = Some("missing_call_id");
            }
        }
        let plan = if verification || arguments.contains("write_stdin") {
            continuation::plan(name, arguments)
        } else {
            None
        };
        if let Some(plan) = plan {
            if let Some(id) = id {
                if plan.unobserved_call
                    && let Some(&index) = self.calls.get(id)
                {
                    self.checks[index].reason = Some("unobserved_native_call");
                }
                for (slot, entry) in plan.slots.iter().enumerate() {
                    if let Some(key) = &entry.poll {
                        self.add_poll(id, key.clone(), Some((slot, plan.slots.len())));
                    }
                }
                self.plans.insert(id.to_owned(), plan.slots);
            }
        } else if let Some(id) = id
            && let Some(key) = continuation::key(name, arguments)
        {
            self.add_poll(id, key, None);
        }
    }

    fn add_poll(&mut self, id: &str, key: String, slot: Option<(usize, usize)>) {
        self.literal_polls += 1;
        if let Some(waiting) = self.pending.get(&key) {
            self.linked_polls += 1;
            self.polls.entry(id.to_owned()).or_default().push(Poll {
                slot,
                key,
                waiting: waiting.clone(),
            });
        }
    }

    fn emission(outcome: &Outcome, slot: usize, count: usize) -> Option<Outcome> {
        match &outcome.emissions {
            Some(parts) if parts.len() == count => parts.get(slot).cloned(),
            None if count == 1 && slot == 0 => Some(outcome.clone()),
            _ => None,
        }
    }

    fn track(&mut self, index: usize) {
        let check = &self.checks[index];
        if check.reason.is_some() {
            return;
        }
        let Some(outcome) = check
            .outcome
            .as_ref()
            .filter(|outcome| outcome.state() == State::Running)
        else {
            return;
        };
        let keys = outcome.pending_keys.clone();
        for key in keys {
            if self.ambiguous_keys.contains(&key) {
                self.checks[index].reason = Some("ambiguous_process_handle");
            } else if let Some(waiting) = self
                .pending
                .get(&key)
                .filter(|waiting| !waiting.contains(&index))
            {
                for &old in waiting {
                    self.checks[old].reason = Some("ambiguous_process_handle");
                }
                self.checks[index].reason = Some("ambiguous_process_handle");
                self.pending.remove(&key);
                if self.ambiguous_keys.len() < 16_384 {
                    self.ambiguous_keys.insert(key);
                }
            } else if self.pending.len() + self.ambiguous_keys.len() >= 16_384 {
                self.checks[index].reason = Some("correlation_budget_exceeded");
            } else {
                self.pending.entry(key).or_default().insert(index);
            }
        }
    }

    fn untrack(&mut self, index: usize, keys: &BTreeSet<String>) {
        for key in keys {
            if let Some(waiting) = self.pending.get_mut(key) {
                waiting.remove(&index);
                if waiting.is_empty() {
                    self.pending.remove(key);
                }
            }
        }
    }

    pub(super) fn output(&mut self, id: &str, outcome: Outcome) {
        if let Some(polls) = self.polls.remove(id) {
            for poll in polls {
                let polled = match poll.slot {
                    Some((slot, count)) => Self::emission(&outcome, slot, count),
                    None => Some(outcome.clone()),
                };
                if let Some(polled) = polled {
                    self.complete_poll(poll.key, poll.waiting, &polled);
                }
            }
        }
        if let Some(&index) = self.calls.get(id) {
            let outcome = if let Some(plan) = self.plans.get(id) {
                let mut required = Vec::new();
                for (slot, entry) in plan
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| entry.verification)
                {
                    let _ = entry;
                    let Some(value) = Self::emission(&outcome, slot, plan.len()) else {
                        self.checks[index].reason = Some("unmatched_result_shape");
                        return;
                    };
                    required.push(value);
                }
                let Some(value) = Outcome::selected(required) else {
                    self.checks[index].reason = Some("verification_target_unavailable");
                    return;
                };
                value
            } else {
                outcome
            };
            let check = &mut self.checks[index];
            let old_keys = check
                .outcome
                .as_ref()
                .map(|outcome| outcome.pending_keys.clone())
                .unwrap_or_default();
            if let Some(old) = &check.outcome
                && matches!(old.state(), State::Success | State::Failure)
            {
                if matches!(outcome.state(), State::Success | State::Failure)
                    && old.state() != outcome.state()
                {
                    check.reason = Some("conflicting_terminal_results");
                }
                return;
            }
            check.outcome = Some(outcome);
            self.untrack(index, &old_keys);
            self.track(index);
        }
    }

    fn complete_poll(&mut self, key: String, waiting: BTreeSet<usize>, outcome: &Outcome) {
        if waiting.len() != 1 {
            for index in waiting {
                self.checks[index].reason = Some("ambiguous_process_handle");
            }
            return;
        }
        for index in waiting {
            if self.checks[index].reason.is_some() {
                continue;
            }
            let old_keys = self.checks[index]
                .outcome
                .as_ref()
                .map(|outcome| outcome.pending_keys.clone())
                .unwrap_or_default();
            if let Some(previous) = &mut self.checks[index].outcome {
                if previous.state() != State::Running || !previous.pending_keys.contains(&key) {
                    continue;
                }
                previous.complete_handle(&key, outcome);
                if matches!(previous.state(), State::Success | State::Failure) {
                    self.checks[index].recovered = true;
                }
            }
            self.untrack(index, &old_keys);
            self.track(index);
        }
    }

    pub(super) fn finish(self) -> Counts {
        let mut counts = Counts {
            literal_polls: self.literal_polls,
            linked_polls: self.linked_polls,
            ..Counts::default()
        };
        for check in self.checks {
            let reason = if let Some(reason) = check.reason {
                reason
            } else if let Some(outcome) = check.outcome {
                match outcome.state() {
                    State::Success => {
                        counts.success += 1;
                        counts.recovered += u64::from(check.recovered);
                        continue;
                    }
                    State::Failure => {
                        counts.failure += 1;
                        counts.recovered += u64::from(check.recovered);
                        continue;
                    }
                    _ => outcome.unresolved_reason(),
                }
            } else {
                "missing_output"
            };
            *counts.reasons.entry(reason).or_default() += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, Value, json};
    #[test]
    fn unobserved_checks_cannot_borrow_another_success() {
        let mut v = Verifications::default();
        v.call(Some("check"), "exec", "const hidden = await tools.exec_command({cmd:'cargo test'}); text(await tools.exec_command({cmd:'cargo check'}));", true);
        v.output("check", exit(0));
        let counts = v.finish();
        assert_eq!(counts.success, 0);
        assert_eq!(counts.reasons.get("unobserved_native_call"), Some(&1));
    }
    fn output(value: Value) -> Outcome {
        super::super::tool_outcome::from_value(&value, &Map::new())
    }
    fn running(id: u64) -> Outcome {
        output(json!({"session_id":id,"output":"still working"}))
    }
    fn exit(code: i64) -> Outcome {
        output(json!({"exit_code":code,"output":"timeout rejected words are stdout"}))
    }
    fn start(v: &mut Verifications, id: &str, value: Outcome) {
        v.call(Some(id), "exec", "cargo test", true);
        v.output(id, value);
    }
    fn poll(v: &mut Verifications, id: &str, session: u64, value: Outcome) {
        v.call(
            Some(id),
            "write_stdin",
            &format!(r#"{{"session_id":{session}}}"#),
            false,
        );
        v.output(id, value);
    }
    #[test]
    fn follows_multistep_polls_and_counts_terminal_result_once() {
        for code in [0, 1] {
            let mut v = Verifications::default();
            start(&mut v, "test", running(12));
            poll(&mut v, "pending", 12, running(12));
            poll(&mut v, "done", 12, exit(code));
            v.output("done", exit(code));
            poll(&mut v, "extra", 12, exit(code));
            let c = v.finish();
            assert_eq!(c.success, u64::from(code == 0));
            assert_eq!(c.failure, u64::from(code != 0));
            assert_eq!(c.recovered, 1);
            assert!(c.reasons.is_empty());
        }
    }
    #[test]
    fn follows_cell_to_process_without_crossing_namespaces() {
        let mut v = Verifications::default();
        start(
            &mut v,
            "test",
            output(json!("Script running with cell ID 12")),
        );
        poll(&mut v, "wrong", 12, exit(0));
        v.call(Some("cell"), "wait", r#"{"cell_id":"12"}"#, false);
        v.output("cell", running(13));
        poll(&mut v, "done", 13, exit(0));
        let c = v.finish();
        assert_eq!(c.success, 1);
        assert_eq!(c.recovered, 1);
        assert!(c.reasons.is_empty());
    }
    #[test]
    fn waits_for_every_batch_member_and_preserves_unknown_members() {
        for unknown in [false, true] {
            let mut parts = vec![
                json!({"exit_code":0}),
                json!({"session_id":12}),
                json!({"session_id":13}),
            ];
            if unknown {
                parts.push(json!({"result":"unmeasured"}));
            }
            let mut v = Verifications::default();
            start(&mut v, "test", output(json!(parts)));
            poll(&mut v, "one", 12, exit(0));
            poll(&mut v, "two", 13, exit(0));
            let c = v.finish();
            assert_eq!(c.success, u64::from(!unknown));
            assert_eq!(
                c.reasons.get("mixed_batch_evidence").copied().unwrap_or(0),
                u64::from(unknown)
            );
        }
    }
    #[test]
    fn missing_unlinked_and_conflicting_evidence_stays_explicit() {
        let mut v = Verifications::default();
        v.call(None, "exec", "cargo test", true);
        v.call(Some("missing"), "exec", "cargo test", true);
        start(&mut v, "unknown", output(json!({"output":"ok"})));
        start(&mut v, "running", running(12));
        poll(&mut v, "wrong", 13, exit(0));
        start(&mut v, "conflict", exit(0));
        v.output("conflict", exit(1));
        let c = v.finish();
        assert_eq!(c.success + c.failure, 0);
        for reason in [
            "missing_call_id",
            "missing_output",
            "status_metadata_unavailable",
            "pending_completion",
            "conflicting_terminal_results",
        ] {
            assert_eq!(c.reasons[reason], 1);
        }
    }
    #[test]
    fn reused_live_handle_is_ambiguous_but_finished_handle_can_be_reused() {
        let mut v = Verifications::default();
        start(&mut v, "one", running(12));
        start(&mut v, "two", running(12));
        poll(&mut v, "done", 12, exit(0));
        let c = v.finish();
        assert_eq!(c.reasons["ambiguous_process_handle"], 2);
        assert_eq!(c.success, 0);
        let mut v = Verifications::default();
        start(&mut v, "one", running(12));
        poll(&mut v, "done", 12, exit(0));
        start(&mut v, "two", running(12));
        poll(&mut v, "done2", 12, exit(0));
        assert_eq!(v.finish().success, 2);
    }
    #[test]
    fn streaming_projection_preserves_all_obligations_without_raw_handles() {
        let raw =
            output(json!([{"exit_code":0},{"session_id":12},{"session_id":13},{"note":"unknown"}]));
        let projected = raw.projection();
        assert!(!projected.to_string().contains("session_id"));
        let mut v = Verifications::default();
        start(&mut v, "test", output(projected));
        poll(&mut v, "done", 12, exit(0));
        poll(&mut v, "done2", 13, exit(0));
        assert_eq!(v.finish().reasons["mixed_batch_evidence"], 1);
    }

    fn emissions(values: Vec<Value>) -> Outcome {
        let mut blocks = vec![
            json!({"type":"text","text":"Script completed\nWall time 0.1 seconds\nOutput:\n"}),
        ];
        blocks.extend(
            values
                .into_iter()
                .map(|value| json!({"type":"text","text":value.to_string()})),
        );
        let raw = output(json!(blocks));
        output(raw.projection())
    }
    #[test]
    fn batched_polls_use_their_own_emission_and_never_another_commands_success() {
        let mut v = Verifications::default();
        v.call(Some("test"),"exec","text(await tools.exec_command({cmd:'cargo test'})); text(await tools.exec_command({cmd:'git status'}));",true);
        v.output(
            "test",
            emissions(vec![json!({"session_id":12}), json!({"exit_code":0})]),
        );
        v.call(Some("poll"),"exec","text(await tools.write_stdin({session_id:12,chars:''})); text(await tools.exec_command({cmd:'git status'}));",false);
        v.output(
            "poll",
            emissions(vec![json!({"exit_code":1}), json!({"exit_code":0})]),
        );
        let c = v.finish();
        assert_eq!(c.success, 0);
        assert_eq!(c.failure, 1);
        assert_eq!(c.recovered, 1);
        assert!(c.reasons.is_empty());
    }
    #[test]
    fn multiple_poll_slots_are_independent_and_missing_emissions_stay_pending() {
        for truncated in [false, true] {
            let mut v = Verifications::default();
            start(&mut v, "one", running(12));
            start(&mut v, "two", running(13));
            v.call(Some("poll"),"exec","text(await tools.write_stdin({session_id:12})); text(await tools.write_stdin({session_id:13}));",false);
            let mut results = vec![json!({"exit_code":0}), json!({"exit_code":1})];
            if truncated {
                results.pop();
            }
            v.output("poll", emissions(results));
            let c = v.finish();
            assert_eq!(c.success, u64::from(!truncated));
            assert_eq!(c.failure, u64::from(!truncated));
            assert_eq!(
                c.reasons.get("pending_completion").copied().unwrap_or(0),
                if truncated { 2 } else { 0 }
            );
        }
    }
    #[test]
    fn known_annotations_and_inspections_are_not_verification_results() {
        let mut v = Verifications::default();
        v.call(Some("test"),"exec","text('validation'); const r=await tools.exec_command({cmd:'cargo test'}); text(r); text(await tools.exec_command({cmd:'rg missing'}));",true);
        v.output(
            "test",
            emissions(vec![
                json!("validation"),
                json!({"exit_code":0}),
                json!({"exit_code":1}),
            ]),
        );
        let c = v.finish();
        assert_eq!(c.success, 1);
        assert_eq!(c.failure, 0);
    }
}
