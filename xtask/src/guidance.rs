//! Validate callable skill metadata and local references, not prose semantics.
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

use super::XtaskError;
use super::package::regular_bytes;

const IMPLICIT_SKILLS: &[&str] = &[
    "align-agent-home",
    "close-live-work",
    "reconcile-current-state",
];

#[derive(Deserialize)]
struct SkillIndex {
    kind: String,
    schema: u32,
    skills: Vec<String>,
}

#[derive(Deserialize)]
struct SkillUi {
    interface: Interface,
    policy: Policy,
}

#[derive(Deserialize)]
struct Interface {
    display_name: String,
    short_description: String,
    default_prompt: String,
}

#[derive(Deserialize)]
struct Policy {
    allow_implicit_invocation: bool,
}

fn text(path: &Path) -> Result<String, XtaskError> {
    String::from_utf8(regular_bytes(path)?).map_err(|_| XtaskError::InvalidSource)
}

fn local_links(root: &Path, file: &Path, body: &str) -> Result<(), XtaskError> {
    static LINKS: OnceLock<Regex> = OnceLock::new();
    let links = LINKS.get_or_init(|| Regex::new(r"\[[^\]\n]*\]\(([^)\s]+)\)").unwrap());
    for found in links.captures_iter(body) {
        let target = found[1].split('#').next().unwrap_or("");
        if target.is_empty() || target.starts_with("https://") || target.starts_with("http://") {
            continue;
        }
        if Path::new(target).is_absolute() || target.contains(':') || target.contains('\\') {
            return Err(XtaskError::InvalidSource);
        }
        let joined = file.parent().ok_or(XtaskError::InvalidSource)?.join(target);
        let resolved = joined
            .canonicalize()
            .map_err(|_| XtaskError::InvalidSource)?;
        if !resolved.starts_with(root) {
            return Err(XtaskError::InvalidSource);
        }
        regular_bytes(&joined)?;
    }
    Ok(())
}

fn verify_skill(root: &Path, name: &str) -> Result<(), XtaskError> {
    let skill = root.join("skills").join(name);
    let entry = skill.join("SKILL.md");
    let document = groundline_contracts::skill::parse(&text(&entry)?)
        .map_err(|_| XtaskError::InvalidSource)?;
    let ui: SkillUi = serde_saphyr::from_str(&text(&skill.join("agents/openai.yaml"))?)
        .map_err(|_| XtaskError::InvalidSource)?;
    let invocation = format!("${name}");
    let invokes_skill = ui.interface.default_prompt.split_whitespace().any(|word| {
        word.trim_matches(|c: char| matches!(c, '`' | ',' | '.' | ';' | ':' | '(' | ')'))
            == invocation
    });
    if document.metadata.name != name
        || ui.interface.display_name.trim().is_empty()
        || !(25..=64).contains(&ui.interface.short_description.chars().count())
        || !invokes_skill
        || ui.policy.allow_implicit_invocation != IMPLICIT_SKILLS.contains(&name)
    {
        return Err(XtaskError::InvalidSource);
    }
    local_links(root, &entry, &document.body)
}

