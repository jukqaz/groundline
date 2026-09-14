//! Lossless dashboard transport within TrueNAS's authenticated RPC size limit.

use std::io::{Read, Write};

use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::{Compression, bufread::GzDecoder, write::GzEncoder};
use serde_json::{Value, json};

use crate::DeployError;

pub(crate) const MAX_RPC_BYTES: usize = 64 * 1024;
const MAX_DASHBOARD_BYTES: usize = 2 * 1024 * 1024;
const START: &str = include_str!("grafana-start.sh");
const DASHBOARDS: &[(&str, &str)] = &[
    ("grafana_dashboard", "groundline-insights.json"),
    ("grafana_analysis_dashboard", "groundline-analysis.json"),
];

fn invalid() -> DeployError {
    DeployError::InvalidCurrentConfiguration
}

pub(crate) fn dashboard(config: &Value, name: &str) -> Result<Value, DeployError> {
    let filename = DASHBOARDS
        .iter()
        .find(|(key, _)| *key == name)
        .ok_or_else(invalid)?
        .1;
    let mounts = config
        .pointer("/services/grafana/configs")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    let matching = mounts
        .iter()
        .filter(|m| m["source"] == name)
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err(invalid());
    }
    let content = config
        .pointer(&format!("/configs/{name}/content"))
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    if content.len() > MAX_DASHBOARD_BYTES {
        return Err(invalid());
    }
    if matching[0]["target"] == format!("/var/lib/grafana/dashboards/{filename}") {
        return Ok(serde_json::from_str(content)?);
    }
    if matching[0]["target"] != format!("/run/groundline/{filename}.gz.b64") {
        return Err(invalid());
    }
    let bytes = STANDARD.decode(content.trim()).map_err(|_| invalid())?;
    let mut decoder = GzDecoder::new(bytes.as_slice());
    let mut decoded = Vec::new();
    decoder
        .by_ref()
        .take((MAX_DASHBOARD_BYTES + 1) as u64)
        .read_to_end(&mut decoded)
        .map_err(|_| invalid())?;
    if decoded.len() > MAX_DASHBOARD_BYTES || !decoder.into_inner().is_empty() {
        return Err(invalid());
    }
    Ok(serde_json::from_slice(&decoded)?)
}

