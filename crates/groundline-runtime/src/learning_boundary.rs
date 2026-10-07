//! Small, immutable hook observations. Native hook IDs are correlation hints,
//! never independent work units, response ownership, or acceptance evidence.
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use groundline_contracts::learning::continuous::{
    Boundary, BoundaryTarget, LearningProfile, bytes_sha256,
};
use rustix::fs::{
    AtFlags, Mode, OFlags, RenameFlags, fcntl_getfl, fcntl_setfl, mkdirat, open, openat, renameat,
    renameat_with, unlinkat,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use uuid::Uuid;

const MAX_INPUT_BYTES: u64 = 64 * 1024;
const MAX_PROFILE_BYTES: u64 = 16 * 1024;
const MAX_BOUNDARY_BYTES: u64 = 32 * 1024;
const MAX_BOUNDARIES: usize = 256;
const MAX_STORE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_STATE_BYTES: u64 = 256 * 1024;
const MAX_TARGET_BYTES: u64 = 256 * 1024;
const MAX_TARGET_TOTAL_BYTES: u64 = 1024 * 1024;
const MAX_OPERATION_TOTAL_BYTES: u64 = 1024 * 1024;
const MAX_CORE_BYTES: u64 = 64 * 1024 * 1024;
// Core's consume readout contains at most 1,000 active assessments and bounded
// readiness/category aggregates. Keep a separate ceiling for this projection;
// it is never a transcript or a copy of the 16 MiB learning record store.
const MAX_CORE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_CORE_ATTEMPTS: u8 = 4;
const CORE_RETRY_DELAY: Duration = Duration::from_millis(100);
const MAX_CORE_READOUT_ENTRIES: usize = 1000;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct BoundaryError(&'static str);

fn io<T>(result: std::io::Result<T>) -> Result<T, BoundaryError> {
    result.map_err(|_| BoundaryError("unavailable"))
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn serialized<T: Serialize>(value: &T) -> Result<Vec<u8>, BoundaryError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|_| BoundaryError("invalid_state"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn owner(file: &File, directory: bool, private: bool) -> Result<(), BoundaryError> {
    let meta = io(file.metadata())?;
    if meta.uid() != rustix::process::geteuid().as_raw()
        || (directory && !meta.is_dir())
        || (!directory && (!meta.is_file() || meta.nlink() != 1))
        || meta.permissions().mode() & if private { 0o077 } else { 0o022 } != 0
    {
        return Err(BoundaryError("unsafe_local_state"));
    }
    Ok(())
}

fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

fn open_absolute(path: &Path) -> Result<File, BoundaryError> {
    if !path.is_absolute() {
        return Err(BoundaryError("unsafe_local_state"));
    }
    let mut file = File::from(
        open("/", dir_flags(), Mode::empty()).map_err(|_| BoundaryError("unavailable"))?,
    );
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                file = File::from(
                    openat(&file, name, dir_flags(), Mode::empty())
                        .map_err(|_| BoundaryError("unsafe_local_state"))?,
                );
            }
            _ => return Err(BoundaryError("unsafe_local_state")),
        }
    }
    Ok(file)
}

struct Directory {
    file: File,
    path: PathBuf,
    identity: (u64, u64),
    private: bool,
}

impl Directory {
    fn absolute(path: &Path, private: bool) -> Result<Self, BoundaryError> {
        let file = open_absolute(path)?;
        owner(&file, true, private)?;
        Self::from_file(file, path.to_owned(), private)
    }

    fn from_file(file: File, path: PathBuf, private: bool) -> Result<Self, BoundaryError> {
        let meta = io(file.metadata())?;
        Ok(Self {
            file,
            path,
            identity: (meta.dev(), meta.ino()),
            private,
        })
    }

    fn validate(&self) -> Result<(), BoundaryError> {
        let current = open_absolute(&self.path)?;
        owner(&current, true, self.private)?;
        let meta = io(current.metadata())?;
        if (meta.dev(), meta.ino()) != self.identity {
            return Err(BoundaryError("changed"));
        }
        Ok(())
    }

    fn child(
        &self,
        name: &str,
        private: bool,
        create: bool,
    ) -> Result<Option<Self>, BoundaryError> {
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.len() > 255
            || name.contains('/')
            || name.chars().any(char::is_control)
        {
            return Err(BoundaryError("unsafe_local_state"));
        }
        self.validate()?;
        if create {
            match mkdirat(&self.file, name, Mode::from_raw_mode(0o700)) {
                Ok(()) => io(self.file.sync_all())?,
                Err(rustix::io::Errno::EXIST) => {}
                Err(_) => return Err(BoundaryError("unavailable")),
            }
        }
        let file = match openat(&self.file, name, dir_flags(), Mode::empty()) {
            Ok(file) => File::from(file),
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(_) => return Err(BoundaryError("unsafe_local_state")),
        };
        owner(&file, true, private)?;
        Self::from_file(file, self.path.join(name), private).map(Some)
    }

    fn read(&self, name: &str, maximum: u64) -> Result<Option<Vec<u8>>, BoundaryError> {
        self.validate()?;
        let mut file = match openat(
            &self.file,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(file) => File::from(file),
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(_) => return Err(BoundaryError("unsafe_local_state")),
        };
        owner(&file, false, self.private)?;
        let before = io(file.metadata())?;
        if before.len() > maximum {
            return Err(BoundaryError("oversize"));
        }
        let mut bytes = Vec::new();
        io(Read::by_ref(&mut file)
            .take(maximum + 1)
            .read_to_end(&mut bytes))?;
        let after = io(file.metadata())?;
        if bytes.len() as u64 > maximum
            || before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
        {
            return Err(BoundaryError("changed"));
        }
        self.validate()?;
        Ok(Some(bytes))
    }

    fn json<T: DeserializeOwned>(
        &self,
        name: &str,
        maximum: u64,
    ) -> Result<Option<T>, BoundaryError> {
        self.read(name, maximum)?
            .map(|bytes| serde_json::from_slice(&bytes).map_err(|_| BoundaryError("invalid_state")))
            .transpose()
    }

    fn lock(&self, name: &str, create: bool) -> Result<Lock, BoundaryError> {
        self.validate()?;
        let flags = OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
        let file = if create {
            match openat(
                &self.file,
                name,
                flags | OFlags::CREATE | OFlags::EXCL,
                Mode::from_raw_mode(0o600),
            ) {
                Ok(file) => File::from(file),
                Err(rustix::io::Errno::EXIST) => File::from(
                    openat(&self.file, name, flags, Mode::empty())
                        .map_err(|_| BoundaryError("unsafe_local_state"))?,
                ),
                Err(_) => return Err(BoundaryError("unavailable")),
            }
        } else {
            File::from(
                openat(&self.file, name, flags, Mode::empty())
                    .map_err(|_| BoundaryError("unavailable"))?,
            )
        };
        owner(&file, false, true)?;
        file.try_lock().map_err(|_| BoundaryError("busy"))?;
        Ok(Lock(file))
    }

    fn names(&self) -> Result<Vec<String>, BoundaryError> {
        self.validate()?;
        let mut names = Vec::new();
        for entry in io(std::fs::read_dir(&self.path))?.take(MAX_BOUNDARIES + 2) {
            let name = io(entry)?
                .file_name()
                .into_string()
                .map_err(|_| BoundaryError("invalid_state"))?;
            if name == "archive"
                && self.path.file_name().and_then(|s| s.to_str()) == Some("boundaries")
            {
                let _ = self
                    .child("archive", true, false)?
                    .ok_or(BoundaryError("unsafe_local_state"))?;
                continue;
            }
            names.push(name);
        }
        self.validate()?;
        if names.len() > MAX_BOUNDARIES {
            return Err(BoundaryError("storage_limit"));
        }
        Ok(names)
    }

    fn write(&self, name: &str, bytes: &[u8], replace: bool) -> Result<(), BoundaryError> {
        self.validate()?;
        if bytes.len() as u64 > MAX_BOUNDARY_BYTES {
            return Err(BoundaryError("oversize"));
        }
        // A replacement is only a wakeup/status slot; UUID evidence never replaces.
        if replace {
            let _ = self.read(name, MAX_BOUNDARY_BYTES)?;
        }
        let temporary = format!(".{}.tmp", Uuid::new_v4());
        let mut candidate = File::from(
            openat(
                &self.file,
                temporary.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|_| BoundaryError("unavailable"))?,
        );
        let result = (|| {
            io(candidate.write_all(bytes))?;
            io(candidate.sync_all())?;
            self.validate()?;
            if replace {
                renameat(&self.file, temporary.as_str(), &self.file, name)
                    .map_err(|_| BoundaryError("unavailable"))?;
            } else {
                renameat_with(
                    &self.file,
                    temporary.as_str(),
                    &self.file,
                    name,
                    RenameFlags::NOREPLACE,
                )
                .map_err(|_| BoundaryError("unavailable"))?;
            }
            io(self.file.sync_all())
        })();
        if result.is_err() {
            let _ = unlinkat(&self.file, temporary.as_str(), AtFlags::empty());
        }
        result
    }
}

struct Lock(File);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn learning_directory(home: &Path) -> Result<Option<Directory>, BoundaryError> {
    crate::environment::Environment::current()
        .map_err(|_| BoundaryError("unsupported_runtime_environment"))?;
    let home = io(std::fs::canonicalize(home))?;
    let root = Directory::absolute(&home, false)?;
    let Some(groundline) = root.child("groundline", false, false)? else {
        return Ok(None);
    };
    groundline.child("learning", true, false)
}

fn profile(directory: &Directory) -> Result<Option<LearningProfile>, BoundaryError> {
    let value: Option<LearningProfile> = directory.json("profile.json", MAX_PROFILE_BYTES)?;
    if let Some(profile) = &value {
        profile
            .validate()
            .map_err(|_| BoundaryError("invalid_profile"))?;
    }
    Ok(value)
}

pub fn enabled(home: &Path) -> Result<bool, BoundaryError> {
    let Some(directory) = learning_directory(home)? else {
        return Ok(false);
    };
    Ok(profile(&directory)?.is_some_and(|p| p.enabled))
}

pub fn event_for_trigger(trigger: &str) -> Option<&'static str> {
    match trigger {
        "session_start_hook" => Some("SessionStart"),
        "user_prompt_submit_hook" => Some("UserPromptSubmit"),
        "stop_hook" => Some("Stop"),
        "post_compact_hook" => Some("PostCompact"),
        "session_end_hook" => Some("SessionEnd"),
        _ => None,
    }
}

