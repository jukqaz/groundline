//! Shared skill syntax validation. Callers own filesystem and invocation policy.
use crate::ContractError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub description: String,
}

pub struct Document {
    pub metadata: Metadata,
    pub body: String,
}

pub fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn parse(source: &str) -> Result<Document, ContractError> {
    let invalid = || ContractError("invalid_skill_metadata".to_owned());
    let source = source.replace("\r\n", "\n");
    let (header, body) = source
        .strip_prefix("---\n")
        .and_then(|s| s.split_once("\n---\n"))
        .ok_or_else(invalid)?;
    let metadata: Metadata = serde_saphyr::from_str(header).map_err(|_| invalid())?;
    if !valid_name(&metadata.name)
        || metadata.description.trim().is_empty()
        || body.trim().is_empty()
    {
        return Err(invalid());
    }
    Ok(Document {
        metadata,
        body: body.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_supports_block_yaml_and_optional_native_fields() {
        let source = "---\nname: valid-skill\ndescription: |\n  A useful skill.\nmetadata:\n  short-description: Useful\n---\n# Body\n";
        for source in [source.to_owned(), source.replace('\n', "\r\n")] {
            let document = parse(&source).unwrap();
            assert_eq!(document.metadata.name, "valid-skill");
            assert_eq!(document.metadata.description.trim(), "A useful skill.");
            assert_eq!(document.body, "# Body\n");
        }
    }

    #[test]
    fn malformed_or_ambiguous_metadata_never_leaks_source_in_errors() {
        for source in [
            "PRIVATE_SENTINEL",
            "---\nname: valid\ndescription: []\n---\nbody",
            "---\nname: valid\nname: other\ndescription: PRIVATE_SENTINEL\n---\nbody",
            "---\nname: UpperCase\ndescription: PRIVATE_SENTINEL\n---\nbody",
            "---\nname: valid\ndescription: ''\n---\nbody",
            "---\nname: valid\ndescription: PRIVATE_SENTINEL\n---\n ",
        ] {
            assert_eq!(
                parse(source).err().unwrap().to_string(),
                "invalid_skill_metadata"
            );
        }
    }
}
