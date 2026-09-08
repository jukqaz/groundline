//! Borrow record payloads until a consumer actually needs their contents.
use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::de::IgnoredAny;
use serde_json::{Value, value::RawValue};

pub struct Record<'a> {
    fields: BTreeMap<String, &'a RawValue>,
}

impl<'a> Record<'a> {
    /// Validate JSON syntax, retaining only a small envelope and borrowed slices.
    /// As with `Value::Object`, duplicate keys use their last value.
    pub fn parse(line: &'a str) -> serde_json::Result<Option<Self>> {
        if !line.trim_start().starts_with('{') {
            serde_json::from_str::<IgnoredAny>(line)?;
            return Ok(None);
        }
        serde_json::from_str(line).map(|fields| Some(Self { fields }))
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
        for name in ["type", "timestamp", "ordinal"] {
            if let Some(value) = self.value(name)? {
                envelope.insert(name.to_owned(), value);
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
        ) && let Some(mut payload) = self.value("payload")?
        {
            if let Some(fields) = payload.as_object_mut() {
                let item = fields
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                if kind.as_deref() == Some("response_item")
                    && matches!(
                        item.as_str(),
                        "function_call_output"
                            | "custom_tool_call_output"
                            | "local_shell_call_output"
                    )
                    && let Some(output) = fields.get("output")
                {
                    let output = crate::audit::project_tool_output(output, fields);
                    fields.insert("output".to_owned(), output);
                }
                fields.retain(|name, _| match kind.as_deref() {
                    Some("session_meta") => matches!(
                        name.as_str(),
                        "id" | "originator"
                            | "timestamp"
                            | "history_mode"
                            | "history_base"
                            | "forked_from_id"
                            | "forked_from_ordinal_exclusive"
                            | "subagent_history_start_ordinal"
                    ),
                    Some("turn_context") => matches!(
                        name.as_str(),
                        "turn_id" | "model" | "effort" | "reasoning_effort"
                    ),
                    Some("token_usage_record") => matches!(
                        name.as_str(),
                        "thread_id" | "turn_id" | "response_id" | "usage" | "thread_token_usage"
                    ),
                    Some("event_msg") => {
                        name == "type"
                            || match item.as_str() {
                                "token_count" => name == "info",
                                "task_started" | "task_complete" => {
                                    matches!(name.as_str(), "turn_id" | "duration_ms")
                                }
                                "user_message" => name == "message",
                                _ => false,
                            }
                    }
                    Some("response_item") => {
                        name == "type"
                            || match item.as_str() {
                                "function_call" | "custom_tool_call" | "local_shell_call"
                                | "tool_search_call" => matches!(
                                    name.as_str(),
                                    "name" | "arguments" | "input" | "call_id"
                                ),
                                "function_call_output"
                                | "custom_tool_call_output"
                                | "local_shell_call_output" => matches!(
                                    name.as_str(),
                                    "output" | "call_id" | "is_error" | "status"
                                ),
                                _ => false,
                            }
                    }
                    _ => false,
                });
            }
            envelope.insert("payload".to_owned(), payload);
        }
        serde_json::to_string(&envelope)
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
}
