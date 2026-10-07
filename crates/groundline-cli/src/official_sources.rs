//! Explicit, bounded official-source observations. A changed digest asks for
//! review; it neither changes guidance nor establishes a recommendation.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clap::Subcommand;
use groundline_contracts::{ContractError, learning::SourceObservation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

mod storage;

const MAX_SOURCES: usize = 32;
const MAX_CACHE_ENTRIES: usize = 128;
const MAX_CONTENT_BYTES: usize = 512 * 1024;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_SNAPSHOT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 256 * 1024;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Compare explicitly supplied official text snapshots offline; changes require review.
    Check {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        state_dir: PathBuf,
    },
}

fn error(code: &str) -> ContractError {
    ContractError(format!("sources_{code}"))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn official_url(raw: &str) -> Result<(), ContractError> {
    // Reuse the maintained URL parser and official-host policy in the source
    // contract; this module does not acquire content or follow redirects.
    SourceObservation {
        url: raw.to_owned(),
        checked_at_utc: Utc::now().to_rfc3339(),
        content_sha256: digest(b"url-policy-validation"),
        model: None,
        runtime: None,
        affected_target_ids: vec!["source-cache".to_owned()],
    }
    .validate()
    .map_err(|_| error("invalid_official_url"))
}

fn observed_time(value: &str) -> Result<(), ContractError> {
    let time =
        DateTime::parse_from_rfc3339(value).map_err(|_| error("invalid_observation_time"))?;
    if time.with_timezone(&Utc) > Utc::now() + chrono::Duration::minutes(5) {
        return Err(error("invalid_observation_time"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    source_id: String,
    url: String,
    model: Option<String>,
    runtime: Option<groundline_contracts::learning::RuntimeObservation>,
    affected_target_ids: Vec<String>,
}

impl Source {
    fn validate(&self) -> Result<(), ContractError> {
        official_url(&self.url)?;
        let mut targets = BTreeSet::new();
        if !identifier(&self.source_id)
            || self.affected_target_ids.is_empty()
            || self.affected_target_ids.len() > 32
            || self
                .affected_target_ids
                .iter()
                .any(|id| !identifier(id) || !targets.insert(id))
            || self.model.as_ref().is_some_and(|model| {
                groundline_contracts::model::identity_kind(model) != "public_model_id"
            })
            || self.runtime.as_ref().is_some_and(|runtime| {
                !["codex_app", "codex_cli"].contains(&runtime.family.as_str())
                    || !identifier(&runtime.version)
                    || !valid_digest(&runtime.evidence_sha256)
            })
        {
            return Err(error("invalid_source_mapping"));
        }
        self.observation(&Content {
            url: self.url.clone(),
            observed_at_utc: Utc::now().to_rfc3339(),
            content_sha256: digest(b"mapping-validation"),
            content_bytes: 1,
            provenance: "explicit_snapshot_import".to_owned(),
            snapshot_sha256: digest(b"mapping-validation"),
        })
        .validate()
        .map_err(|_| error("invalid_source_mapping"))
    }

    fn mapping_sha256(&self) -> Result<String, ContractError> {
        let mut mapping = self.clone();
        mapping.affected_target_ids.sort();
        Ok(digest(
            &serde_json::to_vec(&mapping).map_err(|_| error("serialization_failed"))?,
        ))
    }

    fn observation(&self, content: &Content) -> SourceObservation {
        SourceObservation {
            url: content.url.clone(),
            checked_at_utc: content.observed_at_utc.clone(),
            content_sha256: content.content_sha256.clone(),
            model: self.model.clone(),
            runtime: self.runtime.clone(),
            affected_target_ids: self.affected_target_ids.clone(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    kind: String,
    schema: u8,
    sources: Vec<Source>,
}

fn manifest(bytes: &[u8]) -> Result<Manifest, ContractError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| error("invalid_manifest"))?;
    // Optional associations are explicit nulls, never inferred from native state.
    if value
        .get("sources")
        .and_then(Value::as_array)
        .is_none_or(|sources| {
            sources
                .iter()
                .any(|s| s.get("model").is_none() || s.get("runtime").is_none())
        })
    {
        return Err(error("explicit_mapping_required"));
    }
    let manifest: Manifest =
        serde_json::from_value(value).map_err(|_| error("invalid_manifest"))?;
    if manifest.kind != "groundline-official-source-manifest"
        || manifest.schema != 1
        || manifest.sources.is_empty()
        || manifest.sources.len() > MAX_SOURCES
    {
        return Err(error("invalid_manifest"));
    }
    let mut ids = BTreeSet::new();
    for source in &manifest.sources {
        source.validate()?;
        if !ids.insert(&source.source_id) {
            return Err(error("duplicate_source_id"));
        }
    }
    Ok(manifest)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshots {
    kind: String,
    schema: u8,
    snapshots: Vec<Snapshot>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    source_id: String,
    url: String,
    observed_at_utc: String,
    content: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Content {
    url: String,
    observed_at_utc: String,
    content_sha256: String,
    content_bytes: usize,
    provenance: String,
    snapshot_sha256: String,
}

fn content(
    bytes: &[u8],
    url: String,
    time: String,
    snapshot_sha256: String,
) -> Result<Content, &'static str> {
    if bytes.is_empty() || bytes.len() > MAX_CONTENT_BYTES {
        return Err("content_size_limit");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "invalid_utf8_text")?;
    let start = text
        .trim_start()
        .chars()
        .take(64)
        .collect::<String>()
        .to_ascii_lowercase();
    if text.trim().is_empty()
        || text.contains('\0')
        || start.starts_with("<!doctype html")
        || start.starts_with("<html")
        || start.starts_with("<?xml")
    {
        return Err("text_snapshot_required");
    }
    Ok(Content {
        url,
        observed_at_utc: time,
        content_sha256: digest(bytes),
        content_bytes: bytes.len(),
        provenance: "explicit_snapshot_import".to_owned(),
        snapshot_sha256,
    })
}

fn snapshots(
    bytes: &[u8],
    manifest: &Manifest,
) -> Result<BTreeMap<String, Result<Content, &'static str>>, ContractError> {
    let snapshot_sha256 = digest(bytes);
    let input: Snapshots = serde_json::from_slice(bytes).map_err(|_| error("invalid_snapshot"))?;
    if input.kind != "groundline-official-source-snapshots"
        || input.schema != 1
        || input.snapshots.len() > MAX_SOURCES
    {
        return Err(error("invalid_snapshot"));
    }
    let mut result = BTreeMap::new();
    for snapshot in input.snapshots {
        let source = manifest
            .sources
            .iter()
            .find(|s| s.source_id == snapshot.source_id)
            .ok_or_else(|| error("unmapped_snapshot"))?;
        official_url(&snapshot.url)?;
        observed_time(&snapshot.observed_at_utc)?;
        if snapshot.url != source.url || result.contains_key(&snapshot.source_id) {
            return Err(error("snapshot_source_mismatch"));
        }
        result.insert(
            snapshot.source_id,
            content(
                snapshot.content.as_bytes(),
                snapshot.url,
                snapshot.observed_at_utc,
                snapshot_sha256.clone(),
            ),
        );
    }
    Ok(result)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cache {
    kind: String,
    schema: u8,
    entries: BTreeMap<String, CachedSource>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CachedSource {
    mapping_sha256: String,
    origin_manifest_sha256: String,
    latest_observed_at_utc: String,
    content: Content,
}

impl Cache {
    fn empty() -> Self {
        Self {
            kind: "groundline-official-source-cache".to_owned(),
            schema: 1,
            entries: BTreeMap::new(),
        }
    }

    fn parse(bytes: Option<&[u8]>) -> Result<Self, ContractError> {
        let cache: Self = match bytes {
            Some(bytes) => serde_json::from_slice(bytes).map_err(|_| error("invalid_cache"))?,
            None => return Ok(Self::empty()),
        };
        if cache.kind != "groundline-official-source-cache"
            || cache.schema != 1
            || cache.entries.len() > MAX_CACHE_ENTRIES
        {
            return Err(error("invalid_cache"));
        }
        for (id, entry) in &cache.entries {
            official_url(&entry.content.url)?;
            observed_time(&entry.content.observed_at_utc)?;
            observed_time(&entry.latest_observed_at_utc)?;
            if !identifier(id)
                || !valid_digest(&entry.mapping_sha256)
                || !valid_digest(&entry.origin_manifest_sha256)
                || !valid_digest(&entry.content.snapshot_sha256)
                || !valid_digest(&entry.content.content_sha256)
                || !(1..=MAX_CONTENT_BYTES).contains(&entry.content.content_bytes)
                || entry.content.provenance != "explicit_snapshot_import"
                || DateTime::parse_from_rfc3339(&entry.latest_observed_at_utc).ok()
                    < DateTime::parse_from_rfc3339(&entry.content.observed_at_utc).ok()
            {
                return Err(error("invalid_cache"));
            }
        }
        Ok(cache)
    }
}

fn compare(
    manifest: &Manifest,
    manifest_sha256: &str,
    cache: &mut Cache,
    mut contents: BTreeMap<String, Result<Content, &'static str>>,
) -> Result<(Vec<Value>, bool), ContractError> {
    let mut records = Vec::new();
    let mut cache_changed = false;
    for source in &manifest.sources {
        let mapping = source.mapping_sha256()?;
        let previous = cache.entries.get(&source.source_id);
        let next = contents
            .remove(&source.source_id)
            .unwrap_or(Err("snapshot_unavailable"));
        let content = match next {
            Err(reason) => {
                records.push(
                    json!({"source_id":source.source_id,"status":"unavailable","reason":reason,
                    "requested_url":source.url,"previous_state_preserved":true,
                    "review_required":false,"affected_target_ids":[],"observation":null}),
                );
                continue;
            }
            Ok(content) => content,
        };
        let same_mapping = previous.is_some_and(|old| old.mapping_sha256 == mapping);
        // Freshness belongs to the observed document, independently of which
        // guidance/model/runtime association currently maps to that document.
        let same_url = previous.is_some_and(|old| old.content.url == content.url);
        if same_url
            && previous.is_some_and(|old| {
                DateTime::parse_from_rfc3339(&content.observed_at_utc).ok()
                    < DateTime::parse_from_rfc3339(&old.latest_observed_at_utc).ok()
            })
        {
            records.push(json!({"source_id":source.source_id,"status":"unavailable","reason":"stale_snapshot",
                "requested_url":source.url,"previous_state_preserved":true,
                "review_required":false,"affected_target_ids":[],"observation":null}));
            continue;
        }
        if same_url
            && previous.is_some_and(|old| {
                DateTime::parse_from_rfc3339(&content.observed_at_utc).ok()
                    == DateTime::parse_from_rfc3339(&old.latest_observed_at_utc).ok()
                    && old.content.content_sha256 != content.content_sha256
            })
        {
            records.push(json!({"source_id":source.source_id,"status":"unavailable","reason":"conflicting_snapshot_time",
                "requested_url":source.url,"previous_state_preserved":true,
                "review_required":false,"affected_target_ids":[],"observation":null}));
            continue;
        }
        let status = if !same_mapping {
            "baseline_created"
        } else if previous.is_some_and(|old| old.content.content_sha256 == content.content_sha256) {
            "unchanged"
        } else {
            "changed"
        };
        let review = status == "changed";
        let fresh_observation = previous.is_none_or(|old| {
            DateTime::parse_from_rfc3339(&content.observed_at_utc).ok()
                > DateTime::parse_from_rfc3339(&old.latest_observed_at_utc).ok()
        });
        let reason = if previous.is_none() {
            "first_observation"
        } else if !same_mapping {
            "source_mapping_changed"
        } else if review {
            "content_digest_changed"
        } else {
            "content_digest_unchanged"
        };
        records.push(json!({"source_id":source.source_id,"status":status,"reason":reason,
            "requested_url":source.url,"review_required":review,
            "affected_target_ids":if review { source.affected_target_ids.clone() } else { vec![] },
            "mapping_sha256":mapping,"previous_content_sha256":previous.map(|p| &p.content.content_sha256),
            "snapshot_sha256":content.snapshot_sha256,
            "previous_snapshot_sha256":previous.map(|p| &p.content.snapshot_sha256),
            "previous_manifest_sha256":previous.map(|p| &p.origin_manifest_sha256),
            "latest_observed_at_utc":content.observed_at_utc,
            "cache_content_updated":status != "unchanged",
            "cache_metadata_updated":status == "unchanged" && fresh_observation,
            "provenance":content.provenance,"authenticity_verified":false,
            "observed_url":content.url,"observed_at_utc":content.observed_at_utc,
            "content_sha256":content.content_sha256,"content_bytes":content.content_bytes,
            "analysis_skipped":!review,"observation":source.observation(&content)}));
        // Keep the content baseline's original provenance, but advance the
        // watermark for fresh unchanged observations so an older differing
        // snapshot cannot masquerade as a subsequent change.
        if status != "unchanged" {
            if !cache.entries.contains_key(&source.source_id)
                && cache.entries.len() >= MAX_CACHE_ENTRIES
            {
                return Err(error("cache_entry_limit"));
            }
            cache.entries.insert(
                source.source_id.clone(),
                CachedSource {
                    mapping_sha256: mapping,
                    origin_manifest_sha256: manifest_sha256.to_owned(),
                    latest_observed_at_utc: content.observed_at_utc.clone(),
                    content,
                },
            );
            cache_changed = true;
        } else if fresh_observation {
            cache
                .entries
                .get_mut(&source.source_id)
                .ok_or_else(|| error("invalid_cache"))?
                .latest_observed_at_utc = content.observed_at_utc;
            cache_changed = true;
        }
    }
    Ok((records, cache_changed))
}

pub(crate) fn run(command: Command) -> Result<Value, ContractError> {
    let (manifest_path, snapshot_path, state_path) = match command {
        Command::Check {
            manifest,
            snapshot,
            state_dir,
        } => (manifest, snapshot, state_dir),
    };
    let manifest_bytes = storage::read_private(&manifest_path, MAX_MANIFEST_BYTES)?;
    let manifest = manifest(&manifest_bytes)?;
    let snapshot_bytes = storage::read_private(&snapshot_path, MAX_SNAPSHOT_BYTES)?;
    let contents = snapshots(&snapshot_bytes, &manifest)?;
    // Hold one owner-private descriptor/lock through read, observation and atomic commit.
    let state = storage::State::open(&state_path)?;
    let cache_bytes = state.read(MAX_CACHE_BYTES)?;
    let mut cache = Cache::parse(cache_bytes.as_deref())?;
    let (records, changed) = compare(&manifest, &digest(&manifest_bytes), &mut cache, contents)?;
    if changed {
        let bytes = serde_json::to_vec_pretty(&cache).map_err(|_| error("serialization_failed"))?;
        if bytes.len() as u64 > MAX_CACHE_BYTES {
            return Err(error("cache_size_limit"));
        }
        state.save(&bytes)?;
    }
    let unavailable = records
        .iter()
        .filter(|r| r["status"] == "unavailable")
        .count();
    let review = records.iter().any(|r| r["review_required"] == true);
    Ok(json!({"kind":"groundline-official-source-check","schema":1,
        "status":if unavailable != 0 { "PARTIAL" } else if review { "REVIEW_REQUIRED" } else { "PASS" },
        "manifest_sha256":digest(&manifest_bytes),"snapshot_sha256":digest(&snapshot_bytes),"observations":records,
        "network_fetch_performed":false,"cache_updated":changed,
        "guidance_changed":false,"recommendation_inferred":false,"analysis_performed":false,
        "native_availability_verified":false,"selection_inferred":false,
        "native_settings_changed":false,"raw_content_emitted":false,"private_paths_emitted":false,
        "cache_authenticity_verified":false,"snapshot_authenticity_verified":false,
        "model_calls":0,"polling_started":false}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_urls_reject_untrusted_authorities_and_token_bearing_inputs() {
        for raw in [
            "http://developers.openai.com/x.md",
            "https://developers.openai.com.evil.invalid/x.md",
            "https://user@developers.openai.com/x.md",
            "https://developers.openai.com:8443/x.md",
            "https://developers.openai.com/x.md?key=private",
            "https://developers.openai.com/x.md#fragment",
            "https://127.0.0.1/x.md",
            "https://github.com/another/codex/x.md",
        ] {
            assert!(official_url(raw).is_err(), "accepted {raw}");
        }
        assert!(official_url("https://developers.openai.com/blog/eval-skills.md").is_ok());
        assert!(official_url("https://learn.chatgpt.com/docs/hooks.md").is_ok());
    }

    #[test]
    fn unavailable_snapshot_keeps_existing_observation_exactly() {
        let manifest = manifest(serde_json::to_vec(&json!({"kind":"groundline-official-source-manifest","schema":1,
            "sources":[{"source_id":"skills","url":"https://developers.openai.com/blog/eval-skills.md",
                "model":null,"runtime":null,"affected_target_ids":["workflow"]}]})).unwrap().as_slice()).unwrap();
        let mut cache = Cache::empty();
        let first = content(
            b"first",
            manifest.sources[0].url.clone(),
            Utc::now().to_rfc3339(),
            digest(b"fixture-snapshot"),
        )
        .unwrap();
        compare(
            &manifest,
            &digest(b"fixture-manifest"),
            &mut cache,
            BTreeMap::from([("skills".to_owned(), Ok(first))]),
        )
        .unwrap();
        let before = serde_json::to_vec(&cache).unwrap();
        let (records, changed) = compare(
            &manifest,
            &digest(b"fixture-manifest"),
            &mut cache,
            BTreeMap::from([("skills".to_owned(), Err("snapshot_unavailable"))]),
        )
        .unwrap();
        assert!(!changed);
        assert_eq!(serde_json::to_vec(&cache).unwrap(), before);
        assert_eq!(records[0]["status"], "unavailable");
        assert_eq!(records[0]["previous_state_preserved"], true);
    }
}
