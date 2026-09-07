use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn execute(args: &[&str]) -> (bool, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .args(args)
        .output()
        .unwrap();
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!value.to_string().contains("private-instructions"));
    assert!(!value.to_string().contains("private-profile"));
    (output.status.success(), value)
}
fn arg(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn cli_inventory_capture_reaudit_and_new_skill_discovery() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("skills");
    fs::create_dir_all(root.join("example")).unwrap();
    fs::write(
        root.join("example/SKILL.md"),
        "---\nname: example\ndescription: A bounded task\n---\nprivate-instructions\n",
    )
    .unwrap();
    let profile = temp.path().join("private-profile.json");
    fs::write(
        &profile,
        serde_json::to_vec(
            &json!({"kind":"groundline-guidance-profile","schema":1,"roots":{"personal":root}}),
        )
        .unwrap(),
    )
    .unwrap();
    let baseline = temp.path().join("baseline.json");
    let (ok, value) = execute(&["guidance", "audit", "--profile", arg(&profile), "--json"]);
    assert!(ok);
    assert_eq!(value["status"], "NOT_BASELINED");
    let (ok, value) = execute(&[
        "guidance",
        "snapshot",
        "--profile",
        arg(&profile),
        "--output",
        arg(&baseline),
        "--json",
    ]);
    assert!(ok);
    assert_eq!(value["private_receipt_written"], true);
    let (ok, value) = execute(&[
        "guidance",
        "audit",
        "--profile",
        arg(&profile),
        "--baseline",
        arg(&baseline),
        "--json",
    ]);
    assert!(ok);
    assert_eq!(value["status"], "PASS");
    fs::create_dir(root.join("new")).unwrap();
    fs::copy(root.join("example/SKILL.md"), root.join("new/SKILL.md")).unwrap();
    let (ok, value) = execute(&[
        "guidance",
        "audit",
        "--profile",
        arg(&profile),
        "--baseline",
        arg(&baseline),
        "--json",
    ]);
    assert!(ok);
    assert_eq!(value["installed_counts"]["added"], 1);
    assert_eq!(value["duplicate_name_count"], 1);
    assert_eq!(value["mutation_performed"], false);
    let old = fs::read(&baseline).unwrap();
    let (ok, _) = execute(&[
        "guidance",
        "snapshot",
        "--profile",
        arg(&profile),
        "--output",
        arg(&baseline),
        "--json",
    ]);
    assert!(!ok);
    assert_eq!(fs::read(&baseline).unwrap(), old);
}
#[test]
fn legacy_command_and_schema_are_rejected() {
    let output = Command::new(env!("CARGO_BIN_EXE_groundline"))
        .args(["guidance", "init", "--help"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let temp = tempdir().unwrap();
    let old = temp.path().join("private-profile.json");
    fs::write(
        &old,
        r#"{"schema_version":1,"automatic_updates":false,"skills":[]}"#,
    )
    .unwrap();
    let (ok, result) = execute(&["guidance", "audit", "--profile", arg(&old), "--json"]);
    assert!(!ok);
    assert_eq!(result["status"], "FAIL");
    assert_eq!(result["private_paths_emitted"], false);
}
