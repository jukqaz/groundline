//! Explicit, developer-only native inference; never part of Core or verify-source.
//! Synthetic inputs and in-memory events only. Reports contain derived verdicts.
#![forbid(unsafe_code)]

use clap::{Parser, ValueEnum};
use command_group::CommandGroup;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MAX_FRAME: u64 = 2 * 1024 * 1024;
const MAX_STREAM: usize = 8 * 1024 * 1024;
const OUTPUT_BUDGET: usize = 32 * 1024;
static NATIVE_STARTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Case {
    ReportReview,
    OrdinaryFix,
    VerificationReuse,
}

#[derive(Parser)]
#[command(about = "Explicit synthetic native Codex behavior check; consumes inference")]
struct Args {
    #[arg(long)]
    run_native: bool,
    #[arg(long)]
    codex: PathBuf,
    #[arg(long)]
    plugin_root: PathBuf,
    /// Existing owner-private directory; a new fixture is created inside it.
    #[arg(long)]
    work: PathBuf,
    /// New owner-private derived report; raw model/tool events are never saved.
    #[arg(long)]
    report: PathBuf,
    #[arg(long, value_enum)]
    case: Case,
    /// Pass the owner's observed current selection, without changing configuration.
    #[arg(long)]
    model: String,
    #[arg(long)]
    effort: String,
    #[arg(long)]
    service_tier: String,
    #[arg(long, default_value_t = 240, value_parser = clap::value_parser!(u64).range(30..=600))]
    timeout_seconds: u64,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn file_hash(path: &Path) -> Result<String, &'static str> {
    let mut file = fs::File::open(path).map_err(|_| "binary_unavailable")?;
    let mut state = Sha256::new();
    let mut buffer = [0; 32 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|_| "binary_unavailable")?;
        if n == 0 {
            break;
        }
        state.update(&buffer[..n]);
    }
    Ok(format!("{:x}", state.finalize()))
}

fn exact_command(command: &str, expected: &[&str]) -> bool {
    let Some(words) = shlex::split(command) else {
        return false;
    };
    if words == expected {
        return true;
    }
    words.len() == 3
        && ["/bin/sh", "/bin/zsh", "/bin/bash"].contains(&words[0].as_str())
        && ["-c", "-lc"].contains(&words[1].as_str())
        && shlex::split(&words[2]).is_some_and(|inner| inner == expected)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), &'static str> {
    fs::create_dir_all(to).map_err(|_| "fixture_write_failed")?;
    for entry in fs::read_dir(from).map_err(|_| "guidance_unavailable")? {
        let entry = entry.map_err(|_| "guidance_unavailable")?;
        let kind = entry.file_type().map_err(|_| "guidance_unavailable")?;
        if kind.is_symlink() {
            return Err("guidance_symlink_rejected");
        }
        if kind.is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        } else if kind.is_file() {
            fs::copy(entry.path(), to.join(entry.file_name()))
                .map_err(|_| "fixture_write_failed")?;
        } else {
            return Err("guidance_nonregular_rejected");
        }
    }
    Ok(())
}

fn snapshot(root: &Path) -> Result<BTreeMap<String, String>, &'static str> {
    let mut out = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.map_err(|_| "fixture_read_failed")?;
        if entry.file_type().is_symlink() {
            return Err("fixture_symlink_rejected");
        }
        if entry.file_type().is_file() {
            let bytes = fs::read(entry.path()).map_err(|_| "fixture_read_failed")?;
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| "fixture_read_failed")?;
            out.insert(relative.to_string_lossy().into_owned(), hash(&bytes));
        }
    }
    Ok(out)
}

