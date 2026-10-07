#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn executable(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn fake_platform(directory: &Path) -> std::ffi::OsString {
    fs::create_dir(directory).unwrap();
    executable(&directory.join("uname"), b"#!/bin/sh\ncase \"$1\" in -s) printf '%s\\n' \"$DISPATCH_OS\";; -m) printf '%s\\n' \"$DISPATCH_ARCH\";; *) exit 1;; esac\n");
    executable(&directory.join("sysctl"), b"#!/bin/sh\n[ \"$*\" = '-in hw.optional.arm64' ] || exit 1\n[ \"$DISPATCH_ARM64\" != failure ] || exit 1\nprintf '%s\\n' \"$DISPATCH_ARM64\"\n");
    let existing = std::env::var_os("PATH").unwrap();
    std::env::join_paths(
        std::iter::once(directory.to_owned()).chain(std::env::split_paths(&existing)),
    )
    .unwrap()
}

fn observed(path: &Path) -> String {
    if path.exists() {
        fs::read_to_string(path).unwrap()
    } else {
        String::new()
    }
}

const CASES: &[(&str, &str, &str, Option<&str>)] = &[
    ("Darwin", "arm64", "failure", Some("aarch64-apple-darwin")),
    ("Darwin", "x86_64", "1", Some("aarch64-apple-darwin")),
    ("Darwin", "x86_64", "0", None),
    ("Darwin", "x86_64", "failure", None),
    (
        "Linux",
        "aarch64",
        "failure",
        Some("aarch64-unknown-linux-musl"),
    ),
    (
        "Linux",
        "x86_64",
        "failure",
        Some("x86_64-unknown-linux-musl"),
    ),
    ("MINGW_NT-10.0", "x86_64", "failure", None),
];

fn fake_products(root: &Path, name: &str) {
    for target in groundline_runtime::platform::SUPPORTED_TARGETS
        .iter()
        .copied()
        .chain(["x86_64-apple-darwin"])
    {
        let directory = root.join("bin").join(target);
        fs::create_dir_all(&directory).unwrap();
        executable(&directory.join(name), format!("#!/bin/sh\nprintf '%s|%s\\n' '{target}' \"$*\" >> \"$DISPATCH_BINARY_LOG\"\nexit 17\n").as_bytes());
    }
}

#[test]
fn lifecycle_hooks_use_arm_under_rosetta_and_skip_intel_and_unknown_hosts() {
    let hooks: Value = serde_json::from_slice(include_bytes!(
        "../../plugins/groundline-insights/hooks/hooks.json"
    ))
    .unwrap();
    for &(os, architecture, arm64, target) in CASES {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let path = fake_platform(&root.join("tools"));
        let plugin = root.join("plugin tree");
        fake_products(&plugin, "groundline-insights");
        let log = root.join("binary.log");
        for (event, trigger) in [
            ("SessionStart", "session_start_hook"),
            ("Stop", "stop_hook"),
            ("PostCompact", "post_compact_hook"),
            ("SessionEnd", "session_end_hook"),
            ("UserPromptSubmit", "user_prompt_submit_hook"),
        ] {
            fs::write(&log, "").unwrap();
            let output = Command::new("/bin/sh")
                .args([
                    "-c",
                    hooks["hooks"][event][0]["hooks"][0]["command"]
                        .as_str()
                        .unwrap(),
                ])
                .env("PATH", &path)
                .env("DISPATCH_OS", os)
                .env("DISPATCH_ARCH", architecture)
                .env("DISPATCH_ARM64", arm64)
                .env("DISPATCH_BINARY_LOG", &log)
                .env("PLUGIN_ROOT", &plugin)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{event} {os} {architecture} {arm64}"
            );
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty());
            assert_eq!(
                observed(&log),
                target
                    .map(|target| format!(
                        "{target}|checkpoint {trigger} --plugin-root {}\n",
                        plugin.display()
                    ))
                    .unwrap_or_default()
            );
        }
    }
}

#[test]
fn installer_rejects_intel_before_native_calls_and_dispatches_supported_hardware() {
    for &(os, architecture, arm64, target) in CASES {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let path = fake_platform(&root.join("tools"));
        let distribution = root.join("distribution");
        fs::create_dir(&distribution).unwrap();
        fs::write(
            distribution.join("install.sh"),
            include_bytes!("../../install.sh"),
        )
        .unwrap();
        fake_products(&distribution.join("plugins/groundline"), "groundline");
        let codex = root.join("fake-codex");
        executable(&codex, b"#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$DISPATCH_NATIVE_LOG\"\ncase \"$*\" in\n  *' --help') exit 0;;\n  'plugin marketplace list --json') printf '%s\\n' '{\"marketplaces\":[]}' ;;\n  'plugin list --json') printf '%s\\n' '{\"installed\":[]}' ;;\n  *) exit 91;;\nesac\n");
        let home = root.join("codex-home");
        fs::create_dir(&home).unwrap();
        fs::write(home.join("config.toml"), b"owner-config-preserve\n").unwrap();
        let native_log = root.join("native.log");
        let binary_log = root.join("binary.log");
        let output = Command::new("/bin/bash")
            .arg(distribution.join("install.sh"))
            .arg("--codex")
            .arg(&codex)
            .env("PATH", &path)
            .env("CODEX_HOME", &home)
            .env("DISPATCH_OS", os)
            .env("DISPATCH_ARCH", architecture)
            .env("DISPATCH_ARM64", arm64)
            .env("DISPATCH_NATIVE_LOG", &native_log)
            .env("DISPATCH_BINARY_LOG", &binary_log)
            .output()
            .unwrap();
        // A supported fixture stops at its selected provider before any install.
        assert_eq!(output.status.code(), Some(1));
        let stdout = String::from_utf8(output.stdout).unwrap();
        let result: Value =
            serde_json::from_str(stdout.lines().rfind(|line| !line.is_empty()).unwrap()).unwrap();
        assert_eq!(result["status"], "FAIL");
        if let Some(target) = target {
            assert_eq!(result["stages"]["preflight"]["status"], "PASS");
            assert_eq!(result["stages"]["distribution_groundline"]["exit_code"], 17);
            assert!(observed(&binary_log).starts_with(&format!("{target}|provider-smoke ")));
            assert_eq!(observed(&native_log).lines().count(), 6);
        } else {
            assert_eq!(result["stages"]["preflight"]["status"], "FAIL");
            assert!(observed(&native_log).is_empty());
            assert!(observed(&binary_log).is_empty());
            assert!(result["stages"].get("native_source").is_none());
        }
        assert_eq!(
            fs::read(home.join("config.toml")).unwrap(),
            b"owner-config-preserve\n"
        );
        assert_eq!(fs::read_dir(home).unwrap().count(), 1);
    }
}
