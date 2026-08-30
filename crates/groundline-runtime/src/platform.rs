use std::path::PathBuf;

use groundline_contracts::ContractError;

pub const SUPPORTED_TARGETS: &[&str] = &[
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
    "aarch64-pc-windows-msvc",
    "x86_64-pc-windows-msvc",
];

pub fn current_target() -> Result<&'static str, ContractError> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-musl"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-musl"),
        ("windows", "aarch64") => Ok("aarch64-pc-windows-msvc"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        _ => Err(ContractError("unsupported_platform".to_owned())),
    }
}

pub fn packaged_binary_path(target: &str) -> Result<PathBuf, ContractError> {
    packaged_product_binary_path(target, "groundline")
}

pub fn packaged_insights_binary_path(target: &str) -> Result<PathBuf, ContractError> {
    packaged_product_binary_path(target, "groundline-insights")
}

fn packaged_product_binary_path(
    target: &str,
    executable_name: &str,
) -> Result<PathBuf, ContractError> {
    if !SUPPORTED_TARGETS.contains(&target) {
        return Err(ContractError("unsupported_target".to_owned()));
    }
    let executable = if target.ends_with("windows-msvc") {
        format!("{executable_name}.exe")
    } else {
        executable_name.to_owned()
    };
    Ok(PathBuf::from("bin").join(target).join(executable))
}

#[cfg(test)]
mod tests {
    use super::{
        SUPPORTED_TARGETS, current_target, packaged_binary_path, packaged_insights_binary_path,
    };

    #[test]
    fn all_supported_targets_have_bounded_package_paths() {
        for target in SUPPORTED_TARGETS {
            let path = packaged_binary_path(target).expect("supported target");
            let insights = packaged_insights_binary_path(target).expect("supported target");
            assert_eq!(path.components().count(), 3);
            assert_eq!(insights.components().count(), 3);
            assert!(!path.is_absolute());
            assert!(!insights.is_absolute());
            assert!(path.starts_with("bin"));
            assert!(insights.starts_with("bin"));
            assert_ne!(path, insights);
        }
    }

    #[test]
    fn current_test_host_is_supported() {
        assert!(SUPPORTED_TARGETS.contains(&current_target().expect("supported host")));
    }

    #[test]
    fn arbitrary_target_is_rejected() {
        assert_eq!(
            packaged_binary_path("../../arbitrary").unwrap_err().0,
            "unsupported_target"
        );
    }
}
