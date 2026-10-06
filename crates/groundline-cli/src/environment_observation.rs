//! Bounded, owner-local native metadata reads. Discovery is not execution evidence.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use groundline_contracts::ContractError;
use groundline_runtime::local_file::{open_bounded_regular_file, owned_by_current_user};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const CALL_TIMEOUT: Duration = Duration::from_secs(5);
const VERSION_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;
const MAX_STREAM_BYTES: usize = 8 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 512 * 1024;
const METHODS: &[&str] = &["initialize", "config/read", "skills/list", "hooks/list"];

#[derive(Clone, Copy)]
struct Failure {
    reason: &'static str,
    code: Option<i64>,
}

impl Failure {
    fn new(reason: &'static str) -> Self {
        Self { reason, code: None }
    }

    fn value(self) -> Value {
        json!({"reason": self.reason, "native_error_code": self.code})
    }
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn path_fingerprint(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    fingerprint(path.as_os_str().as_bytes())
}

fn file_hash(path: &Path) -> Option<String> {
    let mut file = open_bounded_regular_file(path, 0, MAX_FILE_BYTES).ok()?;
    if !owned_by_current_user(&file) {
        return None;
    }
    let mut data = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut data)
        .ok()?;
    (data.len() as u64 <= MAX_FILE_BYTES).then(|| fingerprint(&data))
}

fn bound_path(native: &str, expected: &Path) -> bool {
    let path = Path::new(native);
    path.is_absolute()
        && path.canonicalize().ok().as_deref() == expected.canonicalize().ok().as_deref()
        && expected.canonicalize().is_ok()
}

fn nonblocking(pipe: &impl std::os::fd::AsFd) -> Result<(), Failure> {
    use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
    let flags = fcntl_getfl(pipe).map_err(|_| Failure::new("pipe_setup_failed"))?;
    fcntl_setfl(pipe, flags | OFlags::NONBLOCK).map_err(|_| Failure::new("pipe_setup_failed"))
}

fn command(cli: &Path, home: &Path, cwd: &Path) -> Command {
    use std::os::unix::process::CommandExt;
    let mut command = Command::new(cli);
    command.env_clear();
    // Do not forward API keys, proxy credentials, or token variables.
    for key in [
        "PATH", "HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "LC_ALL", "TERM",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env("CODEX_HOME", home)
        .current_dir(cwd)
        .process_group(0);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

fn terminate(child: &mut Child) -> bool {
    use rustix::process::{Pid, Signal, kill_process_group};
    // The unreaped child owns this newly created process group.
    if let Some(pid) = i32::try_from(child.id()).ok().and_then(Pid::from_raw) {
        let _ = kill_process_group(pid, Signal::TERM);
        std::thread::sleep(Duration::from_millis(50));
        let _ = kill_process_group(pid, Signal::KILL);
    }
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Err(_) => return false,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(None) => return false,
        }
    }
}

struct NativeReader {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
    pending: Vec<u8>,
    bytes: usize,
    ident: u64,
    warnings: usize,
    activity: bool,
    failed: bool,
    closed: bool,
}

impl NativeReader {
    fn start(cli: &Path, home: &Path, cwd: &Path) -> Result<Self, Failure> {
        let mut child = command(cli, home, cwd)
            .args(["app-server", "--strict-config", "--listen", "stdio://"])
            .spawn()
            .map_err(|_| Failure::new("spawn_failed"))?;
        let input = child.stdin.take().expect("piped stdin");
        let output = child.stdout.take().expect("piped stdout");
        if let Err(error) = nonblocking(&input).and_then(|()| nonblocking(&output)) {
            terminate(&mut child);
            return Err(error);
        }
        Ok(Self {
            child,
            input: Some(input),
            output,
            pending: Vec::new(),
            bytes: 0,
            ident: 0,
            warnings: 0,
            activity: false,
            failed: false,
            closed: false,
        })
    }

