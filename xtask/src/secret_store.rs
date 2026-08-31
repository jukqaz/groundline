use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use groundline_runtime::local_file::{open_bounded_regular_file, private_for_current_user};
use secrecy::SecretString;
use thiserror::Error;

pub const MAX_SECRET_STORE_BYTES: u64 = 16 * 1024;
pub const SECRET_KEYS: &[&str] = &[
    "ADMIN_TOKEN",
    "CLICKHOUSE_PASSWORD",
    "ENROLLMENT_TOKEN",
    "GRAFANA_ADMIN_PASSWORD",
    "GRAFANA_READER_PASSWORD",
    "PROXY_TOKEN",
];

#[derive(Debug, Error)]
#[error("invalid_secret_store")]
pub struct SecretStoreError;

pub fn load_private_secret_store(
    path: &Path,
) -> Result<BTreeMap<String, String>, SecretStoreError> {
    let mut file =
        open_bounded_regular_file(path, 1, MAX_SECRET_STORE_BYTES).map_err(|_| SecretStoreError)?;
    if !private_for_current_user(&file) {
        return Err(SecretStoreError);
    }
    let mut bytes =
        Vec::with_capacity(file.metadata().map_err(|_| SecretStoreError)?.len() as usize);
    file.by_ref()
        .take(MAX_SECRET_STORE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| SecretStoreError)?;
    if bytes.len() as u64 > MAX_SECRET_STORE_BYTES {
        return Err(SecretStoreError);
    }
    let values: BTreeMap<String, String> =
        serde_json::from_slice(&bytes).map_err(|_| SecretStoreError)?;
    if values.len() != SECRET_KEYS.len()
        || !SECRET_KEYS.iter().all(|key| {
            values
                .get(*key)
                .is_some_and(|value| (32..=4096).contains(&value.len()))
        })
    {
        return Err(SecretStoreError);
    }
    Ok(values)
}

pub fn load_private_secret(path: &Path, key: &str) -> Result<SecretString, SecretStoreError> {
    if !SECRET_KEYS.contains(&key) {
        return Err(SecretStoreError);
    }
    load_private_secret_store(path)?
        .remove(key)
        .map(SecretString::from)
        .ok_or(SecretStoreError)
}

#[cfg(test)]
mod tests {
    use groundline_runtime::local_file::atomic_write_private;
    use tempfile::tempdir;

    use super::{SECRET_KEYS, load_private_secret, load_private_secret_store};

    #[test]
    fn secret_store_requires_the_exact_bounded_contract() {
        let root = tempdir().expect("temporary directory");
        let path = root.path().join("secrets.json");
        let values = SECRET_KEYS
            .iter()
            .map(|key| ((*key).to_owned(), "x".repeat(32)))
            .collect::<std::collections::BTreeMap<_, _>>();
        atomic_write_private(&path, &serde_json::to_vec(&values).unwrap()).unwrap();
        assert_eq!(load_private_secret_store(&path).unwrap().len(), 6);
        assert!(load_private_secret(&path, "GRAFANA_ADMIN_PASSWORD").is_ok());
        assert!(load_private_secret(&path, "UNKNOWN").is_err());
        atomic_write_private(&path, br#"{"GRAFANA_ADMIN_PASSWORD":"short"}"#).unwrap();
        assert!(load_private_secret_store(&path).is_err());
    }
}
