use chrono::NaiveDate;
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

/// Derive YYYY.MM.DD-sequence from the canonical year.MMDD.ordinal release version.
/// Historical and wire versions continue to use `strict_version` independently.
pub fn release_display_name(value: &str) -> Result<String, ContractError> {
    let version = strict_version(value)?;
    let invalid = || ContractError("invalid_release_date_version".to_owned());
    if !(1..=9999).contains(&version.major) || version.patch == 0 {
        return Err(invalid());
    }
    let month = u32::try_from(version.minor / 100).map_err(|_| invalid())?;
    let day = (version.minor % 100) as u32;
    let date = NaiveDate::from_ymd_opt(version.major as i32, month, day).ok_or_else(invalid)?;
    let mut ordinal = version.patch;
    let mut suffix = Vec::new();
    while ordinal > 0 {
        ordinal -= 1;
        suffix.push(char::from(b'a' + (ordinal % 26) as u8));
        ordinal /= 26;
    }
    let suffix: String = suffix.into_iter().rev().collect();
    Ok(format!("{}-{suffix}", date.format("%Y.%m.%d")))
}

#[cfg(test)]
mod tests {
    use super::{is_monotonic_upgrade, release_display_name, strict_version};

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

    #[test]
    fn release_names_derive_dates_and_daily_letter_sequences() {
        for (version, expected) in [
            ("2026.929.1", "2026.09.29-a"),
            ("2026.929.2", "2026.09.29-b"),
            ("2026.929.26", "2026.09.29-z"),
            ("2026.929.27", "2026.09.29-aa"),
            ("2026.929.52", "2026.09.29-az"),
            ("2026.929.53", "2026.09.29-ba"),
            ("2026.929.702", "2026.09.29-zz"),
            ("2026.929.703", "2026.09.29-aaa"),
            ("2026.930.1", "2026.09.30-a"),
            ("2026.1001.1", "2026.10.01-a"),
            ("2027.101.1", "2027.01.01-a"),
            ("2000.229.1", "2000.02.29-a"),
            ("2028.229.1", "2028.02.29-a"),
        ] {
            assert_eq!(release_display_name(version).unwrap(), expected);
        }
    }

    #[test]
    fn release_names_reject_invalid_dates_and_zero_ordinals_only_at_display_boundary() {
        for invalid in [
            "0.19.0",
            "2026.929.0",
            "2026.0.1",
            "2026.100.1",
            "2026.132.1",
            "2026.229.1",
            "1900.229.1",
            "2100.229.1",
            "2026.431.1",
            "2026.1301.1",
            "10000.101.1",
            "2026.18446744073709551615.1",
        ] {
            assert!(strict_version(invalid).is_ok(), "wire version: {invalid}");
            assert_eq!(
                release_display_name(invalid).unwrap_err().0,
                "invalid_release_date_version",
                "{invalid}"
            );
        }
        assert!(release_display_name("2026.929.18446744073709551615").is_ok());
        for invalid in ["2026.09.29-a", "2026.0929.1", "2026.929.1+build"] {
            assert_eq!(
                release_display_name(invalid).unwrap_err().0,
                "invalid_version"
            );
        }
    }

    #[test]
    fn release_order_preserves_daily_monthly_yearly_progress_and_rejects_rollbacks() {
        for (current, next) in [
            ("0.29.0", "2026.929.1"),
            ("2026.929.1", "2026.929.2"),
            ("2026.929.26", "2026.929.27"),
            ("2026.929.703", "2026.930.1"),
            ("2026.930.2", "2026.1001.1"),
            ("2026.1231.26", "2027.101.1"),
            ("2028.228.1", "2028.229.1"),
            ("2028.229.26", "2028.301.1"),
        ] {
            assert!(is_monotonic_upgrade(current, next).unwrap());
            assert!(!is_monotonic_upgrade(next, current).unwrap());
            assert!(!is_monotonic_upgrade(next, next).unwrap());
        }
    }
}