// Serde skips every other native field, including prompt, cwd and transcript.
#[derive(Default, Deserialize)]
struct NativePayload {
    hook_event_name: Option<String>,
    session_id: Option<String>,
    turn_id: Option<String>,
    model: Option<String>,
    permission_mode: Option<String>,
}

fn correlation_hash(domain: &str, value: &str) -> String {
    bytes_sha256(format!("{domain}\0{value}").as_bytes())
}

fn native_input(reader: impl Read, event: &str, time: &str) -> Boundary {
    let mut bytes = Vec::new();
    let read = reader.take(MAX_INPUT_BYTES + 1).read_to_end(&mut bytes);
    let (mut payload, mut status) = if read.is_err() {
        (NativePayload::default(), "unavailable")
    } else if bytes.len() as u64 > MAX_INPUT_BYTES {
        (NativePayload::default(), "oversize")
    } else if bytes.is_empty() {
        (NativePayload::default(), "unavailable")
    } else if bytes.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'{') {
        (NativePayload::default(), "invalid")
    } else {
        match serde_json::from_slice::<NativePayload>(&bytes) {
            Ok(value) => (value, "complete"),
            Err(_) => (NativePayload::default(), "invalid"),
        }
    };
    if !["UserPromptSubmit", "Stop", "PostCompact"].contains(&event) {
        payload.turn_id = None;
    }
    let fields = [
        payload.session_id.as_deref(),
        payload.turn_id.as_deref(),
        payload.model.as_deref(),
        payload.permission_mode.as_deref(),
    ];
    if status == "complete"
        && (payload
            .hook_event_name
            .as_deref()
            .is_some_and(|name| name != event)
            || fields.into_iter().flatten().any(|value| !safe_id(value)))
    {
        status = "invalid";
    }
    let valid = status == "complete";
    let mut missing_fields = Vec::new();
    if valid {
        for (name, value) in [
            ("hook_event_name", payload.hook_event_name.as_deref()),
            ("session_id", payload.session_id.as_deref()),
            ("model", payload.model.as_deref()),
            ("permission_mode", payload.permission_mode.as_deref()),
        ] {
            if value.is_none() {
                missing_fields.push(name.to_owned());
            }
        }
        if ["UserPromptSubmit", "Stop", "PostCompact"].contains(&event) && payload.turn_id.is_none()
        {
            missing_fields.push("turn_id".into());
        }
        if !missing_fields.is_empty() {
            status = "missing_fields";
        }
    }
    let session_hash = valid
        .then(|| {
            payload
                .session_id
                .as_deref()
                .map(|s| correlation_hash("groundline-hook-session", s))
        })
        .flatten();
    let turn_hash = valid
        .then(|| {
            payload
                .turn_id
                .as_deref()
                .map(|s| correlation_hash("groundline-hook-turn", s))
        })
        .flatten();
    let model = valid.then_some(payload.model).flatten();
    let permission_mode = valid.then_some(payload.permission_mode).flatten();
    let evidence = valid.then(|| {
        bytes_sha256(&serde_json::to_vec(&json!({
        "hook_event_name":payload.hook_event_name,"session_hash":session_hash,"turn_hash":turn_hash,
        "model":model,"permission_mode":permission_mode,
    })).expect("allowlisted JSON"))
    });
    Boundary {
        kind: "groundline-learning-boundary".into(),
        schema: 1,
        boundary_id: Uuid::new_v4().to_string(),
        event: event.into(),
        observed_at_utc: time.into(),
        session_hash,
        turn_hash,
        model,
        permission_mode,
        payload_status: status.into(),
        missing_fields,
        native_evidence_sha256: evidence,
        source_verified: false,
        device_id: None,
        targets: Vec::new(),
        native_activation: "UNVERIFIED".into(),
    }
}

#[derive(Deserialize)]
struct Head {
    kind: String,
    schema: u8,
    revision: String,
    baseline_sha256: String,
    bindings_sha256: String,
    registry_sha256: String,
}
#[derive(Deserialize)]
struct Baseline {
    kind: String,
    schema: u8,
    revision: String,
    parent_revision: Option<String>,
    source_revision: String,
    authority_revision: String,
    authority_ref: String,
    managed_targets: Vec<ManagedTarget>,
}
#[derive(Serialize, Deserialize)]
struct ManagedBlock {
    start_marker: String,
    end_marker: String,
}

