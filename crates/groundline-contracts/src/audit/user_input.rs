//! Classify native input envelopes locally; never retain or emit message text.
//!
//! Fixed host delimiters use std string operations. JSON payloads use the
//! existing Serde parser so quoted closing tags cannot terminate an envelope.
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Inputs {
    plain: u64,
    annotations: u64,
    answers: u64,
    file_requests: u64,
    automation: u64,
    context: u64,
    empty: u64,
    unresolved: u64,
    unavailable: u64,
    human_text: u64,
    unique: u64,
    event_records: u64,
    response_records: u64,
    duplicates: u64,
    non_text_blocks: u64,
    unsupported_blocks: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    Event,
    Response,
}

/// Pair exact text only across the two native representations of an input.
/// The pending multiset is rollout-local and cleared by assistant progress or
/// an explicit turn boundary. Same-origin repeats are separate actions; paired
/// entries are consumed, so another event/response pair counts again. This is
/// a bounded representation deduplication contract, not semantic equivalence
/// or proof that an input burst contains every user action.
#[derive(Default)]
pub(super) struct InputBurst {
    pending: BTreeMap<[u8; 32], (u64, u64)>,
    turn: Option<[u8; 32]>,
}

impl InputBurst {
    pub fn boundary(&mut self, record_type: &str, payload: &serde_json::Map<String, Value>) {
        let item = payload.get("type").and_then(Value::as_str);
        if record_type == "turn_context"
            || (record_type == "event_msg" && item == Some("task_started"))
        {
            if let Some(turn) = payload.get("turn_id").and_then(Value::as_str) {
                let turn = digest(turn);
                if self.turn.is_some_and(|previous| previous != turn) {
                    self.pending.clear();
                }
                self.turn = Some(turn);
            } else {
                // An unowned turn boundary is not evidence that two equal
                // native records belong to the same user action.
                self.pending.clear();
                self.turn = None;
            }
        } else if (record_type == "response_item"
            && !(item == Some("message")
                && payload.get("role").and_then(Value::as_str) == Some("user")))
            || (record_type == "event_msg"
                && matches!(
                    item,
                    Some("agent_message" | "agent_reasoning" | "task_complete" | "turn_aborted")
                ))
        {
            self.pending.clear();
            if matches!(item, Some("task_complete" | "turn_aborted")) {
                self.turn = None;
            }
        }
    }

    fn duplicate(&mut self, source: Source, text: &str) -> bool {
        let key = digest(text);
        let entry = self.pending.entry(key).or_default();
        let (own, other) = match source {
            Source::Event => (&mut entry.0, &mut entry.1),
            Source::Response => (&mut entry.1, &mut entry.0),
        };
        if *other > 0 {
            *other -= 1;
            if entry.0 == 0 && entry.1 == 0 {
                self.pending.remove(&key);
            }
            true
        } else {
            *own += 1;
            false
        }
    }
}

fn digest(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

struct NativeText {
    text: Option<String>,
    non_text_blocks: u64,
    unsupported_blocks: u64,
}

fn response_text(content: Option<&Value>) -> NativeText {
    let mut input = NativeText {
        text: None,
        non_text_blocks: 0,
        unsupported_blocks: 0,
    };
    let Some(blocks) = content.and_then(Value::as_array) else {
        return input;
    };
    let mut segments = Vec::new();
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("input_text" | "text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    segments.push(text);
                } else {
                    input.unsupported_blocks += 1;
                }
            }
            Some("input_image" | "image" | "input_audio" | "audio" | "input_video" | "video") => {
                input.non_text_blocks += 1;
            }
            _ => input.unsupported_blocks += 1,
        }
    }
    if !segments.is_empty() || blocks.is_empty() {
        input.text = Some(segments.join("\n"));
    }
    input
}

pub(super) struct TextObservation {
    pub length: u64,
    pub broad_scope: bool,
}

enum Input {
    Plain(TextObservation),
    Annotations(Option<TextObservation>),
    Answers(Option<TextObservation>),
    FileRequest(Option<TextObservation>),
    Automation,
    Context,
    Empty,
    Unresolved,
}

