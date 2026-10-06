//! Attribute only native owned response usage with an explicit turn link.
use super::{BTreeMap, ContractError, Map, Usage, Value, json};

type Label = (String, String);

fn count_add(target: &mut u64, count: u64) -> Result<(), ContractError> {
    *target = target
        .checked_add(count)
        .ok_or_else(|| ContractError("model_response_count_overflow".to_owned()))?;
    Ok(())
}

#[derive(Default)]
struct Observation {
    usage: Usage,
    response_count: u64,
}

impl Observation {
    fn add(&mut self, usage: &Usage, response_count: u64) -> Result<(), ContractError> {
        self.usage.add_checked(usage)?;
        count_add(&mut self.response_count, response_count)
    }
}

fn coherent(usage: &Usage) -> bool {
    let n = |field| usage.values.get(field).copied().unwrap_or(0);
    n("cached_input_tokens") <= n("input_tokens")
        && n("reasoning_output_tokens") <= n("output_tokens")
        && n("input_tokens")
            .checked_add(n("output_tokens"))
            .is_some_and(|v| v <= n("total_tokens"))
}

#[derive(Default)]
pub(super) struct Responses {
    contexts: BTreeMap<String, Option<Label>>,
    usage: BTreeMap<String, Observation>,
    total: Usage,
    response_count: u64,
}

impl Responses {
    pub(super) fn context(&mut self, payload: &Map<String, Value>) {
        let Some(turn) = payload
            .get("turn_id")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            return;
        };
        let model = crate::model::observed_label(
            payload
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
        );
        let effort = crate::model::effort(
            payload
                .get("effort")
                .and_then(Value::as_str)
                .or_else(|| payload.get("reasoning_effort").and_then(Value::as_str))
                .unwrap_or("unknown"),
        );
        let label = (model, effort.to_owned());
        self.contexts
            .entry(turn.to_owned())
            .and_modify(|old| {
                if old.as_ref() != Some(&label) {
                    *old = None;
                }
            })
            .or_insert(Some(label));
    }

    pub(super) fn response(
        &mut self,
        payload: &Map<String, Value>,
        usage: &Usage,
    ) -> Result<(), ContractError> {
        self.total.add_checked(usage)?;
        count_add(&mut self.response_count, 1)?;
        let turn = payload
            .get("turn_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        self.usage.entry(turn.to_owned()).or_default().add(usage, 1)
    }
}

#[derive(Default)]
pub(super) struct Aggregate {
    buckets: BTreeMap<Label, Observation>,
    unknown: Usage,
    observed_response_count: u64,
    unattributed_response_count: u64,
    overflow_response_count: u64,
}