pub(crate) fn pack(config: &mut Value) -> Result<(), DeployError> {
    let entrypoint = json!(["/bin/sh", "-ec", START]);
    let service = config.pointer("/services/grafana").ok_or_else(invalid)?;
    if service.get("entrypoint").is_some_and(|v| v != &entrypoint)
        || service.get("command").is_some()
    {
        return Err(invalid());
    }
    for &(name, filename) in DASHBOARDS {
        let raw = config
            .pointer(&format!("/configs/{name}/content"))
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        // Compose would unescape these dollars in an inline config. Compressed
        // content must carry the exact bytes Grafana would otherwise receive.
        let value: Value = serde_json::from_str(&raw.replace("$$", "$"))?;
        let bytes = serde_json::to_vec(&value)?;
        if bytes.len() > MAX_DASHBOARD_BYTES {
            return Err(invalid());
        }
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&bytes).map_err(|_| invalid())?;
        let packed = STANDARD.encode(encoder.finish().map_err(|_| invalid())?);
        config["configs"][name]["content"] = json!(packed);
        let mounts = config
            .pointer_mut("/services/grafana/configs")
            .and_then(Value::as_array_mut)
            .ok_or_else(invalid)?;
        if mounts.iter().any(|mount| {
            mount["source"] != name
                && (mount["target"] == format!("/run/groundline/{filename}.gz.b64")
                    || mount["target"] == format!("/var/lib/grafana/dashboards/{filename}"))
        }) {
            return Err(invalid());
        }
        let matching = mounts
            .iter_mut()
            .filter(|m| m["source"] == name)
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(invalid());
        }
        for mount in matching {
            if mount["target"] != format!("/var/lib/grafana/dashboards/{filename}")
                && mount["target"] != format!("/run/groundline/{filename}.gz.b64")
            {
                return Err(invalid());
            }
            mount["target"] = json!(format!("/run/groundline/{filename}.gz.b64"));
        }
    }
    let provider = config
        .pointer("/configs/grafana_dashboard_provider/content")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    let mut value: Value = serde_saphyr::from_str(provider).map_err(|_| invalid())?;
    let providers = value
        .get_mut("providers")
        .and_then(Value::as_array_mut)
        .ok_or_else(invalid)?;
    let mut changed = 0;
    for provider in providers {
        if provider["options"]["path"] == "/var/lib/grafana/dashboards"
            || provider["options"]["path"] == "/tmp/groundline-dashboards"
        {
            if provider["type"] != "file" {
                return Err(invalid());
            }
            provider["options"]["path"] = json!("/tmp/groundline-dashboards");
            changed += 1;
        }
    }
    if changed != 1 {
        return Err(invalid());
    }
    config["configs"]["grafana_dashboard_provider"]["content"] =
        json!(serde_json::to_string(&value)?);
    config["services"]["grafana"]["entrypoint"] = entrypoint;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Value {
        serde_saphyr::from_str(include_str!("../../infrastructure/compose.template.yaml")).unwrap()
    }

    #[test]
    fn source_dashboards_fit_rpc_and_preserve_runtime_json() {
        let before = source();
        let mut after = before.clone();
        assert!(serde_json::to_vec(&before).unwrap().len() > MAX_RPC_BYTES);
        pack(&mut after).unwrap();
        assert!(serde_json::to_vec(&after).unwrap().len() + 1024 < MAX_RPC_BYTES);
        for &(name, _) in DASHBOARDS {
            let expected: Value = serde_json::from_str(
                &before["configs"][name]["content"]
                    .as_str()
                    .unwrap()
                    .replace("$$", "$"),
            )
            .unwrap();
            assert_eq!(dashboard(&after, name).unwrap(), expected);
        }
        assert_eq!(
            after["services"]["grafana"]["environment"],
            before["services"]["grafana"]["environment"]
        );
        assert_eq!(
            after["services"]["grafana"]["volumes"],
            before["services"]["grafana"]["volumes"]
        );
        assert_eq!(after["services"]["api"], before["services"]["api"]);
    }

    #[test]
    fn compressed_content_rejects_corruption_trailing_bytes_and_expansion() {
        let mut config = source();
        pack(&mut config).unwrap();
        let bytes = STANDARD
            .decode(
                config["configs"]["grafana_dashboard"]["content"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
        for payload in [
            b"invalid".to_vec(),
            [bytes.as_slice(), b"trailing"].concat(),
            bytes[..bytes.len() - 1].to_vec(),
        ] {
            config["configs"]["grafana_dashboard"]["content"] = json!(STANDARD.encode(payload));
            assert!(dashboard(&config, "grafana_dashboard").is_err());
        }
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&vec![b' '; MAX_DASHBOARD_BYTES + 1])
            .unwrap();
        config["configs"]["grafana_dashboard"]["content"] =
            json!(STANDARD.encode(encoder.finish().unwrap()));
        assert!(dashboard(&config, "grafana_dashboard").is_err());
    }

    #[test]
    fn packing_preserves_owner_start_commands_by_rejecting_them() {
        for key in ["command", "entrypoint"] {
            let mut config = source();
            config["services"]["grafana"][key] = json!(["owner-start"]);
            assert!(pack(&mut config).is_err());
        }
    }

    #[test]
    fn packing_rejects_conflicting_owner_mounts() {
        let mut config = source();
        config["services"]["grafana"]["configs"]
            .as_array_mut()
            .unwrap()
            .push(json!({"source":"owner","target":"/run/groundline/groundline-insights.json.gz.b64"}));
        assert!(pack(&mut config).is_err());
    }
}
