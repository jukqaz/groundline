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
    let Some(words) = shlex::split(command) else {
        return "other_command";
    };
    let Some((program, args)) = words.split_first() else {
        return "other_command";
    };
    let program = program.rsplit('/').next().unwrap_or(program);
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
}