#[derive(Deserialize)]
struct Annotation {
    annotation: Option<String>,
}

#[derive(Deserialize)]
struct Answer {
    answer: String,
}

impl Inputs {
    pub fn observe_native(
        &mut self,
        source: Source,
        payload: &serde_json::Map<String, Value>,
        burst: &mut InputBurst,
    ) -> Option<TextObservation> {
        let native = match source {
            Source::Event => {
                self.event_records += 1;
                NativeText {
                    text: payload
                        .get("message")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    non_text_blocks: 0,
                    unsupported_blocks: 0,
                }
            }
            Source::Response => {
                self.response_records += 1;
                response_text(payload.get("content"))
            }
        };
        // Count unavailable non-text coverage even if its text representation
        // also occurs in event_msg. Deduplication must not hide mixed input.
        self.non_text_blocks += native.non_text_blocks;
        self.unsupported_blocks += native.unsupported_blocks;
        if native
            .text
            .as_deref()
            .is_some_and(|text| burst.duplicate(source, text))
        {
            self.duplicates += 1;
            return None;
        }
        self.observe(native.text.as_deref())
    }

    pub fn observe(&mut self, message: Option<&str>) -> Option<TextObservation> {
        self.unique += 1;
        let Some(message) = message else {
            self.unavailable += 1;
            return None;
        };
        let observed = match classify(message) {
            Input::Plain(text) => {
                self.plain += 1;
                Some(text)
            }
            Input::Annotations(text) => {
                self.annotations += 1;
                text
            }
            Input::Answers(text) => {
                self.answers += 1;
                text
            }
            Input::FileRequest(text) => {
                self.file_requests += 1;
                text
            }
            Input::Automation => {
                self.automation += 1;
                None
            }
            Input::Context => {
                self.context += 1;
                None
            }
            Input::Empty => {
                self.empty += 1;
                None
            }
            Input::Unresolved => {
                self.unresolved += 1;
                None
            }
        };
        self.human_text += u64::from(observed.is_some());
        observed
    }

    pub fn response_records(&self) -> u64 {
        self.response_records
    }

    pub fn record_count(&self) -> u64 {
        self.event_records + self.response_records
    }

    pub fn duplicate_count(&self) -> u64 {
        self.duplicates
    }

    pub fn unique_count(&self) -> u64 {
        self.unique
    }

    pub fn coverage_observation_complete(&self) -> bool {
        self.empty == 0
            && self.unresolved == 0
            && self.unavailable == 0
            && self.non_text_blocks == 0
            && self.unsupported_blocks == 0
    }

    pub fn json(&self) -> Value {
        json!({
            "revision":3,
            "basis":"recognized_native_input_envelopes",
            "coverage_scope":"selected_native_user_records_only",
            "deduplication_basis":"exact_text_cross_origin_pairs_within_rollout_input_burst",
            "coverage_observation_complete":self.coverage_observation_complete(),
            "coverage_issue_counts":{
                "empty_text":self.empty,
                "unresolved_envelope":self.unresolved,
                "unavailable_text":self.unavailable,
                "non_text_content":self.non_text_blocks,
                "unsupported_content":self.unsupported_blocks,
            },
            "complete":self.human_text > 0 && self.coverage_observation_complete(),
            "native_record_count":self.record_count(),
            "duplicate_native_record_count":self.duplicates,
            "unique_native_input_count":self.unique,
            "human_text_input_count":self.human_text,
            "non_text_content_block_count":self.non_text_blocks,
            "unsupported_content_block_count":self.unsupported_blocks,
            "plain_text_count":self.plain,
            "annotation_envelope_count":self.annotations,
            "question_answer_envelope_count":self.answers,
            "file_request_envelope_count":self.file_requests,
            "automation_envelope_count":self.automation,
            "context_envelope_count":self.context,
            "empty_text_count":self.empty,
            "unresolved_envelope_count":self.unresolved,
            "unavailable_text_count":self.unavailable,
        })
    }
}

