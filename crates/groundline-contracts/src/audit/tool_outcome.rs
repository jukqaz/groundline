//! Interpret native result metadata, never words in command output.
use std::collections::BTreeSet;

use serde::Deserializer;
use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value, json, value::RawValue};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum State {
    Success,
    Failure,
    Running,
    #[default]
    Unknown,
}

#[derive(Clone, Default)]
pub(super) struct Outcome {
    exit_code: Option<i64>,
    is_error: bool,
    status: Option<&'static str>,
    has_session: bool,
    ignored: bool,
    pub(super) pending_keys: BTreeSet<String>,
    unknown_parts: bool,
    batch: bool,
    header: bool,
    pub(super) emissions: Option<Vec<Self>>,
}

pub(super) fn handle_key(kind: &str, value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    let digest = Sha256::digest(format!("{kind}:{value}").as_bytes());
    Some(format!("{kind}_{digest:x}"))
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
        } else if !self.pending_keys.is_empty()
            || self.has_session
            || self.status == Some("running")
        {
            State::Running
        } else if self.unknown_parts {
            State::Unknown
        } else if self.exit_code == Some(0) {
            State::Success
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
            "_groundline_outcome": 2,
            "exit_code": self.exit_code,
            "is_error": self.is_error,
            "status": self.status.or(if self.has_session { Some("running") } else { None }),
            "pending_keys": self.pending_keys,
            "unknown_parts": self.unknown_parts,
            "batch": self.batch,
            "emissions": self.emissions.as_ref().map(|parts| parts.iter().map(Self::projection).collect::<Vec<_>>()),
        })
    }

    pub(super) fn unresolved_reason(&self) -> &'static str {
        match self.state() {
            State::Running if self.pending_keys.is_empty() => "running_without_handle",
            State::Running => "pending_completion",
            State::Unknown if self.batch => "mixed_batch_evidence",
            _ => "status_metadata_unavailable",
        }
    }

    pub(super) fn selected(parts: impl IntoIterator<Item = Self>) -> Option<Self> {
        let mut parts = parts.into_iter();
        let mut result = parts.next()?;
        for next in parts {
            result.combine(next);
            result.batch = true;
        }
        result.emissions = None;
        Some(result)
    }

    /// Replace only the explicitly polled handle; all other batch obligations remain.
    pub(super) fn complete_handle(&mut self, key: &str, next: &Self) {
        if !self.pending_keys.remove(key) {
            return;
        }
        if self.pending_keys.is_empty() {
            self.has_session = false;
            if self.status == Some("running") {
                self.status = None;
            }
        }
        // The consumed pending obligation is neutral, not a new missing result.
        if self.exit_code.is_none() {
            self.exit_code = Some(0);
        }
        self.combine(next.clone());
    }

    fn combine(&mut self, next: Self) {
        let was_unknown = self.state() == State::Unknown && self.exit_code.is_none();
        let next_unknown = next.state() == State::Unknown;
        self.is_error |= next.is_error;
        if next.exit_code.is_some_and(|code| code != 0) || self.exit_code.is_none() {
            self.exit_code = next.exit_code;
        }
        if next.status.is_some_and(|s| s != "running") || self.status.is_none() {
            self.status = next.status;
        }
        self.has_session |= next.has_session;
        self.pending_keys.extend(next.pending_keys);
        self.unknown_parts |= next.unknown_parts || was_unknown || next_unknown;
        self.batch |= next.batch;
        if self.pending_keys.len() > 128 {
            self.pending_keys.clear();
            self.has_session = true;
        }
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
    if text.starts_with("Script failed\nWall time ")
        || text.starts_with("Script failed\nWall time:")
    {
        result.status = Some("failed");
        return result;
    }
    let mut header = text.lines();
    if header.next() == Some("Script completed")
        && header
            .next()
            .is_some_and(|line| line.starts_with("Wall time:") || line.starts_with("Wall time "))
        && header.next() == Some("Output:")
        && header.next().is_none()
    {
        result.ignored = true;
        result.header = true;
        return result;
    }
    for line in text.lines().take(32) {
        if let Some(code) = line.strip_prefix("Process exited with code ") {
            result.exit_code = code.trim().parse().ok();
            return result;
        }
        for (prefix, kind) in [
            ("Process running with session ID ", "process"),
            ("Script running with cell ID ", "cell"),
        ] {
            if let Some(id) = line.strip_prefix(prefix).filter(|id| !id.trim().is_empty()) {
                result.has_session = true;
                result.pending_keys.extend(handle_key(kind, id.trim()));
                return result;
            }
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
        let mut session = None;
        let mut projection = false;
        let mut pending_raw = None;
        let mut unknown_raw = None;
        let mut batch_raw = None;
        let mut emissions_raw = None;
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
                    "is_error" | "isError" => result.is_error |= value.as_bool() == Some(true),
                    "status" => {
                        result.status = value.as_str().and_then(status);
                        fulfilled = value.as_str() == Some("fulfilled");
                    }
                    "type" => kind = value.as_str().map(str::to_owned),
                    "session_id" => {
                        result.has_session =
                            value.is_u64() || value.as_str().is_some_and(|id| !id.is_empty());
                        session = value
                            .as_u64()
                            .map(|id| id.to_string())
                            .or_else(|| value.as_str().map(str::to_owned));
                    }
                    _ => unreachable!(),
                }
            } else {
                match key.as_str() {
                    "_groundline_outcome" => {
                        projection = map.next_value::<&RawValue>()?.get() == "2"
                    }
                    "pending_keys" => pending_raw = Some(map.next_value::<&RawValue>()?),
                    "unknown_parts" => unknown_raw = Some(map.next_value::<&RawValue>()?),
                    "batch" => batch_raw = Some(map.next_value::<&RawValue>()?),
                    "emissions" => emissions_raw = Some(map.next_value::<&RawValue>()?),
                    "text" => text = Some(map.next_value::<&RawValue>()?),
                    "content" => content = Some(map.next_value::<&RawValue>()?),
                    "value" => value_field = Some(map.next_value::<&RawValue>()?),
                    _ => {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
            }
        }
        if projection {
            if let Some(raw) = pending_raw {
                if raw.get().len() > 11_000 {
                    return Err(serde::de::Error::custom("native handle limit exceeded"));
                }
                let keys: Vec<String> =
                    serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
                if keys.len() > 128 || keys.iter().any(|key| key.len() > 80) {
                    return Err(serde::de::Error::custom("native handle limit exceeded"));
                }
                result.pending_keys.extend(keys);
            }
            result.unknown_parts = unknown_raw.is_some_and(|raw| raw.get() == "true");
            result.batch = batch_raw.is_some_and(|raw| raw.get() == "true");
            if let Some(raw) = emissions_raw.filter(|raw| raw.get() != "null") {
                if raw.get().len() > 2 * 1024 * 1024 {
                    return Err(serde::de::Error::custom("native emission limit exceeded"));
                }
                let values: Vec<&RawValue> =
                    serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
                if values.len() > 128 {
                    return Err(serde::de::Error::custom("native emission limit exceeded"));
                }
                result.emissions = Some(
                    values
                        .into_iter()
                        .map(|raw| fragment(raw, self.depth + 1))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(serde::de::Error::custom)?,
                );
            }
        }
        if result.exit_code.is_some() {
            result.has_session = false;
        } else if let Some(session) = session {
            result.pending_keys.extend(handle_key("process", &session));
        }
        if result.state() != State::Unknown {
            return Ok(result);
        }
        let nested = match kind.as_deref() {
            Some("text" | "input_text") => text,
            Some("image" | "image_url" | "input_image") => {
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
        let mut emissions = None;
        while let Some(raw) = seq.next_element::<&RawValue>()? {
            count += 1;
            if count > 128 {
                return Err(serde::de::Error::custom(
                    "native tool content limit exceeded",
                ));
            }
            let next = fragment(raw, self.depth + 1).map_err(serde::de::Error::custom)?;
            if self.depth == 0 && count == 1 && next.header {
                emissions = Some(Vec::new());
            } else if let Some(parts) = &mut emissions {
                parts.push(next.clone());
            }
            if next.ignored {
                continue;
            }
            if let Some(old) = combined.as_mut() {
                old.combine(next);
                old.batch = true;
            } else {
                combined = Some(next);
            }
        }
        let mut result = combined.unwrap_or_default();
        result.emissions = emissions;
        Ok(result)
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
