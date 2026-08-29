use semver::Version;

use crate::ContractError;

pub fn strict_version(value: &str) -> Result<Version, ContractError> {
    let parsed = Version::parse(value).map_err(|_| ContractError("invalid_version".to_owned()))?;
    if !parsed.pre.is_empty()
        || !parsed.build.is_empty()
        || value != format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch)
    {
        return Err(ContractError("invalid_version".to_owned()));
    }
    Ok(parsed)
}

pub fn is_monotonic_upgrade(current: &str, candidate: &str) -> Result<bool, ContractError> {
    Ok(strict_version(candidate)? > strict_version(current)?)
}

#[cfg(test)]
mod tests {
    use super::{is_monotonic_upgrade, strict_version};

    #[test]
    fn accepts_only_canonical_three_part_versions() {
        assert!(strict_version("0.19.0").is_ok());
        for invalid in ["v0.19.0", "0.19", "00.19.0", "0.19.0-alpha.1", "0.19.0+1"] {
            assert_eq!(strict_version(invalid).unwrap_err().0, "invalid_version");
        }
    }

    #[test]
    fn monotonic_upgrade_rejects_equal_or_older_versions() {
        assert!(is_monotonic_upgrade("0.18.9", "0.19.0").unwrap());
        assert!(!is_monotonic_upgrade("0.19.0", "0.19.0").unwrap());
        assert!(!is_monotonic_upgrade("0.19.0", "0.18.9").unwrap());
    }
}