    fn write(&mut self, value: &Value, deadline: Instant) -> Result<(), Failure> {
        let mut bytes =
            serde_json::to_vec(value).map_err(|_| Failure::new("request_encoding_failed"))?;
        bytes.push(b'\n');
        let mut offset = 0;
        while offset < bytes.len() {
            if Instant::now() >= deadline {
                return Err(Failure::new("timeout"));
            }
            match self
                .input
                .as_mut()
                .ok_or(Failure::new("transport_closed"))?
                .write(&bytes[offset..])
            {
                Ok(0) => return Err(Failure::new("transport_closed")),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return Err(Failure::new("transport_write_failed")),
            }
        }
        Ok(())
    }

    fn next(&mut self, deadline: Instant) -> Result<Value, Failure> {
        loop {
            if Instant::now() >= deadline {
                return Err(Failure::new("timeout"));
            }
            if let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
                if end > MAX_FRAME_BYTES {
                    return Err(Failure::new("frame_limit"));
                }
                let frame: Vec<_> = self.pending.drain(..=end).collect();
                return serde_json::from_slice(&frame)
                    .map_err(|_| Failure::new("invalid_json_frame"));
            }
            if self.pending.len() > MAX_FRAME_BYTES {
                return Err(Failure::new("frame_limit"));
            }
            let mut data = [0; 8192];
            match self.output.read(&mut data) {
                Ok(0) => return Err(Failure::new("transport_eof")),
                Ok(count) => {
                    self.bytes += count;
                    if self.bytes > MAX_STREAM_BYTES {
                        return Err(Failure::new("stream_limit"));
                    }
                    self.pending.extend_from_slice(&data[..count]);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return Err(Failure::new("transport_read_failed")),
            }
        }
    }

