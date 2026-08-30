use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::ContractError;

type HmacSha256 = Hmac<Sha256>;

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> Result<String, ContractError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| ContractError("invalid_hmac_key".to_owned()))?;
    mac.update(message);
    Ok(format!("{:x}", mac.finalize().into_bytes()))
}

pub fn verify_hmac_sha256_hex(
    key: &[u8],
    message: &[u8],
    expected: &str,
) -> Result<bool, ContractError> {
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(false);
    }
    let mut expected_bytes = [0_u8; 32];
    let (pairs, remainder) = expected.as_bytes().as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for (index, pair) in pairs.iter().enumerate() {
        let pair = std::str::from_utf8(pair)
            .map_err(|_| ContractError("invalid_hmac_encoding".to_owned()))?;
        expected_bytes[index] = u8::from_str_radix(pair, 16)
            .map_err(|_| ContractError("invalid_hmac_encoding".to_owned()))?;
    }
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| ContractError("invalid_hmac_key".to_owned()))?;
    mac.update(message);
    Ok(mac.verify_slice(&expected_bytes).is_ok())
}

#[cfg(test)]
mod tests {
    use super::{hmac_sha256_hex, sha256_hex, verify_hmac_sha256_hex};

    #[test]
    fn fingerprints_are_stable_sha256() {
        assert_eq!(
            sha256_hex(b"groundline"),
            "3d9ab9e3c115004ae8ea836e40a6f7e4f747f55987337869e3bb9251076b1bae"
        );
    }

    #[test]
    fn hmac_verification_rejects_mutation_and_invalid_encoding() {
        let signature = hmac_sha256_hex(b"private-key", b"bounded-message").unwrap();
        assert!(verify_hmac_sha256_hex(b"private-key", b"bounded-message", &signature).unwrap());
        assert!(!verify_hmac_sha256_hex(b"private-key", b"mutated", &signature).unwrap());
        assert!(!verify_hmac_sha256_hex(b"private-key", b"bounded-message", "not-hex").unwrap());
    }
}
