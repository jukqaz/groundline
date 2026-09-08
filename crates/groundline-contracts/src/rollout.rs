//! Borrow record payloads until a consumer actually needs their contents.
use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, value::RawValue};

pub struct Record<'a> {
    fields: BTreeMap<String, &'a RawValue>,
}

struct Fields<'a>(BTreeMap<String, &'a RawValue>);
impl<'de> Deserialize<'de> for Fields<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldsVisitor;
        impl<'de> Visitor<'de> for FieldsVisitor {
            type Value = Fields<'de>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a bounded native record object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut fields = BTreeMap::new();
                let mut count = 0;
                while let Some(key) = map.next_key::<String>()? {
                    count += 1;
                    if count > 128 {
                        return Err(serde::de::Error::custom(
                            "native envelope field limit exceeded",
                        ));
                    }
                    fields.insert(key, map.next_value::<&RawValue>()?);
                }
                Ok(Fields(fields))
            }
        }
        deserializer.deserialize_map(FieldsVisitor)
    }
}

impl<'a> Record<'a> {
    /// Validate JSON syntax, retaining only a small envelope and borrowed slices.
    /// As with `Value::Object`, duplicate keys use their last value.
    pub fn parse(line: &'a str) -> serde_json::Result<Option<Self>> {
        if !line.trim_start().starts_with('{') {
            serde_json::from_str::<IgnoredAny>(line)?;
            return Ok(None);
        }
        serde_json::from_str::<Fields<'a>>(line).map(|Fields(fields)| Some(Self { fields }))
    }

    pub fn string(&self, name: &str) -> Option<Cow<'a, str>> {
        let raw = self.fields.get(name)?.get().trim();
        if raw.starts_with('"') && raw.ends_with('"') && !raw.contains('\\') {
            Some(Cow::Borrowed(&raw[1..raw.len() - 1]))
        } else {
            serde_json::from_str::<String>(raw).ok().map(Cow::Owned)
        }
    }

    pub fn unsigned(&self, name: &str) -> Option<u64> {
        serde_json::from_str(self.fields.get(name)?.get()).ok()
    }

    pub fn value(&self, name: &str) -> serde_json::Result<Option<Value>> {
        self.fields
            .get(name)
            .map(|raw| serde_json::from_str(raw.get()))
            .transpose()
    }

    /// Retain the audit envelope while dropping bodies the audit never reads.
    /// Parsing the original record has already validated every JSON value.
    pub fn audit_projection(&self) -> serde_json::Result<String> {
        let kind = self.string("type");
        let mut envelope = serde_json::Map::new();
        let mut remaining = 4 * 1024 * 1024;
        for name in ["type", "timestamp", "ordinal"] {
            if let Some(raw) = self.fields.get(name) {
                envelope.insert(name.to_owned(), bounded_value(raw, &mut remaining)?);
            }
        }
        if matches!(
            kind.as_deref(),
            Some(
                "session_meta"
                    | "compacted"
                    | "turn_context"
                    | "event_msg"
                    | "response_item"
                    | "token_usage_record"
            )
        ) && let Some(raw) = self.fields.get("payload")
        {
            // Borrow unused nested values instead of materializing compaction
            // histories, images, or item_completed bodies as a Value tree.
            let payload = if raw.get().trim_start().starts_with('{') {
                let Fields(borrowed) = serde_json::from_str::<Fields<'_>>(raw.get())?;
                let item = borrowed
                    .get("type")
                    .map(|raw| bounded_value(raw, &mut remaining))
                    .transpose()?
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                let mut fields = serde_json::Map::new();
                for (name, raw) in borrowed {
                    if keep_payload_field(kind.as_deref(), &item, &name) {
                        fields.insert(name, bounded_value(raw, &mut remaining)?);
                    }
                }
                if kind.as_deref() == Some("response_item")
                    && matches!(
                        item.as_str(),
                        "function_call_output"
                            | "custom_tool_call_output"
                            | "local_shell_call_output"
                    )
                    && let Some(output) = fields.get("output")
                {
                    let output = crate::audit::project_tool_output(output, &fields);
                    fields.insert("output".to_owned(), output);
                }
                Value::Object(fields)
            } else {
                bounded_value(raw, &mut remaining)?
            };
            envelope.insert("payload".to_owned(), payload);
        }
        let projected = serde_json::to_string(&envelope)?;
        if projected.len() > 4 * 1024 * 1024 {
            return Err(projection_limit());
        }
        Ok(projected)
    }
}