    fn notification(&mut self, value: &Value) -> Result<(), Failure> {
        match value.get("method").and_then(Value::as_str) {
            Some(
                "thread/started"
                | "turn/started"
                | "thread/tokenUsage/updated"
                | "hook/started"
                | "hook/completed",
            ) => {
                self.activity = true;
                Err(Failure::new("unexpected_execution_activity"))
            }
            Some("configWarning") => {
                self.warnings += 1;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn rpc(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value, Failure> {
        if !METHODS.contains(&method) {
            return Err(Failure::new("method_not_allowed"));
        }
        if self.failed || self.closed {
            return Err(Failure::new("transport_unavailable"));
        }
        self.ident += 1;
        let id = self.ident;
        let deadline = Instant::now() + timeout;
        let result = (|| {
            self.write(
                &json!({"id": id, "method": method, "params": params}),
                deadline,
            )?;
            loop {
                let value = self.next(deadline)?;
                if !value.is_object() {
                    return Err(Failure::new("invalid_response"));
                }
                self.notification(&value)?;
                if value.get("method").is_some() && value.get("id").is_some() {
                    return Err(Failure::new("unexpected_server_request"));
                }
                if value.get("id").and_then(Value::as_u64) != Some(id) {
                    continue;
                }
                if let Some(error) = value.get("error") {
                    return Err(Failure {
                        reason: "native_rpc_error",
                        code: error.get("code").and_then(Value::as_i64),
                    });
                }
                return value
                    .get("result")
                    .cloned()
                    .ok_or(Failure::new("missing_result"));
            }
        })();
        if let Err(error) = result
            && error.reason != "native_rpc_error"
        {
            self.failed = true;
        }
        result
    }

    fn close(&mut self) -> bool {
        self.input.take();
        let terminal = if self.closed {
            false
        } else {
            terminate(&mut self.child)
        };
        self.closed = true;
        // Inspect late notifications as well; never retain their payloads.
        let deadline = Instant::now() + Duration::from_millis(50);
        while let Ok(value) = self.next(deadline) {
            let _ = self.notification(&value);
        }
        terminal
    }
}

impl Drop for NativeReader {
    fn drop(&mut self) {
        if !self.closed {
            self.close();
        }
    }
}

fn version(cli: &Path, home: &Path, cwd: &Path, timeout: Duration) -> Value {
    let Ok(mut child) = command(cli, home, cwd).arg("--version").spawn() else {
        return json!({"value": null, "error": "spawn_failed"});
    };
    child.stdin.take();
    let mut output = child.stdout.take().expect("piped stdout");
    let mut data = Vec::new();
    let mut exited = false;
    let result = (|| {
        nonblocking(&output)?;
        let deadline = Instant::now() + timeout;
        loop {
            if Instant::now() >= deadline {
                return Err(Failure::new("timeout"));
            }
            let mut bytes = [0; 512];
            match output.read(&mut bytes) {
                Ok(0) => break,
                Ok(count) => {
                    data.extend_from_slice(&bytes[..count]);
                    if data.len() > 512 {
                        return Err(Failure::new("version_output_limit"));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(_) => return Err(Failure::new("transport_read_failed")),
            }
        }
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    exited = true;
                    if !status.success() {
                        return Err(Failure::new("version_command_failed"));
                    }
                    break;
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                _ => return Err(Failure::new("version_exit_unverified")),
            }
        }
        let text =
            std::str::from_utf8(&data).map_err(|_| Failure::new("version_format_unknown"))?;
        let words: Vec<_> = text.split_whitespace().collect();
        if words.len() != 2 || words[0] != "codex-cli" {
            return Err(Failure::new("version_format_unknown"));
        }
        let parts: Vec<_> = words[1].split('.').collect();
        if parts.len() != 3
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(Failure::new("version_format_unknown"));
        }
        Ok(words[1].to_owned())
    })();
    let terminated = exited || terminate(&mut child);
    match result {
        Ok(value) => json!({"value": value, "error": null, "process_terminated": terminated}),
        Err(error) => {
            json!({"value": null, "error": error.value(), "process_terminated": terminated})
        }
    }
}

fn read_result(
    result: Result<Value, Failure>,
    select: impl FnOnce(&Value) -> Option<Value>,
) -> Value {
    match result {
        Ok(value) => match select(&value) {
            Some(selected) => {
                json!({"supported": true, "discovery_status":"observed", "observed": selected, "error": null})
            }
            None => {
                json!({"supported": null, "discovery_status":"unknown", "observed": null, "error": {"reason":"response_schema_unknown"}})
            }
        },
        Err(error) => {
            json!({"supported": if error.code == Some(-32601) { Some(false) } else { None },
            "discovery_status":if error.code == Some(-32601) { "unsupported" } else { "error" },
            "observed": null, "error": error.value()})
        }
    }
}

fn config_observation(value: &Value) -> Option<Value> {
    let config = value.get("config")?.as_object()?;
    value.get("origins")?.as_object()?;
    let selected: BTreeMap<_, _> = [
        "model",
        "model_provider",
        "model_reasoning_effort",
        "approval_policy",
        "approvals_reviewer",
        "sandbox_mode",
    ]
    .into_iter()
    .filter_map(|key| config.get(key).map(|value| (key, value)))
    .collect();
    Some(
        json!({"selection_sha256": fingerprint(&serde_json::to_vec(&selected).ok()?),
        "selection_keys_observed": selected.keys().collect::<Vec<_>>(),
        "runtime_effect_verified": false}),
    )
}

fn skill_observation(value: &Value, cwd: &Path, targets: &[PathBuf]) -> Option<Value> {
    let entries = value.get("data")?.as_array()?;
    if entries.len() != 1 || !bound_path(entries[0].get("cwd")?.as_str()?, cwd) {
        return None;
    }
    let entry = &entries[0];
    let errors = entry.get("errors")?.as_array()?;
    let skills = entry.get("skills")?.as_array()?;
    if skills.len() > 4096 {
        return None;
    }
    for skill in skills {
        skill.get("name")?.as_str()?;
        skill.get("description")?.as_str()?;
        skill.get("path")?.as_str()?;
        skill.get("enabled")?.as_bool()?;
        if !["user", "repo", "system", "admin"].contains(&skill.get("scope")?.as_str()?) {
            return None;
        }
    }
    let target_observations: Vec<_> = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let matches: Vec<_> = skills
                .iter()
                .filter(|skill| bound_path(skill["path"].as_str().unwrap_or(""), target))
                .collect();
            let unambiguous =
                matches.len() == 1 && errors.is_empty() && file_hash(target).is_some();
            json!({"target_index": index, "matches": matches.len(),
            "discovered": !matches.is_empty(),
            "enabled": if unambiguous { matches[0]["enabled"].as_bool() } else { None },
            "scope": if unambiguous { matches[0]["scope"].as_str() } else { None },
            "execution_effect_verified": false})
        })
        .collect();
    Some(
        json!({"cwd_bound": true, "discovery_error_count": errors.len(), "skill_count": skills.len(),
        "targets": target_observations, "existing_chat_reload_verified": false}),
    )
}

fn hook_observation(value: &Value, cwd: &Path, targets: &[PathBuf]) -> Option<Value> {
    let entries = value.get("data")?.as_array()?;
    if entries.len() != 1 || !bound_path(entries[0].get("cwd")?.as_str()?, cwd) {
        return None;
    }
    let entry = &entries[0];
    let errors = entry.get("errors")?.as_array()?;
    let warnings = entry.get("warnings")?.as_array()?;
    let hooks = entry.get("hooks")?.as_array()?;
    if hooks.len() > 4096 {
        return None;
    }
    let mut trust = BTreeMap::<&str, usize>::new();
    let mut enabled_count = 0;
    for hook in hooks {
        let status = hook.get("trustStatus")?.as_str()?;
        if !["managed", "untrusted", "trusted", "modified"].contains(&status) {
            return None;
        }
        *trust.entry(status).or_default() += 1;
        enabled_count += usize::from(hook.get("enabled")?.as_bool()?);
        hook.get("sourcePath")?.as_str()?;
        hook.get("currentHash")?.as_str()?;
        hook.get("key")?.as_str()?;
        hook.get("displayOrder")?.as_i64()?;
        hook.get("timeoutSec")?.as_u64()?;
        if ![
            "system",
            "user",
            "project",
            "mdm",
            "sessionFlags",
            "plugin",
            "cloudRequirements",
            "cloudManagedConfig",
            "legacyManagedConfigFile",
            "legacyManagedConfigMdm",
            "unknown",
        ]
        .contains(&hook.get("source")?.as_str()?)
        {
            return None;
        }
        if ![
            "preToolUse",
            "permissionRequest",
            "postToolUse",
            "preCompact",
            "postCompact",
            "sessionStart",
            "sessionEnd",
            "userPromptSubmit",
            "subagentStart",
            "subagentStop",
            "stop",
            "interrupt",
        ]
        .contains(&hook.get("eventName")?.as_str()?)
        {
            return None;
        }
        hook.get("isManaged")?.as_bool()?;
        match hook.get("handlerType")?.as_str()? {
            "command" => {
                hook.get("command")?.as_str()?;
            }
            "mcpTool" => {
                hook.get("server")?.as_str()?;
                hook.get("tool")?.as_str()?;
            }
            "prompt" | "agent" => (),
            _ => return None,
        }
    }
    let target_observations: Vec<_> = targets.iter().enumerate().map(|(index, target)| {
        let matches: Vec<_> = hooks.iter().filter(|hook| bound_path(hook["sourcePath"].as_str().unwrap_or(""), target)).map(|hook| {
            let reliable = errors.is_empty() && file_hash(target).is_some();
            json!({"enabled": if reliable { hook["enabled"].as_bool() } else { None },
                "trust_status": if reliable { hook["trustStatus"].as_str() } else { None },
                "native_hash_fingerprint": fingerprint(hook["currentHash"].as_str().unwrap_or("").as_bytes()),
                "execution_effect_verified": false})
        }).collect();
        json!({"target_index": index, "matches": matches})
    }).collect();
    Some(
        json!({"cwd_bound": true, "discovery_error_count": errors.len(), "warning_count": warnings.len(),
        "hook_count": hooks.len(), "enabled_metadata_count": enabled_count, "trust_metadata_counts": trust,
        "targets": target_observations, "execution_effect_verified": false}),
    )
}

fn lane(cli: &Path, home: &Path, cwd: &Path, targets: &[PathBuf], timeout: Duration) -> Value {
    let mut result = json!({"entrypoint_path_fingerprint": cli.canonicalize().ok().map(|path| path_fingerprint(&path)),
        "version": version(cli, home, cwd, timeout.min(VERSION_TIMEOUT)),
        "initialize": null, "config": null, "skills": null, "hooks": null,
        "process_terminated": null, "execution_effect_verified": false});
    let mut client = match NativeReader::start(cli, home, cwd) {
        Ok(client) => client,
        Err(error) => {
            result["initialize"] = json!({"error": error.value()});
            return result;
        }
    };
    let initialized = client.rpc(
        "initialize",
        json!({"clientInfo":{"name":"groundline_environment_observation","version":"1"},
        "capabilities":{"experimentalApi":false}}),
        timeout,
    );
    let home_bound = initialized.as_ref().ok().is_some_and(|value| {
        value["codexHome"]
            .as_str()
            .is_some_and(|path| bound_path(path, home))
            && value["platformFamily"] == "unix"
            && matches!(value["platformOs"].as_str(), Some("macos" | "linux"))
            && value["userAgent"].is_string()
    });
    result["initialize"] = match initialized {
        Ok(_) => {
            json!({"home_bound": home_bound, "error": if home_bound { None } else { Some("home_binding_unverified") }})
        }
        Err(error) => json!({"home_bound": false, "error": error.value()}),
    };
    if home_bound {
        let notification = client.write(&json!({"method":"initialized"}), Instant::now() + timeout);
        if notification.is_ok() {
            result["config"] = read_result(
                client.rpc(
                    "config/read",
                    json!({"cwd":cwd,"includeLayers":false}),
                    timeout,
                ),
                config_observation,
            );
            result["skills"] = read_result(
                client.rpc(
                    "skills/list",
                    json!({"cwds":[cwd],"forceReload":false}),
                    timeout,
                ),
                |value| skill_observation(value, cwd, targets),
            );
            result["hooks"] = read_result(
                client.rpc("hooks/list", json!({"cwds":[cwd]}), timeout),
                |value| hook_observation(value, cwd, targets),
            );
        } else {
            result["initialize"]["error"] = json!("initialized_notification_failed");
        }
    }
    result["process_terminated"] = json!(client.close());
    result["native_warning_count"] = json!(client.warnings);
    result["unexpected_execution_activity_observed"] = json!(client.activity);
    result
}

/// The caller must save this fingerprint-only snapshot in owner-private state.
/// Native list metadata never establishes existing-chat reload or actual effects.
pub(crate) fn inspect(
    app_cli: &Path,
    path_cli: &Path,
    codex_home: &Path,
    targets: &[PathBuf],
) -> Result<Value, ContractError> {
    if targets.len() > 64 {
        return Err(ContractError("environment_observation_target_limit".into()));
    }
    let home = codex_home
        .canonicalize()
        .map_err(|_| ContractError("environment_observation_home_unavailable".into()))?;
    let home_file = File::open(&home)
        .map_err(|_| ContractError("environment_observation_home_unavailable".into()))?;
    if !home_file.metadata().is_ok_and(|metadata| metadata.is_dir())
        || !owned_by_current_user(&home_file)
    {
        return Err(ContractError(
            "environment_observation_owner_directory_required".into(),
        ));
    }
    let cwd = std::env::current_dir()
        .and_then(|path| path.canonicalize())
        .map_err(|_| ContractError("environment_observation_cwd_unavailable".into()))?;
    let config = home.join("config.toml");
    let before = file_hash(&config);
    let target_before: Vec<_> = targets.iter().map(|path| file_hash(path)).collect();
    let app = lane(app_cli, &home, &cwd, targets, CALL_TIMEOUT);
    let path = lane(path_cli, &home, &cwd, targets, CALL_TIMEOUT);
    let after = file_hash(&config);
    let disk: Vec<_> = targets.iter().enumerate().map(|(index, target)| {
        let after = file_hash(target);
        json!({"target_index":index, "path_fingerprint":path_fingerprint(target), "sha256":target_before[index],
            "regular_owner_file_observed":target_before[index].is_some(),
            "unchanged":if target_before[index].is_some() && after.is_some() { Some(target_before[index] == after) } else { None },
            "activation_status":"unknown", "native_activation_verified":false})
    }).collect();
    let complete = lane_complete(&app)
        && lane_complete(&path)
        && before.is_some()
        && before == after
        && disk.iter().all(|target| target["unchanged"] == true);
    Ok(
        json!({"kind":"groundline-environment-observation","schema":1,
        "status":if complete { "PASS" } else { "PARTIAL" }, "observed_at":chrono::Utc::now().to_rfc3339(),
        "visibility":"owner_private","home_fingerprint":path_fingerprint(&home),"cwd_fingerprint":path_fingerprint(&cwd),
        "per_call_timeout_ms":CALL_TIMEOUT.as_millis(),"native_methods_allowed":METHODS,
        "config_file":{"sha256":before,"unchanged":if before.is_some() && after.is_some() { Some(before == after) } else { None }},
        "disk_targets":disk,"app_cli":app,"path_cli":path,
        "existing_chat_reload_verified":false,"execution_effect_verified":false,
        "model_calls_requested":0,"native_mutations_requested":0,
        "network_performed":false,"mutation_performed":false,
        "raw_content_emitted":false,"private_paths_emitted":false,
        "operation_boundary":"stdio_metadata_and_disk_reads",
        "native_background_network_verified":false,"native_background_state_mutations_verified":false}),
    )
}

fn lane_complete(value: &Value) -> bool {
    value["version"]["value"].is_string()
        && value["version"]["process_terminated"] == true
        && value["initialize"]["home_bound"] == true
        && value["initialize"]["error"].is_null()
        && value["process_terminated"] == true
        && value["unexpected_execution_activity_observed"] == false
        && value["config"]["supported"] == true
        && value["skills"]["supported"] == true
        && value["skills"]["observed"]["discovery_error_count"] == 0
        && (value["hooks"]["discovery_status"] == "unsupported"
            || (value["hooks"]["supported"] == true
                && value["hooks"]["observed"]["discovery_error_count"] == 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use tempfile::TempDir;

    fn quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    fn mock_cli(root: &Path, home: &Path, cwd: &Path, target: &Path, hook_error: bool) -> PathBuf {
        let path = root.join("mock-codex");
        let initialization = json!({"id":1,"result":{"codexHome":home,"platformFamily":"unix","platformOs":"macos","userAgent":"private-user-agent"}});
        let config = json!({"id":2,"result":{"config":{"model":"private-model","instructions":"PRIVATE_CONFIGURATION"},"origins":{}}});
        let skills = json!({"id":3,"result":{"data":[{"cwd":cwd,"errors":[],"skills":[{"path":target,"name":"PRIVATE_SKILL_NAME","description":"PRIVATE_DESCRIPTION","scope":"user","enabled":true}]}]}});
        let hooks = if hook_error {
            json!({"id":4,"error":{"code":-32601,"message":"PRIVATE_NATIVE_ERROR"}})
        } else {
            json!({"id":4,"result":hook_fixture(cwd, target)})
        };
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then printf '%s\\n' 'codex-cli 0.159.2'; exit 0; fi\nwhile IFS= read -r request; do\n printf '%s\\n' \"$request\" >> {}\n case \"$request\" in\n *'\"method\":\"initialize\"'*) printf '%s\\n' {};;\n *'\"method\":\"config/read\"'*) printf '%s\\n' {};;\n *'\"method\":\"skills/list\"'*) printf '%s\\n' {};;\n *'\"method\":\"hooks/list\"'*) printf '%s\\n' {};;\n esac\ndone\n",
            quote(&root.join("requests").to_string_lossy()),
            quote(&initialization.to_string()),
            quote(&config.to_string()),
            quote(&skills.to_string()),
            quote(&hooks.to_string())
        );
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn hook_fixture(cwd: &Path, target: &Path) -> Value {
        json!({"data":[{"cwd":cwd,"errors":[],"warnings":[],"hooks":[{
            "key":"PRIVATE_HOOK_KEY","sourcePath":target,"source":"user","eventName":"sessionStart",
            "currentHash":"PRIVATE_NATIVE_HASH","trustStatus":"trusted","enabled":true,"isManaged":false,
            "displayOrder":0,"timeoutSec":10,"handlerType":"command","command":"PRIVATE_HOOK_COMMAND"
        }]}]})
    }

    fn setup() -> (TempDir, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("owned-home");
        std::fs::create_dir(&home).unwrap();
        std::fs::write(
            home.join("config.toml"),
            "private = 'PRIVATE_CONFIG_FILE'\n",
        )
        .unwrap();
        let target = root.path().join("SKILL.md");
        std::fs::write(&target, "PRIVATE_SKILL_CONTENT").unwrap();
        (root, home, target)
    }

    #[test]
    fn environment_observation_distinguishes_disk_native_metadata_and_execution() {
        let (root, home, target) = setup();
        let cwd = std::env::current_dir().unwrap();
        let cli = mock_cli(root.path(), &home, &cwd, &target, false);
        let before = file_hash(&home.join("config.toml"));
        let snapshot = inspect(&cli, &cli, &home, std::slice::from_ref(&target)).unwrap();
        for lane in ["app_cli", "path_cli"] {
            assert_eq!(snapshot[lane]["version"]["value"], "0.159.2");
            assert_eq!(
                snapshot[lane]["skills"]["observed"]["targets"][0]["enabled"],
                true
            );
            assert_eq!(
                snapshot[lane]["hooks"]["observed"]["trust_metadata_counts"]["trusted"],
                1
            );
            assert_eq!(snapshot[lane]["process_terminated"], true);
            assert_eq!(snapshot[lane]["execution_effect_verified"], false);
        }
        assert_eq!(snapshot["disk_targets"][0]["activation_status"], "unknown");
        assert_eq!(snapshot["config_file"]["unchanged"], true);
        assert_eq!(before, file_hash(&home.join("config.toml")));
        let text = snapshot.to_string();
        assert!(!text.contains(&root.path().to_string_lossy().to_string()));
        assert!(!text.contains("PRIVATE_"));
        assert!(!text.contains("private-model"));
        let requests = std::fs::read_to_string(root.path().join("requests")).unwrap();
        assert!(!requests.contains("turn/start"));
        assert!(!requests.contains("thread/start"));
        assert!(!requests.contains("account/"));
        assert!(!requests.contains("forceReload\":true"));
    }

    #[test]
    fn environment_observation_app_failure_does_not_hide_path_success() {
        let (root, home, target) = setup();
        let cli = mock_cli(
            root.path(),
            &home,
            &std::env::current_dir().unwrap(),
            &target,
            true,
        );
        let result = inspect(&root.path().join("missing"), &cli, &home, &[target]).unwrap();
        assert_eq!(
            result["app_cli"]["initialize"]["error"]["reason"],
            "spawn_failed"
        );
        assert_eq!(result["path_cli"]["skills"]["supported"], true);
        assert_eq!(result["path_cli"]["hooks"]["supported"], false);
        assert!(!result.to_string().contains("PRIVATE_NATIVE_ERROR"));
    }

    #[test]
    fn environment_observation_home_mismatch_stops_metadata_reads() {
        let (root, home, target) = setup();
        let cli = mock_cli(
            root.path(),
            root.path(),
            &std::env::current_dir().unwrap(),
            &target,
            false,
        );
        let result = lane(
            &cli,
            &home,
            &std::env::current_dir().unwrap(),
            &[target],
            Duration::from_secs(1),
        );
        assert_eq!(result["initialize"]["home_bound"], false);
        assert!(result["config"].is_null());
        let requests = std::fs::read_to_string(root.path().join("requests")).unwrap();
        assert!(!requests.contains("config/read"));
        assert!(!requests.contains("skills/list"));
    }

    #[test]
    fn environment_observation_duplicate_or_errored_skill_is_unknown() {
        let (_root, _home, target) = setup();
        let cwd = std::env::current_dir().unwrap();
        let skill = json!({"path":target,"name":"skill","description":"description","scope":"user","enabled":true});
        let mut value =
            json!({"data":[{"cwd":cwd,"errors":[],"skills":[skill.clone(),skill]}],"active":true});
        let result = skill_observation(&value, &cwd, std::slice::from_ref(&target)).unwrap();
        assert_eq!(result["targets"][0]["matches"], 2);
        assert!(result["targets"][0]["enabled"].is_null());
        value["data"][0]["skills"].as_array_mut().unwrap().pop();
        value["data"][0]["errors"] = json!([{"message":"error","path":"private"}]);
        assert!(
            skill_observation(&value, &cwd, &[target]).unwrap()["targets"][0]["enabled"].is_null()
        );
    }

    #[test]
    fn environment_observation_rejects_missing_fields_and_foreign_cwd() {
        let (_root, _home, target) = setup();
        let cwd = std::env::current_dir().unwrap();
        let mut value = hook_fixture(&cwd, &target);
        value["data"][0].as_object_mut().unwrap().remove("errors");
        assert!(hook_observation(&value, &cwd, std::slice::from_ref(&target)).is_none());
        value = hook_fixture(&cwd, &target);
        value["data"][0]["cwd"] = json!("/missing-private-directory");
        assert!(hook_observation(&value, &cwd, &[target]).is_none());
    }

    #[test]
    fn environment_observation_symlink_is_not_a_regular_owned_target() {
        let (root, _home, target) = setup();
        let link = root.path().join("link.md");
        symlink(&target, &link).unwrap();
        assert!(file_hash(&link).is_none());
        let cwd = std::env::current_dir().unwrap();
        let value = json!({"data":[{"cwd":cwd,"errors":[],"skills":[{"name":"skill","description":"description","path":target,"scope":"user","enabled":true}]}]});
        assert!(
            skill_observation(&value, &cwd, &[link]).unwrap()["targets"][0]["enabled"].is_null()
        );
    }

    #[test]
    fn environment_observation_rpc_timeout_reaps_the_child() {
        let (root, home, _target) = setup();
        let cli = root.path().join("hang");
        std::fs::write(&cli, "#!/bin/sh\nexec /bin/sleep 30\n").unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut client =
            NativeReader::start(&cli, &home, root.path()).unwrap_or_else(|_| panic!("mock spawn"));
        let start = Instant::now();
        let result = client.rpc("initialize", json!({}), Duration::from_millis(30));
        assert_eq!(result.err().unwrap().reason, "timeout");
        assert!(client.close());
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(client.child.try_wait().unwrap().is_some());
    }

    #[test]
    fn environment_observation_disallows_execution_and_detects_late_activity() {
        let (root, home, target) = setup();
        let cli = mock_cli(root.path(), &home, root.path(), &target, false);
        let mut client =
            NativeReader::start(&cli, &home, root.path()).unwrap_or_else(|_| panic!("mock spawn"));
        assert_eq!(
            client
                .rpc("turn/start", json!({}), Duration::from_millis(30))
                .err()
                .unwrap()
                .reason,
            "method_not_allowed"
        );
        client.pending.extend_from_slice(
            b"{\"method\":\"turn/started\",\"params\":{\"private\":\"PRIVATE_EVENT\"}}\n",
        );
        assert!(client.close());
        assert!(client.activity);
    }

    #[test]
    fn environment_observation_failed_version_does_not_trust_stdout() {
        let (root, home, _target) = setup();
        let cli = root.path().join("bad-version");
        std::fs::write(
            &cli,
            "#!/bin/sh\nprintf '%s\\n' 'codex-cli 0.159.2'\nexit 7\n",
        )
        .unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let result = version(&cli, &home, root.path(), Duration::from_secs(1));
        assert!(result["value"].is_null());
        assert_eq!(result["error"]["reason"], "version_command_failed");
    }
}