impl Aggregate {
    pub(super) fn add(
        &mut self,
        responses: Responses,
        authoritative: &Usage,
    ) -> Result<(), ContractError> {
        count_add(&mut self.observed_response_count, responses.response_count)?;
        // Different native/UI counter baselines can disagree. Do not clamp or
        // distribute an inconsistent response total across models.
        if !authoritative.covers(&responses.total)
            || responses.usage.values().any(|r| !coherent(&r.usage))
            || !coherent(&authoritative.subtract(&responses.total))
        {
            count_add(
                &mut self.unattributed_response_count,
                responses.response_count,
            )?;
            return self.unknown.add_checked(authoritative);
        }
        self.unknown
            .add_checked(&authoritative.subtract(&responses.total))?;
        for (turn, observation) in responses.usage {
            match responses.contexts.get(&turn).and_then(Option::as_ref) {
                Some(label) if label.0 != "unknown" && label.1 != "unknown" => {
                    if self.buckets.contains_key(label)
                        || self.buckets.len() < crate::model::MAX_MODEL_CONTEXTS
                    {
                        self.buckets
                            .entry(label.clone())
                            .or_default()
                            .add(&observation.usage, observation.response_count)?;
                    } else {
                        self.unknown.add_checked(&observation.usage)?;
                        count_add(
                            &mut self.unattributed_response_count,
                            observation.response_count,
                        )?;
                        count_add(
                            &mut self.overflow_response_count,
                            observation.response_count,
                        )?;
                    }
                }
                _ => {
                    self.unknown.add_checked(&observation.usage)?;
                    count_add(
                        &mut self.unattributed_response_count,
                        observation.response_count,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn json(&self) -> Value {
        json!({
            "basis":"owned_response_turn_link",
            "buckets":self.buckets.iter().map(|((model,effort),observation)| json!({
                "model_family":model,"effort":effort,"tokens":observation.usage.as_json(),
                "response_count":observation.response_count,
            })).collect::<Vec<_>>(),
            "unattributed":self.unknown.as_json(),
            "observed_response_count":self.observed_response_count,
            "unattributed_response_count":self.unattributed_response_count,
            "overflow_response_count":self.overflow_response_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(n: u64) -> Usage {
        Usage::from_value(&json!({"total_tokens":n})).unwrap()
    }
    fn context(turn: &str, model: &str) -> Map<String, Value> {
        json!({"turn_id":turn,"model":model,"effort":"high"})
            .as_object()
            .unwrap()
            .clone()
    }
    #[test]
    fn mixed_models_missing_links_and_residual_conserve_usage() {
        let mut r = Responses::default();
        r.context(&context("one", "gpt-6-astra"));
        r.context(&context("two", "gpt-5.6-sol"));
        for (turn, n) in [("one", 10), ("two", 20), ("missing", 3)] {
            r.response(&context(turn, "unused"), &usage(n)).unwrap();
        }
        let mut a = Aggregate::default();
        a.add(r, &usage(40)).unwrap();
        let v = a.json();
        assert_eq!(v["buckets"].as_array().unwrap().len(), 2);
        assert_eq!(v["unattributed"]["total_tokens"], 10);
        assert_eq!(v["observed_response_count"], 3);
        assert_eq!(v["unattributed_response_count"], 1);
        assert_eq!(v["overflow_response_count"], 0);
        assert_eq!(
            a.buckets
                .values()
                .map(|r| r.usage.values["total_tokens"])
                .sum::<u64>()
                + a.unknown.values["total_tokens"],
            40
        );
    }
    #[test]
    fn conflicting_turn_context_and_counter_mismatch_remain_unknown() {
        for mismatch in [false, true] {
            let mut r = Responses::default();
            r.context(&context("one", "gpt-6-astra"));
            if !mismatch {
                r.context(&context("one", "gpt-5.6-sol"));
            }
            r.response(&context("one", "unused"), &usage(20)).unwrap();
            let mut a = Aggregate::default();
            let total = if mismatch { 10 } else { 20 };
            a.add(r, &usage(total)).unwrap();
            assert!(a.buckets.is_empty());
            assert_eq!(a.unknown.values["total_tokens"], total);
            assert_eq!(a.observed_response_count, 1);
            assert_eq!(a.unattributed_response_count, 1);
        }
    }

    #[test]
    fn cap_conserves_all_token_counters_and_response_observations() {
        let one = Usage::from_value(&json!({
            "input_tokens":5,"cached_input_tokens":2,"cache_write_input_tokens":1,
            "output_tokens":3,"reasoning_output_tokens":1,"total_tokens":8
        }))
        .unwrap();
        let mut r = Responses::default();
        let mut total = Usage::default();
        for index in 0..(crate::model::MAX_MODEL_CONTEXTS + 5) {
            let turn = format!("{index:04}");
            r.context(&context(&turn, &format!("gpt-{}-sol", index + 10)));
            r.response(&context(&turn, "unused"), &one).unwrap();
            total.add_checked(&one).unwrap();
        }
        let mut a = Aggregate::default();
        a.add(r, &total).unwrap();
        assert_eq!(a.buckets.len(), crate::model::MAX_MODEL_CONTEXTS);
        assert_eq!(a.observed_response_count, 133);
        assert_eq!(a.unattributed_response_count, 5);
        assert_eq!(a.overflow_response_count, 5);
        for field in super::super::TOKEN_FIELDS {
            assert_eq!(
                a.buckets
                    .values()
                    .map(|r| r.usage.values[field])
                    .sum::<u64>()
                    + a.unknown.values[field],
                total.values[field]
            );
        }
        // A known pair remains attributable once the cardinality bound is full.
        let mut r = Responses::default();
        r.context(&context("again", "gpt-10-sol"));
        r.response(&context("again", "unused"), &one).unwrap();
        a.add(r, &one).unwrap();
        assert_eq!(
            a.buckets[&("gpt-10-sol".to_owned(), "high".to_owned())].response_count,
            2
        );
        assert_eq!(a.overflow_response_count, 5);
    }

    #[test]
    fn private_ids_and_fallback_effort_keep_distinct_response_counts() {
        let mut r = Responses::default();
        for model in ["private/one", "private/two"] {
            let payload =
                json!({"turn_id":model,"model":model,"effort":null,"reasoning_effort":"high"});
            r.context(payload.as_object().unwrap());
            r.response(payload.as_object().unwrap(), &usage(1)).unwrap();
        }
        let mut a = Aggregate::default();
        a.add(r, &usage(2)).unwrap();
        assert_eq!(a.buckets.len(), 2);
        assert_eq!(a.observed_response_count, 2);
        assert_eq!(a.unattributed_response_count, 0);
        assert!(a.buckets.values().all(|r| r.response_count == 1));
        assert!(!a.json().to_string().contains("private/"));
    }
}
