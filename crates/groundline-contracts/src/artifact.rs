//! Exact checksum records with native LF or CRLF line terminators.

/// Compare the single checksum record against an independently computed digest.
/// Only the line terminator varies; digest, separator, and filename stay exact.
pub fn checksum_matches(bytes: &[u8], sha256: &str, executable: &str) -> bool {
    let line = bytes
        .strip_suffix(b"\r\n")
        .or_else(|| bytes.strip_suffix(b"\n"));
    line == Some(format!("{sha256}  {executable}").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::checksum_matches;

    #[test]
    fn native_endings_preserve_exact_checksum_validation() {
        let hash = "a".repeat(64);
        let record = format!("{hash}  groundline.exe");
        for ending in ["\n", "\r\n"] {
            assert!(checksum_matches(
                format!("{record}{ending}").as_bytes(),
                &hash,
                "groundline.exe"
            ));
        }
        for invalid in [
            record.clone(),
            format!("{record}\r"),
            format!("{record}\n\n"),
            format!("{record}\r\nextra\r\n"),
            format!("{record} \r\n"),
            format!(" {record}\r\n"),
            format!("{hash} groundline.exe\r\n"),
            format!("{hash}  another.exe\r\n"),
            format!("{}  groundline.exe\r\n", "b".repeat(64)),
            format!("\u{feff}{record}\r\n"),
        ] {
            assert!(!checksum_matches(
                invalid.as_bytes(),
                &hash,
                "groundline.exe"
            ));
        }
    }
}