fn fixture(root: &Path, case: Case) -> Result<String, &'static str> {
    fs::write(root.join("AGENTS.md"), "Work only on this synthetic fixture. Do not delegate, create Goals, install tools, use external services, or change settings. Reviews are read-only. Preserve unrelated files. Use the smallest relevant verification.\n")
        .map_err(|_| "fixture_write_failed")?;
    match case {
        Case::ReportReview => {
            for (name, count, failures) in [("report-a.json", 701, 17), ("report-b.json", 903, 29)]
            {
                let rows: Vec<_> = (0..count).map(|i| json!({"check":i,"status":if i>=count-failures{"FAIL"}else{"PASS"},"detail":"synthetic filler ".repeat(40)})).collect();
                fs::write(root.join(name), serde_json::to_vec(&rows).unwrap())
                    .map_err(|_| "fixture_write_failed")?;
            }
            Ok("Use $optimize-codex-workflow for this read-only assessment. Read its complete SKILL.md and the complete .agents/references/task-continuity.md so the evaluator can confirm actual instruction exposure. Inspect report-a.json and report-b.json. Return only a JSON object with a_total, a_failed, b_total, b_failed. Include every row in the counts. Do not change any file or create a report file. These are synthetic checks, not usage or delivery evidence.".into())
        }
        Case::OrdinaryFix => {
            fs::write(root.join("message.txt"), "Helo GroundLine\n")
                .map_err(|_| "fixture_write_failed")?;
            Ok("Fix the typo in message.txt so it reads Hello GroundLine on one line. Change only this file. Verify the exact line by running /usr/bin/grep -qx 'Hello GroundLine' message.txt as a separate command, then report the result briefly.".into())
        }
        Case::VerificationReuse => {
            fs::write(root.join("README.txt"), "A smal fixture.\n")
                .map_err(|_| "fixture_write_failed")?;
            fs::write(root.join("payload.txt"), "unchanged\n")
                .map_err(|_| "fixture_write_failed")?;
            fs::write(root.join("check.sh"), "#!/bin/sh\nset -eu\nn=$(cat runs.txt)\nprintf '%s\\n' \"$((n+1))\" > runs.txt\ngrep -qx unchanged payload.txt\n").map_err(|_| "fixture_write_failed")?;
            fs::write(root.join("runs.txt"), "0\n").map_err(|_| "fixture_write_failed")?;
            let result = Command::new("/bin/sh")
                .arg("check.sh")
                .current_dir(root)
                .output()
                .map_err(|_| "baseline_check_failed")?;
            if !result.status.success() {
                return Err("baseline_check_failed");
            }
            fs::write(root.join("verification.json"), serde_json::to_vec(&json!({"exit_code":0,"payload_sha256":hash(b"unchanged\n"),"check_sha256":hash(&fs::read(root.join("check.sh")).unwrap()),"runs":1})).unwrap()).map_err(|_| "fixture_write_failed")?;
            Ok("Use $optimize-codex-workflow. Read its complete SKILL.md and the complete .agents/references/task-continuity.md so the evaluator can confirm actual instruction exposure. Fix only smal -> small in README.txt. The fixture owner has already executed check.sh successfully against the unchanged payload; verification.json binds the checked bytes. Inspect the relevant bytes and reuse that result if still applicable. Do not rerun an unchanged check just because a new turn started. Report whether the result is reused; do not claim a new test execution.".into())
        }
    }
}