fn broad_scope(text: &str) -> bool {
    // These are lexical hints, not semantic scope, permission, or rework proof.
    ["다 ", "전체", "완전", "모든"]
        .iter()
        .any(|term| text.contains(term))
        || text.split(|c: char| !c.is_ascii_alphabetic()).any(|word| {
            ["all", "everything", "entire", "completely"]
                .iter()
                .any(|term| word.eq_ignore_ascii_case(term))
        })
}

fn text<'a>(segments: impl Iterator<Item = &'a str>) -> Option<TextObservation> {
    let mut length = 0;
    let mut broad = false;
    let mut count = 0;
    for segment in segments.filter(|segment| !segment.trim().is_empty()) {
        length += segment.chars().count() as u64 + u64::from(count > 0);
        broad |= broad_scope(segment);
        count += 1;
    }
    (count > 0).then_some(TextObservation {
        length,
        broad_scope: broad,
    })
}

fn array<T: DeserializeOwned>(input: &str) -> Option<(Vec<T>, &str)> {
    let mut stream = serde_json::Deserializer::from_str(input).into_iter::<Vec<T>>();
    let values = stream.next()?.ok()?;
    Some((values, input.get(stream.byte_offset()..)?.trim_start()))
}

fn environment(input: &str) -> bool {
    input.starts_with("<environment_context>") && input.ends_with("</environment_context>")
}

fn owner_requested_manual_run(input: &str) -> bool {
    // Current manual automation callers omit Automation memory. Recognize
    // their ordered header and exact run kind; ID-only, arbitrary run kinds
    // and quoted copies stay literal human input.
    let mut header = input.lines();
    header.next().is_some_and(|line| {
        line.strip_prefix("Automation: ")
            .is_some_and(|name| !name.trim().is_empty())
    }) && header.next().is_some_and(|line| {
        line.strip_prefix("Automation ID: ")
            .is_some_and(|id| !id.trim().is_empty())
    }) && header.next() == Some("Run type: owner-requested one-time manual execution")
}

fn classify(message: &str) -> Input {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Input::Empty;
    }
    if trimmed.starts_with("<heartbeat>") {
        return match trimmed.strip_suffix("</heartbeat>") {
            Some(body)
                if body.contains("<automation_id>")
                    && body.contains("</automation_id>")
                    && body.contains("<instructions>")
                    && body.contains("</instructions>") =>
            {
                Input::Automation
            }
            _ => Input::Unresolved,
        };
    }
    if (trimmed.starts_with("Automation:")
        && trimmed
            .lines()
            .any(|line| line.starts_with("Automation ID:"))
        && trimmed
            .lines()
            .any(|line| line.starts_with("Automation memory:")))
        || owner_requested_manual_run(trimmed)
    {
        return Input::Automation;
    }
    if trimmed.starts_with("<environment_context>") {
        return if environment(trimmed) {
            Input::Context
        } else {
            Input::Unresolved
        };
    }
    if trimmed.starts_with("# AGENTS.md instructions for ") {
        return match trimmed.split_once("</INSTRUCTIONS>") {
            Some((head, tail))
                if head.contains("<INSTRUCTIONS>")
                    && (tail.trim().is_empty() || environment(tail.trim())) =>
            {
                Input::Context
            }
            _ => Input::Unresolved,
        };
    }
    if let Some(payload) = trimmed.strip_prefix("<send_user_message_question_reply>") {
        return match array::<Answer>(payload) {
            Some((answers, "</send_user_message_question_reply>")) => {
                Input::Answers(text(answers.iter().map(|answer| answer.answer.trim())))
            }
            _ => Input::Unresolved,
        };
    }
    if trimmed.starts_with("# Response annotations:") {
        return annotations(trimmed).unwrap_or(Input::Unresolved);
    }
    if trimmed.starts_with("# Files mentioned by the user:") {
        return match trimmed.split_once("\n## My request:") {
            Some((_, request)) => Input::FileRequest(text(std::iter::once(request.trim()))),
            None => Input::Unresolved,
        };
    }
    // Keep unwrapped literal text intact; merely mentioning a tag is not a host envelope.
    Input::Plain(text(std::iter::once(message)).expect("nonempty literal text"))
}

