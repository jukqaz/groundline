//! Stage real binaries with the same manifest/checksum contract as release packages.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

pub fn receipt(output: &Output) -> Value {
    // Native commands print diagnostics before the final JSON receipt. Parse
    // the trailing object without depending on shell-specific indentation.
    let text = String::from_utf8_lossy(&output.stdout);
    for (start, _) in text.match_indices('{').rev() {
        if let Ok(value) = serde_json::from_str::<Value>(text[start..].trim())
            && value["kind"] == "groundline-installation"
        {
            return value;
        }
    }
    panic!(
        "missing installation receipt: {text}; {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn stage(root: &Path, binary: &Path, product: &str, target: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins")
        .join(product);
    for entry in walkdir::WalkDir::new(&source) {
        let entry = entry.unwrap();
        assert!(!entry.file_type().is_symlink());
        let relative = entry.path().strip_prefix(&source).unwrap();
        if relative.starts_with("bin") {
            continue;
        }
        let destination = root.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(destination).unwrap();
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
    let executable = if cfg!(windows) {
        format!("{product}.exe")
    } else {
        product.to_owned()
    };
    let bin = root.join("bin").join(target);
    fs::create_dir_all(&bin).unwrap();
    let installed = bin.join(&executable);
    fs::copy(binary, &installed).unwrap();
    // Match release's stripped fixture without changing the real build or bounds.
    #[cfg(target_os = "linux")]
    assert!(
        std::process::Command::new("strip")
            .arg("--strip-debug")
            .arg(&installed)
            .status()
            .unwrap()
            .success()
    );
    let bytes = fs::read(&installed).unwrap();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    fs::write(
        bin.join(format!("{executable}.sha256")),
        format!("{hash}  {executable}\n"),
    )
    .unwrap();
    fs::write(bin.join("manifest.json"), serde_json::to_vec(&json!({
        "schema_version":1,"kind":"groundline-binary-artifact","groundline_version":env!("CARGO_PKG_VERSION"),
        "target":target,"executable":executable,"size_bytes":bytes.len(),"sha256":hash
    })).unwrap()).unwrap();
    installed
}
