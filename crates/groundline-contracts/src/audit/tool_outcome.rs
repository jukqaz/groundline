//! Interpret native result metadata, never words in command output.
use std::collections::BTreeSet;

use serde::Deserializer;
use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value, json, value::RawValue};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum State {
    Success,
    Failure,
    Running,
    #[default]
    Unknown,
}

#[derive(Default)]
pub(super) struct Outcome {
    exit_code: Option<i64>,
    is_error: bool,
    status: Option<&'static str>,
    has_session: bool,
    ignored: bool,
}

fn status(value: &str) -> Option<&'static str> {
    match value {
        "error" | "failed" => Some("failed"),
        "timeout" | "timed_out" => Some("timeout"),
        "rejected" | "permission_denied" => Some("rejected"),
        "invalid_arguments" => Some("invalid_arguments"),
        "running" => Some("running"),
        _ => None,
    }
}

impl Outcome {
    pub(super) fn state(&self) -> State {
        if self.is_error
            || self.exit_code.is_some_and(|code| code != 0)
            || matches!(
                self.status,
                Some("failed" | "timeout" | "rejected" | "invalid_arguments")
            )
        {
            State::Failure
        } else if self.exit_code == Some(0) {
            State::Success
        } else if self.has_session || self.status == Some("running") {
            State::Running
        } else {
            State::Unknown
        }
    }

    pub(super) fn signals(&self) -> BTreeSet<&'static str> {
        let mut signals = BTreeSet::new();
        if self.is_error
            || self.exit_code.is_some_and(|code| code != 0)
            || self.status == Some("failed")
        {
            signals.insert("nonzero_exit");
        }
        if let Some(reason @ ("timeout" | "rejected" | "invalid_arguments")) = self.status {
            signals.insert(reason);
        }
        if self.state() == State::Running {
            signals.insert("yielded_for_wait");
        }
        signals
    }

    pub(super) fn projection(&self) -> Value {
        json!({
            "exit_code": self.exit_code,
            "is_error": self.is_error,
            "status": self.status.or(if self.has_session { Some("running") } else { None }),
        })
    }

    fn with_payload(mut self, payload: &Map<String, Value>) -> Self {
        self.is_error |= payload.get("is_error").and_then(Value::as_bool) == Some(true);
        if let Some(value) = payload
            .get("status")
            .and_then(Value::as_str)
            .and_then(status)
            && (value != "running" || self.status.is_none())
        {
            self.status = Some(value);
        }
        self
    }
}

// Read only the native preamble. Stop before stdout, including text that merely
// quotes an exit status. Orchestrator completion does not prove nested test success.
fn text_outcome(text: &str) -> Outcome {
    let mut result = Outcome::default();
    let mut header = text.lines();
    if header.next() == Some("Script completed")
        && header
            .next()
            .is_some_and(|line| line.starts_with("Wall time:"))
        && header.next() == Some("Output:")
        && header.next().is_none()
    {
        result.ignored = true;
        return result;
    }
    for line in text.lines().take(32) {
        if let Some(code) = line.strip_prefix("Process exited with code ") {
            result.exit_code = code.trim().parse().ok();
            return result;
        }
        if [
            "Process running with session ID ",
            "Script running with cell ID ",
        ]
        .iter()
        .any(|prefix| {
            line.strip_prefix(prefix)
                .is_some_and(|id| !id.trim().is_empty())
        }) {
            result.has_session = true;
            return result;
        }
        if !["Chunk ID:", "Wall time:", "Original token count:"]
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            break;
        }
    }
    result
}

