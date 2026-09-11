//! Exact Codex source classification shared by collectors and local state routing.
use thiserror::Error;

#[derive(Debug, Error)]
#[error("unsupported_runtime_environment")]
pub struct EnvironmentError;

pub fn codex_originator(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "codex desktop" | "codex_app" | "codex vscode" | "codex_vscode" => Some("codex_app"),
        "codex_tui" | "codex_exec" | "codex_cli" => Some("codex_cli"),
        _ => None,
    }
}

pub fn foreign_originator(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "claude code" | "hermes" | "gemini cli" | "antigravity"
    )
}

#[derive(Debug, PartialEq, Eq)]
pub struct Environment {
    pub runtime: &'static str,
    pub mode: &'static str,
}

impl Environment {
    pub fn current() -> Result<Self, EnvironmentError> {
        fn read(name: &str) -> Result<Option<String>, EnvironmentError> {
            match std::env::var(name) {
                Ok(value) => Ok(Some(value)),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(_) => Err(EnvironmentError),
            }
        }
        Self::resolve(
            read("GROUNDLINE_RUNTIME_FAMILY")?.as_deref(),
            read("GROUNDLINE_EXECUTION_MODE")?.as_deref(),
            read("CODEX_INTERNAL_ORIGINATOR_OVERRIDE")?.as_deref(),
        )
    }

    fn resolve(
        runtime: Option<&str>,
        mode: Option<&str>,
        originator: Option<&str>,
    ) -> Result<Self, EnvironmentError> {
        let runtime = match runtime.map(str::to_ascii_lowercase).as_deref() {
            Some("codex_app") => "codex_app",
            Some("codex_cli") => "codex_cli",
            Some(_) => return Err(EnvironmentError),
            None => match originator {
                Some(value) => codex_originator(value).ok_or(EnvironmentError)?,
                None => "codex_cli",
            },
        };
        let mode = match mode.map(str::to_ascii_lowercase).as_deref() {
            Some("desktop") => "desktop",
            Some("local_headless") => "local_headless",
            Some("remote_headless") => "remote_headless",
            Some(_) => return Err(EnvironmentError),
            None if runtime == "codex_app" => "desktop",
            None => "local_headless",
        };
        Ok(Self { runtime, mode })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_explicit_values_never_fall_back_to_codex() {
        for value in ["claude_code", "hermes", "gemini", "unknown", ""] {
            assert!(Environment::resolve(Some(value), None, Some("codex desktop")).is_err());
        }
        assert!(Environment::resolve(Some("codex_app"), Some("invalid"), None).is_err());
        assert!(Environment::resolve(None, None, Some("noncodex-desktop")).is_err());
        assert!(Environment::resolve(None, None, Some("Claude Code")).is_err());
    }
    #[test]
    fn native_sources_and_explicit_remote_execution_are_preserved() {
        assert_eq!(
            Environment::resolve(None, None, None).unwrap(),
            Environment {
                runtime: "codex_cli",
                mode: "local_headless"
            }
        );
        assert_eq!(
            Environment::resolve(None, None, Some("Codex Desktop")).unwrap(),
            Environment {
                runtime: "codex_app",
                mode: "desktop"
            }
        );
        assert_eq!(
            Environment::resolve(Some("codex_cli"), Some("remote_headless"), None)
                .unwrap()
                .mode,
            "remote_headless"
        );
        for value in ["Claude Code", "Hermes", "Gemini CLI", "Antigravity"] {
            assert!(foreign_originator(value));
            assert!(codex_originator(value).is_none());
        }
        assert!(!foreign_originator("future-codex"));
    }
}