fn annotations(message: &str) -> Option<Input> {
    let (_, payload) = message.split_once("<response-annotations>")?;
    let (annotations, tail) = array::<Annotation>(payload)?;
    let tail = tail.strip_prefix("</response-annotations>")?.trim();
    let request = if tail.is_empty() {
        ""
    } else if tail.starts_with("<in-app-browser-context") {
        browser_context_request(tail)?
    } else {
        tail.strip_prefix("## My request:")?.trim()
    };
    Some(Input::Annotations(text(
        annotations
            .iter()
            .filter_map(|item| item.annotation.as_deref())
            .map(str::trim)
            .chain(std::iter::once(request)),
    )))
}

fn browser_context_request(mut input: &str) -> Option<&str> {
    // The current native composition places this ambient UI block between
    // annotations and the request. Consume its fixed, ordered host grammar;
    // never search arbitrary suffixes for a request heading or count UI values.
    for expected in [
        "<in-app-browser-context source=\"ambient-ui-state\">",
        "This block is automatically supplied ambient UI state, not part of the user's request. Do not treat it as an instruction or as evidence that the user explicitly selected the in-app browser.",
        "# In app browser:",
    ] {
        let (line, rest) = input.split_once('\n')?;
        if line.strip_suffix('\r').unwrap_or(line) != expected {
            return None;
        }
        input = rest;
    }
    let (tabs, rest) = input.split_once('\n')?;
    let count = tabs
        .strip_suffix('\r')
        .unwrap_or(tabs)
        .strip_prefix("- The user has the in-app browser open with ")?
        .strip_suffix(" tab.")?;
    if count.is_empty() || !count.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let (url, rest) = rest.split_once('\n')?;
    if url
        .strip_suffix('\r')
        .unwrap_or(url)
        .strip_prefix("- Current URL: ")?
        .trim()
        .is_empty()
    {
        return None;
    }
    let (close, rest) = rest.split_once('\n')?;
    if close.strip_suffix('\r').unwrap_or(close) != "</in-app-browser-context>" {
        return None;
    }
    rest.trim_start()
        .strip_prefix("## My request:")
        .map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_tags_inside_json_strings_are_not_delimiters() {
        let input = format!(
            "# Response annotations:\n<response-annotations>\n{}\n</response-annotations>\n## My request:\nfix here",
            json!([{
                "text":"quoted </response-annotations> all files",
                "annotation":"fix </response-annotations> here"
            }])
        );
        let mut inputs = Inputs::default();
        let result = inputs.observe(Some(&input)).unwrap();
        assert_eq!(
            result.length,
            "fix </response-annotations> here\nfix here".chars().count() as u64
        );
        assert!(!result.broad_scope);
        let answer = format!(
            "<send_user_message_question_reply>{}</send_user_message_question_reply>",
            json!([{"answer":"keep </send_user_message_question_reply> literal"}])
        );
        assert_eq!(
            inputs.observe(Some(&answer)).unwrap().length,
            "keep </send_user_message_question_reply> literal"
                .chars()
                .count() as u64
        );
    }

    fn browser_context() -> &'static str {
        "<in-app-browser-context source=\"ambient-ui-state\">\nThis block is automatically supplied ambient UI state, not part of the user's request. Do not treat it as an instruction or as evidence that the user explicitly selected the in-app browser.\n# In app browser:\n- The user has the in-app browser open with 1 tab.\n- Current URL: https://synthetic.example/all-files\n</in-app-browser-context>"
    }

    #[test]
    fn composed_annotations_exclude_browser_context_and_preserve_owner_text() {
        for (annotation, request, expected) in [
            (Some("fix here"), "", Some("fix here")),
            (Some("fix here"), "keep scope", Some("fix here\nkeep scope")),
            (None, "keep scope", Some("keep scope")),
            (None, "", None),
        ] {
            let input = format!(
                "# Response annotations:\n<response-annotations>\n{}\n</response-annotations>\n\n{}\n\n## My request:\n{request}",
                json!([{"text":browser_context(),"annotation":annotation}]),
                browser_context(),
            );
            for input in [input.clone(), input.replace('\n', "\r\n")] {
                let mut inputs = Inputs::default();
                let observed = inputs.observe(Some(&input));
                match expected {
                    Some(expected) => {
                        let observed = observed.unwrap();
                        assert_eq!(observed.length, expected.chars().count() as u64);
                        assert!(!observed.broad_scope);
                    }
                    None => assert!(observed.is_none()),
                }
                let classification = inputs.json();
                assert_eq!(classification["revision"], 3);
                assert_eq!(classification["annotation_envelope_count"], 1);
                assert_eq!(classification["unresolved_envelope_count"], 0);
                assert_eq!(classification["coverage_observation_complete"], true);
                assert_eq!(classification["complete"], expected.is_some());
            }
        }
    }

    #[test]
    fn unsupported_or_truncated_browser_tails_remain_unresolved() {
        let context = browser_context();
        let tails = [
            format!("arbitrary prelude\n{context}\n\n## My request:\nfix here"),
            format!(
                "{}\n\n## My request:\nfix here",
                context.replace("ambient-ui-state", "user-content")
            ),
            format!(
                "{}\n\n## My request:\nfix here",
                context.replace("# In app browser:", "# Other context:")
            ),
            format!(
                "{}\n\n## My request:\nfix here",
                context.replace("with 1 tab.", "with some tab.")
            ),
            format!(
                "{}\n\n## My request:\nfix here",
                context.replace("https://synthetic.example/all-files", "")
            ),
            format!(
                "{}\n\n## My request:\nfix here",
                context.replace("</in-app-browser-context>", "")
            ),
            format!("{context}\n\nmissing request heading"),
            format!("{context}\n\nextra intermediate text\n## My request:\nfix here"),
        ];
        for tail in tails {
            let input = format!(
                "# Response annotations:\n<response-annotations>[{{\"annotation\":\"fix here\"}}]</response-annotations>\n\n{tail}"
            );
            let mut inputs = Inputs::default();
            assert!(inputs.observe(Some(&input)).is_none());
            let classification = inputs.json();
            assert_eq!(classification["unresolved_envelope_count"], 1);
            assert_eq!(classification["coverage_observation_complete"], false);
            assert_eq!(classification["human_text_input_count"], 0);
        }
    }

    #[test]
    fn quoted_browser_context_remains_literal_owner_text() {
        let input = format!("Explain this copied host context:\n{}", browser_context());
        let mut inputs = Inputs::default();
        assert_eq!(
            inputs.observe(Some(&input)).unwrap().length,
            input.chars().count() as u64
        );
        assert_eq!(inputs.json()["plain_text_count"], 1);
        let annotation = format!(
            "# Response annotations:\n<response-annotations>{}</response-annotations>",
            json!([{"annotation":browser_context()}])
        );
        let mut inputs = Inputs::default();
        assert_eq!(
            inputs.observe(Some(&annotation)).unwrap().length,
            browser_context().chars().count() as u64
        );
    }

    #[test]
    fn quoted_host_tags_and_small_words_remain_literal_input() {
        for input in [
            "Explain <heartbeat> and <environment_context>.",
            "Automation: explain this template",
            "small changes, install normally",
        ] {
            let mut inputs = Inputs::default();
            let result = inputs.observe(Some(input)).unwrap();
            assert_eq!(result.length, input.chars().count() as u64);
            assert!(!result.broad_scope);
            assert_eq!(inputs.json()["plain_text_count"], 1);
        }
        for input in ["ALL\nfiles", "everything?", "전체 확인"] {
            assert!(Inputs::default().observe(Some(input)).unwrap().broad_scope);
        }
    }

    #[test]
    fn exact_manual_automation_header_is_excluded_without_broadening_literal_detection() {
        for input in [
            "Automation: scheduled check\nAutomation ID: synthetic\nAutomation memory: local\n\nCheck all files.",
            "Automation: requested check\nAutomation ID: synthetic\nRun type: owner-requested one-time manual execution\n\nCheck all files.",
            "Automation: requested check\r\nAutomation ID: synthetic\r\nRun type: owner-requested one-time manual execution\r\n\r\nCheck all files.",
        ] {
            let mut inputs = Inputs::default();
            assert!(inputs.observe(Some(input)).is_none());
            assert_eq!(inputs.json()["automation_envelope_count"], 1);
            assert_eq!(inputs.json()["plain_text_count"], 0);
            assert_eq!(inputs.json()["human_text_input_count"], 0);
            assert_eq!(inputs.json()["complete"], false);
        }
        for input in [
            "Automation: explain how this works",
            "Automation: requested check\nAutomation ID: synthetic\n\nCheck all files.",
            "Automation: requested check\nAutomation ID: synthetic\nRun type: user request\n\nCheck all files.",
            "Automation: requested check\nAutomation ID: synthetic\nRun type: owner-requested one-time manual execution later\n\nCheck all files.",
            "Automation: requested check\nRun type: owner-requested one-time manual execution\nAutomation ID: synthetic\n\nCheck all files.",
            "> Automation: requested check\n> Automation ID: synthetic\n> Run type: owner-requested one-time manual execution",
            "```text\nAutomation: requested check\nAutomation ID: synthetic\nRun type: owner-requested one-time manual execution\n```",
            "Explain this header:\nAutomation: requested check\nAutomation ID: synthetic\nRun type: owner-requested one-time manual execution",
        ] {
            let mut inputs = Inputs::default();
            let observed = inputs.observe(Some(input)).unwrap();
            assert_eq!(observed.length, input.chars().count() as u64);
            assert_eq!(inputs.json()["automation_envelope_count"], 0);
            assert_eq!(inputs.json()["plain_text_count"], 1);
        }
    }

    #[test]
    fn context_is_separate_from_user_requests() {
        for input in [
            "<environment_context>all files</environment_context>",
            "# AGENTS.md instructions for project\n<INSTRUCTIONS>all files</INSTRUCTIONS>",
            "# AGENTS.md instructions for project\n<INSTRUCTIONS>all files</INSTRUCTIONS>\n<environment_context>all</environment_context>",
        ] {
            let mut inputs = Inputs::default();
            assert!(inputs.observe(Some(input)).is_none());
            assert_eq!(inputs.json()["context_envelope_count"], 1);
        }
        let input = "# Files mentioned by the user:\r\n## all-documents.md\r\nHost guidance.\r\n## My request:\r\nfix here";
        let mut inputs = Inputs::default();
        let result = inputs.observe(Some(input)).unwrap();
        assert_eq!(result.length, 8);
        assert!(!result.broad_scope);
        assert_eq!(inputs.json()["file_request_envelope_count"], 1);
    }

    #[test]
    fn incomplete_envelopes_do_not_fall_back_to_counting_host_text() {
        for input in [
            "<heartbeat><instructions>all</instructions></heartbeat>",
            "<environment_context>unfinished",
            "# AGENTS.md instructions for project\nmissing tags",
            "# Files mentioned by the user:\nall files",
            "<send_user_message_question_reply>[{\"answer\":2}]</send_user_message_question_reply>",
            "<send_user_message_question_reply>[{\"answer\":\"all\"}]</send_user_message_question_reply> trailing text",
            "# Response annotations:\n<response-annotations>[]</response-annotations> unexpected suffix",
        ] {
            let mut inputs = Inputs::default();
            assert!(inputs.observe(Some(input)).is_none());
            assert_eq!(inputs.json()["unresolved_envelope_count"], 1);
            assert_eq!(inputs.json()["complete"], false);
        }
        let mut inputs = Inputs::default();
        assert!(inputs.observe(None).is_none());
        assert_eq!(inputs.json()["unavailable_text_count"], 1);
        assert_eq!(inputs.json()["complete"], false);
        assert!(inputs.observe(Some(" \n")).is_none());
        assert_eq!(inputs.json()["empty_text_count"], 1);
    }
}