fn projection_limit() -> serde_json::Error {
    serde::de::Error::custom("audit projection byte limit exceeded")
}
fn bounded_value(raw: &RawValue, remaining: &mut usize) -> serde_json::Result<Value> {
    *remaining = remaining
        .checked_sub(raw.get().len())
        .ok_or_else(projection_limit)?;
    serde_json::from_str(raw.get())
}
fn keep_payload_field(kind: Option<&str>, item: &str, name: &str) -> bool {
    match kind {
        Some("session_meta") => matches!(
            name,
            "id" | "originator"
                | "timestamp"
                | "history_mode"
                | "history_base"
                | "forked_from_id"
                | "forked_from_ordinal_exclusive"
                | "subagent_history_start_ordinal"
        ),
        Some("turn_context") => matches!(name, "turn_id" | "model" | "effort" | "reasoning_effort"),
        Some("token_usage_record") => matches!(
            name,
            "thread_id" | "turn_id" | "response_id" | "usage" | "thread_token_usage"
        ),
        Some("event_msg") => {
            name == "type"
                || match item {
                    "token_count" => name == "info",
                    "task_started" | "task_complete" => {
                        matches!(name, "turn_id" | "duration_ms")
                    }
                    "user_message" => name == "message",
                    _ => false,
                }
        }
        Some("response_item") => {
            name == "type"
                || match item {
                    "function_call" | "custom_tool_call" | "local_shell_call"
                    | "tool_search_call" => {
                        matches!(name, "name" | "arguments" | "input" | "call_id")
                    }
                    "function_call_output"
                    | "custom_tool_call_output"
                    | "local_shell_call_output" => {
                        matches!(name, "output" | "call_id" | "is_error" | "status")
                    }
                    _ => false,
                }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_strings_and_escaped_keys_match_value_semantics() {
        let record = Record::parse(r#"{"type":"old","ty\u0070e":"event_msg","timestamp":"2026-09-01T00:00:00Z","ordinal":7,"payload":{"message":"not exported"}}"#).unwrap().unwrap();
        assert!(matches!(
            record.string("type"),
            Some(Cow::Borrowed("event_msg"))
        ));
        assert_eq!(record.unsigned("ordinal"), Some(7));
        assert_eq!(
            record.value("payload").unwrap().unwrap()["message"],
            "not exported"
        );
        let escaped = Record::parse(r#"{"type":"event\u005fmsg"}"#)
            .unwrap()
            .unwrap();
        assert_eq!(escaped.string("type").as_deref(), Some("event_msg"));
        assert!(Record::parse("null").unwrap().is_none());
        let wrong = Record::parse(r#"{"type":42,"ordinal":-1,"payload":null}"#)
            .unwrap()
            .unwrap();
        assert!(wrong.string("type").is_none());
        assert!(wrong.unsigned("ordinal").is_none());
        assert_eq!(wrong.value("payload").unwrap(), Some(Value::Null));
    }

    #[test]
    fn malformed_unused_payloads_are_still_rejected() {
        for invalid in [
            r#"{"type":"world_state","payload":{"nested":[1,]}}"#,
            r#"{"type":"world_state","payload":"unterminated}"#,
            r#"{"type":"world_state"} trailing"#,
            "[1,]",
            "",
            "not-json",
        ] {
            assert!(Record::parse(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn large_unused_bodies_are_borrowed_and_retained_fields_stay_bounded() {
        let body = "x".repeat(5 * 1024 * 1024);
        for (kind, payload) in [
            (
                "compacted",
                serde_json::json!({"replacement_history":[body],"guardian_history":[body]}),
            ),
            (
                "event_msg",
                serde_json::json!({"type":"item_completed","item":{"content":body}}),
            ),
            (
                "response_item",
                serde_json::json!({"type":"message","content":[{"image_url":body}]}),
            ),
            (
                "session_meta",
                serde_json::json!({"id":"owner","originator":"codex_app","base_instructions":body}),
            ),
        ] {
            let input = serde_json::json!({"type":kind,"timestamp":"2026-09-08T00:00:00Z","payload":payload}).to_string();
            let projected = Record::parse(&input)
                .unwrap()
                .unwrap()
                .audit_projection()
                .unwrap();
            assert!(projected.len() < 200);
            assert!(!projected.contains("xxxxx"));
        }
        let input = serde_json::json!({"type":"event_msg","payload":{"type":"user_message","message":body}}).to_string();
        assert!(
            Record::parse(&input)
                .unwrap()
                .unwrap()
                .audit_projection()
                .is_err()
        );
        let many = format!(
            "{{{}}}",
            (0..129)
                .map(|n| format!("\"key{n}\":null"))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(Record::parse(&many).is_err());
        let input = format!("{{\"type\":\"compacted\",\"payload\":{many}}}");
        assert!(
            Record::parse(&input)
                .unwrap()
                .unwrap()
                .audit_projection()
                .is_err()
        );
    }
}
