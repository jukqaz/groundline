use super::Command;
use super::files::{self, Candidate, Directory, Identity, ReadFile, error};
use groundline_contracts::ContractError;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_TARGETS: usize = 64;

#[cfg(test)]
thread_local! {
    static AFTER_ROLLBACK_PREPARE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn id(value: &str) -> Result<(), ContractError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b))
    {
        Err(error("invalid_identifier"))
    } else {
        Ok(())
    }
}
fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ContractError> {
    serde_json::from_slice(bytes).map_err(|_| error("invalid_document"))
}
fn bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ContractError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|_| error("serialization"))?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn contract(kind: &str, schema: u8, expected: &str) -> Result<(), ContractError> {
    if kind == expected && schema == 1 {
        Ok(())
    } else {
        Err(error("unsupported_document"))
    }
}
fn output(operation: &str) -> Value {
    json!({"kind":"groundline-environment-result","schema":1,"operation":operation,
        "native_activation":"UNVERIFIED","active_revision":null,"model_changed":false,
        "permissions_changed":false,"network_performed":false,"raw_content_emitted":false,
        "private_paths_emitted":false})
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TargetKind {
    SkillFile,
    AgentsBlock,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Block {
    start_marker: String,
    end_marker: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ManagedTarget {
    target_id: String,
    kind: TargetKind,
    desired_sha256: String,
    dependencies: Vec<String>,
    managed_block: Option<Block>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Baseline {
    kind: String,
    schema: u8,
    revision: String,
    parent_revision: Option<String>,
    source_revision: String,
    authority_revision: String,
    exception_revision: String,
    authority_ref: String,
    managed_targets: Vec<ManagedTarget>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RootInput {
    root_id: String,
    path: PathBuf,
    aliases: Vec<PathBuf>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TargetInput {
    target_id: String,
    root_id: String,
    relative_path: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Bindings {
    kind: String,
    schema: u8,
    revision: String,
    device_id: String,
    roots: Vec<RootInput>,
    targets: Vec<TargetInput>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisteredRoot {
    input: RootInput,
    canonical_path: PathBuf,
    binding: Identity,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisteredTarget {
    input: TargetInput,
    parent_binding: Identity,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    kind: String,
    schema: u8,
    device_id: String,
    roots: Vec<RegisteredRoot>,
    targets: Vec<RegisteredTarget>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Head {
    kind: String,
    schema: u8,
    revision: String,
    parent_revision: Option<String>,
    baseline_sha256: String,
    bindings_sha256: String,
    registry_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    target_id: String,
    content: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Proposal {
    kind: String,
    schema: u8,
    proposal_id: String,
    basis_revision: String,
    source_revision: String,
    authority_ref: String,
    changes: Vec<Change>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency {
    target_id: String,
    desired_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanEntry {
    target_id: String,
    before_sha256: Option<String>,
    after_sha256: String,
    desired_sha256: String,
    before_leaf: Option<Identity>,
    parent_binding: Identity,
    before_content: Option<String>,
    after_content: String,
    exact_diff: String,
    dependencies: Vec<Dependency>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    kind: String,
    schema: u8,
    proposal_id: String,
    basis_revision: String,
    source_revision: String,
    authority_ref: String,
    authority_revision: String,
    exception_revision: String,
    baseline_sha256: String,
    bindings_sha256: String,
    registry_sha256: String,
    device_id: String,
    entries: Vec<PlanEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationEntry {
    target_id: String,
    before_sha256: Option<String>,
    after_sha256: Option<String>,
    status: String,
    backup_ref: Option<String>,
    before_leaf: Option<Identity>,
    after_leaf: Option<Identity>,
    parent_binding: Identity,
    after_content: Option<String>,
    dependencies: Vec<Dependency>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    kind: String,
    schema: u8,
    operation_id: String,
    proposal_id: String,
    basis_revision: String,
    source_revision: String,
    authority_ref: String,
    authority_revision: String,
    exception_revision: String,
    plan_sha256: String,
    baseline_sha256: String,
    bindings_sha256: String,
    registry_sha256: String,
    device_id: String,
    action: String,
    original_operation_id: Option<String>,
    entries: Vec<OperationEntry>,
    status: String,
    native_activation: String,
}

struct Store {
    root: Directory,
}
struct Current {
    head: Head,
    baseline: Baseline,
    registry: Registry,
}
impl Store {
    fn open(path: &Path, create: bool) -> Result<Self, ContractError> {
        Ok(Self {
            root: files::open_state(path, create)?,
        })
    }
    fn artifacts(&self, directory: &str, create: bool) -> Result<Directory, ContractError> {
        self.root.child(directory, create)
    }
    fn head(&self) -> Result<Option<Head>, ContractError> {
        self.root
            .read("head.json")?
            .map(|r| parse(&r.bytes))
            .transpose()
    }
    fn current(&self) -> Result<Current, ContractError> {
        let head = self.head()?.ok_or_else(|| error("not_registered"))?;
        contract(&head.kind, head.schema, "groundline-environment-head")?;
        let snapshots = self.artifacts("snapshots", false)?;
        let baseline: Baseline = parse(&artifact(
            &snapshots,
            &format!("baseline-{}.json", head.baseline_sha256),
            &head.baseline_sha256,
        )?)?;
        let bindings: Bindings = parse(&artifact(
            &snapshots,
            &format!("bindings-{}.json", head.bindings_sha256),
            &head.bindings_sha256,
        )?)?;
        let registry: Registry = parse(&artifact(
            &snapshots,
            &format!("registry-{}.json", head.registry_sha256),
            &head.registry_sha256,
        )?)?;
        validate_baseline(&baseline)?;
        validate_bindings(&baseline, &bindings)?;
        contract(
            &registry.kind,
            registry.schema,
            "groundline-environment-registry",
        )?;
        if head.revision != baseline.revision
            || registry.device_id != bindings.device_id
            || registry.roots.len() != bindings.roots.len()
            || registry.targets.len() != bindings.targets.len()
            || registry
                .roots
                .iter()
                .map(|r| &r.input)
                .ne(bindings.roots.iter())
            || registry
                .targets
                .iter()
                .map(|r| &r.input)
                .ne(bindings.targets.iter())
        {
            return Err(error("snapshot_binding_mismatch"));
        }
        for t in &registry.targets {
            let root = root_for(&registry, t)?;
            allowed_path(
                &target(&baseline, &t.input.target_id)?.kind,
                &root.canonical_path.join(&t.input.relative_path),
            )?;
        }
        Ok(Current {
            head,
            baseline,
            registry,
        })
    }
    fn check(&self, plan: &Plan) -> Result<Current, ContractError> {
        let current = self.current()?;
        if current.head.revision != plan.basis_revision
            || current.head.baseline_sha256 != plan.baseline_sha256
            || current.head.bindings_sha256 != plan.bindings_sha256
            || current.head.registry_sha256 != plan.registry_sha256
            || current.baseline.authority_revision != plan.authority_revision
            || current.baseline.exception_revision != plan.exception_revision
            || current.baseline.authority_ref != plan.authority_ref
            || current.baseline.source_revision != plan.source_revision
            || current.registry.device_id != plan.device_id
        {
            return Err(error("stale_basis_or_authority"));
        }
        Ok(current)
    }
    fn plan(&self, proposal_id: &str) -> Result<(Plan, String), ContractError> {
        id(proposal_id)?;
        let raw = self
            .artifacts("plans", false)?
            .read(&format!("{proposal_id}.json"))?
            .ok_or_else(|| error("plan_missing"))?
            .bytes;
        let plan: Plan = parse(&raw)?;
        if plan.proposal_id != proposal_id {
            return Err(error("plan_id_mismatch"));
        }
        validate_plan(&plan)?;
        Ok((plan, hash(&raw)))
    }
    fn operation(&self, operation_id: &str) -> Result<Operation, ContractError> {
        id(operation_id)?;
        let raw = self
            .artifacts("operations", false)?
            .read(&format!("{operation_id}.json"))?
            .ok_or_else(|| error("operation_missing"))?
            .bytes;
        let op: Operation = parse(&raw)?;
        check_operation(&op)?;
        if op.operation_id != operation_id {
            return Err(error("operation_id_mismatch"));
        }
        Ok(op)
    }
    fn save_operation(&self, op: &Operation, replace: bool) -> Result<(), ContractError> {
        check_operation(op)?;
        self.artifacts("operations", true)?.write(
            &format!("{}.json", op.operation_id),
            &bytes(op)?,
            replace,
        )
    }
}

fn artifact(dir: &Directory, name: &str, digest: &str) -> Result<Vec<u8>, ContractError> {
    if !valid_hash(digest) {
        return Err(error("invalid_digest"));
    }
    let raw = dir
        .read(name)?
        .ok_or_else(|| error("snapshot_missing"))?
        .bytes;
    if hash(&raw) != digest {
        return Err(error("snapshot_digest_mismatch"));
    }
    Ok(raw)
}
fn target<'a>(baseline: &'a Baseline, target_id: &str) -> Result<&'a ManagedTarget, ContractError> {
    baseline
        .managed_targets
        .iter()
        .find(|t| t.target_id == target_id)
        .ok_or_else(|| error("target_not_registered"))
}
fn binding<'a>(
    registry: &'a Registry,
    target_id: &str,
) -> Result<&'a RegisteredTarget, ContractError> {
    registry
        .targets
        .iter()
        .find(|t| t.input.target_id == target_id)
        .ok_or_else(|| error("target_not_registered"))
}
fn root_for<'a>(
    registry: &'a Registry,
    target: &RegisteredTarget,
) -> Result<&'a RegisteredRoot, ContractError> {
    registry
        .roots
        .iter()
        .find(|r| r.input.root_id == target.input.root_id)
        .ok_or_else(|| error("root_not_registered"))
}
fn open_root(root: &RegisteredRoot) -> Result<Directory, ContractError> {
    let current = files::canonical_dir(&root.canonical_path, false)?;
    if current.binding != root.binding {
        return Err(error("root_binding_changed"));
    }
    for logical in std::iter::once(&root.input.path).chain(root.input.aliases.iter()) {
        let alias = files::canonical_dir(logical, false)?;
        if alias.path != root.canonical_path || alias.binding != root.binding {
            return Err(error("alias_binding_changed"));
        }
    }
    Ok(current)
}
fn open_target(
    registry: &Registry,
    target_id: &str,
) -> Result<(Directory, String, Option<ReadFile>), ContractError> {
    let binding = binding(registry, target_id)?;
    let root = open_root(root_for(registry, binding)?)?;
    let parent = files::target_parent(&root, &binding.input.relative_path)?;
    if parent.binding != binding.parent_binding {
        return Err(error("parent_binding_changed"));
    }
    let leaf = binding
        .input
        .relative_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| error("invalid_relative_path"))?
        .to_owned();
    let content = files::read_at(&parent.file, Path::new(&leaf), false, files::MAX_BYTES)?;
    Ok((parent, leaf, content))
}

fn validate_baseline(baseline: &Baseline) -> Result<(), ContractError> {
    contract(
        &baseline.kind,
        baseline.schema,
        "groundline-environment-baseline",
    )?;
    for value in [
        &baseline.revision,
        &baseline.source_revision,
        &baseline.authority_revision,
        &baseline.exception_revision,
        &baseline.authority_ref,
    ] {
        id(value)?;
    }
    if let Some(parent) = &baseline.parent_revision {
        id(parent)?;
        if parent == &baseline.revision {
            return Err(error("self_parent_revision"));
        }
    }
    if baseline.managed_targets.is_empty() || baseline.managed_targets.len() > MAX_TARGETS {
        return Err(error("target_limit"));
    }
    let mut ids = BTreeSet::new();
    for t in &baseline.managed_targets {
        id(&t.target_id)?;
        if !ids.insert(&t.target_id)
            || !valid_hash(&t.desired_sha256)
            || t.dependencies.len() > MAX_TARGETS
        {
            return Err(error("invalid_managed_target"));
        }
        match (&t.kind, &t.managed_block) {
            (TargetKind::SkillFile, None) => {}
            (TargetKind::AgentsBlock, Some(block))
                if !block.start_marker.is_empty()
                    && !block.end_marker.is_empty()
                    && block.start_marker.len() <= 200
                    && block.end_marker.len() <= 200
                    && !block.start_marker.contains(['\n', '\r', '\0'])
                    && !block.end_marker.contains(['\n', '\r', '\0'])
                    && block.start_marker != block.end_marker => {}
            _ => return Err(error("invalid_managed_block")),
        }
    }
    for t in &baseline.managed_targets {
        let mut deps = BTreeSet::new();
        for dep in &t.dependencies {
            id(dep)?;
            if dep == &t.target_id || !ids.contains(dep) || !deps.insert(dep) {
                return Err(error("invalid_dependency"));
            }
        }
    }
    let all = baseline
        .managed_targets
        .iter()
        .map(|t| t.target_id.clone())
        .collect();
    topo(baseline, &all)?;
    Ok(())
}
fn validate_bindings(baseline: &Baseline, bindings: &Bindings) -> Result<(), ContractError> {
    contract(
        &bindings.kind,
        bindings.schema,
        "groundline-environment-device-bindings",
    )?;
    id(&bindings.revision)?;
    id(&bindings.device_id)?;
    if bindings.revision != baseline.exception_revision
        || bindings.roots.is_empty()
        || bindings.roots.len() > MAX_TARGETS
        || bindings.targets.len() != baseline.managed_targets.len()
    {
        return Err(error("bindings_revision_or_scope"));
    }
    let mut roots = BTreeSet::new();
    for root in &bindings.roots {
        id(&root.root_id)?;
        if !roots.insert(&root.root_id)
            || !root.path.is_absolute()
            || root.aliases.len() > 8
            || root.aliases.iter().any(|a| !a.is_absolute())
        {
            return Err(error("invalid_root_binding"));
        }
    }
    let mut targets = BTreeSet::new();
    for t in &bindings.targets {
        id(&t.target_id)?;
        files::relative(&t.relative_path)?;
        target(baseline, &t.target_id)?;
        if !targets.insert(&t.target_id) || !roots.contains(&t.root_id) {
            return Err(error("invalid_target_binding"));
        }
    }
    Ok(())
}

fn protected_path(path: &Path) -> bool {
    let names: Vec<_> = path
        .components()
        .filter_map(|c| {
            if let std::path::Component::Normal(n) = c {
                n.to_str().map(|n| n.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect();
    names.iter().any(|n| {
        matches!(
            n.as_str(),
            "config.toml"
                | "auth.json"
                | "credentials.json"
                | "permissions"
                | "memories"
                | "memory"
                | "sessions"
                | "session_index.jsonl"
                | "cache"
                | "providers"
                | "bundled"
                | "automation.toml"
                | ".system"
                | "hub"
                | "hubs"
        )
    }) || path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|e| ["db", "sqlite", "sqlite3"].contains(&e.to_ascii_lowercase().as_str()))
        || names
            .windows(2)
            .any(|p| p[0] == "plugins" && p[1] == "cache")
}
fn allowed_path(kind: &TargetKind, path: &Path) -> Result<(), ContractError> {
    if protected_path(path) {
        return Err(error("protected_target_use_native_setup_or_config_repair"));
    }
    match kind {
        TargetKind::AgentsBlock
            if path.file_name().and_then(|n| n.to_str()) == Some("AGENTS.md") =>
        {
            Ok(())
        }
        TargetKind::SkillFile
            if path.components().any(|p| p.as_os_str() == "skills")
                && !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("AGENTS.md")) =>
        {
            Ok(())
        }
        _ => Err(error("unsupported_target_kind_or_location")),
    }
}

fn build_registry(baseline: &Baseline, bindings: &Bindings) -> Result<Registry, ContractError> {
    let mut registry = Registry {
        kind: "groundline-environment-registry".into(),
        schema: 1,
        device_id: bindings.device_id.clone(),
        roots: vec![],
        targets: vec![],
    };
    for input in &bindings.roots {
        let current = files::canonical_dir(&input.path, false)?;
        let registered = RegisteredRoot {
            input: input.clone(),
            canonical_path: current.path,
            binding: current.binding,
        };
        open_root(&registered)?;
        registry.roots.push(registered);
    }
    let mut names = BTreeSet::new();
    for input in &bindings.targets {
        let root = registry
            .roots
            .iter()
            .find(|r| r.input.root_id == input.root_id)
            .ok_or_else(|| error("root_not_registered"))?;
        allowed_path(
            &target(baseline, &input.target_id)?.kind,
            &root.canonical_path.join(&input.relative_path),
        )?;
        let parent = files::target_parent(&open_root(root)?, &input.relative_path)?;
        let leaf = input
            .relative_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| error("invalid_relative_path"))?;
        if !names.insert((parent.binding.dev, parent.binding.ino, leaf.to_owned())) {
            return Err(error("duplicate_parent_leaf_binding"));
        }
        files::read_at(&parent.file, Path::new(leaf), false, files::MAX_BYTES)?;
        registry.targets.push(RegisteredTarget {
            input: input.clone(),
            parent_binding: parent.binding,
        });
    }
    Ok(registry)
}

fn topo(baseline: &Baseline, selected: &BTreeSet<String>) -> Result<Vec<String>, ContractError> {
    fn visit(
        id: &str,
        baseline: &Baseline,
        selected: &BTreeSet<String>,
        visiting: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), ContractError> {
        if done.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.to_owned()) {
            return Err(error("dependency_cycle"));
        }
        for dep in &target(baseline, id)?.dependencies {
            if selected.contains(dep) {
                visit(dep, baseline, selected, visiting, done, order)?;
            }
        }
        visiting.remove(id);
        done.insert(id.to_owned());
        order.push(id.to_owned());
        Ok(())
    }
    let mut visiting = BTreeSet::new();
    let mut done = BTreeSet::new();
    let mut order = Vec::new();
    for id in selected {
        visit(id, baseline, selected, &mut visiting, &mut done, &mut order)?;
    }
    Ok(order)
}

fn text(read: &Option<ReadFile>) -> Result<Option<String>, ContractError> {
    read.as_ref()
        .map(|r| String::from_utf8(r.bytes.clone()).map_err(|_| error("target_not_utf8")))
        .transpose()
}
fn block_range(text: &str, block: &Block) -> Result<Option<(usize, usize)>, ContractError> {
    let mut offset = 0;
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for line in text.split_inclusive('\n') {
        let plain = line
            .strip_suffix('\n')
            .unwrap_or(line)
            .strip_suffix('\r')
            .unwrap_or(line.strip_suffix('\n').unwrap_or(line));
        if plain == block.start_marker {
            starts.push(offset + line.len());
        }
        if plain == block.end_marker {
            ends.push(offset);
        }
        offset += line.len();
    }
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => Ok(None),
        ([start], [end]) if start <= end => Ok(Some((*start, *end))),
        _ => Err(error("ambiguous_managed_block")),
    }
}
fn managed_digest(
    t: &ManagedTarget,
    read: &Option<ReadFile>,
) -> Result<Option<String>, ContractError> {
    let Some(read) = read else {
        return Ok(None);
    };
    match &t.kind {
        TargetKind::SkillFile => Ok(Some(hash(&read.bytes))),
        TargetKind::AgentsBlock => {
            let text = std::str::from_utf8(&read.bytes).map_err(|_| error("target_not_utf8"))?;
            let range = block_range(
                text,
                t.managed_block
                    .as_ref()
                    .ok_or_else(|| error("invalid_managed_block"))?,
            )?;
            Ok(range.map(|(start, end)| hash(&text.as_bytes()[start..end])))
        }
    }
}
fn render(
    t: &ManagedTarget,
    previous: Option<&str>,
    content: &str,
) -> Result<String, ContractError> {
    if content.len() as u64 > files::MAX_BYTES || content.contains('\0') {
        return Err(error("invalid_target_content"));
    }
    if t.kind == TargetKind::SkillFile {
        return Ok(content.to_owned());
    }
    let block = t
        .managed_block
        .as_ref()
        .ok_or_else(|| error("invalid_managed_block"))?;
    if (!content.is_empty() && !content.ends_with('\n'))
        || content
            .lines()
            .any(|l| l == block.start_marker || l == block.end_marker)
    {
        return Err(error("managed_content_requires_complete_lines"));
    }
    let previous = previous.unwrap_or("");
    if let Some((start, end)) = block_range(previous, block)? {
        Ok(format!(
            "{}{}{}",
            &previous[..start],
            content,
            &previous[end..]
        ))
    } else {
        Ok(format!(
            "{}{}{}\n{}{}\n",
            previous,
            if !previous.is_empty() && !previous.ends_with('\n') {
                "\n"
            } else {
                ""
            },
            block.start_marker,
            content,
            block.end_marker
        ))
    }
}
fn exact_diff(target_id: &str, before: Option<&str>, after: &str) -> String {
    if before == Some(after) {
        return String::new();
    }
    let mut diff = format!("--- {target_id}:before\n+++ {target_id}:after\n");
    for (prefix, text) in [('-', before.unwrap_or("")), ('+', after)] {
        for line in text.split_inclusive('\n') {
            diff.push(prefix);
            diff.push_str(line);
            if !line.ends_with('\n') {
                diff.push_str("\n\\ No newline at end of file\n");
            }
        }
    }
    diff
}

pub(super) fn run(command: Command) -> Result<Value, ContractError> {
    match command {
        Command::Register {
            state_dir,
            baseline,
            bindings,
            ..
        } => register(&state_dir, &baseline, &bindings),
        Command::Inspect { state_dir, .. } => inspect(&state_dir),
        Command::Plan {
            state_dir,
            proposal,
            ..
        } => plan(&state_dir, &proposal),
        Command::Apply {
            state_dir,
            proposal_id,
            ..
        } => apply(&state_dir, &proposal_id),
        Command::Rollback {
            state_dir,
            operation_id,
            ..
        } => rollback(&state_dir, &operation_id),
        Command::Recover {
            state_dir,
            operation_id,
            ..
        } => recover(&state_dir, &operation_id),
    }
}

fn register(
    state_dir: &Path,
    baseline_input: &Path,
    bindings_input: &Path,
) -> Result<Value, ContractError> {
    let baseline: Baseline = parse(&files::private_input(baseline_input)?)?;
    let bindings: Bindings = parse(&files::private_input(bindings_input)?)?;
    validate_baseline(&baseline)?;
    validate_bindings(&baseline, &bindings)?;
    let registry = build_registry(&baseline, &bindings)?;
    let store = Store::open(state_dir, true)?;
    let _lock = store.root.lock()?;
    let previous = store.head()?;
    let baseline_raw = bytes(&baseline)?;
    let bindings_raw = bytes(&bindings)?;
    let registry_raw = bytes(&registry)?;
    let mut result = output("register");
    result["desired_revision"] = json!(baseline.revision);
    if let Some(previous) = &previous {
        let current = store.current()?;
        if previous.baseline_sha256 == hash(&baseline_raw)
            && previous.bindings_sha256 == hash(&bindings_raw)
            && previous.registry_sha256 == hash(&registry_raw)
        {
            result["status"] = json!("NO_CHANGE");
            result["mutation_performed"] = json!(false);
            return Ok(result);
        }
        if baseline.parent_revision.as_deref() != Some(previous.revision.as_str())
            || baseline.revision == previous.revision
        {
            return Err(error("stale_parent_revision"));
        }
        if (current.head.bindings_sha256 != hash(&bindings_raw)
            || current.head.registry_sha256 != hash(&registry_raw))
            && current.baseline.exception_revision == baseline.exception_revision
        {
            return Err(error("exception_revision_not_advanced"));
        }
        let old_scope: Vec<_> = current
            .baseline
            .managed_targets
            .iter()
            .map(|t| (&t.target_id, &t.kind, &t.managed_block))
            .collect();
        let new_scope: Vec<_> = baseline
            .managed_targets
            .iter()
            .map(|t| (&t.target_id, &t.kind, &t.managed_block))
            .collect();
        if old_scope != new_scope
            && current.baseline.authority_revision == baseline.authority_revision
        {
            return Err(error("authority_revision_not_advanced"));
        }
    } else if baseline.parent_revision.is_some() {
        return Err(error("initial_parent_must_be_null"));
    }
    let snapshots = store.artifacts("snapshots", true)?;
    let head = Head {
        kind: "groundline-environment-head".into(),
        schema: 1,
        revision: baseline.revision.clone(),
        parent_revision: baseline.parent_revision.clone(),
        baseline_sha256: hash(&baseline_raw),
        bindings_sha256: hash(&bindings_raw),
        registry_sha256: hash(&registry_raw),
    };
    snapshots.write(
        &format!("baseline-{}.json", head.baseline_sha256),
        &baseline_raw,
        false,
    )?;
    snapshots.write(
        &format!("bindings-{}.json", head.bindings_sha256),
        &bindings_raw,
        false,
    )?;
    snapshots.write(
        &format!("registry-{}.json", head.registry_sha256),
        &registry_raw,
        false,
    )?;
    if bytes(&store.head()?)? != bytes(&previous)? {
        return Err(error("stale_parent_revision"));
    }
    store.root.write("head.json", &bytes(&head)?, true)?;
    result["status"] = json!("REGISTERED");
    result["mutation_performed"] = json!(true);
    result["managed_target_count"] = json!(baseline.managed_targets.len());
    result["authority_revision"] = json!(baseline.authority_revision);
    result["exception_revision"] = json!(baseline.exception_revision);
    Ok(result)
}

fn inspect(state_dir: &Path) -> Result<Value, ContractError> {
    let store = Store::open(state_dir, false)?;
    let current = store.current()?;
    let mut result = output("inspect");
    let mut entries = Vec::new();
    for t in &current.baseline.managed_targets {
        match open_target(&current.registry, &t.target_id).and_then(|(_,_,r)| Ok((managed_digest(t,&r)?,r.as_ref().map(|r| hash(&r.bytes))))) {
            Ok((managed,disk)) => entries.push(json!({"target_id":t.target_id,"desired_sha256":t.desired_sha256,"disk_sha256":disk,"disk_managed_sha256":managed,"desired_matches_disk":managed.as_deref()==Some(t.desired_sha256.as_str()),"status":"OBSERVED","native_activation":"UNVERIFIED"})),
            Err(e) => entries.push(json!({"target_id":t.target_id,"desired_sha256":t.desired_sha256,"disk_sha256":null,"disk_managed_sha256":null,"desired_matches_disk":false,"status":"CONFLICT","reason":e.0,"native_activation":"UNVERIFIED"})),
        }
    }
    let matches = entries.iter().all(|e| e["desired_matches_disk"] == true);
    result["desired_revision"] = json!(current.head.revision);
    result["disk_revision"] = if matches {
        json!(current.head.revision)
    } else {
        Value::Null
    };
    result["disk_status"] = json!(if matches {
        "MATCHES_DESIRED"
    } else {
        "DRIFT_OR_CONFLICT"
    });
    result["entries"] = json!(entries);
    result["mutation_performed"] = json!(false);
    result["registered_alias_count"] = json!(
        current
            .registry
            .roots
            .iter()
            .map(|r| r.input.aliases.len())
            .sum::<usize>()
    );
    Ok(result)
}

fn plan(state_dir: &Path, proposal_input: &Path) -> Result<Value, ContractError> {
    let proposal: Proposal = parse(&files::private_input(proposal_input)?)?;
    contract(
        &proposal.kind,
        proposal.schema,
        "groundline-environment-proposal",
    )?;
    id(&proposal.proposal_id)?;
    id(&proposal.basis_revision)?;
    id(&proposal.source_revision)?;
    id(&proposal.authority_ref)?;
    if proposal.proposal_id.len() > 61 {
        return Err(error("proposal_id_too_long"));
    }
    if proposal.changes.is_empty() || proposal.changes.len() > MAX_TARGETS {
        return Err(error("proposal_change_limit"));
    }
    let store = Store::open(state_dir, false)?;
    let _lock = store.root.lock()?;
    let current = store.current()?;
    if current.head.revision != proposal.basis_revision
        || current.baseline.source_revision != proposal.source_revision
        || current.baseline.authority_ref != proposal.authority_ref
    {
        return Err(error("stale_basis_or_authority"));
    }
    let mut changes = BTreeMap::new();
    for change in proposal.changes {
        id(&change.target_id)?;
        if changes.insert(change.target_id.clone(), change).is_some() {
            return Err(error("duplicate_proposal_target"));
        }
    }
    let selected = changes.keys().cloned().collect();
    let mut plan = Plan {
        kind: "groundline-environment-plan".into(),
        schema: 1,
        proposal_id: proposal.proposal_id,
        basis_revision: proposal.basis_revision,
        source_revision: proposal.source_revision,
        authority_ref: proposal.authority_ref,
        authority_revision: current.baseline.authority_revision.clone(),
        exception_revision: current.baseline.exception_revision.clone(),
        baseline_sha256: current.head.baseline_sha256.clone(),
        bindings_sha256: current.head.bindings_sha256.clone(),
        registry_sha256: current.head.registry_sha256.clone(),
        device_id: current.registry.device_id.clone(),
        entries: vec![],
    };
    for target_id in topo(&current.baseline, &selected)? {
        let change = changes
            .get(&target_id)
            .ok_or_else(|| error("proposal_target_missing"))?;
        let t = target(&current.baseline, &target_id)?;
        if hash(change.content.as_bytes()) != t.desired_sha256 {
            return Err(error("content_does_not_match_desired_baseline"));
        }
        let (parent, _, read) = open_target(&current.registry, &target_id)?;
        let before_content = text(&read)?;
        let after = render(t, before_content.as_deref(), &change.content)?;
        if after.len() as u64 > files::MAX_BYTES {
            return Err(error("file_too_large"));
        }
        let dependencies = t
            .dependencies
            .iter()
            .map(|dep| {
                Ok(Dependency {
                    target_id: dep.clone(),
                    desired_sha256: target(&current.baseline, dep)?.desired_sha256.clone(),
                })
            })
            .collect::<Result<Vec<_>, ContractError>>()?;
        plan.entries.push(PlanEntry {
            target_id: target_id.clone(),
            before_sha256: read.as_ref().map(|r| hash(&r.bytes)),
            after_sha256: hash(after.as_bytes()),
            desired_sha256: t.desired_sha256.clone(),
            before_leaf: read.map(|r| r.binding),
            parent_binding: parent.binding,
            exact_diff: exact_diff(&target_id, before_content.as_deref(), &after),
            before_content,
            after_content: after,
            dependencies,
        });
    }
    validate_plan(&plan)?;
    store.check(&plan)?;
    let raw = bytes(&plan)?;
    let digest = hash(&raw);
    store
        .artifacts("plans", true)?
        .write(&format!("{}.json", plan.proposal_id), &raw, false)?;
    let mut result = output("plan");
    result["proposal_id"] = json!(plan.proposal_id);
    result["basis_revision"] = json!(plan.basis_revision);
    result["source_revision"] = json!(plan.source_revision);
    result["plan_sha256"] = json!(digest);
    result["status"] = json!("PLANNED");
    result["mutation_performed"] = json!(false);
    result["private_plan_saved"] = json!(true);
    result["entries"] = json!(plan.entries.iter().map(|e| json!({"target_id":e.target_id,"before_sha256":e.before_sha256,"after_sha256":e.after_sha256,"change_required":e.before_sha256.as_deref()!=Some(e.after_sha256.as_str()),"dependencies":e.dependencies})).collect::<Vec<_>>());
    Ok(result)
}

fn validate_plan(plan: &Plan) -> Result<(), ContractError> {
    contract(&plan.kind, plan.schema, "groundline-environment-plan")?;
    for value in [
        &plan.proposal_id,
        &plan.basis_revision,
        &plan.source_revision,
        &plan.authority_ref,
        &plan.authority_revision,
        &plan.exception_revision,
        &plan.device_id,
    ] {
        id(value)?;
    }
    if plan.proposal_id.len() > 61
        || ![
            &plan.baseline_sha256,
            &plan.bindings_sha256,
            &plan.registry_sha256,
        ]
        .into_iter()
        .all(|h| valid_hash(h))
        || plan.entries.is_empty()
        || plan.entries.len() > MAX_TARGETS
    {
        return Err(error("invalid_plan"));
    }
    let mut ids = BTreeSet::new();
    for e in &plan.entries {
        id(&e.target_id)?;
        if !ids.insert(&e.target_id)
            || !valid_hash(&e.after_sha256)
            || !valid_hash(&e.desired_sha256)
            || e.before_sha256.as_ref().is_some_and(|h| !valid_hash(h))
            || e.before_sha256 != e.before_content.as_ref().map(|c| hash(c.as_bytes()))
            || e.before_leaf.is_some() != e.before_content.is_some()
            || hash(e.after_content.as_bytes()) != e.after_sha256
            || e.exact_diff
                != exact_diff(&e.target_id, e.before_content.as_deref(), &e.after_content)
        {
            return Err(error("invalid_plan_entry"));
        }
        for dep in &e.dependencies {
            id(&dep.target_id)?;
            if !valid_hash(&dep.desired_sha256) {
                return Err(error("invalid_dependency"));
            }
        }
    }
    Ok(())
}

fn check_operation(op: &Operation) -> Result<(), ContractError> {
    contract(&op.kind, op.schema, "groundline-environment-operation")?;
    for value in [
        &op.operation_id,
        &op.proposal_id,
        &op.basis_revision,
        &op.source_revision,
        &op.authority_ref,
        &op.authority_revision,
        &op.exception_revision,
        &op.device_id,
    ] {
        id(value)?;
    }
    if let Some(original) = &op.original_operation_id {
        id(original)?;
    }
    if ![
        &op.baseline_sha256,
        &op.bindings_sha256,
        &op.registry_sha256,
        &op.plan_sha256,
    ]
    .into_iter()
    .all(|h| valid_hash(h))
        || !["apply", "rollback"].contains(&op.action.as_str())
        || op.native_activation != "UNVERIFIED"
        || !["PREPARED", "APPLIED", "PARTIAL", "ROLLED_BACK"].contains(&op.status.as_str())
        || op.entries.is_empty()
        || op.entries.len() > MAX_TARGETS
    {
        return Err(error("invalid_operation"));
    }
    let mut ids = BTreeSet::new();
    for e in &op.entries {
        id(&e.target_id)?;
        if !ids.insert(&e.target_id)
            || e.before_sha256.as_ref().is_some_and(|h| !valid_hash(h))
            || e.after_sha256.as_ref().is_some_and(|h| !valid_hash(h))
            || e.after_sha256 != e.after_content.as_ref().map(|c| hash(c.as_bytes()))
            || e.before_leaf.is_some() != e.before_sha256.is_some()
            || ![
                "PREPARED",
                "APPLIED",
                "UNCHANGED",
                "CONFLICT",
                "UNVERIFIED",
                "DEPENDENCY_PENDING",
                "ROLLED_BACK",
                "RETAINED_DEPENDENCY",
                "DISK_OBSERVED_BEFORE",
                "DISK_OBSERVED_AFTER_ONLY",
            ]
            .contains(&e.status.as_str())
        {
            return Err(error("invalid_operation_entry"));
        }
        if let Some(backup) = &e.backup_ref {
            files::leaf(backup)?;
        }
        for dep in &e.dependencies {
            id(&dep.target_id)?;
            if !valid_hash(&dep.desired_sha256) {
                return Err(error("invalid_dependency"));
            }
        }
    }
    if op.status == "APPLIED"
        && op.entries.iter().any(|e| {
            !["APPLIED", "UNCHANGED"].contains(&e.status.as_str())
                || (e.after_sha256.is_some() && e.after_leaf.is_none())
        })
    {
        return Err(error("invalid_applied_receipt"));
    }
    if (op.action == "apply") != op.original_operation_id.is_none() {
        return Err(error("invalid_operation_action"));
    }
    Ok(())
}
pub(super) fn validate_operation(value: &Value) -> Result<(), ContractError> {
    let op: Operation =
        serde_json::from_value(value.clone()).map_err(|_| error("invalid_operation"))?;
    check_operation(&op)
}

fn bound_plan_entry(current: &Current, entry: &PlanEntry) -> Result<(), ContractError> {
    let t = target(&current.baseline, &entry.target_id)?;
    let declared: Vec<_> = t
        .dependencies
        .iter()
        .map(|d| {
            Ok((
                d.clone(),
                target(&current.baseline, d)?.desired_sha256.clone(),
            ))
        })
        .collect::<Result<_, ContractError>>()?;
    if t.desired_sha256 != entry.desired_sha256
        || binding(&current.registry, &entry.target_id)?.parent_binding != entry.parent_binding
        || declared
            != entry
                .dependencies
                .iter()
                .map(|d| (d.target_id.clone(), d.desired_sha256.clone()))
                .collect::<Vec<_>>()
    {
        return Err(error("plan_scope_or_dependency_changed"));
    }
    let actual = Some(ReadFile {
        bytes: entry.after_content.as_bytes().to_vec(),
        binding: Identity { dev: 0, ino: 0 },
        mode: 0o600,
    });
    if managed_digest(t, &actual)?.as_deref() != Some(t.desired_sha256.as_str()) {
        return Err(error("plan_content_not_authorized"));
    }
    if t.kind == TargetKind::AgentsBlock {
        let managed = block_range(
            &entry.after_content,
            t.managed_block
                .as_ref()
                .ok_or_else(|| error("invalid_managed_block"))?,
        )?
        .ok_or_else(|| error("managed_block_missing"))?;
        let rendered = render(
            t,
            entry.before_content.as_deref(),
            &entry.after_content[managed.0..managed.1],
        )?;
        if rendered != entry.after_content {
            return Err(error("unmanaged_content_changed"));
        }
    }
    Ok(())
}
fn before_matches(entry: &PlanEntry, read: &Option<ReadFile>) -> bool {
    entry.before_sha256 == read.as_ref().map(|r| hash(&r.bytes))
        && entry.before_leaf == read.as_ref().map(|r| r.binding.clone())
}
fn dependencies_ready(current: &Current, dependencies: &[Dependency]) -> Result<(), ContractError> {
    for dep in dependencies {
        let t = target(&current.baseline, &dep.target_id)?;
        if t.desired_sha256 != dep.desired_sha256 {
            return Err(error("dependency_revision_changed"));
        }
        let (_, _, read) = open_target(&current.registry, &dep.target_id)?;
        if managed_digest(t, &read)?.as_deref() != Some(dep.desired_sha256.as_str()) {
            return Err(error("dependency_not_ready"));
        }
    }
    Ok(())
}

fn operation_from_plan(plan: &Plan, digest: &str) -> Operation {
    Operation {
        kind: "groundline-environment-operation".into(),
        schema: 1,
        operation_id: format!("op-{}", plan.proposal_id),
        proposal_id: plan.proposal_id.clone(),
        basis_revision: plan.basis_revision.clone(),
        source_revision: plan.source_revision.clone(),
        authority_ref: plan.authority_ref.clone(),
        authority_revision: plan.authority_revision.clone(),
        exception_revision: plan.exception_revision.clone(),
        plan_sha256: digest.into(),
        baseline_sha256: plan.baseline_sha256.clone(),
        bindings_sha256: plan.bindings_sha256.clone(),
        registry_sha256: plan.registry_sha256.clone(),
        device_id: plan.device_id.clone(),
        action: "apply".into(),
        original_operation_id: None,
        entries: plan
            .entries
            .iter()
            .map(|e| OperationEntry {
                target_id: e.target_id.clone(),
                before_sha256: e.before_sha256.clone(),
                after_sha256: Some(e.after_sha256.clone()),
                status: if e.before_sha256.as_deref() == Some(e.after_sha256.as_str()) {
                    "UNCHANGED"
                } else {
                    "PREPARED"
                }
                .into(),
                backup_ref: None,
                before_leaf: e.before_leaf.clone(),
                after_leaf: if e.before_sha256.as_deref() == Some(e.after_sha256.as_str()) {
                    e.before_leaf.clone()
                } else {
                    None
                },
                parent_binding: e.parent_binding.clone(),
                after_content: Some(e.after_content.clone()),
                dependencies: e.dependencies.clone(),
            })
            .collect(),
        status: "PREPARED".into(),
        native_activation: "UNVERIFIED".into(),
    }
}
fn summary(op: &Operation, operation: &str, mutation: bool, receipt_saved: bool) -> Value {
    let mut result = output(operation);
    for (key, value) in [
        ("operation_id", json!(op.operation_id)),
        ("proposal_id", json!(op.proposal_id)),
        ("basis_revision", json!(op.basis_revision)),
        ("source_revision", json!(op.source_revision)),
        ("plan_sha256", json!(op.plan_sha256)),
        ("status", json!(op.status)),
        ("mutation_performed", json!(mutation)),
        ("receipt_saved", json!(receipt_saved)),
    ] {
        result[key] = value;
    }
    result["entries"]=json!(op.entries.iter().map(|e|json!({"target_id":e.target_id,"before_sha256":e.before_sha256,"after_sha256":e.after_sha256,"status":e.status})).collect::<Vec<_>>());
    result
}

fn apply(state_dir: &Path, proposal_id: &str) -> Result<Value, ContractError> {
    let store = Store::open(state_dir, false)?;
    let _lock = store.root.lock()?;
    let (plan, digest) = store.plan(proposal_id)?;
    let current = store.check(&plan)?;
    for entry in &plan.entries {
        bound_plan_entry(&current, entry)?;
    }
    let mut op = operation_from_plan(&plan, &digest);
    let no_change = op.entries.iter().all(|e| e.status == "UNCHANGED");
    if !no_change
        && store
            .artifacts("operations", true)?
            .read(&format!("{}.json", op.operation_id))?
            .is_some()
    {
        let existing = store.operation(&op.operation_id)?;
        if existing.plan_sha256 != digest {
            return Err(error("operation_plan_mismatch"));
        }
        let mut result = summary(&existing, "apply", false, true);
        result["status"] = json!("ALREADY_RECORDED");
        result["requires_inspect_or_recover"] = json!(true);
        return Ok(result);
    }
    for entry in &plan.entries {
        let (_, _, read) = open_target(&current.registry, &entry.target_id)?;
        if !before_matches(entry, &read) {
            return Err(error("target_changed_since_plan"));
        }
    }
    let selected: BTreeSet<_> = plan.entries.iter().map(|e| e.target_id.clone()).collect();
    for entry in &plan.entries {
        let external: Vec<_> = entry
            .dependencies
            .iter()
            .filter(|d| !selected.contains(&d.target_id))
            .cloned()
            .collect();
        dependencies_ready(&current, &external)?;
    }
    if no_change {
        let mut result = output("apply");
        result["proposal_id"] = json!(proposal_id);
        result["operation_id"] = Value::Null;
        result["basis_revision"] = json!(plan.basis_revision);
        result["plan_sha256"] = json!(digest);
        result["status"] = json!("NO_CHANGE");
        result["mutation_performed"] = json!(false);
        result["backup_created"] = json!(false);
        return Ok(result);
    }
    // Recovery evidence is durable before the first target temporary file is created.
    let backups = store.artifacts("backups", true)?;
    for (entry, record) in plan.entries.iter().zip(op.entries.iter_mut()) {
        if record.status == "UNCHANGED" {
            continue;
        }
        if let Some(before) = &entry.before_content {
            let name = format!("{}-{}.backup", op.operation_id, entry.target_id);
            backups.write(&name, before.as_bytes(), false)?;
            record.backup_ref = Some(name);
        }
    }
    store.save_operation(&op, false)?;
    let mut mutated = false;
    let mut stop = false;
    for index in 0..plan.entries.len() {
        if op.entries[index].status == "UNCHANGED" {
            continue;
        }
        if stop {
            op.entries[index].status = "DEPENDENCY_PENDING".into();
            continue;
        }
        let entry = &plan.entries[index];
        let mut renamed = false;
        let result = (|| {
            let current = store.check(&plan)?;
            bound_plan_entry(&current, entry)?;
            dependencies_ready(&current, &entry.dependencies)?;
            let (parent, leaf, read) = open_target(&current.registry, &entry.target_id)?;
            if !before_matches(entry, &read) {
                return Err(error("target_changed_since_plan"));
            }
            let mode = read.as_ref().map_or(0o600, |r| r.mode);
            let mut candidate = Candidate::new(&parent, entry.after_content.as_bytes(), mode)?;
            let current = store.check(&plan)?;
            let (last_parent, _, last) = open_target(&current.registry, &entry.target_id)?;
            if parent.binding != last_parent.binding || !before_matches(entry, &last) {
                return Err(error("target_changed_before_commit"));
            }
            let committed = candidate.commit(&parent, &leaf, entry.before_sha256.is_some());
            renamed = candidate.renamed();
            committed?;
            let (_, _, actual) = open_target(&current.registry, &entry.target_id)?;
            if actual.as_ref().map(|r| hash(&r.bytes)).as_deref()
                != Some(entry.after_sha256.as_str())
            {
                return Err(error("target_readback_mismatch"));
            }
            op.entries[index].after_leaf = actual.map(|r| r.binding);
            Ok(())
        })();
        mutated |= renamed;
        op.entries[index].status = match result {
            Ok(()) => "APPLIED",
            Err(_) if renamed => {
                stop = true;
                "UNVERIFIED"
            }
            Err(e) if e.0 == "environment_dependency_not_ready" => "DEPENDENCY_PENDING",
            Err(_) => "CONFLICT",
        }
        .into();
        if store.save_operation(&op, true).is_err() {
            op.status = "PARTIAL".into();
            if renamed {
                op.entries[index].status = "UNVERIFIED".into();
            }
            return Ok(summary(&op, "apply", mutated, false));
        }
    }
    op.status = if op
        .entries
        .iter()
        .all(|e| ["APPLIED", "UNCHANGED"].contains(&e.status.as_str()))
    {
        "APPLIED"
    } else {
        "PARTIAL"
    }
    .into();
    let saved = store.save_operation(&op, true).is_ok();
    if !saved {
        op.status = "PARTIAL".into();
    }
    Ok(summary(&op, "apply", mutated, saved))
}

fn plan_for_operation(store: &Store, op: &Operation) -> Result<(Plan, Current), ContractError> {
    let (plan, digest) = store.plan(&op.proposal_id)?;
    if digest != op.plan_sha256
        || op.baseline_sha256 != plan.baseline_sha256
        || op.registry_sha256 != plan.registry_sha256
        || op.bindings_sha256 != plan.bindings_sha256
    {
        return Err(error("operation_plan_mismatch"));
    }
    let current = store.check(&plan)?;
    if op.basis_revision != plan.basis_revision
        || op.authority_revision != plan.authority_revision
        || op.exception_revision != plan.exception_revision
        || op.authority_ref != plan.authority_ref
        || op.source_revision != plan.source_revision
        || op.device_id != plan.device_id
    {
        return Err(error("operation_authority_mismatch"));
    }
    for e in &plan.entries {
        bound_plan_entry(&current, e)?;
    }
    if op.entries.len() != plan.entries.len() {
        return Err(error("operation_plan_entry_mismatch"));
    }
    for entry in &op.entries {
        let expected = plan
            .entries
            .iter()
            .find(|p| p.target_id == entry.target_id)
            .ok_or_else(|| error("operation_plan_entry_mismatch"))?;
        if entry.parent_binding != expected.parent_binding
            || bytes(&entry.dependencies)? != bytes(&expected.dependencies)?
        {
            return Err(error("operation_plan_entry_mismatch"));
        }
        let original = entry.before_sha256 == expected.before_sha256
            && entry.after_sha256.as_deref() == Some(expected.after_sha256.as_str())
            && entry.before_leaf == expected.before_leaf
            && entry.after_content.as_deref() == Some(expected.after_content.as_str());
        let restored = op.action == "rollback"
            && entry.before_sha256.as_deref() == Some(expected.after_sha256.as_str())
            && entry.after_sha256 == expected.before_sha256
            && entry.after_content == expected.before_content;
        if !original && !restored {
            return Err(error("operation_plan_entry_mismatch"));
        }
        if entry.before_sha256 != entry.after_sha256 {
            if entry.before_sha256.is_some() {
                let expected_backup = if restored {
                    format!("{}-{}.backup", op.operation_id, entry.target_id)
                } else {
                    format!("op-{}-{}.backup", op.proposal_id, entry.target_id)
                };
                if entry.backup_ref.as_deref() != Some(expected_backup.as_str()) {
                    return Err(error("operation_backup_binding_mismatch"));
                }
            } else if entry.backup_ref.is_some() {
                return Err(error("operation_backup_binding_mismatch"));
            }
        }
    }
    Ok((plan, current))
}
fn actual_matches(
    read: &Option<ReadFile>,
    digest: &Option<String>,
    binding: &Option<Identity>,
) -> bool {
    read.as_ref().map(|r| hash(&r.bytes)) == *digest
        && read.as_ref().map(|r| r.binding.clone()) == *binding
}
fn backup(store: &Store, entry: &OperationEntry) -> Result<Option<String>, ContractError> {
    match (&entry.before_sha256, &entry.backup_ref) {
        (None, None) => Ok(None),
        (Some(digest), Some(name)) => {
            let read = store
                .artifacts("backups", false)?
                .read(name)?
                .ok_or_else(|| error("backup_missing"))?;
            if hash(&read.bytes) != *digest {
                return Err(error("backup_digest_mismatch"));
            }
            String::from_utf8(read.bytes)
                .map(Some)
                .map_err(|_| error("backup_not_utf8"))
        }
        _ => Err(error("backup_missing")),
    }
}

fn closure(
    baseline: &Baseline,
    roots: &BTreeSet<String>,
) -> Result<BTreeSet<String>, ContractError> {
    let mut preserve = BTreeSet::new();
    let mut pending: Vec<_> = roots.iter().cloned().collect();
    while let Some(id) = pending.pop() {
        for dep in &target(baseline, &id)?.dependencies {
            if preserve.insert(dep.clone()) {
                pending.push(dep.clone());
            }
        }
    }
    Ok(preserve)
}

fn rollback(state_dir: &Path, operation_id: &str) -> Result<Value, ContractError> {
    let store = Store::open(state_dir, false)?;
    let _lock = store.root.lock()?;
    let mut original = store.operation(operation_id)?;
    if original.action != "apply" {
        return Err(error("rollback_requires_apply_operation"));
    }
    let (plan, current) = plan_for_operation(&store, &original)?;
    let mut restore = BTreeMap::new();
    let mut retained = BTreeSet::new();
    let mut statuses = BTreeMap::new();
    for e in &original.entries {
        if e.before_sha256 == e.after_sha256 {
            statuses.insert(e.target_id.clone(), "UNCHANGED".to_owned());
            continue;
        }
        let actual = open_target(&current.registry, &e.target_id).map(|(_, _, r)| r);
        match actual {
            Ok(read) if actual_matches(&read, &e.after_sha256, &e.after_leaf) => {
                match backup(&store, e) {
                    Ok(content) => {
                        restore.insert(e.target_id.clone(), content);
                    }
                    Err(_) => {
                        retained.insert(e.target_id.clone());
                        statuses.insert(e.target_id.clone(), "CONFLICT".into());
                    }
                }
            }
            Ok(read) if read.as_ref().map(|r| hash(&r.bytes)) == e.before_sha256 => {
                statuses.insert(e.target_id.clone(), "UNCHANGED".into());
            }
            _ => {
                retained.insert(e.target_id.clone());
                statuses.insert(e.target_id.clone(), "CONFLICT".into());
            }
        }
    }
    // Consumers outside this operation are also preservation roots, including shared ones.
    for t in &current.baseline.managed_targets {
        let restored_before = original
            .entries
            .iter()
            .find(|e| e.target_id == t.target_id)
            .is_some_and(|e| {
                e.before_sha256 != e.after_sha256
                    && statuses.get(&t.target_id).is_some_and(|s| s == "UNCHANGED")
            });
        if t.dependencies.is_empty() || restore.contains_key(&t.target_id) || restored_before {
            continue;
        }
        if open_target(&current.registry, &t.target_id).map_or(true, |(_, _, r)| r.is_some()) {
            retained.insert(t.target_id.clone());
        }
    }
    let preserve = closure(&current.baseline, &retained)?;
    for dependency in preserve {
        if restore.remove(&dependency).is_some() {
            statuses.insert(dependency, "RETAINED_DEPENDENCY".into());
        }
    }
    if restore.is_empty() {
        let mut view = original.clone();
        view.status = if statuses.values().all(|s| s == "UNCHANGED") {
            "ROLLED_BACK"
        } else {
            "PARTIAL"
        }
        .into();
        for e in &mut view.entries {
            if let Some(status) = statuses.get(&e.target_id) {
                e.status = status.clone();
            }
        }
        let mut result = summary(&view, "rollback", false, false);
        result["rollback_operation_id"] = Value::Null;
        result["backup_created"] = json!(false);
        return Ok(result);
    }
    let mut op = original.clone();
    op.operation_id = format!("rollback-{}", Uuid::new_v4());
    op.action = "rollback".into();
    op.original_operation_id = Some(original.operation_id.clone());
    op.status = "PREPARED".into();
    op.entries.reverse();
    let backups = store.artifacts("backups", true)?;
    for e in &mut op.entries {
        let Some(content) = restore.get(&e.target_id) else {
            e.status = statuses
                .get(&e.target_id)
                .cloned()
                .unwrap_or_else(|| "UNCHANGED".into());
            continue;
        };
        let (_, _, read) = open_target(&current.registry, &e.target_id)?;
        let applied = original
            .entries
            .iter()
            .find(|a| a.target_id == e.target_id)
            .ok_or_else(|| error("rollback_entry_missing"))?;
        if !actual_matches(&read, &applied.after_sha256, &applied.after_leaf) {
            return Err(error("rollback_user_edit_conflict"));
        }
        let old = read.ok_or_else(|| error("rollback_original_missing"))?;
        let name = format!("{}-{}.backup", op.operation_id, e.target_id);
        backups.write(&name, &old.bytes, false)?;
        e.before_sha256 = Some(hash(&old.bytes));
        e.before_leaf = Some(old.binding);
        e.after_sha256 = content.as_ref().map(|c| hash(c.as_bytes()));
        e.after_content = content.clone();
        e.backup_ref = Some(name);
        e.after_leaf = None;
        e.status = "PREPARED".into();
    }
    store.check(&plan)?;
    store.save_operation(&op, false)?;
    #[cfg(test)]
    AFTER_ROLLBACK_PREPARE.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
    let mut mutated = false;
    let mut restored: BTreeMap<String, (Option<String>, Option<Identity>)> = BTreeMap::new();
    let mut stop = false;
    for index in 0..op.entries.len() {
        if op.entries[index].status != "PREPARED" {
            continue;
        }
        let target_id = op.entries[index].target_id.clone();
        if stop {
            op.entries[index].status = "DEPENDENCY_PENDING".into();
            continue;
        }
        // Recheck consumers before restoring a dependency, including consumers restored earlier.
        for consumer in &current.baseline.managed_targets {
            if consumer.dependencies.is_empty() {
                continue;
            }
            let read = open_target(&current.registry, &consumer.target_id).map(|(_, _, r)| r);
            let expected = if let Some(expected) = restored.get(&consumer.target_id) {
                Some(expected.clone())
            } else {
                original
                    .entries
                    .iter()
                    .find(|e| {
                        e.target_id == consumer.target_id
                            && restore.contains_key(&consumer.target_id)
                    })
                    .map(|e| (e.after_sha256.clone(), e.after_leaf.clone()))
            };
            if let Some((digest, binding)) = expected {
                if read
                    .as_ref()
                    .map_or(true, |r| !actual_matches(r, &digest, &binding))
                {
                    retained.insert(consumer.target_id.clone());
                }
            } else if read.as_ref().map_or(true, |r| r.is_some()) {
                retained.insert(consumer.target_id.clone());
            }
        }
        if closure(&current.baseline, &retained)?.contains(&target_id) {
            op.entries[index].status = "RETAINED_DEPENDENCY".into();
            continue;
        }
        let entry = op.entries[index].clone();
        let mut changed = false;
        let result = (|| {
            let now = store.check(&plan)?;
            let (parent, leaf, read) = open_target(&now.registry, &target_id)?;
            if !actual_matches(&read, &entry.before_sha256, &entry.before_leaf) {
                return Err(error("rollback_user_edit_conflict"));
            }
            if let Some(content) = &entry.after_content {
                let mut candidate = Candidate::new(
                    &parent,
                    content.as_bytes(),
                    read.as_ref().map_or(0o600, |r| r.mode),
                )?;
                let now = store.check(&plan)?;
                let (last_parent, _, last) = open_target(&now.registry, &target_id)?;
                if last_parent.binding != parent.binding
                    || !actual_matches(&last, &entry.before_sha256, &entry.before_leaf)
                {
                    return Err(error("rollback_user_edit_conflict"));
                }
                let committed = candidate.commit(&parent, &leaf, true);
                changed = candidate.renamed();
                committed?;
            } else {
                // Deletion requires the same owner leaf identity and post-apply digest.
                store.check(&plan)?;
                let (_, _, last) = open_target(&now.registry, &target_id)?;
                if !actual_matches(&last, &entry.before_sha256, &entry.before_leaf) {
                    return Err(error("rollback_user_edit_conflict"));
                }
                changed = true;
                files::remove(&parent, &leaf)?;
            }
            let (_, _, actual) = open_target(&now.registry, &target_id)?;
            if actual.as_ref().map(|r| hash(&r.bytes)) != entry.after_sha256 {
                return Err(error("rollback_readback_mismatch"));
            }
            op.entries[index].after_leaf = actual.as_ref().map(|r| r.binding.clone());
            restored.insert(
                target_id.clone(),
                (entry.after_sha256.clone(), actual.map(|r| r.binding)),
            );
            Ok(())
        })();
        mutated |= changed;
        op.entries[index].status = match result {
            Ok(()) => "ROLLED_BACK",
            Err(_) if changed => {
                stop = true;
                "UNVERIFIED"
            }
            Err(_) => {
                retained.insert(target_id);
                "CONFLICT"
            }
        }
        .into();
        if store.save_operation(&op, true).is_err() {
            op.status = "PARTIAL".into();
            return Ok(summary(&op, "rollback", mutated, false));
        }
    }
    op.status = if op
        .entries
        .iter()
        .all(|e| ["ROLLED_BACK", "UNCHANGED"].contains(&e.status.as_str()))
    {
        "ROLLED_BACK"
    } else {
        "PARTIAL"
    }
    .into();
    let saved = store.save_operation(&op, true).is_ok();
    if !saved {
        op.status = "PARTIAL".into();
    }
    if saved {
        for e in &mut original.entries {
            if let Some(rollback) = op.entries.iter().find(|r| r.target_id == e.target_id) {
                e.status = rollback.status.clone();
            }
        }
        original.status = op.status.clone();
        if store.save_operation(&original, true).is_err() {
            op.status = "PARTIAL".into();
        }
    }
    let mut result = summary(&op, "rollback", mutated, saved);
    result["original_operation_id"] = json!(operation_id);
    result["rollback_operation_id"] = json!(op.operation_id);
    Ok(result)
}

fn recover(state_dir: &Path, operation_id: &str) -> Result<Value, ContractError> {
    let store = Store::open(state_dir, false)?;
    let _lock = store.root.lock()?;
    let mut op = store.operation(operation_id)?;
    let (_, current) = plan_for_operation(&store, &op)?;
    for e in &mut op.entries {
        // A mismatched backup is a conflict even if candidate bytes happen to match.
        let backup_ok = e.before_sha256 == e.after_sha256 || backup(&store, e).is_ok();
        match open_target(&current.registry, &e.target_id) {
            Ok((_, _, read)) if backup_ok => {
                let actual = read.as_ref().map(|r| hash(&r.bytes));
                if actual == e.before_sha256 {
                    e.status = "DISK_OBSERVED_BEFORE".into();
                } else if actual == e.after_sha256 {
                    e.status = "DISK_OBSERVED_AFTER_ONLY".into();
                    e.after_leaf = read.map(|r| r.binding);
                } else {
                    e.status = "CONFLICT".into();
                }
            }
            _ => e.status = "CONFLICT".into(),
        }
    }
    // Observed bytes cannot prove who performed the mutation or native instruction loading.
    op.status = if op
        .entries
        .iter()
        .all(|e| e.status == "DISK_OBSERVED_BEFORE")
    {
        "PREPARED"
    } else {
        "PARTIAL"
    }
    .into();
    let saved = store.save_operation(&op, true).is_ok();
    let mut result = summary(&op, "recover", false, saved);
    result["disk_observation_only"] = json!(true);
    result["automatic_resume_performed"] = json!(false);
    Ok(result)
}

#[cfg(test)]
mod tests;
