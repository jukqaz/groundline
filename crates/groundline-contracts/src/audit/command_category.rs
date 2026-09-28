//! Classify a bounded literal command, never text mentioned in its arguments.
//! This is not a shell interpreter: compound/dynamic programs remain unclassified.
use serde_json::Value;

pub(super) fn category(arguments: &str) -> &'static str {
    if arguments.len() > 8192 {
        return "other_command";
    }
    let decoded;
    let command = if arguments.trim_start().starts_with('{') {
        decoded = serde_json::from_str::<Value>(arguments).ok();
        let Some(command) = decoded
            .as_ref()
            .and_then(|v| v.get("cmd"))
            .and_then(Value::as_str)
        else {
            return "other_command";
        };
        command
    } else {
        arguments
    };
    // Shlex tokenizes quoting, not control flow or expansions. Reject those
    // shapes before tokenization, even inside quoted data, rather than guessing.
    if command.contains([
        '\n', '\r', '\0', ';', '&', '|', '<', '>', '(', ')', '`', '$',
    ]) {
        return "other_command";
    }
    let Some((command, cargo_environment)) = strip_cargo_environment(command) else {
        return "other_command";
    };
    let Some(words) = shlex::split(command) else {
        return "other_command";
    };
    let Some((program, args)) = words.split_first() else {
        return "other_command";
    };
    let program = program.rsplit('/').next().unwrap_or(program);
    if cargo_environment && program != "cargo" {
        return "other_command";
    }
    let first = args.first().map(String::as_str);
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "--version" | "-V"))
    {
        return "inspection";
    }
    if program == "cargo" && first == Some("fmt") && !args.iter().any(|arg| arg == "--check") {
        return "mutation";
    }
    let verification = match program {
        "cargo" => matches!(first, Some("test" | "clippy" | "check" | "fmt")),
        "npm" | "pnpm" | "yarn" | "flutter" => first == Some("test"),
        "pytest" | "pytest-3" | "actionlint" => true,
        "python" | "python3" => {
            first == Some("-m")
                && matches!(args.get(1).map(String::as_str), Some("pytest" | "unittest"))
        }
        _ => false,
    };
    if verification {
        "verification"
    } else if matches!(program, "git" | "gh") {
        "git_or_github"
    } else if matches!(
        program,
        "rg" | "grep" | "sed" | "head" | "tail" | "find" | "ls" | "cat" | "wc" | "pwd"
    ) {
        "inspection"
    } else {
        "other_command"
    }
}

fn strip_cargo_environment(command: &str) -> Option<(&str, bool)> {
    let mut command = command.trim_start_matches([' ', '\t']);
    let mut found = false;
    loop {
        let (word, rest) = command.split_once([' ', '\t']).unwrap_or((command, ""));
        let Some((name, path)) = word.split_once('=') else {
            break;
        };
        // Check the original spelling before shlex removes quotes/escapes. Only
        // these Cargo paths are supported, not arbitrary shell assignments.
        if !matches!(name, "CARGO_HOME" | "CARGO_TARGET_DIR")
            || !path.starts_with('/')
            || !path.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-')
            })
        {
            return None;
        }
        found = true;
        command = rest.trim_start_matches([' ', '\t']);
    }
    Some((command, found))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_execution_not_quoted_search_or_display_text() {
        for command in [
            "rg -n 'cargo test' src",
            "rg -e pytest tests",
            "git log --grep='cargo test'",
            "echo 'cargo test'",
            "printf 'pytest'",
            "cargo test --help",
            "pytest --version",
            "cargo fmt",
        ] {
            assert_ne!(category(command), "verification", "{command}");
        }
        for command in [
            "cargo test --locked",
            "cargo clippy",
            "/usr/bin/pytest -q",
            "python3 -m unittest",
            "npm test",
            "flutter test",
            "cargo fmt --check",
        ] {
            assert_eq!(category(command), "verification", "{command}");
            assert_eq!(
                category(&serde_json::json!({"cmd":command}).to_string()),
                "verification"
            );
        }
    }
    #[test]
    fn does_not_infer_control_flow_or_dynamic_commands() {
        for command in [
            "echo ok; cargo test",
            "rg 'cargo test' src\ncargo test",
            "false && cargo test",
            "sh -c 'cargo test'",
            "$(echo cargo) test",
            "cargo test | tee log",
            "'cargo test'",
            "cargo 'test",
            "{\"other\":\"cargo test\"}",
        ] {
            assert_eq!(category(command), "other_command", "{command}");
        }
    }

    #[test]
    fn classifies_literal_cargo_environment_paths() {
        for (command, expected) in [
            (
                "CARGO_HOME=/private/tmp/cargo-union.1 cargo test --locked",
                "verification",
            ),
            ("CARGO_TARGET_DIR=/tmp/build cargo check", "verification"),
            (
                "\t CARGO_HOME=/tmp/cargo\tCARGO_TARGET_DIR=/tmp/build /usr/bin/cargo clippy",
                "verification",
            ),
            ("CARGO_HOME=/tmp/cargo cargo fmt --check", "verification"),
            ("CARGO_HOME=/tmp/cargo cargo test --help", "inspection"),
            ("CARGO_HOME=/tmp/cargo cargo fmt", "mutation"),
        ] {
            assert_eq!(category(command), expected, "{command}");
            assert_eq!(
                category(&serde_json::json!({"cmd": command}).to_string()),
                expected,
                "{command}"
            );
        }
    }

    #[test]
    fn rejects_nonliteral_or_non_cargo_environment_prefixes() {
        for command in [
            "CARGO_HOME=/tmp/cargo",
            "CARGO_HOME= cargo test",
            "CARGO_HOME=relative cargo test",
            "CARGO_HOME=~/cargo cargo test",
            "CARGO_HOME='/tmp/cargo' cargo test",
            "CARGO_HOME=\"/tmp/cargo\" cargo test",
            "'CARGO_HOME=/tmp/cargo' cargo test",
            "\"CARGO_HOME\"=/tmp/cargo cargo test",
            "CARGO_HOME=/tmp/cargo\\ dir cargo test",
            "CARGO_HOME=/tmp/* cargo test",
            "CARGO_HOME=/tmp/cargo? cargo test",
            "CARGO_HOME=/tmp/{cargo,other} cargo test",
            "CARGO_HOME=$HOME/cargo cargo test",
            "CARGO_HOME=$(pwd) cargo test",
            "CARGO_HOME=C:/cargo cargo test",
            "CARGO_HOME=/tmp/한글 cargo test",
            "OTHER=/tmp/cargo cargo test",
            "CARGO_HOME=/tmp/cargo OTHER=/tmp/build cargo test",
            "env CARGO_HOME=/tmp/cargo cargo test",
            "sh -c 'CARGO_HOME=/tmp/cargo cargo test'",
            "CARGO_HOME=/tmp/cargo cargo test && cargo clippy",
            "CARGO_HOME=/tmp/cargo cargo test\ncargo clippy",
            "CARGO_HOME=/tmp/cargo pytest",
            "CARGO_HOME=/tmp/cargo rg 'cargo test' src",
        ] {
            assert_eq!(category(command), "other_command", "{command}");
            assert_eq!(
                category(&serde_json::json!({"cmd": command}).to_string()),
                "other_command",
                "{command}"
            );
        }
    }
}