pub(super) fn verify(package: &Path, names: &BTreeSet<String>) -> Result<(), XtaskError> {
    let root = package
        .canonicalize()
        .map_err(|_| XtaskError::InvalidSource)?;
    let index: SkillIndex =
        serde_json::from_slice(&regular_bytes(&root.join("references/skill-index.json"))?)?;
    let indexed = index.skills.iter().cloned().collect::<BTreeSet<_>>();
    if index.kind != "groundline-skill-index"
        || index.schema != 1
        || indexed != *names
        || indexed.len() != index.skills.len()
    {
        return Err(XtaskError::InvalidSource);
    }
    for name in names {
        verify_skill(&root, name)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::{TempDir, tempdir};

    fn fixture() -> (TempDir, BTreeSet<String>) {
        let root = tempdir().unwrap();
        let skill = root.path().join("skills/close-live-work");
        fs::create_dir_all(skill.join("agents")).unwrap();
        fs::create_dir(root.path().join("references")).unwrap();
        fs::write(
            root.path().join("references/skill-index.json"),
            r#"{"kind":"groundline-skill-index","schema":1,"skills":["close-live-work"]}"#,
        )
        .unwrap();
        fs::write(skill.join("SKILL.md"), "---\nname: close-live-work\ndescription: |\n  Verify live evidence.\n---\n# Live\n[Contract](../../references/live.md)\n").unwrap();
        fs::write(root.path().join("references/live.md"), "# Live proof\n").unwrap();
        fs::write(skill.join("agents/openai.yaml"), "interface:\n  display_name: Live\n  short_description: Verify the requested live outcome\n  default_prompt: Use $close-live-work to check evidence.\npolicy:\n  allow_implicit_invocation: true\n").unwrap();
        (root, BTreeSet::from(["close-live-work".to_owned()]))
    }

    #[test]
    fn valid_metadata_supports_block_yaml_and_crlf() {
        let (root, names) = fixture();
        assert!(verify(root.path(), &names).is_ok());
        let path = root.path().join("skills/close-live-work/SKILL.md");
        fs::write(
            &path,
            fs::read_to_string(&path).unwrap().replace('\n', "\r\n"),
        )
        .unwrap();
        assert!(verify(root.path(), &names).is_ok());
    }

    #[test]
    fn missing_entry_or_interface_fails_even_when_directory_exists() {
        for relative in ["SKILL.md", "agents/openai.yaml"] {
            let (root, names) = fixture();
            fs::remove_file(root.path().join("skills/close-live-work").join(relative)).unwrap();
            assert!(verify(root.path(), &names).is_err(), "{relative}");
        }
    }

    #[test]
    fn malformed_wrong_name_and_empty_descriptions_are_rejected() {
        for header in [
            "name: [",
            "name: other\ndescription: Live",
            "name: close-live-work\ndescription: ''",
            "name: close-live-work\nname: other\ndescription: Live",
        ] {
            let (root, names) = fixture();
            fs::write(
                root.path().join("skills/close-live-work/SKILL.md"),
                format!("---\n{header}\n---\n# Live\n"),
            )
            .unwrap();
            assert!(verify(root.path(), &names).is_err(), "{header}");
        }
    }

    #[test]
    fn index_must_match_without_duplicates() {
        for skills in [
            vec![],
            vec!["other"],
            vec!["close-live-work", "close-live-work"],
        ] {
            let (root, names) = fixture();
            fs::write(root.path().join("references/skill-index.json"), serde_json::to_vec(&serde_json::json!({"kind":"groundline-skill-index","schema":1,"skills":skills})).unwrap()).unwrap();
            assert!(verify(root.path(), &names).is_err());
        }
    }

    #[test]
    fn wrong_invocation_and_implicit_policy_are_rejected() {
        for (from, to) in [
            ("$close-live-work", "$other"),
            ("$close-live-work", "$close-live-work-extra"),
            (
                "allow_implicit_invocation: true",
                "allow_implicit_invocation: false",
            ),
        ] {
            let (root, names) = fixture();
            let path = root
                .path()
                .join("skills/close-live-work/agents/openai.yaml");
            fs::write(&path, fs::read_to_string(&path).unwrap().replace(from, to)).unwrap();
            assert!(verify(root.path(), &names).is_err());
        }
    }

    #[test]
    fn links_must_exist_and_stay_inside_package() {
        for target in [
            "../../references/missing.md",
            "../../../../outside.md",
            "file:private",
            "C:\\private",
        ] {
            let (root, names) = fixture();
            let path = root.path().join("skills/close-live-work/SKILL.md");
            fs::write(
                &path,
                fs::read_to_string(&path)
                    .unwrap()
                    .replace("../../references/live.md", target),
            )
            .unwrap();
            assert!(verify(root.path(), &names).is_err(), "{target}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn links_reject_symlink_escape_to_existing_file() {
        let (root, names) = fixture();
        let external = tempdir().unwrap();
        let file = external.path().join("live.md");
        fs::write(&file, "private").unwrap();
        let link = root.path().join("references/live.md");
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert!(verify(root.path(), &names).is_err());
    }

    #[test]
    fn canonical_repository_skills_validate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("plugins/groundline");
        let names = fs::read_dir(root.join("skills"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        verify(&root, &names).unwrap();
    }
}