#[derive(Serialize, Deserialize)]
struct ManagedTarget {
    target_id: String,
    kind: String,
    desired_sha256: String,
    dependencies: Vec<String>,
    managed_block: Option<ManagedBlock>,
}
#[derive(Serialize)]
struct CommonBaseline<'a> {
    kind: &'static str,
    schema: u8,
    revision: &'a str,
    parent_revision: &'a Option<String>,
    source_revision: &'a str,
    authority_revision: &'a str,
    authority_ref: &'a str,
    managed_targets: &'a [ManagedTarget],
}
#[derive(Deserialize)]
struct Registry {
    kind: String,
    schema: u8,
    device_id: String,
    roots: Vec<RegisteredRoot>,
    targets: Vec<RegisteredTarget>,
}
#[derive(Deserialize)]
struct RegisteredRoot {
    input: RootInput,
    canonical_path: PathBuf,
    binding: Identity,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct RootInput {
    root_id: String,
    path: PathBuf,
    aliases: Vec<PathBuf>,
}
#[derive(Deserialize)]
struct RegisteredTarget {
    input: TargetInput,
    parent_binding: Identity,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct TargetInput {
    target_id: String,
    root_id: String,
    relative_path: PathBuf,
}
#[derive(Deserialize)]
struct Identity {
    dev: u64,
    ino: u64,
}

#[derive(Deserialize)]
struct Bindings {
    kind: String,
    schema: u8,
    device_id: String,
    roots: Vec<RootInput>,
    targets: Vec<TargetInput>,
}

fn artifact<T: DeserializeOwned>(
    directory: &Directory,
    prefix: &str,
    digest: &str,
) -> Result<T, BoundaryError> {
    if !groundline_contracts::learning::digest(digest) {
        return Err(BoundaryError("invalid_state"));
    }
    let bytes = directory
        .read(&format!("{prefix}-{digest}.json"), MAX_STATE_BYTES)?
        .ok_or(BoundaryError("invalid_state"))?;
    if bytes_sha256(&bytes) != digest {
        return Err(BoundaryError("invalid_state"));
    }
    serde_json::from_slice(&bytes).map_err(|_| BoundaryError("invalid_state"))
}

fn target_digest(
    registry: &Registry,
    target: &RegisteredTarget,
    kind: &str,
) -> Result<Option<Vec<u8>>, BoundaryError> {
    let root = registry
        .roots
        .iter()
        .find(|r| r.input.root_id == target.input.root_id)
        .ok_or(BoundaryError("invalid_state"))?;
    let mut parent = Directory::absolute(&root.canonical_path, false)?;
    if parent.identity != (root.binding.dev, root.binding.ino) || root.input.aliases.len() > 64 {
        return Err(BoundaryError("changed"));
    }
    for alias in std::iter::once(&root.input.path).chain(&root.input.aliases) {
        if io(std::fs::canonicalize(alias))? != root.canonical_path {
            return Err(BoundaryError("changed"));
        }
    }
    let names: Option<Vec<_>> = target
        .input
        .relative_path
        .components()
        .map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    let names = names.ok_or(BoundaryError("unsupported_target"))?;
    let Some((leaf, parents)) = names.split_last() else {
        return Err(BoundaryError("unsupported_target"));
    };
    let full_path = root.canonical_path.join(&target.input.relative_path);
    let full_names: Vec<_> = full_path
        .components()
        .filter_map(|p| p.as_os_str().to_str())
        .collect();
    if names.len() > 32
        || full_names.iter().any(|name| {
            [
                "config.toml",
                "auth.json",
                "credentials.json",
                "permissions",
                "memories",
                "memory",
                "sessions",
                "session_index.jsonl",
                "cache",
                "providers",
                "bundled",
                "automation.toml",
                ".system",
                "hub",
                "hubs",
            ]
            .contains(name)
        })
        || full_names
            .windows(2)
            .any(|pair| pair == ["plugins", "cache"])
        || full_path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|e| ["db", "sqlite", "sqlite3"].contains(&e.to_ascii_lowercase().as_str()))
        || !(kind == "agents_block" && *leaf == "AGENTS.md"
            || kind == "skill_file" && *leaf != "AGENTS.md" && full_names.contains(&"skills"))
    {
        return Err(BoundaryError("unsupported_target"));
    }
    for name in parents {
        parent = parent
            .child(name, false, false)?
            .ok_or(BoundaryError("unavailable"))?;
    }
    if parent.identity != (target.parent_binding.dev, target.parent_binding.ino) {
        return Err(BoundaryError("changed"));
    }
    let before = parent.read(leaf, MAX_TARGET_BYTES)?;
    // No environment or user file is written. Recheck the same registered file.
    if parent.read(leaf, MAX_TARGET_BYTES)? != before {
        return Err(BoundaryError("changed"));
    }
    Ok(before)
}

fn operation_observations(
    state: &Directory,
    deadline: Instant,
    remaining_bytes: &mut u64,
) -> Result<BTreeMap<String, String>, BoundaryError> {
    if Instant::now() > deadline {
        return Err(BoundaryError("budget_exceeded"));
    }
    let mut observations = BTreeMap::new();
    if let Some(operations) = state.child("operations", true, false)? {
        // Directory::names bounds the complete operation set to 256 entries.
        let names = operations.names()?;
        for name in names.into_iter().filter(|name| name.ends_with(".json")) {
            if Instant::now() > deadline || *remaining_bytes == 0 {
                return Err(BoundaryError("budget_exceeded"));
            }
            let bytes = operations
                .read(&name, *remaining_bytes)?
                .ok_or(BoundaryError("changed"))?;
            if bytes.is_empty() {
                return Err(BoundaryError("invalid_state"));
            }
            *remaining_bytes -= bytes.len() as u64;
            observations.insert(name, bytes_sha256(&bytes));
        }
        operations.validate()?;
    }
    if Instant::now() > deadline {
        return Err(BoundaryError("budget_exceeded"));
    }
    Ok(observations)
}

fn target_snapshots(profile: &LearningProfile, time: &str) -> Vec<BoundaryTarget> {
    let deadline = Instant::now() + Duration::from_millis(250);
    let observation = (|| {
        let state = Directory::absolute(Path::new(&profile.environment_state), true)?;
        let _lock = state.lock("writer.lock", false)?;
        let original = state
            .read("head.json", 4096)?
            .ok_or(BoundaryError("invalid_state"))?;
        let head: Head =
            serde_json::from_slice(&original).map_err(|_| BoundaryError("invalid_state"))?;
        let snapshots = state
            .child("snapshots", true, false)?
            .ok_or(BoundaryError("invalid_state"))?;
        let baseline: Baseline = artifact(&snapshots, "baseline", &head.baseline_sha256)?;
        let registry: Registry = artifact(&snapshots, "registry", &head.registry_sha256)?;
        let bindings: Bindings = artifact(&snapshots, "bindings", &head.bindings_sha256)?;
        if head.kind != "groundline-environment-head"
            || head.schema != 1
            || baseline.kind != "groundline-environment-baseline"
            || baseline.schema != 1
            || registry.kind != "groundline-environment-registry"
            || registry.schema != 1
            || bindings.kind != "groundline-environment-device-bindings"
            || bindings.schema != 1
            || head.revision != baseline.revision
            || !safe_id(&baseline.source_revision)
            || registry.device_id != profile.device_id
            || bindings.device_id != registry.device_id
            || baseline.managed_targets.len() > 64
            || registry.targets.len() > 64
            || registry.roots.len() > 64
            || registry.roots.len() != bindings.roots.len()
            || registry.targets.len() != bindings.targets.len()
            || registry.targets.len() != baseline.managed_targets.len()
            || registry
                .roots
                .iter()
                .map(|root| &root.input)
                .ne(bindings.roots.iter())
            || registry
                .targets
                .iter()
                .map(|target| &target.input)
                .ne(bindings.targets.iter())
        {
            return Err(BoundaryError("invalid_state"));
        }
        let mut operation_budget = MAX_OPERATION_TOTAL_BYTES;
        let operations = operation_observations(&state, deadline, &mut operation_budget)?;
        let observation_state_sha256 = bytes_sha256(&serialized(&json!({
            "state_identity": [state.identity.0, state.identity.1],
            "baseline_sha256": head.baseline_sha256,
            "bindings_sha256": head.bindings_sha256,
            "registry_sha256": head.registry_sha256,
            "operations": operations,
        }))?);
        let common = CommonBaseline {
            kind: "groundline-environment-common-baseline",
            schema: 1,
            revision: &baseline.revision,
            parent_revision: &baseline.parent_revision,
            source_revision: &baseline.source_revision,
            authority_revision: &baseline.authority_revision,
            authority_ref: &baseline.authority_ref,
            managed_targets: &baseline.managed_targets,
        };
        let common_hash = bytes_sha256(&serialized(&common)?);
        let mut total_bytes = 0;
        let mut targets = Vec::new();
        for target_id in &profile.target_ids {
            let result = if Instant::now() > deadline || total_bytes >= MAX_TARGET_TOTAL_BYTES {
                Err(BoundaryError("budget_exceeded"))
            } else {
                (|| {
                    let managed = baseline
                        .managed_targets
                        .iter()
                        .find(|t| t.target_id == *target_id)
                        .ok_or(BoundaryError("invalid_state"))?;
                    let registered = registry
                        .targets
                        .iter()
                        .find(|t| t.input.target_id == *target_id)
                        .ok_or(BoundaryError("invalid_state"))?;
                    let bytes = target_digest(&registry, registered, &managed.kind)?;
                    total_bytes += bytes.as_ref().map_or(0, |b| b.len() as u64);
                    if total_bytes > MAX_TARGET_TOTAL_BYTES {
                        return Err(BoundaryError("budget_exceeded"));
                    }
                    Ok(bytes.map(|b| bytes_sha256(&b)))
                })()
            };
            targets.push(BoundaryTarget {
                target_id: target_id.clone(),
                snapshot_code: match &result {
                    Ok(Some(_)) => "observed",
                    Ok(None) => "target_missing",
                    Err(error) => error.0,
                }
                .into(),
                skill_revision: result.ok().flatten(),
                environment_revision: None,
                source_revision: Some(baseline.source_revision.clone()),
                baseline_sha256: Some(common_hash.clone()),
                observation_state_sha256: Some(observation_state_sha256.clone()),
                observed_at_utc: time.into(),
            });
        }
        if state.read("head.json", 4096)?.as_deref() != Some(original.as_slice()) {
            return Err(BoundaryError("changed"));
        }
        if operation_observations(&state, deadline, &mut operation_budget)? != operations {
            return Err(BoundaryError("changed"));
        }
        Ok(targets)
    })();
    observation.unwrap_or_else(|error: BoundaryError| {
        profile
            .target_ids
            .iter()
            .map(|target_id| BoundaryTarget {
                target_id: target_id.clone(),
                snapshot_code: error.0.into(),
                skill_revision: None,
                environment_revision: None,
                source_revision: None,
                baseline_sha256: None,
                observation_state_sha256: None,
                observed_at_utc: time.into(),
            })
            .collect()
    })
}

fn capture_status(directory: &Directory, code: &str, boundary: Option<&Boundary>) {
    let Ok(_lock) = directory.lock("capture-status.lock", true) else {
        return;
    };
    let Ok(previous) = read_status(
        directory,
        "capture-status.json",
        "groundline-learning-boundary-capture-status",
    ) else {
        return;
    };
    let captured_count = previous
        .as_ref()
        .map_or(0, |p| p.captured_count)
        .saturating_add(u64::from(boundary.is_some()));
    let failed_capture_count = previous
        .as_ref()
        .map_or(0, |p| p.failed_capture_count)
        .saturating_add(u64::from(boundary.is_none()));
    let incomplete_payload_count = previous
        .as_ref()
        .map_or(0, |p| p.incomplete_payload_count)
        .saturating_add(u64::from(
            boundary.is_some_and(|b| b.payload_status != "complete"),
        ));
    let unknown_snapshot_count = previous
        .as_ref()
        .map_or(0, |p| p.unknown_snapshot_count)
        .saturating_add(u64::from(boundary.is_some_and(|b| {
            b.targets.iter().any(|t| t.snapshot_code != "observed")
        })));
    let value = json!({"kind":"groundline-learning-boundary-capture-status","schema":1,
        "result_code":code,"observed_at_utc":now(),"payload_status":boundary.map(|b| &b.payload_status),
        "missing_fields":boundary.map(|b| &b.missing_fields),"source_verified":false,
        "captured_count":captured_count,"failed_capture_count":failed_capture_count,
        "incomplete_payload_count":incomplete_payload_count,"unknown_snapshot_count":unknown_snapshot_count,
        "native_activation":"UNVERIFIED","network_performed":false});
    if let Ok(bytes) = serialized(&value) {
        let _ = directory.write("capture-status.json", &bytes, true);
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusSlot {
    kind: String,
    schema: u8,
    result_code: String,
    observed_at_utc: String,
    payload_status: Option<String>,
    missing_fields: Option<Vec<String>>,
    source_verified: bool,
    native_activation: String,
    network_performed: bool,
    #[serde(default)]
    captured_count: u64,
    #[serde(default)]
    failed_capture_count: u64,
    #[serde(default)]
    incomplete_payload_count: u64,
    #[serde(default)]
    unknown_snapshot_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum CoreStatus {
    Consumed,
    Pending,
    Unconfigured,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PendingReason {
    LearningStateBusy,
    LearningBoundaryBusy,
    LearningProfileBusy,
    ActualNativeEndOrArtifactRequired,
    LearningResponseSourceChanged,
    CorePendingOther,
}

impl PendingReason {
    fn from_core(reason: &str) -> Self {
        match reason {
            "learning_state_busy" => Self::LearningStateBusy,
            "learning_boundary_busy" => Self::LearningBoundaryBusy,
            "learning_profile_busy" => Self::LearningProfileBusy,
            "actual_native_end_or_artifact_required" => Self::ActualNativeEndOrArtifactRequired,
            "learning_response_source_changed" => Self::LearningResponseSourceChanged,
            _ => Self::CorePendingOther,
        }
    }

    fn retryable(self) -> bool {
        self != Self::CorePendingOther
    }
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum AssessmentStatus {
    Pending,
    Finalized,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoreAssessment {
    status: AssessmentStatus,
    reason: Option<String>,
    retry_safe: Option<bool>,
    mutation_performed: Option<bool>,
    assessment_sha256: Option<String>,
    task_sha256: String,
    outcome_sha256: Option<String>,
    native_response_proof_sha256: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoreReadout {
    kind: String,
    schema: u8,
    status: CoreStatus,
    native_activation: String,
    network_performed: bool,
    mutation_performed: bool,
    raw_content_emitted: bool,
    private_paths_emitted: bool,
    reason: Option<String>,
    retry_safe: Option<bool>,
    archived_boundary_count: Option<u64>,
    archived_record_count: Option<u64>,
    resumed_finalize_count: Option<u64>,
    assessments: Option<Vec<CoreAssessment>>,
    reconciliations: Option<Vec<serde::de::IgnoredAny>>,
    readiness: Option<serde::de::IgnoredAny>,
    quality_inferred_from_boundaries: Option<bool>,
    capacity: Option<serde::de::IgnoredAny>,
}

struct CoreObservation {
    status: CoreStatus,
    pending_assessment_count: Option<u64>,
    finalized_assessment_count: Option<u64>,
    final_reason: Option<PendingReason>,
    retryable: bool,
}

impl CoreReadout {
    fn observation(self) -> Result<CoreObservation, BoundaryError> {
        let invalid = || BoundaryError("core_output_invalid");
        if self.kind != "groundline-learning-result"
            || self.schema != 1
            || self.native_activation != "UNVERIFIED"
            || self.network_performed
            || self.raw_content_emitted
            || self.private_paths_emitted
        {
            return Err(invalid());
        }
        match self.status {
            CoreStatus::Pending => {
                if self.mutation_performed || self.assessments.is_some() {
                    return Err(invalid());
                }
                let reason = PendingReason::from_core(self.reason.as_deref().ok_or_else(invalid)?);
                Ok(CoreObservation {
                    status: self.status,
                    pending_assessment_count: None,
                    finalized_assessment_count: None,
                    final_reason: Some(reason),
                    retryable: self.retry_safe == Some(true) && reason.retryable(),
                })
            }
            CoreStatus::Consumed => {
                let rows = self.assessments.ok_or_else(invalid)?;
                let resumed = self.resumed_finalize_count.ok_or_else(invalid)?;
                if rows.len() > MAX_CORE_READOUT_ENTRIES
                    || resumed > MAX_CORE_READOUT_ENTRIES as u64
                    || self.archived_boundary_count.is_none()
                    || self.archived_record_count.is_none()
                    || self.readiness.is_none()
                    || self.capacity.is_none()
                    || self.quality_inferred_from_boundaries != Some(false)
                    || self
                        .reconciliations
                        .as_ref()
                        .is_none_or(|v| v.len() > MAX_CORE_READOUT_ENTRIES)
                    || self.reason.is_some()
                    || self.retry_safe.is_some()
                {
                    return Err(invalid());
                }
                let mut pending = 0;
                let mut finalized = resumed;
                let mut reason = None;
                let mut retryable = true;
                for row in rows {
                    if !groundline_contracts::learning::digest(&row.task_sha256)
                        || row
                            .assessment_sha256
                            .as_deref()
                            .is_none_or(|hash| !groundline_contracts::learning::digest(hash))
                    {
                        return Err(invalid());
                    }
                    match row.status {
                        AssessmentStatus::Finalized => {
                            if row.mutation_performed != Some(true)
                                || row.reason.is_some()
                                || row.retry_safe.is_some()
                                || row.outcome_sha256.as_deref().is_none_or(|hash| {
                                    !groundline_contracts::learning::digest(hash)
                                })
                                || row
                                    .native_response_proof_sha256
                                    .as_deref()
                                    .is_none_or(|hash| {
                                        !groundline_contracts::learning::digest(hash)
                                    })
                            {
                                return Err(invalid());
                            }
                            finalized += 1;
                        }
                        AssessmentStatus::Pending => {
                            if row.mutation_performed.is_some()
                                || row.outcome_sha256.is_some()
                                || row.native_response_proof_sha256.is_some()
                            {
                                return Err(invalid());
                            }
                            pending += 1;
                            let current = PendingReason::from_core(
                                row.reason.as_deref().ok_or_else(invalid)?,
                            );
                            retryable &= row.retry_safe == Some(true) && current.retryable();
                            // An unknown/permanent reason takes precedence over
                            // a transient reason. Never copy arbitrary Core text.
                            if reason.is_none() || !current.retryable() {
                                reason = Some(current);
                            }
                        }
                    }
                }
                Ok(CoreObservation {
                    status: self.status,
                    pending_assessment_count: Some(pending),
                    finalized_assessment_count: Some(finalized),
                    final_reason: reason,
                    retryable: pending > 0 && retryable,
                })
            }
            CoreStatus::Unconfigured | CoreStatus::Disabled => {
                if self.mutation_performed || self.assessments.is_some() || self.reason.is_some() {
                    return Err(invalid());
                }
                Ok(CoreObservation {
                    status: self.status,
                    pending_assessment_count: None,
                    finalized_assessment_count: None,
                    final_reason: None,
                    retryable: false,
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerStatusSlot {
    kind: String,
    schema: u8,
    result_code: String,
    observed_at_utc: String,
    core_status: Option<CoreStatus>,
    core_counts_scope: String,
    consume_attempt_count: u8,
    retry_count: u8,
    pending_assessment_count: Option<u64>,
    finalized_assessment_count: Option<u64>,
    last_retry_reason: Option<PendingReason>,
    final_reason: Option<PendingReason>,
    source_verified: bool,
    native_activation: String,
    network_performed: bool,
}

fn read_worker_status(directory: &Directory) -> Result<Option<WorkerStatusSlot>, BoundaryError> {
    let value: Option<WorkerStatusSlot> =
        directory.json("worker-status.json", MAX_PROFILE_BYTES)?;
    if let Some(v) = &value
        && (v.kind != "groundline-learning-worker-status"
            || v.schema != 1
            || v.source_verified
            || v.network_performed
            || v.native_activation != "UNVERIFIED"
            || v.core_counts_scope != "last_core_attempt"
            || chrono::DateTime::parse_from_rfc3339(&v.observed_at_utc).is_err()
            || v.consume_attempt_count > MAX_CORE_ATTEMPTS
            || v.retry_count != v.consume_attempt_count.saturating_sub(1)
            || (v.retry_count == 0) != v.last_retry_reason.is_none()
            || v.last_retry_reason
                .is_some_and(|reason| !reason.retryable())
            || v.pending_assessment_count
                .is_some_and(|count| count > MAX_CORE_READOUT_ENTRIES as u64)
            || v.finalized_assessment_count
                .is_some_and(|count| count > 2 * MAX_CORE_READOUT_ENTRIES as u64)
            || ![
                "unavailable",
                "unsafe_local_state",
                "changed",
                "oversize",
                "busy",
                "invalid_state",
                "invalid_profile",
                "core_unavailable",
                "core_pin_mismatch",
                "core_spawn_failed",
                "core_consume_failed",
                "core_consume_timeout",
                "core_output_invalid",
                "core_output_oversize",
                "consumed",
                "pending",
                "not_enabled",
            ]
            .contains(&v.result_code.as_str())
            || (v.result_code == "consumed"
                && (v.core_status != Some(CoreStatus::Consumed)
                    || v.pending_assessment_count != Some(0)
                    || v.final_reason.is_some()))
            || (v.result_code == "pending" && v.final_reason.is_none()))
    {
        return Err(BoundaryError("invalid_state"));
    }
    Ok(value)
}

fn read_status(
    directory: &Directory,
    name: &str,
    kind: &str,
) -> Result<Option<StatusSlot>, BoundaryError> {
    let value: Option<StatusSlot> = directory.json(name, MAX_PROFILE_BYTES)?;
    if let Some(value) = &value
        && (value.kind != kind
            || value.schema != 1
            || value.source_verified
            || value.network_performed
            || value.native_activation != "UNVERIFIED"
            || chrono::DateTime::parse_from_rfc3339(&value.observed_at_utc).is_err()
            || ![
                "captured",
                "unavailable",
                "unsafe_local_state",
                "changed",
                "oversize",
                "busy",
                "storage_limit",
                "invalid_state",
                "invalid_profile",
                "invalid_boundary",
                "core_unavailable",
                "core_pin_mismatch",
                "core_spawn_failed",
                "core_consume_failed",
                "core_consume_timeout",
                "consumed",
            ]
            .contains(&value.result_code.as_str())
            || value.payload_status.as_ref().is_some_and(|p| {
                ![
                    "complete",
                    "missing_fields",
                    "invalid",
                    "oversize",
                    "unavailable",
                ]
                .contains(&p.as_str())
            })
            || value.missing_fields.as_ref().is_some_and(|fields| {
                fields.len() > 5
                    || fields.iter().any(|field| {
                        ![
                            "hook_event_name",
                            "session_id",
                            "turn_id",
                            "model",
                            "permission_mode",
                        ]
                        .contains(&field.as_str())
                    })
            }))
    {
        return Err(BoundaryError("invalid_state"));
    }
    Ok(value)
}

/// Preserve one bounded allowlist observation synchronously, without a native
/// CLI query, network call, context injection, or dependency on Insights consent.
pub fn capture(
    home: &Path,
    trigger: &str,
    reader: impl Read,
) -> Result<Option<String>, BoundaryError> {
    let event = event_for_trigger(trigger).ok_or(BoundaryError("invalid_trigger"))?;
    let Some(directory) = learning_directory(home)? else {
        return Ok(None);
    };
    let result = (|| {
        let Some(profile) = profile(&directory)? else {
            return Ok(None);
        };
        if !profile.enabled {
            return Ok(None);
        }
        let _lock = directory.lock("boundary.lock", true)?;
        let time = now();
        let mut boundary = native_input(reader, event, &time);
        boundary.device_id = Some(profile.device_id.clone());
        boundary.targets = target_snapshots(&profile, &time);
        boundary
            .validate()
            .map_err(|_| BoundaryError("invalid_boundary"))?;
        let records = directory
            .child("boundaries", true, true)?
            .ok_or(BoundaryError("unavailable"))?;
        let names = records.names()?;
        if names.len() >= MAX_BOUNDARIES {
            return Err(BoundaryError("storage_limit"));
        }
        let mut size = 0;
        for name in names {
            size += records
                .read(&name, MAX_BOUNDARY_BYTES)?
                .ok_or(BoundaryError("invalid_state"))?
                .len() as u64;
        }
        let bytes = serialized(&boundary)?;
        if size + bytes.len() as u64 > MAX_STORE_BYTES {
            return Err(BoundaryError("storage_limit"));
        }
        records.write(&format!("{}.json", boundary.boundary_id), &bytes, false)?;
        capture_status(&directory, "captured", Some(&boundary));
        Ok(Some(boundary.boundary_id))
    })();
    if let Err(error) = &result {
        capture_status(&directory, error.0, None);
    }
    result
}

/// Read-only coverage. Counts do not authenticate native dispatch or outcomes.
pub fn status(home: &Path) -> Value {
    let observation = (|| {
        let Some(directory) = learning_directory(home)? else {
            return Ok(json!({"state":"not_configured"}));
        };
        let Some(profile) = profile(&directory)? else {
            return Ok(json!({"state":"not_configured"}));
        };
        let capture = read_status(
            &directory,
            "capture-status.json",
            "groundline-learning-boundary-capture-status",
        )?;
        let worker = read_worker_status(&directory)?;
        let mut count = 0;
        let mut incomplete = 0;
        let mut unknown = 0;
        let mut events = std::collections::BTreeSet::new();
        if let Some(records) = directory.child("boundaries", true, false)? {
            for name in records.names()? {
                let boundary: Boundary = records
                    .json(&name, MAX_BOUNDARY_BYTES)?
                    .ok_or(BoundaryError("invalid_state"))?;
                boundary
                    .validate()
                    .map_err(|_| BoundaryError("invalid_state"))?;
                if name != format!("{}.json", boundary.boundary_id) {
                    return Err(BoundaryError("invalid_state"));
                }
                count += 1;
                incomplete += usize::from(boundary.payload_status != "complete");
                unknown += usize::from(
                    boundary
                        .targets
                        .iter()
                        .any(|t| t.snapshot_code != "observed"),
                );
                events.insert(boundary.event);
            }
        }
        let missing_events: Vec<_> = [
            "SessionStart",
            "UserPromptSubmit",
            "Stop",
            "PostCompact",
            "SessionEnd",
        ]
        .into_iter()
        .filter(|event| !events.contains(*event))
        .collect();
        let failed_captures = capture
            .as_ref()
            .map_or(0, |value| value.failed_capture_count);
        Ok(
            json!({"state":if profile.enabled {"enabled"} else {"disabled"},"boundary_count":count,
            "incomplete_payload_count":incomplete,"unknown_snapshot_count":unknown,"missing_events":missing_events,
            "observation_scope":"bounded_active_queue","archive_coverage":"uninspected",
            "failed_capture_count":failed_captures,"capture_incomplete":failed_captures > 0,
            "coverage":if count == 0 {"unobserved"} else if incomplete > 0 || failed_captures > 0 || !missing_events.is_empty() {"partial"} else {"observed"},
            "last_capture":capture,"last_worker":worker}),
        )
    })();
    let mut result = observation.unwrap_or_else(
        |error: BoundaryError| json!({"state":"unavailable","result_code":error.0}),
    );
    result["source_verified"] = json!(false);
    result["native_activation"] = json!("UNVERIFIED");
    result["mutation_performed"] = json!(false);
    result["network_performed"] = json!(false);
    result
}

fn checked_core(profile: &LearningProfile) -> Result<File, BoundaryError> {
    let path = Path::new(&profile.core_executable);
    let parent = Directory::absolute(
        path.parent().ok_or(BoundaryError("core_unavailable"))?,
        false,
    )?;
    let leaf = path.file_name().ok_or(BoundaryError("core_unavailable"))?;
    let mut file = File::from(
        openat(
            &parent.file,
            leaf,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| BoundaryError("core_unavailable"))?,
    );
    owner(&file, false, false).map_err(|_| BoundaryError("core_unavailable"))?;
    let meta = io(file.metadata())?;
    if meta.len() == 0 || meta.len() > MAX_CORE_BYTES || meta.permissions().mode() & 0o111 == 0 {
        return Err(BoundaryError("core_unavailable"));
    }
    let mut bytes = Vec::new();
    io(Read::by_ref(&mut file)
        .take(MAX_CORE_BYTES + 1)
        .read_to_end(&mut bytes))?;
    let after = io(file.metadata())?;
    if bytes.len() as u64 > MAX_CORE_BYTES
        || bytes_sha256(&bytes) != profile.core_sha256
        || bytes.len() as u64 != meta.len()
        || meta.len() != after.len()
        || meta.modified().ok() != after.modified().ok()
    {
        return Err(BoundaryError("core_pin_mismatch"));
    }
    parent.validate()?;
    Ok(file)
}

/// Runs only in the detached Insights worker, including when owner collection
/// is disabled. One local lock and timeout bound concurrent Core consumers.
pub async fn consume(home: &Path) -> Value {
    consume_with_limit(home, Duration::from_secs(10)).await
}

struct CoreChild(std::process::Child);

impl Drop for CoreChild {
    fn drop(&mut self) {
        // Also covers pipe errors, oversized output and an inherited pipe that
        // remains open after the direct child exits. Never leave Core running.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn core_attempt(
    profile: &LearningProfile,
    home: &Path,
    pin: &File,
    deadline: Instant,
) -> Result<CoreObservation, BoundaryError> {
    if Instant::now() >= deadline {
        return Err(BoundaryError("core_consume_timeout"));
    }
    let mut child = CoreChild(
        Command::new(&profile.core_executable)
            .args(["learning", "consume", "--codex-home"])
            .arg(home)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| BoundaryError("core_spawn_failed"))?,
    );
    let mut stdout = child.0.stdout.take().ok_or(BoundaryError("unavailable"))?;
    let flags = fcntl_getfl(&stdout).map_err(|_| BoundaryError("unavailable"))?;
    fcntl_setfl(&stdout, flags | OFlags::NONBLOCK).map_err(|_| BoundaryError("unavailable"))?;
    let mut bytes = Vec::new();
    let mut exit = None;
    let mut eof = false;
    loop {
        if Instant::now() >= deadline {
            return Err(BoundaryError("core_consume_timeout"));
        }
        // Drain while Core is alive so a readout larger than the pipe buffer
        // cannot prevent its exit. A bound applies before any JSON allocation.
        while !eof {
            let mut buffer = [0_u8; 8192];
            match stdout.read(&mut buffer) {
                Ok(0) => eof = true,
                Ok(count) => {
                    if bytes.len() + count > MAX_CORE_OUTPUT_BYTES {
                        return Err(BoundaryError("core_output_oversize"));
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                    if Instant::now() >= deadline {
                        return Err(BoundaryError("core_consume_timeout"));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(BoundaryError("unavailable")),
            }
        }
        if exit.is_none() {
            exit = io(child.0.try_wait())?;
        }
        if let Some(exit) = exit
            && eof
        {
            // Keep the verified inode through each attempt and reject a path
            // replacement even if its bytes still match the profile's SHA.
            let current = checked_core(profile)?;
            let before = io(pin.metadata())?;
            let after = io(current.metadata())?;
            if (before.dev(), before.ino()) != (after.dev(), after.ino()) {
                return Err(BoundaryError("core_pin_mismatch"));
            }
            if Instant::now() >= deadline {
                return Err(BoundaryError("core_consume_timeout"));
            }
            if !exit.success() {
                return Err(BoundaryError("core_consume_failed"));
            }
            let readout: CoreReadout =
                serde_json::from_slice(&bytes).map_err(|_| BoundaryError("core_output_invalid"))?;
            return readout.observation();
        }
        tokio::time::sleep(
            Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())),
        )
        .await;
    }
}

async fn consume_with_limit(home: &Path, limit: Duration) -> Value {
    let observation: Result<Option<(Directory, LearningProfile)>, BoundaryError> = (|| {
        let Some(directory) = learning_directory(home)? else {
            return Ok(None);
        };
        let Some(profile) = profile(&directory)? else {
            return Ok(None);
        };
        if !profile.enabled {
            return Ok(None);
        }
        Ok(Some((directory, profile)))
    })();
    let (directory, profile) = match observation {
        Ok(Some(value)) => value,
        Ok(None) => return json!({"result_code":"not_enabled","network_performed":false}),
        Err(error) => return json!({"result_code":error.0,"network_performed":false}),
    };
    let deadline = Instant::now() + limit;
    let mut attempts = 0;
    let mut last_retry_reason = None;
    let mut last_observation = None;
    let result = async {
        let _lock = directory.lock("consume.lock", true)?;
        let pin = checked_core(&profile)?;
        loop {
            if Instant::now() >= deadline {
                return Err(BoundaryError("core_consume_timeout"));
            }
            if attempts > 0 {
                // A pin may change between the bounded retries. Check before
                // spawning again as well as after the process has terminated.
                let current = checked_core(&profile)?;
                if io(pin.metadata())?.ino() != io(current.metadata())?.ino()
                    || io(pin.metadata())?.dev() != io(current.metadata())?.dev()
                {
                    return Err(BoundaryError("core_pin_mismatch"));
                }
                last_retry_reason = last_observation
                    .as_ref()
                    .and_then(|observation: &CoreObservation| observation.final_reason);
            }
            attempts += 1;
            let observation = core_attempt(&profile, home, &pin, deadline).await?;
            let result_code = match observation.status {
                CoreStatus::Unconfigured | CoreStatus::Disabled => "not_enabled",
                CoreStatus::Pending => "pending",
                CoreStatus::Consumed if observation.pending_assessment_count != Some(0) => {
                    "pending"
                }
                CoreStatus::Consumed => "consumed",
            };
            let retry = observation.retryable && attempts < MAX_CORE_ATTEMPTS;
            last_observation = Some(observation);
            if !retry {
                return Ok(result_code);
            }
            if deadline.saturating_duration_since(Instant::now()) <= CORE_RETRY_DELAY {
                return Ok(result_code);
            }
            tokio::time::sleep(CORE_RETRY_DELAY).await;
        }
    }
    .await;
    let value = json!({"kind":"groundline-learning-worker-status","schema":1,
        "result_code":result.unwrap_or_else(|error| error.0),"observed_at_utc":now(),
        "core_status":last_observation.as_ref().map(|v| v.status),"core_counts_scope":"last_core_attempt",
        "consume_attempt_count":attempts,"retry_count":attempts.saturating_sub(1),
        "pending_assessment_count":last_observation.as_ref().and_then(|v| v.pending_assessment_count),
        "finalized_assessment_count":last_observation.as_ref().and_then(|v| v.finalized_assessment_count),
        "last_retry_reason":last_retry_reason,
        "final_reason":last_observation.as_ref().and_then(|v| v.final_reason),
        "network_performed":false,"source_verified":false,"native_activation":"UNVERIFIED"});
    if let Ok(bytes) = serialized(&value) {
        let _ = directory.write("worker-status.json", &bytes, true);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{TempDir, tempdir};

    fn fixture() -> (TempDir, LearningProfile) {
        let home = tempdir().unwrap();
        let canonical = std::fs::canonicalize(home.path()).unwrap();
        let root = Directory::absolute(&canonical, false).unwrap();
        let groundline = root.child("groundline", false, true).unwrap().unwrap();
        let learning = groundline.child("learning", true, true).unwrap().unwrap();
        let profile = LearningProfile {
            kind: "groundline-learning-profile".into(),
            schema: 1,
            enabled: true,
            environment_state: canonical.join("environment").to_str().unwrap().into(),
            learning_state: canonical.join("records").to_str().unwrap().into(),
            deliveries: canonical.join("deliveries").to_str().unwrap().into(),
            target_ids: vec!["skill".into()],
            device_id: "device".into(),
            core_executable: canonical.join("core").to_str().unwrap().into(),
            core_sha256: "0".repeat(64),
        };
        learning
            .write("profile.json", &serialized(&profile).unwrap(), false)
            .unwrap();
        (home, profile)
    }

    fn consume_readout(assessments: Vec<Value>) -> Value {
        json!({"kind":"groundline-learning-result","schema":1,"status":"CONSUMED",
            "native_activation":"UNVERIFIED","network_performed":false,
            "mutation_performed":!assessments.is_empty(),"raw_content_emitted":false,
            "private_paths_emitted":false,"archived_boundary_count":0,"archived_record_count":0,
            "resumed_finalize_count":0,"assessments":assessments,"reconciliations":[],
            "readiness":{},"capacity":{},"quality_inferred_from_boundaries":false})
    }

    fn pending_readout(reason: &str) -> Value {
        json!({"kind":"groundline-learning-result","schema":1,"status":"PENDING",
            "native_activation":"UNVERIFIED","network_performed":false,
            "mutation_performed":false,"raw_content_emitted":false,"private_paths_emitted":false,
            "reason":reason,"retry_safe":true})
    }

    fn pending_assessment(reason: &str) -> Value {
        json!({"status":"PENDING","assessment_sha256":"a".repeat(64),
            "task_sha256":"b".repeat(64),"reason":reason,"retry_safe":true})
    }

    fn finalized_assessment() -> Value {
        json!({"status":"FINALIZED","assessment_sha256":"a".repeat(64),
            "task_sha256":"b".repeat(64),"outcome_sha256":"c".repeat(64),
            "native_response_proof_sha256":"d".repeat(64),"mutation_performed":true})
    }

    fn emit_readout(value: &Value) -> String {
        format!(
            "cat <<'GROUNDLINE_TEST_READOUT'\n{}\nGROUNDLINE_TEST_READOUT\n",
            serde_json::to_string_pretty(value).unwrap()
        )
    }

    fn configure_core(home: &Path, profile: &mut LearningProfile, script: &str) {
        let core = Path::new(&profile.core_executable);
        std::fs::write(core, format!("#!/bin/sh\n{script}")).unwrap();
        std::fs::set_permissions(core, std::fs::Permissions::from_mode(0o700)).unwrap();
        profile.core_sha256 = bytes_sha256(&std::fs::read(core).unwrap());
        learning_directory(home)
            .unwrap()
            .unwrap()
            .write("profile.json", &serialized(profile).unwrap(), true)
            .unwrap();
    }

    fn payload(event: &str, turn: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({"hook_event_name":event,"session_id":"private-session-id",
            "turn_id":turn,"model":"gpt-6.1-sol","permission_mode":"default",
            "prompt":"PRIVATE_PROMPT_SENTINEL","cwd":"/PRIVATE_CWD_SENTINEL",
            "transcript_path":"/PRIVATE_TRANSCRIPT_SENTINEL","other":{"text":"PRIVATE_UNKNOWN_SENTINEL"}})).unwrap()
    }

    fn record(home: &Path, boundary_id: &str) -> Boundary {
        let bytes = std::fs::read(
            home.join("groundline/learning/boundaries")
                .join(format!("{boundary_id}.json")),
        )
        .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn environment(home: &Path, profile: &LearningProfile) -> (Directory, PathBuf) {
        let canonical = std::fs::canonicalize(home).unwrap();
        let root = Directory::absolute(&canonical, false).unwrap();
        let state = root.child("environment", true, true).unwrap().unwrap();
        let snapshots = state.child("snapshots", true, true).unwrap().unwrap();
        let project = root.child("project", false, true).unwrap().unwrap();
        let skills = project.child("skills", false, true).unwrap().unwrap();
        let skill = skills.child("sample", false, true).unwrap().unwrap();
        skill
            .write("SKILL.md", b"registered skill\n", false)
            .unwrap();
        let root_input = json!({"root_id":"project","path":project.path,"aliases":[]});
        let target_input = json!({"target_id":"skill","root_id":"project","relative_path":"skills/sample/SKILL.md"});
        let baseline = json!({"kind":"groundline-environment-baseline","schema":1,"revision":"revision-1",
            "parent_revision":null,"source_revision":"source-1","authority_revision":"authority-1",
            "exception_revision":"local-1","authority_ref":"maintainer",
            "managed_targets":[{"target_id":"skill","kind":"skill_file","desired_sha256":bytes_sha256(b"registered skill\n"),
                "dependencies":[],"managed_block":null}]});
        let bindings = json!({"kind":"groundline-environment-device-bindings","schema":1,"device_id":profile.device_id,
            "revision":"local-1","roots":[root_input],"targets":[target_input]});
        let registry = json!({"kind":"groundline-environment-registry","schema":1,"device_id":profile.device_id,
            "roots":[{"input":root_input,"canonical_path":project.path,"binding":{"dev":project.identity.0,"ino":project.identity.1}}],
            "targets":[{"input":target_input,"parent_binding":{"dev":skill.identity.0,"ino":skill.identity.1}}]});
        let mut digests = Vec::new();
        for (name, value) in [
            ("baseline", baseline),
            ("bindings", bindings),
            ("registry", registry),
        ] {
            let bytes = serialized(&value).unwrap();
            let digest = bytes_sha256(&bytes);
            snapshots
                .write(&format!("{name}-{digest}.json"), &bytes, false)
                .unwrap();
            digests.push(digest);
        }
        state
            .write(
                "head.json",
                &serialized(&json!({"kind":"groundline-environment-head","schema":1,
            "revision":"revision-1","parent_revision":null,"baseline_sha256":digests[0],
            "bindings_sha256":digests[1],"registry_sha256":digests[2]}))
                .unwrap(),
                false,
            )
            .unwrap();
        drop(state.lock("writer.lock", true).unwrap());
        (state, skill.path.join("SKILL.md"))
    }

    #[test]
    fn immutable_boundaries_preserve_each_turn_and_only_allowlisted_metadata() {
        let (home, _) = fixture();
        let first = capture(
            home.path(),
            "user_prompt_submit_hook",
            payload("UserPromptSubmit", "private-turn-one").as_slice(),
        )
        .unwrap()
        .unwrap();
        let path = home
            .path()
            .join("groundline/learning/boundaries")
            .join(format!("{first}.json"));
        let original = std::fs::read(&path).unwrap();
        let second = capture(
            home.path(),
            "user_prompt_submit_hook",
            payload("UserPromptSubmit", "private-turn-two").as_slice(),
        )
        .unwrap()
        .unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let value = record(home.path(), &first);
        value.validate().unwrap();
        assert_eq!(value.payload_status, "complete");
        assert_eq!(
            value.session_hash,
            Some(correlation_hash(
                "groundline-hook-session",
                "private-session-id"
            ))
        );
        assert_ne!(value.turn_hash, record(home.path(), &second).turn_hash);
        assert!(!value.source_verified);
        assert_eq!(value.native_activation, "UNVERIFIED");
        let stored = String::from_utf8(original).unwrap();
        for forbidden in [
            "PRIVATE_",
            "private-session-id",
            "private-turn-one",
            "prompt",
            "transcript_path",
            "cwd",
        ] {
            assert!(!stored.contains(forbidden), "{forbidden}");
        }
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(status(home.path())["boundary_count"], 2);
    }

    #[test]
    fn bounded_inputs_report_missing_invalid_oversize_and_io_without_raw_content() {
        let (home, _) = fixture();
        let cases = [
            (b"{}".to_vec(), "missing_fields"),
            (b"{\"session_id\":42}".to_vec(), "invalid"),
            (b"{\"hook_event_name\":\"Stop\"}".to_vec(), "invalid"),
            (b"[]".to_vec(), "invalid"),
            (Vec::new(), "unavailable"),
            (vec![b'x'; MAX_INPUT_BYTES as usize + 10], "oversize"),
        ];
        for (input, expected) in cases {
            let id = capture(home.path(), "user_prompt_submit_hook", input.as_slice())
                .unwrap()
                .unwrap();
            let observed = record(home.path(), &id);
            assert_eq!(observed.payload_status, expected);
            if expected == "missing_fields" {
                assert!(observed.missing_fields.contains(&"turn_id".into()));
            } else {
                assert!(observed.native_evidence_sha256.is_none());
                assert!(observed.session_hash.is_none());
            }
        }
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("PRIVATE_IO_SENTINEL"))
            }
        }
        let id = capture(home.path(), "stop_hook", Broken).unwrap().unwrap();
        assert_eq!(record(home.path(), &id).payload_status, "unavailable");
        let observed = status(home.path());
        assert_eq!(observed["coverage"], "partial");
        assert_eq!(observed["incomplete_payload_count"], 7);
        assert!(!observed.to_string().contains("PRIVATE_IO_SENTINEL"));
    }

    #[test]
    fn snapshot_is_registered_disk_evidence_and_contention_preserves_unknown() {
        let (home, profile) = fixture();
        let (state, path) = environment(home.path(), &profile);
        let id = capture(
            home.path(),
            "session_start_hook",
            payload("SessionStart", "turn").as_slice(),
        )
        .unwrap()
        .unwrap();
        let observed = record(home.path(), &id);
        assert_eq!(observed.targets[0].snapshot_code, "observed");
        assert_eq!(
            observed.targets[0].skill_revision,
            Some(bytes_sha256(b"registered skill\n"))
        );
        assert!(observed.targets[0].environment_revision.is_none());
        assert_eq!(
            observed.targets[0].source_revision.as_deref(),
            Some("source-1")
        );
        assert_eq!(
            observed.targets[0].baseline_sha256.as_deref(),
            Some("74d279a5ca266050b679158da38cc85a6eccb85c063b09a53e7977fd16eed401")
        );
        let head: Value = state.json("head.json", 4096).unwrap().unwrap();
        let expected_observation = bytes_sha256(
            &serialized(&json!({
                "state_identity": [state.identity.0, state.identity.1],
                "baseline_sha256": head["baseline_sha256"],
                "bindings_sha256": head["bindings_sha256"],
                "registry_sha256": head["registry_sha256"],
                "operations": {},
            }))
            .unwrap(),
        );
        assert_eq!(
            observed.targets[0].observation_state_sha256,
            Some(expected_observation)
        );
        let _lock = state.lock("writer.lock", false).unwrap();
        let busy = capture(home.path(), "stop_hook", payload("Stop", "turn").as_slice())
            .unwrap()
            .unwrap();
        let unknown = record(home.path(), &busy);
        assert_eq!(unknown.targets[0].snapshot_code, "busy");
        assert!(unknown.targets[0].skill_revision.is_none());
        assert!(unknown.targets[0].observation_state_sha256.is_none());
        assert_eq!(std::fs::read(path).unwrap(), b"registered skill\n");
    }

    #[test]
    fn snapshot_fingerprint_tracks_namespace_and_operation_bytes() {
        let (home, profile) = fixture();
        let (state, _) = environment(home.path(), &profile);
        let first_id = capture(
            home.path(),
            "user_prompt_submit_hook",
            payload("UserPromptSubmit", "turn").as_slice(),
        )
        .unwrap()
        .unwrap();
        let first = record(home.path(), &first_id);
        let operations = state.child("operations", true, true).unwrap().unwrap();
        operations
            .write("operation-1.json", b"{\"observation\":1}\n", false)
            .unwrap();
        operations
            .write(".operation.tmp", b"ignored temporary\n", false)
            .unwrap();
        let second_id = capture(
            home.path(),
            "user_prompt_submit_hook",
            payload("UserPromptSubmit", "turn").as_slice(),
        )
        .unwrap()
        .unwrap();
        let second = record(home.path(), &second_id);
        let head: Value = state.json("head.json", 4096).unwrap().unwrap();
        assert_eq!(
            second.targets[0].observation_state_sha256,
            Some(bytes_sha256(
                &serialized(&json!({
                    "state_identity": [state.identity.0, state.identity.1],
                    "baseline_sha256": head["baseline_sha256"],
                    "bindings_sha256": head["bindings_sha256"],
                    "registry_sha256": head["registry_sha256"],
                    "operations": {"operation-1.json":bytes_sha256(b"{\"observation\":1}\n")},
                }))
                .unwrap()
            ))
        );
        operations
            .write("operation-1.json", b"{\"observation\":2}\n", true)
            .unwrap();
        let third_id = capture(home.path(), "stop_hook", payload("Stop", "turn").as_slice())
            .unwrap()
            .unwrap();
        let third = record(home.path(), &third_id);
        for observed in [&first, &second, &third] {
            assert_eq!(observed.targets[0].snapshot_code, "observed");
            assert_eq!(
                observed.targets[0].skill_revision,
                first.targets[0].skill_revision
            );
            assert_eq!(
                observed.targets[0].baseline_sha256,
                first.targets[0].baseline_sha256
            );
        }
        assert_ne!(
            first.targets[0].observation_state_sha256,
            second.targets[0].observation_state_sha256
        );
        assert_ne!(
            second.targets[0].observation_state_sha256,
            third.targets[0].observation_state_sha256
        );
        assert_eq!(
            record(home.path(), &first_id).targets[0].observation_state_sha256,
            first.targets[0].observation_state_sha256
        );

        let (other_home, other_profile) = fixture();
        environment(other_home.path(), &other_profile);
        let other_id = capture(
            other_home.path(),
            "user_prompt_submit_hook",
            payload("UserPromptSubmit", "turn").as_slice(),
        )
        .unwrap()
        .unwrap();
        let other = record(other_home.path(), &other_id);
        assert_eq!(
            other.targets[0].skill_revision,
            first.targets[0].skill_revision
        );
        assert_eq!(
            other.targets[0].baseline_sha256,
            first.targets[0].baseline_sha256
        );
        assert_ne!(
            other.targets[0].observation_state_sha256,
            first.targets[0].observation_state_sha256
        );
    }

    #[test]
    fn snapshot_operation_limits_and_links_preserve_unknown() {
        let (home, profile) = fixture();
        let (state, _) = environment(home.path(), &profile);
        let operations = state.child("operations", true, true).unwrap().unwrap();
        let large_path = operations.path.join("large.json");
        std::fs::write(
            &large_path,
            vec![b'x'; MAX_OPERATION_TOTAL_BYTES as usize + 1],
        )
        .unwrap();
        std::fs::set_permissions(&large_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let oversized_id = capture(home.path(), "stop_hook", payload("Stop", "turn").as_slice())
            .unwrap()
            .unwrap();
        let oversized = record(home.path(), &oversized_id);
        assert_eq!(oversized.targets[0].snapshot_code, "oversize");
        assert!(oversized.targets[0].observation_state_sha256.is_none());
        assert!(oversized.targets[0].skill_revision.is_none());
        std::fs::remove_file(operations.path.join("large.json")).unwrap();
        std::os::unix::fs::symlink(
            state.path.join("head.json"),
            operations.path.join("linked.json"),
        )
        .unwrap();
        let linked_id = capture(home.path(), "stop_hook", payload("Stop", "turn").as_slice())
            .unwrap()
            .unwrap();
        let linked = record(home.path(), &linked_id);
        assert_eq!(linked.targets[0].snapshot_code, "unsafe_local_state");
        assert!(linked.targets[0].observation_state_sha256.is_none());
        std::fs::remove_file(operations.path.join("linked.json")).unwrap();
        for index in 0..=MAX_BOUNDARIES {
            operations
                .write(&format!("{index}.json"), b"{}\n", false)
                .unwrap();
        }
        let limited_id = capture(home.path(), "stop_hook", payload("Stop", "turn").as_slice())
            .unwrap()
            .unwrap();
        let limited = record(home.path(), &limited_id);
        assert_eq!(limited.targets[0].snapshot_code, "storage_limit");
        assert!(limited.targets[0].observation_state_sha256.is_none());
    }

    #[test]
    fn storage_limit_and_writer_contention_report_incomplete_without_overwrite() {
        let (home, _) = fixture();
        let directory = learning_directory(home.path()).unwrap().unwrap();
        let lock = directory.lock("boundary.lock", true).unwrap();
        assert_eq!(
            capture(home.path(), "stop_hook", b"{}".as_slice())
                .unwrap_err()
                .0,
            "busy"
        );
        drop(lock);
        let records = directory.child("boundaries", true, true).unwrap().unwrap();
        let one = native_input(payload("Stop", "turn").as_slice(), "Stop", &now());
        let original = serialized(&one).unwrap();
        for _ in 0..MAX_BOUNDARIES {
            records
                .write(&format!("{}.json", Uuid::new_v4()), &original, false)
                .unwrap();
        }
        let before = records.names().unwrap();
        assert_eq!(
            capture(home.path(), "stop_hook", b"{}".as_slice())
                .unwrap_err()
                .0,
            "storage_limit"
        );
        assert_eq!(records.names().unwrap().len(), before.len());
        for name in before {
            assert_eq!(
                records.read(&name, MAX_BOUNDARY_BYTES).unwrap().unwrap(),
                original
            );
        }
        let slot = read_status(
            &directory,
            "capture-status.json",
            "groundline-learning-boundary-capture-status",
        )
        .unwrap()
        .unwrap();
        assert_eq!(slot.failed_capture_count, 2);
        assert_eq!(slot.result_code, "storage_limit");
    }

    #[test]
    fn private_permissions_links_and_untrusted_status_are_rejected_read_only() {
        let fresh = tempdir().unwrap();
        assert_eq!(status(fresh.path())["state"], "not_configured");
        assert_eq!(
            capture(fresh.path(), "stop_hook", b"{}".as_slice()).unwrap(),
            None
        );
        assert_eq!(std::fs::read_dir(fresh.path()).unwrap().count(), 0);
        let (home, _) = fixture();
        let directory = learning_directory(home.path()).unwrap().unwrap();
        let profile = directory.path.join("profile.json");
        let original = std::fs::read(&profile).unwrap();
        std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(status(home.path())["state"], "unavailable");
        assert!(capture(home.path(), "stop_hook", b"{}".as_slice()).is_err());
        assert_eq!(std::fs::read(&profile).unwrap(), original);
        std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o600)).unwrap();
        directory
            .write(
                "worker-status.json",
                b"{\"private\":\"PRIVATE_STATUS_SENTINEL\"}",
                false,
            )
            .unwrap();
        let observed = status(home.path());
        assert_eq!(observed["state"], "unavailable");
        assert!(!observed.to_string().contains("PRIVATE_STATUS_SENTINEL"));
        std::fs::remove_file(directory.path.join("worker-status.json")).unwrap();
        let outside = home.path().join("outside");
        std::fs::write(&outside, b"PRIVATE_OUTSIDE_SENTINEL").unwrap();
        std::fs::remove_file(&profile).unwrap();
        std::os::unix::fs::symlink(&outside, &profile).unwrap();
        assert!(enabled(home.path()).is_err());
        assert_eq!(std::fs::read(outside).unwrap(), b"PRIVATE_OUTSIDE_SENTINEL");
    }

    #[tokio::test]
    async fn background_consumer_requires_opt_in_pin_and_does_not_depend_on_collection() {
        let (home, mut configured) = fixture();
        let directory = learning_directory(home.path()).unwrap().unwrap();
        let core = Path::new(&configured.core_executable);
        std::fs::write(
            core,
            format!(
                "#!/bin/sh\n[ \"$1\" = learning ] && [ \"$2\" = consume ] || exit 1\n{}",
                emit_readout(&consume_readout(vec![]))
            ),
        )
        .unwrap();
        std::fs::set_permissions(core, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            consume(home.path()).await["result_code"],
            "core_pin_mismatch"
        );
        configured.core_sha256 = bytes_sha256(&std::fs::read(core).unwrap());
        directory
            .write("profile.json", &serialized(&configured).unwrap(), true)
            .unwrap();
        let observed = consume(home.path()).await;
        assert_eq!(observed["result_code"], "consumed");
        assert_eq!(observed["core_status"], "CONSUMED");
        assert_eq!(observed["consume_attempt_count"], 1);
        assert_eq!(observed["pending_assessment_count"], 0);
        assert_eq!(observed["finalized_assessment_count"], 0);
        assert_eq!(status(home.path())["last_worker"], observed);
        assert!(!home.path().join("groundline/insights").exists());
        let _lock = directory.lock("consume.lock", true).unwrap();
        assert_eq!(consume(home.path()).await["result_code"], "busy");
        configured.enabled = false;
        directory
            .write("profile.json", &serialized(&configured).unwrap(), true)
            .unwrap();
        assert_eq!(consume(home.path()).await["result_code"], "not_enabled");
    }

    #[tokio::test]
    async fn stuck_consumer_is_terminated_and_its_local_lock_is_released() {
        let (home, mut configured) = fixture();
        let directory = learning_directory(home.path()).unwrap().unwrap();
        let core = Path::new(&configured.core_executable);
        std::fs::write(core, b"#!/bin/sh\nexec sleep 60\n").unwrap();
        std::fs::set_permissions(core, std::fs::Permissions::from_mode(0o700)).unwrap();
        configured.core_sha256 = bytes_sha256(&std::fs::read(core).unwrap());
        directory
            .write("profile.json", &serialized(&configured).unwrap(), true)
            .unwrap();
        let timeout = consume_with_limit(home.path(), Duration::from_millis(50)).await;
        assert_eq!(timeout["result_code"], "core_consume_timeout");
        let lock = directory.lock("consume.lock", false).unwrap();
        drop(lock);
        configure_core(
            home.path(),
            &mut configured,
            &emit_readout(&consume_readout(vec![])),
        );
        assert_eq!(
            consume_with_limit(home.path(), Duration::from_secs(1)).await["result_code"],
            "consumed"
        );
    }

    #[tokio::test]
    async fn transient_exit_zero_pending_retries_and_records_the_actual_finalization() {
        for reason in [
            "learning_boundary_busy",
            "learning_state_busy",
            "learning_profile_busy",
        ] {
            let (home, mut profile) = fixture();
            let script = format!(
                "if [ ! -e \"$4/first-attempt\" ]; then\n  : > \"$4/first-attempt\"\n{}else\n{}fi\n",
                emit_readout(&pending_readout(reason)),
                emit_readout(&consume_readout(vec![finalized_assessment()]))
            );
            configure_core(home.path(), &mut profile, &script);
            let observed = consume(home.path()).await;
            assert_eq!(observed["result_code"], "consumed", "{observed}");
            assert_eq!(observed["core_counts_scope"], "last_core_attempt");
            assert_eq!(observed["consume_attempt_count"], 2);
            assert_eq!(observed["retry_count"], 1);
            assert_eq!(observed["last_retry_reason"], reason);
            assert_eq!(observed["final_reason"], Value::Null);
            assert_eq!(observed["pending_assessment_count"], 0);
            assert_eq!(observed["finalized_assessment_count"], 1);
            assert_eq!(status(home.path())["last_worker"], observed);
        }
    }

    #[tokio::test]
    async fn native_wait_retries_are_bounded_and_pending_is_never_reported_as_consumed() {
        for reason in [
            "actual_native_end_or_artifact_required",
            "learning_response_source_changed",
        ] {
            let (home, mut profile) = fixture();
            configure_core(
                home.path(),
                &mut profile,
                &emit_readout(&consume_readout(vec![pending_assessment(reason)])),
            );
            let observed = consume(home.path()).await;
            assert_eq!(observed["result_code"], "pending", "{observed}");
            assert_eq!(observed["core_status"], "CONSUMED");
            assert_eq!(observed["consume_attempt_count"], MAX_CORE_ATTEMPTS);
            assert_eq!(observed["retry_count"], MAX_CORE_ATTEMPTS - 1);
            assert_eq!(observed["final_reason"], reason);
            assert_eq!(observed["pending_assessment_count"], 1);
            assert_eq!(observed["finalized_assessment_count"], 0);
            assert_eq!(status(home.path())["last_worker"], observed);
        }
    }

    #[tokio::test]
    async fn permanent_or_mixed_pending_does_not_retry_or_persist_arbitrary_reason_text() {
        for rows in [
            vec![pending_assessment("ownership_conflict")],
            vec![pending_assessment("PRIVATE_REASON /private/credentials")],
            vec![
                pending_assessment("actual_native_end_or_artifact_required"),
                pending_assessment("verification_evidence_changed"),
            ],
        ] {
            let (home, mut profile) = fixture();
            configure_core(
                home.path(),
                &mut profile,
                &emit_readout(&consume_readout(rows)),
            );
            let observed = consume(home.path()).await;
            assert_eq!(observed["result_code"], "pending");
            assert_eq!(observed["consume_attempt_count"], 1);
            assert_eq!(observed["retry_count"], 0);
            assert_eq!(observed["last_retry_reason"], Value::Null);
            assert_eq!(observed["final_reason"], "core_pending_other");
            let stored =
                std::fs::read_to_string(home.path().join("groundline/learning/worker-status.json"))
                    .unwrap();
            for raw in [
                "PRIVATE_REASON",
                "/private/credentials",
                "ownership_conflict",
                "verification_evidence_changed",
            ] {
                assert!(!stored.contains(raw));
            }
            assert_eq!(status(home.path())["last_worker"], observed);
        }
    }

    #[tokio::test]
    async fn malformed_or_oversized_output_is_rejected_privately_without_a_pipe_hang() {
        for script in [
            "printf '%s' 'PRIVATE_RAW_OUTPUT /private/credentials'\n".to_owned(),
            format!(
                "{}printf '%s' 'PRIVATE_TRAILING_RAW'\n",
                emit_readout(&consume_readout(vec![]))
            ),
            format!("head -c {} /dev/zero\n", MAX_CORE_OUTPUT_BYTES + 1),
        ] {
            let (home, mut profile) = fixture();
            let expected = if script.contains("head -c") {
                "core_output_oversize"
            } else {
                "core_output_invalid"
            };
            configure_core(home.path(), &mut profile, &script);
            let observed = consume_with_limit(home.path(), Duration::from_secs(2)).await;
            assert_eq!(observed["result_code"], expected, "{observed}");
            assert_eq!(observed["consume_attempt_count"], 1);
            assert_eq!(observed["core_status"], Value::Null);
            let stored =
                std::fs::read_to_string(home.path().join("groundline/learning/worker-status.json"))
                    .unwrap();
            assert!(!stored.contains("PRIVATE_"));
            assert!(!stored.contains("/private/credentials"));
            assert_eq!(status(home.path())["last_worker"], observed);
            drop(
                learning_directory(home.path())
                    .unwrap()
                    .unwrap()
                    .lock("consume.lock", false)
                    .unwrap(),
            );
        }
    }

    #[tokio::test]
    async fn streaming_readout_larger_than_the_pipe_buffer_and_resumed_finalization_are_observed() {
        let (home, mut profile) = fixture();
        let mut value = consume_readout(vec![finalized_assessment()]);
        value["readiness"] =
            json!({"ignored_bounded_projection":"PRIVATE_READINESS".repeat(12_000)});
        value["resumed_finalize_count"] = json!(2);
        configure_core(home.path(), &mut profile, &emit_readout(&value));
        let observed = consume_with_limit(home.path(), Duration::from_secs(2)).await;
        assert_eq!(observed["result_code"], "consumed", "{observed}");
        assert_eq!(observed["finalized_assessment_count"], 3);
        assert!(!observed.to_string().contains("PRIVATE_READINESS"));
    }

    #[tokio::test]
    async fn same_bytes_core_replacement_is_rejected_after_execution() {
        let (home, mut profile) = fixture();
        let script = format!(
            "cp \"$0\" \"$0.replacement\"\nmv \"$0.replacement\" \"$0\"\n{}",
            emit_readout(&consume_readout(vec![finalized_assessment()]))
        );
        configure_core(home.path(), &mut profile, &script);
        let observed = consume(home.path()).await;
        assert_eq!(observed["result_code"], "core_pin_mismatch");
        assert_eq!(observed["finalized_assessment_count"], Value::Null);
        assert_eq!(observed["consume_attempt_count"], 1);
        drop(
            learning_directory(home.path())
                .unwrap()
                .unwrap()
                .lock("consume.lock", false)
                .unwrap(),
        );
    }

    #[test]
    fn worker_status_rejects_unknown_fields_codes_and_inconsistent_attempt_metadata() {
        let (home, _) = fixture();
        let directory = learning_directory(home.path()).unwrap().unwrap();
        let valid = json!({"kind":"groundline-learning-worker-status","schema":1,"result_code":"pending",
            "observed_at_utc":now(),"core_status":"CONSUMED","core_counts_scope":"last_core_attempt","consume_attempt_count":4,"retry_count":3,
            "pending_assessment_count":1,"finalized_assessment_count":0,"last_retry_reason":"learning_boundary_busy",
            "final_reason":"actual_native_end_or_artifact_required","source_verified":false,
            "native_activation":"UNVERIFIED","network_performed":false});
        directory
            .write("worker-status.json", &serialized(&valid).unwrap(), true)
            .unwrap();
        assert!(read_worker_status(&directory).unwrap().is_some());
        for (field, value) in [
            ("private", json!("PRIVATE_STATUS")),
            ("result_code", json!("invented_success")),
            ("core_counts_scope", json!("all_worker_attempts")),
            ("retry_count", json!(0)),
            ("consume_attempt_count", json!(5)),
            ("result_code", json!("consumed")),
            ("final_reason", json!("PRIVATE_REASON")),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            directory
                .write("worker-status.json", &serialized(&invalid).unwrap(), true)
                .unwrap();
            assert_eq!(
                read_worker_status(&directory).err().unwrap().0,
                "invalid_state"
            );
        }
    }

    #[test]
    fn archived_evidence_does_not_fill_hot_capture_queue_or_claim_full_coverage() {
        let (home, _) = fixture();
        let id = capture(home.path(), "stop_hook", payload("Stop", "turn").as_slice())
            .unwrap()
            .unwrap();
        let directory = learning_directory(home.path()).unwrap().unwrap();
        let records = directory.child("boundaries", true, false).unwrap().unwrap();
        let archive = records.child("archive", true, true).unwrap().unwrap();
        let shard = archive
            .child(&bytes_sha256(id.as_bytes())[..2], true, true)
            .unwrap()
            .unwrap();
        let name = format!("{id}.json");
        let original = records.read(&name, MAX_BOUNDARY_BYTES).unwrap().unwrap();
        renameat_with(
            &records.file,
            name.as_str(),
            &shard.file,
            name.as_str(),
            RenameFlags::NOREPLACE,
        )
        .unwrap();
        assert_eq!(records.names().unwrap().len(), 0);
        let observed = status(home.path());
        assert_eq!(observed["boundary_count"], 0);
        assert_eq!(observed["archive_coverage"], "uninspected");
        assert_eq!(observed["last_capture"]["captured_count"], 1);
        assert_eq!(
            shard.read(&name, MAX_BOUNDARY_BYTES).unwrap().unwrap(),
            original
        );
        capture(
            home.path(),
            "stop_hook",
            payload("Stop", "next-turn").as_slice(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(status(home.path())["boundary_count"], 1);
        assert_eq!(
            shard.read(&name, MAX_BOUNDARY_BYTES).unwrap().unwrap(),
            original
        );
    }
}
