use std::io::ErrorKind;
use std::path::Path;

use serde_json::{Value, json};

use super::{PROFILE_PATH, check_api_capabilities, default_codex_home, load_profile};

fn receipt(status: &str, result_code: &str, network_attempted: bool) -> Value {
    json!({
        "kind":"groundline-insights-server-check",
        "schema":1,
        "status":status,
        "result_code":result_code,
        "groundline_version":env!("CARGO_PKG_VERSION"),
        "required_basic_contract_revision":groundline_contracts::insights::BASIC_CONTRACT_REVISION,
        "network_attempted":network_attempted,
        "mutation_performed":false,
        "raw_content_emitted":false,
        "private_paths_emitted":false,
        "secret_value_printed":false,
    })
}

/// Check an existing owner's server before replacing the collector binary.
/// This reads only the profile and unauthenticated health endpoint; it never
/// loads credentials, enrolls, initializes state, or changes collection consent.
pub async fn check_server(codex_home: Option<&Path>) -> Value {
    let home = match codex_home
        .map(|path| Ok(path.to_path_buf()))
        .unwrap_or_else(default_codex_home)
    {
        Ok(home) => home,
        Err(_) => return receipt("FAIL", "codex_home_unavailable", false),
    };
    match std::fs::symlink_metadata(home.join(PROFILE_PATH)) {
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return receipt("NOT_CONFIGURED", "owner_profile_not_configured", false);
        }
        Err(_) => return receipt("FAIL", "invalid_owner_profile", false),
        Ok(_) => {}
    }
    let profile = match load_profile(&home) {
        Ok(profile) => profile,
        Err(error) => return receipt("FAIL", &error.to_string(), false),
    };
    match check_api_capabilities(&profile).await {
        Ok(()) => receipt("PASS", "api_compatible", true),
        Err(error) => receipt("FAIL", &error.to_string(), true),
    }
}

#[cfg(test)]
mod tests;