fn native_base(args: &Args, root: &Path) -> Command {
    let mut command = Command::new(&args.codex);
    command.env_clear();
    for key in [
        "HOME",
        "USER",
        "LOGNAME",
        "LANG",
        "LC_ALL",
        "TMPDIR",
        "CODEX_HOME",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .current_dir(root);
    let filesystem = format!(
        "permissions.groundline-eval.filesystem={{ \":minimal\"=\"read\", {}=\"write\" }}",
        serde_json::to_string(&root.to_string_lossy()).unwrap()
    );
    for config in [
        "default_permissions=\"groundline-eval\"".to_owned(),
        filesystem,
        "permissions.groundline-eval.network.enabled=false".to_owned(),
        "features.hooks=false".to_owned(),
        "features.memories=false".to_owned(),
        "features.multi_agent=false".to_owned(),
        "web_search=\"disabled\"".to_owned(),
        format!("model={}", serde_json::to_string(&args.model).unwrap()),
        format!(
            "model_reasoning_effort={}",
            serde_json::to_string(&args.effort).unwrap()
        ),
        format!(
            "service_tier={}",
            serde_json::to_string(&args.service_tier).unwrap()
        ),
    ] {
        command.args(["-c", &config]);
    }
    command
}

fn isolation(args: &Args, root: &Path, denied: &Path) -> Result<(), &'static str> {
    fs::write(root.join("probe.txt"), "allowed\n").map_err(|_| "fixture_write_failed")?;
    fs::write(denied, "excluded synthetic sentinel\n").map_err(|_| "fixture_write_failed")?;
    let yes = native_base(args, root)
        .args(["sandbox", "--", "/bin/cat"])
        .arg(root.join("probe.txt"))
        .output()
        .map_err(|_| "isolation_probe_failed")?;
    let no = native_base(args, root)
        .args(["sandbox", "--", "/bin/cat"])
        .arg(denied)
        .output()
        .map_err(|_| "isolation_probe_failed")?;
    if !yes.status.success()
        || yes.stdout != b"allowed\n"
        || no.status.success()
        || !no.stdout.is_empty()
    {
        return Err("isolation_probe_failed");
    }
    fs::remove_file(root.join("probe.txt")).map_err(|_| "fixture_write_failed")?;
    Ok(())
}

#[derive(Default)]
struct Observed {
    thread_count: usize,
    completed_turns: usize,
    failed_turn: bool,
    command_count: usize,
    failed_commands: usize,
    verification_observed: bool,
    tool_bytes: usize,
    truncated: bool,
    guidance_seen: bool,
    entry_seen: bool,
    final_text: String,
    pending: BTreeSet<String>,
    complete_ids: BTreeSet<String>,
    unsupported: bool,
    unsupported_types: BTreeSet<String>,
    usage: Option<Value>,
}

fn document_seen(output: &str, document: &str) -> bool {
    let lines: Vec<_> = document
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    !lines.is_empty() && lines.iter().all(|line| output.contains(line))
}

impl Observed {
    fn event(&mut self, value: &Value, entry: &str, guidance: &str) {
        match value["type"].as_str().unwrap_or("") {
            "thread.started" => self.thread_count += 1,
            "turn.completed" => {
                self.completed_turns += 1;
                if let Some(usage) = value.get("usage") {
                    self.usage = Some(json!({
                        "input_tokens":usage.get("input_tokens").and_then(Value::as_u64),
                        "cached_input_tokens":usage.get("cached_input_tokens").and_then(Value::as_u64),
                        "output_tokens":usage.get("output_tokens").and_then(Value::as_u64)
                    }));
                }
            }
            "turn.failed" | "error" => self.failed_turn = true,
            "item.started" => {
                if let Some(id) = value["item"]["id"].as_str() {
                    self.pending.insert(id.into());
                }
            }
            "item.completed" => {
                let item = &value["item"];
                let Some(id) = item["id"].as_str() else {
                    self.unsupported = true;
                    return;
                };
                self.pending.remove(id);
                if !self.complete_ids.insert(id.into()) {
                    self.unsupported = true;
                    return;
                }
                match item["type"].as_str().unwrap_or("") {
                    "command_execution" => {
                        self.command_count += 1;
                        if !matches!(item["status"].as_str(), Some("completed" | "failed"))
                            || item["exit_code"].as_i64().is_none()
                        {
                            self.unsupported = true;
                        }
                        if item["exit_code"].as_i64().is_some_and(|code| code != 0) {
                            self.failed_commands += 1;
                        }
                        self.verification_observed |= item["exit_code"] == 0
                            && item["command"].as_str().is_some_and(|command| {
                                exact_command(
                                    command,
                                    &["/usr/bin/grep", "-qx", "Hello GroundLine", "message.txt"],
                                )
                            });
                        let Some(output) = item["aggregated_output"].as_str() else {
                            self.unsupported = true;
                            return;
                        };
                        self.tool_bytes = self.tool_bytes.saturating_add(output.len());
                        self.truncated |= output.contains("Warning: truncated output")
                            || output.contains("tokens truncated");
                        self.entry_seen |= document_seen(output, entry);
                        self.guidance_seen |= document_seen(output, guidance);
                    }
                    "agent_message" => {
                        if let Some(text) = item["text"].as_str() {
                            self.final_text = text.to_owned();
                        }
                    }
                    "reasoning" | "plan" | "file_change" => {}
                    other => {
                        self.unsupported = true;
                        if other.len() <= 64
                            && other
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
                        {
                            self.unsupported_types.insert(format!("item:{other}"));
                        }
                    }
                }
            }
            "turn.started" | "item.updated" => {}
            other => {
                self.unsupported = true;
                if other.len() <= 64
                    && other
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
                {
                    self.unsupported_types.insert(format!("event:{other}"));
                }
            }
        }
    }
}

