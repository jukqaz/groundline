//! Attribute only native owned response usage with an explicit turn link.
use super::{BTreeMap, ContractError, Map, Usage, Value, json};

type Label = (String, String);

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
    usage: BTreeMap<String, Usage>,
    total: Usage,
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
        let model = crate::model::family(
            payload
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
        );
        let effort = crate::model::effort(
            payload
                .get("effort")
                .or_else(|| payload.get("reasoning_effort"))
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
        );
        let label = (model.to_owned(), effort.to_owned());
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
        let turn = payload
            .get("turn_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        self.usage
            .entry(turn.to_owned())
            .or_default()
            .add_checked(usage)
    }
}

#[derive(Default)]
pub(super) struct Aggregate {
    buckets: BTreeMap<Label, Usage>,
    unknown: Usage,
}

impl Aggregate {
    pub(super) fn add(
        &mut self,
        responses: Responses,
        authoritative: &Usage,
    ) -> Result<(), ContractError> {
        // Different native/UI counter baselines can disagree. Do not clamp or
        // distribute an inconsistent response total across models.
        if !authoritative.covers(&responses.total)
            || responses.usage.values().any(|usage| !coherent(usage))
            || !coherent(&authoritative.subtract(&responses.total))
        {
            return self.unknown.add_checked(authoritative);
        }
        self.unknown
            .add_checked(&authoritative.subtract(&responses.total))?;
        for (turn, usage) in responses.usage {
            match responses.contexts.get(&turn).and_then(Option::as_ref) {
                Some(label) if label.0 != "unknown" && label.1 != "unknown" => {
                    self.buckets
                        .entry(label.clone())
                        .or_default()
                        .add_checked(&usage)?;
                }
                _ => self.unknown.add_checked(&usage)?,
            }
        }
        Ok(())
    }

    pub(super) fn json(&self) -> Value {
        json!({
            "basis":"owned_response_turn_link",
            "buckets":self.buckets.iter().map(|((model,effort),usage)| json!({
                "model_family":model,"effort":effort,"tokens":usage.as_json()
            })).collect::<Vec<_>>(),
            "unattributed":self.unknown.as_json(),
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
        assert_eq!(
            a.buckets
                .values()
                .map(|u| u.values["total_tokens"])
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
        }
    }
}