struct NativeOutcome {
    depth: u8,
}
impl<'de> Visitor<'de> for NativeOutcome {
    type Value = Outcome;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a bounded native tool result")
    }
    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Outcome, E> {
        if self.depth > 6 {
            return Ok(Outcome::default());
        }
        // Native text blocks can encode JSON or a settled batch. Follow only
        // these bounded envelope fields, never arbitrary stdout/object trees.
        if text.trim_start().starts_with(['{', '[']) {
            let mut de = serde_json::Deserializer::from_str(text);
            return Ok(de
                .deserialize_any(NativeOutcome {
                    depth: self.depth + 1,
                })
                .ok()
                .filter(|_| de.end().is_ok())
                .unwrap_or_default());
        }
        Ok(text_outcome(text))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Outcome, A::Error> {
        let mut result = Outcome::default();
        let mut count = 0;
        let mut kind = None;
        let mut text = None;
        let mut content = None;
        let mut fulfilled = false;
        let mut value_field = None;
        while let Some(key) = map.next_key::<String>()? {
            count += 1;
            if count > 128 {
                return Err(serde::de::Error::custom("native tool field limit exceeded"));
            }
            if matches!(
                key.as_str(),
                "exit_code" | "is_error" | "isError" | "status" | "session_id" | "type"
            ) {
                let raw = map.next_value::<&RawValue>()?;
                let value = if raw.get().len() <= 128 {
                    serde_json::from_str::<Value>(raw.get()).unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                match key.as_str() {
                    "exit_code" => result.exit_code = value.as_i64(),
                    "is_error" | "isError" => result.is_error = value.as_bool() == Some(true),
                    "status" => {
                        result.status = value.as_str().and_then(status);
                        fulfilled = value.as_str() == Some("fulfilled");
                    }
                    "type" => kind = value.as_str().map(str::to_owned),
                    "session_id" => {
                        result.has_session =
                            value.is_u64() || value.as_str().is_some_and(|id| !id.is_empty())
                    }
                    _ => unreachable!(),
                }
            } else {
                match key.as_str() {
                    "text" => text = Some(map.next_value::<&RawValue>()?),
                    "content" => content = Some(map.next_value::<&RawValue>()?),
                    "value" => value_field = Some(map.next_value::<&RawValue>()?),
                    _ => {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
            }
        }
        if result.state() != State::Unknown {
            return Ok(result);
        }
        let nested = match kind.as_deref() {
            Some("text") => text,
            Some("image" | "image_url") => {
                result.ignored = true;
                return Ok(result);
            }
            _ if fulfilled => value_field,
            _ => content,
        };
        if let Some(raw) = nested {
            return fragment(raw, self.depth + 1).map_err(serde::de::Error::custom);
        }
        Ok(result)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Outcome, A::Error> {
        let mut combined: Option<Outcome> = None;
        let mut count = 0;
        while let Some(raw) = seq.next_element::<&RawValue>()? {
            count += 1;
            if count > 128 {
                return Err(serde::de::Error::custom(
                    "native tool content limit exceeded",
                ));
            }
            let next = fragment(raw, self.depth + 1).map_err(serde::de::Error::custom)?;
            if next.ignored {
                continue;
            }
            // A batch succeeds only when every substantive result is a proven
            // success. Failures dominate, followed by pending/unknown evidence.
            let rank = |outcome: &Outcome| match outcome.state() {
                State::Failure => 3,
                State::Running => 2,
                State::Unknown => 1,
                State::Success => 0,
            };
            if combined.as_ref().is_none_or(|old| rank(&next) > rank(old)) {
                combined = Some(next);
            }
        }
        Ok(combined.unwrap_or_default())
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Outcome, E> {
        Ok(Outcome::default())
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Outcome, E> {
        Ok(Outcome::default())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Outcome, E> {
        Ok(Outcome::default())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Outcome, E> {
        Ok(Outcome::default())
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Outcome, E> {
        Ok(Outcome::default())
    }
}

pub(super) fn from_raw(
    raw: &RawValue,
    payload: &Map<String, Value>,
) -> serde_json::Result<Outcome> {
    if raw.get().len() > 64 * 1024 * 1024 {
        return Err(serde::de::Error::custom("tool output byte limit exceeded"));
    }
    fragment(raw, 0).map(|outcome| outcome.with_payload(payload))
}

fn fragment(raw: &RawValue, depth: u8) -> serde_json::Result<Outcome> {
    if depth > 6 {
        return Ok(Outcome::default());
    }
    raw.deserialize_any(NativeOutcome { depth })
}

pub(super) fn from_value(value: &Value, payload: &Map<String, Value>) -> Outcome {
    // Value is already bounded by the retained-record limit. Use the same parser
    // for direct audits and streaming projections so their semantics cannot drift.
    let raw = serde_json::value::to_raw_value(value).expect("valid JSON value");
    from_raw(&raw, payload).unwrap_or_default()
}