fn run_model(
    args: &Args,
    root: &Path,
    prompt: &str,
    entry: &str,
    guidance: &str,
) -> Result<(Observed, bool), &'static str> {
    let mut command = native_base(args, root);
    command
        .args([
            "exec",
            "--strict-config",
            "--ephemeral",
            "--ignore-user-config",
            "--skip-git-repo-check",
            "--json",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.group_spawn().map_err(|_| "native_start_failed")?;
    NATIVE_STARTS.fetch_add(1, Ordering::Relaxed);
    let input = child
        .inner()
        .stdin
        .take()
        .ok_or("native_input_failed")
        .and_then(|mut stdin| {
            stdin
                .write_all(prompt.as_bytes())
                .map_err(|_| "native_input_failed")
        });
    if let Err(error) = input {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let Some(stdout) = child.inner().stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("native_output_failed");
    };
    let (send, receive) = mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut frame = Vec::new();
            let result = reader
                .by_ref()
                .take(MAX_FRAME + 1)
                .read_until(b'\n', &mut frame);
            match result {
                Ok(0) => break,
                Ok(_) if frame.len() as u64 <= MAX_FRAME => {
                    if send
                        .send(
                            serde_json::from_slice::<Value>(&frame)
                                .map_err(|_| "native_invalid_event"),
                        )
                        .is_err()
                    {
                        break;
                    }
                }
                _ => {
                    let _ = send.send(Err("native_event_budget"));
                    break;
                }
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(args.timeout_seconds);
    let mut observed = Observed::default();
    let mut bytes = 0usize;
    let mut failure = None;
    loop {
        if Instant::now() >= deadline {
            failure = Some("native_timeout");
            break;
        }
        match receive.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                bytes = bytes.saturating_add(event.to_string().len());
                if bytes > MAX_STREAM {
                    failure = Some("native_stream_budget");
                    break;
                }
                observed.event(&event, entry, guidance);
            }
            Ok(Err(error)) => {
                failure = Some(error);
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if let Some(error) = failure {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| "native_wait_failed")? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("native_timeout");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Ok((observed, status.success()))
}

fn verdict(
    case: Case,
    observed: &Observed,
    success: bool,
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> Value {
    let changed: BTreeSet<_> = before
        .keys()
        .chain(after.keys())
        .filter(|key| before.get(*key) != after.get(*key))
        .cloned()
        .collect();
    let trace_complete = success
        && observed.thread_count == 1
        && observed.completed_turns == 1
        && !observed.failed_turn
        && observed.pending.is_empty()
        && !observed.unsupported;
    let (functional, boundary, exposure) = match case {
        Case::ReportReview => (
            serde_json::from_str::<Value>(&observed.final_text).ok()
                == Some(json!({"a_total":701,"a_failed":17,"b_total":903,"b_failed":29})),
            changed.is_empty() && observed.tool_bytes <= OUTPUT_BUDGET && !observed.truncated,
            observed.entry_seen && observed.guidance_seen,
        ),
        Case::OrdinaryFix => (
            after.get("message.txt") == Some(&hash(b"Hello GroundLine\n"))
                && observed.verification_observed,
            changed == BTreeSet::from(["message.txt".to_owned()]),
            !observed.entry_seen && !observed.guidance_seen,
        ),
        Case::VerificationReuse => (
            after.get("README.txt") == Some(&hash(b"A small fixture.\n")),
            changed == BTreeSet::from(["README.txt".to_owned()])
                && after.get("runs.txt") == Some(&hash(b"1\n")),
            observed.entry_seen && observed.guidance_seen,
        ),
    };
    let behavior_status = if !trace_complete || observed.command_count == 0 {
        "INCONCLUSIVE"
    } else if functional && boundary {
        "PASS"
    } else {
        "FAIL"
    };
    let status = if behavior_status == "PASS" && !exposure {
        "INCONCLUSIVE"
    } else {
        behavior_status
    };
    json!({"status":status,"behavior_status":behavior_status,"case":format!("{case:?}"),"trace_complete":trace_complete,"unsupported_types":observed.unsupported_types,"functional_oracle":functional,"boundary_oracle":boundary,"invocation_condition_satisfied":exposure,"entry_read_observed":observed.entry_seen,"reference_read_observed":observed.guidance_seen,"native_model_input_projection_observed":false,"verification_command_observed":observed.verification_observed,"command_count":observed.command_count,"failed_command_count":observed.failed_commands,"tool_output_bytes":observed.tool_bytes,"truncation_observed":observed.truncated,"usage":observed.usage})
}

fn execute(args: &Args) -> Result<Value, &'static str> {
    if !args.run_native {
        return Err("explicit_run_native_required");
    }
    let metadata = fs::symlink_metadata(&args.work).map_err(|_| "private_work_required")?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err("private_work_required");
    }
    if args.report.exists() {
        return Err("new_report_required");
    }
    let workspace = tempfile::Builder::new()
        .prefix("guidance-fixture-")
        .tempdir_in(&args.work)
        .map_err(|_| "fixture_write_failed")?;
    let root = workspace.path().join("project");
    fs::create_dir(&root).map_err(|_| "fixture_write_failed")?;
    for name in ["skills", "references"] {
        copy_tree(
            &args.plugin_root.join(name),
            &root.join(".agents").join(name),
        )?;
    }
    let entry = fs::read_to_string(root.join(".agents/skills/optimize-codex-workflow/SKILL.md"))
        .map_err(|_| "guidance_unavailable")?;
    let guidance = fs::read_to_string(root.join(".agents/references/task-continuity.md"))
        .map_err(|_| "guidance_unavailable")?;
    isolation(args, &root, &workspace.path().join("excluded"))?;
    let prompt = fixture(&root, args.case)?;
    let before = snapshot(&root)?;
    let native_binary = file_hash(&args.codex)?;
    let evaluator_binary = file_hash(&std::env::current_exe().map_err(|_| "binary_unavailable")?)?;
    let (observed, success) = run_model(args, &root, &prompt, &entry, &guidance)?;
    let after = snapshot(&root)?;
    let mut result = verdict(args.case, &observed, success, &before, &after);
    result["kind"] = json!("groundline-native-guidance-fixture");
    result["schema"] = json!(1);
    result["native_binary_sha256"] = json!(native_binary);
    result["evaluator_binary_sha256"] = json!(evaluator_binary);
    result["fixture_before_sha256"] = json!(hash(&serde_json::to_vec(&before).unwrap()));
    result["fixture_after_sha256"] = json!(hash(&serde_json::to_vec(&after).unwrap()));
    result["reference_sha256"] = json!(hash(guidance.as_bytes()));
    result["entry_sha256"] = json!(hash(entry.as_bytes()));
    result["selection_requested"] =
        json!({"model":args.model,"effort":args.effort,"service_tier":args.service_tier});
    result["selection_observed"] = Value::Null;
    result["isolation_probes"] = json!("PASS");
    result["ephemeral"] = json!(true);
    result["raw_events_saved"] = json!(false);
    result["natural_followup"] = json!(false);
    result["causal_effect_established"] = json!(false);
    Ok(result)
}

fn main() {
    let args = Args::parse();
    let safe_destination = args
        .work
        .canonicalize()
        .ok()
        .zip(args.report.parent().and_then(|p| p.canonicalize().ok()))
        .is_some_and(|(work, parent)| work == parent)
        && fs::symlink_metadata(&args.work).is_ok_and(|m| {
            m.is_dir() && !m.file_type().is_symlink() && m.permissions().mode() & 0o077 == 0
        })
        && fs::File::open(&args.work)
            .is_ok_and(|f| groundline_runtime::local_file::owned_by_current_user(&f));
    if !safe_destination {
        eprintln!("private_report_destination_required");
        std::process::exit(2);
    }
    let mut result = match execute(&args) {
        Ok(result) => result,
        Err(reason) => {
            json!({"kind":"groundline-native-guidance-fixture","schema":1,"status":"UNVERIFIED","reason":reason,"raw_events_saved":false,"causal_effect_established":false})
        }
    };
    result["native_process_starts"] = json!(NATIVE_STARTS.load(Ordering::Relaxed));
    let saved = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&args.report)
        .and_then(|mut file| writeln!(file, "{}", serde_json::to_string_pretty(&result).unwrap()));
    if saved.is_err() {
        eprintln!("derived_report_write_failed");
        std::process::exit(2);
    }
    println!("{}", serde_json::to_string(&result).unwrap());
    if result["status"] != "PASS" {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete() -> Observed {
        Observed {
            thread_count: 1,
            completed_turns: 1,
            command_count: 1,
            entry_seen: true,
            guidance_seen: true,
            ..Observed::default()
        }
    }

    #[test]
    fn claims_without_artifact_oracle_do_not_pass() {
        let observed = complete();
        assert_eq!(
            verdict(
                Case::VerificationReuse,
                &observed,
                true,
                &BTreeMap::new(),
                &BTreeMap::new()
            )["status"],
            "FAIL"
        );
    }

    #[test]
    fn interrupted_or_unknown_trace_cannot_pass() {
        let mut observed = complete();
        observed.final_text =
            json!({"a_total":701,"a_failed":17,"b_total":903,"b_failed":29}).to_string();
        observed.completed_turns = 0;
        assert_eq!(
            verdict(
                Case::ReportReview,
                &observed,
                true,
                &BTreeMap::new(),
                &BTreeMap::new()
            )["status"],
            "INCONCLUSIVE"
        );
        observed.completed_turns = 1;
        observed.unsupported = true;
        assert_eq!(
            verdict(
                Case::ReportReview,
                &observed,
                true,
                &BTreeMap::new(),
                &BTreeMap::new()
            )["status"],
            "INCONCLUSIVE"
        );
    }

    #[test]
    fn clipped_or_oversized_correct_answer_still_fails_boundary() {
        let mut observed = complete();
        observed.final_text =
            json!({"a_total":701,"a_failed":17,"b_total":903,"b_failed":29}).to_string();
        observed.tool_bytes = OUTPUT_BUDGET + 1;
        assert_eq!(
            verdict(
                Case::ReportReview,
                &observed,
                true,
                &BTreeMap::new(),
                &BTreeMap::new()
            )["status"],
            "FAIL"
        );
        observed.tool_bytes = 0;
        observed.truncated = true;
        assert_eq!(
            verdict(
                Case::ReportReview,
                &observed,
                true,
                &BTreeMap::new(),
                &BTreeMap::new()
            )["status"],
            "FAIL"
        );
    }

    #[test]
    fn changed_counter_rejects_unnecessary_rerun() {
        let before = BTreeMap::from([
            ("README.txt".into(), hash(b"A smal fixture.\n")),
            ("runs.txt".into(), hash(b"1\n")),
        ]);
        let mut after = before.clone();
        after.insert("README.txt".into(), hash(b"A small fixture.\n"));
        assert_eq!(
            verdict(Case::VerificationReuse, &complete(), true, &before, &after)["status"],
            "PASS"
        );
        after.insert("runs.txt".into(), hash(b"2\n"));
        assert_eq!(
            verdict(Case::VerificationReuse, &complete(), true, &before, &after)["status"],
            "FAIL"
        );
    }

    #[test]
    fn event_exit_failure_and_duplicate_completion_are_retained() {
        let mut observed = Observed::default();
        let event = json!({"type":"item.completed","item":{"id":"x","type":"command_execution","status":"completed","exit_code":7,"aggregated_output":"failed"}});
        observed.event(&event, "entry", "reference");
        assert_eq!(observed.failed_commands, 1);
        observed.event(&event, "entry", "reference");
        assert!(observed.unsupported);
    }

    #[test]
    fn document_name_alone_does_not_prove_read() {
        assert!(!document_seen(
            "read task-continuity.md",
            "# Guidance\nThe actual rule."
        ));
        assert!(document_seen(
            "1: # Guidance\n2: The actual rule.",
            "# Guidance\nThe actual rule."
        ));
    }

    #[test]
    fn named_command_in_echo_or_compound_is_not_verification() {
        let expected = ["/usr/bin/grep", "-qx", "Hello GroundLine", "message.txt"];
        assert!(exact_command(
            "/usr/bin/grep -qx 'Hello GroundLine' message.txt",
            &expected
        ));
        assert!(exact_command(
            "/bin/zsh -lc \"/usr/bin/grep -qx 'Hello GroundLine' message.txt\"",
            &expected
        ));
        assert!(!exact_command(
            "echo /usr/bin/grep -qx 'Hello GroundLine' message.txt",
            &expected
        ));
        assert!(!exact_command(
            "/usr/bin/grep -qx 'Hello GroundLine' message.txt; true",
            &expected
        ));
    }

    #[test]
    fn a_correct_edit_without_requested_verification_fails() {
        let before = BTreeMap::from([("message.txt".into(), hash(b"Helo GroundLine\n"))]);
        let after = BTreeMap::from([("message.txt".into(), hash(b"Hello GroundLine\n"))]);
        let mut observed = Observed {
            entry_seen: false,
            guidance_seen: false,
            ..complete()
        };
        assert_eq!(
            verdict(Case::OrdinaryFix, &observed, true, &before, &after)["status"],
            "FAIL"
        );
        observed.verification_observed = true;
        assert_eq!(
            verdict(Case::OrdinaryFix, &observed, true, &before, &after)["status"],
            "PASS"
        );
    }

    #[test]
    fn missing_exposure_does_not_erase_observed_behavior() {
        let mut observed = complete();
        observed.entry_seen = false;
        observed.guidance_seen = false;
        observed.final_text =
            json!({"a_total":701,"a_failed":17,"b_total":903,"b_failed":29}).to_string();
        let result = verdict(
            Case::ReportReview,
            &observed,
            true,
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert_eq!(result["behavior_status"], "PASS");
        assert_eq!(result["status"], "INCONCLUSIVE");
        observed.tool_bytes = OUTPUT_BUDGET + 1;
        let result = verdict(
            Case::ReportReview,
            &observed,
            true,
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert_eq!(result["behavior_status"], "FAIL");
        assert_eq!(result["status"], "FAIL");
    }
}
