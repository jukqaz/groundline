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
