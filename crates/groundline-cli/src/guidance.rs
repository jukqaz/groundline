//! GroundLine-owned profile and portable baseline contracts; no legacy adapter.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use groundline_contracts::ContractError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

mod files;
#[cfg(test)]
mod tests;
use files::{Budget, inspect, read_json, safe_directory, safe_relative, valid_name};

const MAX_SKILLS: usize = 512;
type Fingerprints = BTreeMap<String, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    kind: String,
    schema: u32,
    #[serde(deserialize_with = "unique_map")]
    roots: BTreeMap<String, PathBuf>,
    #[serde(default, deserialize_with = "unique_map")]
    sources: BTreeMap<String, Source>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    checkout: PathBuf,
    repository: String,
    #[serde(default)]
    declared_revision: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Baseline {
    kind: String,
    schema: u32,
    roots: BTreeSet<String>,
    #[serde(deserialize_with = "unique_map")]
    skills: BTreeMap<String, Skill>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Skill {
    name: String,
    #[serde(deserialize_with = "unique_map")]
    files: Fingerprints,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream: Option<Upstream>,
}
#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Upstream {
    repository: String,
    declared_revision: Option<String>,
    #[serde(deserialize_with = "unique_map")]
    files: Fingerprints,
}

fn unique_map<'de, D, T>(deserializer: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Entries<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Entries<T> {
        type Value = BTreeMap<String, T>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an object with unique keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, T>()? {
                if result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate_key"));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Entries(std::marker::PhantomData))
}
struct Observed {
    baseline: Baseline,
    roots: Vec<PathBuf>,
    metadata: BTreeMap<String, (usize, bool)>,
    bytes: u64,
    orphan_sources: usize,
    known_sources: usize,
}
fn error(code: &str) -> ContractError {
    ContractError(format!("guidance_{code}"))
}
fn report(operation: &str) -> Value {
    json!({"kind":"groundline-guidance", "schema":1, "operation":operation, "status":"PASS",
        "network_performed":false, "mutation_performed":false, "raw_content_emitted":false,
        "private_paths_emitted":false, "scripts_executed":false, "behavior_evaluation":"not_run"})
}
fn valid_key(key: &str, roots: &BTreeSet<String>) -> bool {
    let parts = key.split('/').collect::<Vec<_>>();
    parts.len() == 2
        && roots.contains(parts[0])
        && safe_relative(parts[1])
        && !parts[1].starts_with('.')
}
fn valid_repository(value: &str) -> bool {
    // Provenance is never used as a fetch destination. Still reject credentials
    // and local paths so portable baselines cannot copy them from a profile.
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
    })
}
fn valid_revision(revision: &Option<String>) -> bool {
    revision
        .as_ref()
        .is_none_or(|r| matches!(r.len(), 40 | 64) && r.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn valid_hashes(hashes: &Fingerprints) -> bool {
    !hashes.is_empty()
        && hashes.len() <= files::MAX_ENTRIES
        && hashes.contains_key("SKILL.md")
        && hashes.iter().all(|(path, hash)| {
            safe_relative(path)
                && hash.len() == 64
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}
fn load_profile(path: &Path) -> Result<Profile, ContractError> {
    let mut profile: Profile = read_json(path)?;
    if profile.kind != "groundline-guidance-profile"
        || profile.schema != 1
        || profile.roots.is_empty()
        || profile.roots.len() > 16
        || profile.sources.len() > MAX_SKILLS
    {
        return Err(error("invalid_profile"));
    }
    let mut canonical = Vec::<PathBuf>::new();
    for (id, root) in &mut profile.roots {
        if !valid_name(id) || !root.is_absolute() {
            return Err(error("invalid_root"));
        }
        *root = safe_directory(root)?;
        if canonical
            .iter()
            .any(|p| p.starts_with(&*root) || root.starts_with(p))
        {
            return Err(error("overlapping_roots"));
        }
        canonical.push(root.clone());
    }
    let roots = profile.roots.keys().cloned().collect();
    for (key, source) in &profile.sources {
        if !valid_key(key, &roots)
            || !source.checkout.is_absolute()
            || !valid_repository(&source.repository)
            || !valid_revision(&source.declared_revision)
        {
            return Err(error("invalid_source"));
        }
    }
    Ok(profile)
}
fn load_baseline(path: &Path) -> Result<Baseline, ContractError> {
    let baseline: Baseline = read_json(path)?;
    if baseline.kind != "groundline-guidance-baseline"
        || baseline.schema != 1
        || baseline.roots.is_empty()
        || baseline.roots.len() > 16
        || !baseline.roots.iter().all(|root| valid_name(root))
        || baseline.skills.len() > MAX_SKILLS
    {
        return Err(error("invalid_baseline"));
    }
    for (key, skill) in &baseline.skills {
        if !valid_key(key, &baseline.roots)
            || !valid_name(&skill.name)
            || !valid_hashes(&skill.files)
        {
            return Err(error("invalid_baseline"));
        }
        if let Some(source) = &skill.upstream
            && (!valid_repository(&source.repository)
                || !valid_revision(&source.declared_revision)
                || !valid_hashes(&source.files))
        {
            return Err(error("invalid_baseline_source"));
        }
    }
    Ok(baseline)
}
fn observe(profile: &Profile, with_upstream: bool) -> Result<Observed, ContractError> {
    let mut observed = Observed {
        baseline: Baseline {
            kind: "groundline-guidance-baseline".into(),
            schema: 1,
            roots: profile.roots.keys().cloned().collect(),
            skills: BTreeMap::new(),
        },
        roots: profile.roots.values().cloned().collect(),
        metadata: BTreeMap::new(),
        bytes: 0,
        orphan_sources: 0,
        known_sources: 0,
    };
    let mut budget = Budget::default();
    for (id, root) in &profile.roots {
        for item in fs::read_dir(root).map_err(|_| error("directory_unavailable"))? {
            budget.entries += 1;
            if budget.entries > files::MAX_ENTRIES {
                return Err(error("entry_limit"));
            }
            let item = item.map_err(|_| error("walk_failed"))?;
            let name = item.file_name();
            let name = name.to_str().ok_or_else(|| error("invalid_path"))?;
            if name.starts_with('.') {
                continue;
            }
            let metadata =
                fs::symlink_metadata(item.path()).map_err(|_| error("file_unavailable"))?;
            if metadata.file_type().is_symlink() || files::reparse(&metadata) {
                return Err(error("linked_file"));
            }
            if !metadata.is_dir() {
                continue;
            }
            if !safe_relative(name) {
                return Err(error("invalid_path"));
            }
            // Discover again on every operation, including entirely new skills.
            if !item
                .path()
                .join("SKILL.md")
                .try_exists()
                .map_err(|_| error("file_unavailable"))?
            {
                continue;
            }
            let key = format!("{id}/{name}");
            let actual = inspect(&item.path(), &mut budget)?;
            let mut skill = Skill {
                name: actual.name,
                files: actual.fingerprints,
                upstream: None,
            };
            observed
                .metadata
                .insert(key.clone(), (actual.description_chars, actual.implicit));
            if let Some(source) = profile.sources.get(&key) {
                observed.known_sources += 1;
                if with_upstream {
                    let checkout = safe_directory(&source.checkout)?;
                    let upstream = inspect(&checkout, &mut budget)?;
                    if upstream.name != skill.name {
                        return Err(error("upstream_name_mismatch"));
                    }
                    skill.upstream = Some(Upstream {
                        repository: source.repository.clone(),
                        declared_revision: source.declared_revision.clone(),
                        files: upstream.fingerprints,
                    });
                    observed.roots.push(checkout);
                }
            }
            observed.baseline.skills.insert(key, skill);
            if observed.baseline.skills.len() > MAX_SKILLS {
                return Err(error("skill_limit"));
            }
        }
    }
    observed.orphan_sources = profile
        .sources
        .keys()
        .filter(|k| !observed.baseline.skills.contains_key(*k))
        .count();
    observed.bytes = budget.bytes;
    Ok(observed)
}
fn delta(old: &Fingerprints, current: &Fingerprints) -> Value {
    json!({"added":current.keys().filter(|key| !old.contains_key(*key)).count(),
        "removed":old.keys().filter(|key| !current.contains_key(*key)).count(),
        "modified":current.iter().filter(|(key, hash)| old.get(*key).is_some_and(|old| old != *hash)).count()})
}

pub(super) fn audit(
    profile_path: &Path,
    baseline_path: Option<&Path>,
    with_upstream: bool,
) -> Result<Value, ContractError> {
    let profile = load_profile(profile_path)?;
    let old = baseline_path.map(load_baseline).transpose()?;
    let current = observe(&profile, with_upstream)?;
    if old
        .as_ref()
        .is_some_and(|old| old.roots != current.baseline.roots)
    {
        return Err(error("baseline_scope_mismatch"));
    }
    let empty = BTreeMap::new();
    let previous = old.as_ref().map(|b| &b.skills).unwrap_or(&empty);
    let keys = previous
        .keys()
        .chain(current.baseline.skills.keys())
        .collect::<BTreeSet<_>>();
    let mut rows = Vec::new();
    let mut names = BTreeSet::new();
    let mut duplicates = 0;
    let mut changed = 0;
    let mut unbaselined_sources = 0;
    let mut counts = BTreeMap::<&str, usize>::new();
    for (index, key) in keys.into_iter().enumerate() {
        let before = previous.get(key);
        let after = current.baseline.skills.get(key);
        let state = match (before, after) {
            (None, Some(_)) if old.is_none() => "unbaselined",
            (None, Some(_)) => "added",
            (Some(_), None) => "removed",
            (Some(a), Some(b)) if a.name != b.name || a.files != b.files => "modified",
            _ => "unchanged",
        };
        if matches!(state, "added" | "removed" | "modified") {
            changed += 1;
        }
        *counts.entry(state).or_default() += 1;
        let mut row = json!({"skill_index":index, "installed_status":state, "upstream_status":"unknown_source"});
        if let Some(skill) = after {
            if !names.insert(&skill.name) {
                duplicates += 1;
            }
            row["file_count"] = json!(skill.files.len());
            let (chars, implicit) = current.metadata[key];
            row["description_chars"] = json!(chars);
            row["allow_implicit_invocation"] = json!(implicit);
            if let Some(before) = before {
                row["installed_delta"] = delta(&before.files, &skill.files);
            }
            if profile.sources.contains_key(key) {
                row["upstream_status"] = json!("not_checked");
            }
            if let Some(source) = &skill.upstream {
                row["local_differs_from_upstream"] = json!(skill.files != source.files);
                if let Some(before) = before.and_then(|s| s.upstream.as_ref()) {
                    let different = before != source;
                    changed += usize::from(different);
                    row["upstream_status"] = json!(if different { "changed" } else { "unchanged" });
                    row["upstream_delta"] = delta(&before.files, &source.files);
                } else {
                    row["upstream_status"] = json!("unbaselined");
                    unbaselined_sources += 1;
                }
            } else if with_upstream && before.is_some_and(|s| s.upstream.is_some()) {
                changed += 1;
                row["upstream_status"] = json!("source_removed");
            }
        }
        rows.push(row);
    }
    let mut value = report("audit");
    value["status"] = json!(
        if changed > 0 || duplicates > 0 || current.orphan_sources > 0 {
            "REVIEW_REQUIRED"
        } else if old.is_none() || unbaselined_sources > 0 {
            "NOT_BASELINED"
        } else {
            "PASS"
        }
    );
    value["skills"] = json!(rows);
    value["skill_count"] = json!(current.baseline.skills.len());
    value["file_count"] = json!(
        current
            .baseline
            .skills
            .values()
            .map(|s| s.files.len())
            .sum::<usize>()
    );
    value["installed_counts"] = json!(counts);
    value["duplicate_name_count"] = json!(duplicates);
    value["orphan_source_count"] = json!(current.orphan_sources);
    value["upstream_unbaselined_count"] = json!(unbaselined_sources);
    value["upstream_unverified_count"] = json!(if with_upstream {
        current.baseline.skills.len() - current.known_sources
    } else {
        current.baseline.skills.len()
    });
    value["latest_upstream_verified"] = json!(false);
    value["bytes_read"] = json!(current.bytes);
    Ok(value)
}

pub(super) fn snapshot(
    profile_path: &Path,
    output: &Path,
    with_upstream: bool,
) -> Result<Value, ContractError> {
    let profile = load_profile(profile_path)?;
    let observed = observe(&profile, with_upstream)?;
    if observed.orphan_sources > 0 {
        return Err(error("orphan_sources"));
    }
    // Deterministic, host-path-free data: repeating a capture of unchanged
    // content on another host produces the same baseline bytes.
    let mut bytes =
        serde_json::to_vec_pretty(&observed.baseline).map_err(|_| error("invalid_baseline"))?;
    bytes.push(b'\n');
    files::write_new(
        &bytes,
        &observed.roots,
        output,
        observed.baseline.skills.len(),
    )
}
