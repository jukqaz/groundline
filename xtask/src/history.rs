use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use serde_json::{Value, json};

use super::XtaskError;
use super::package::{
    contains_private_marker, contains_private_marker_outside_scanner_fixtures, private_source_name,
};

const MAX_OBJECT_LIST_BYTES: usize = 16 * 1024 * 1024;
const MAX_HISTORY_OBJECT_BYTES: u64 = 128 * 1024 * 1024;
const MAX_HISTORY_SCAN_BYTES: u64 = 1024 * 1024 * 1024;
const BINARY_PROBE_BYTES: usize = 8 * 1024;
const SCANNER_SOURCE: &str = "xtask/src/package.rs";

fn public_ci_build_roots() -> [Vec<u8>; 3] {
    [
        [b"/".as_slice(), b"Users/runner/"].concat(),
        [b"/".as_slice(), b"home/runner/"].concat(),
        [b"C:\\".as_slice(), b"Users\\runneradmin\\"].concat(),
    ]
}

fn replace_all(bytes: &mut [u8], needle: &[u8]) {
    let mut start = 0_usize;
    while start + needle.len() <= bytes.len() {
        let Some(offset) = bytes[start..]
            .windows(needle.len())
            .position(|window| window == needle)
        else {
            break;
        };
        let match_start = start + offset;
        bytes[match_start..match_start + needle.len()].fill(b'_');
        start = match_start + needle.len();
    }
}

fn contains_historical_private_marker(bytes: &[u8], binary: bool) -> bool {
    if !contains_private_marker(bytes) {
        return false;
    }
    if !binary {
        return true;
    }
    let mut normalized = bytes.to_vec();
    for root in public_ci_build_roots() {
        replace_all(&mut normalized, &root);
    }
    contains_private_marker(&normalized)
}

fn git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg("core.fsmonitor=false")
        .arg("-C")
        .arg(root);
    command
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn reachable_objects(root: &Path) -> Result<BTreeMap<String, BTreeSet<PathBuf>>, XtaskError> {
    let output = git(root)
        .args(["rev-list", "--objects", "--all"])
        .output()
        .map_err(|_| XtaskError::InvalidHistory)?;
    if !output.status.success()
        || output.stdout.is_empty()
        || output.stdout.len() > MAX_OBJECT_LIST_BYTES
    {
        return Err(XtaskError::InvalidHistory);
    }
    let text = String::from_utf8(output.stdout).map_err(|_| XtaskError::InvalidHistory)?;
    let mut objects = BTreeMap::<String, BTreeSet<PathBuf>>::new();
    for line in text.lines() {
        let (object, path) = line.split_once(' ').map_or((line, None), |(object, path)| {
            (object, (!path.is_empty()).then_some(path))
        });
        if !valid_object_id(object) {
            return Err(XtaskError::InvalidHistory);
        }
        let paths = objects.entry(object.to_owned()).or_default();
        if let Some(path) = path {
            if Path::new(path).is_absolute() {
                return Err(XtaskError::InvalidHistory);
            }
            paths.insert(PathBuf::from(path));
        }
    }
    if objects.is_empty() {
        return Err(XtaskError::InvalidHistory);
    }
    Ok(objects)
}

fn reachable_paths(root: &Path) -> Result<BTreeSet<PathBuf>, XtaskError> {
    let output = git(root)
        .args(["log", "--all", "--format=", "--name-only", "-z"])
        .output()
        .map_err(|_| XtaskError::InvalidHistory)?;
    if !output.status.success() || output.stdout.len() > MAX_OBJECT_LIST_BYTES {
        return Err(XtaskError::InvalidHistory);
    }
    let mut paths = BTreeSet::new();
    for raw in output.stdout.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let value = std::str::from_utf8(raw).map_err(|_| XtaskError::InvalidHistory)?;
        if value.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(XtaskError::InvalidHistory);
        }
        let path = PathBuf::from(value);
        if path.is_absolute() || private_source_name(&path) {
            return Err(XtaskError::InvalidHistory);
        }
        paths.insert(path);
    }
    if paths.is_empty() {
        return Err(XtaskError::InvalidHistory);
    }
    Ok(paths)
}

fn object_inventory(
    root: &Path,
    objects: &BTreeMap<String, BTreeSet<PathBuf>>,
) -> Result<Vec<(String, String, u64)>, XtaskError> {
    let mut child = git(root)
        .arg("cat-file")
        .arg("--batch-check=%(objectname) %(objecttype) %(objectsize)")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| XtaskError::InvalidHistory)?;
    let mut stdin = child.stdin.take().ok_or(XtaskError::InvalidHistory)?;
    let object_ids = objects.keys().cloned().collect::<Vec<_>>();
    let writer = thread::spawn(move || -> Result<(), std::io::Error> {
        for object in object_ids {
            writeln!(stdin, "{object}")?;
        }
        Ok(())
    });
    let output = child
        .wait_with_output()
        .map_err(|_| XtaskError::InvalidHistory)?;
    writer
        .join()
        .map_err(|_| XtaskError::InvalidHistory)?
        .map_err(|_| XtaskError::InvalidHistory)?;
    if !output.status.success() || output.stdout.len() > MAX_OBJECT_LIST_BYTES {
        return Err(XtaskError::InvalidHistory);
    }
    let text = String::from_utf8(output.stdout).map_err(|_| XtaskError::InvalidHistory)?;
    let mut inventory = Vec::with_capacity(objects.len());
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let object = fields.next().ok_or(XtaskError::InvalidHistory)?;
        let kind = fields.next().ok_or(XtaskError::InvalidHistory)?;
        let size = fields
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(XtaskError::InvalidHistory)?;
        if fields.next().is_some()
            || !valid_object_id(object)
            || !objects.contains_key(object)
            || !matches!(kind, "blob" | "commit" | "tag" | "tree")
            || size > MAX_HISTORY_OBJECT_BYTES
        {
            return Err(XtaskError::InvalidHistory);
        }
        inventory.push((object.to_owned(), kind.to_owned(), size));
    }
    if inventory.len() != objects.len() {
        return Err(XtaskError::InvalidHistory);
    }
    Ok(inventory)
}

fn scan_objects(
    root: &Path,
    paths: &BTreeMap<String, BTreeSet<PathBuf>>,
    inventory: &[(String, String, u64)],
) -> Result<(usize, usize, usize, u64, u64), XtaskError> {
    let scanned = inventory
        .iter()
        .filter(|(_, kind, _)| matches!(kind.as_str(), "blob" | "commit" | "tag"))
        .collect::<Vec<_>>();
    let mut child = git(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| XtaskError::InvalidHistory)?;
    let mut stdin = child.stdin.take().ok_or(XtaskError::InvalidHistory)?;
    let object_ids = scanned
        .iter()
        .map(|(object, _, _)| (*object).clone())
        .collect::<Vec<_>>();
    let writer = thread::spawn(move || -> Result<(), std::io::Error> {
        for object in object_ids {
            writeln!(stdin, "{object}")?;
        }
        Ok(())
    });
    let stdout = child.stdout.take().ok_or(XtaskError::InvalidHistory)?;
    let mut reader = BufReader::new(stdout);
    let mut text_blob_count = 0_usize;
    let mut metadata_object_count = 0_usize;
    let mut binary_blob_count = 0_usize;
    let mut scanned_bytes = 0_u64;
    let mut binary_bytes = 0_u64;
    for (expected_object, expected_kind, expected_size) in scanned {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|_| XtaskError::InvalidHistory)?;
        let mut fields = header.split_whitespace();
        let object = fields.next().ok_or(XtaskError::InvalidHistory)?;
        let kind = fields.next().ok_or(XtaskError::InvalidHistory)?;
        let size = fields
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(XtaskError::InvalidHistory)?;
        if fields.next().is_some()
            || object != expected_object
            || kind != expected_kind
            || size != *expected_size
        {
            return Err(XtaskError::InvalidHistory);
        }
        let allocation = usize::try_from(size).map_err(|_| XtaskError::InvalidHistory)?;
        let mut bytes = vec![0_u8; allocation];
        reader
            .read_exact(&mut bytes)
            .map_err(|_| XtaskError::InvalidHistory)?;
        let mut delimiter = [0_u8; 1];
        reader
            .read_exact(&mut delimiter)
            .map_err(|_| XtaskError::InvalidHistory)?;
        if delimiter != *b"\n" {
            return Err(XtaskError::InvalidHistory);
        }
        scanned_bytes = scanned_bytes
            .checked_add(size)
            .ok_or(XtaskError::InvalidHistory)?;
        if scanned_bytes > MAX_HISTORY_SCAN_BYTES {
            return Err(XtaskError::InvalidHistory);
        }
        if kind == "blob" {
            let object_paths = paths.get(object).ok_or(XtaskError::InvalidHistory)?;
            if object_paths.is_empty() || object_paths.iter().any(|path| private_source_name(path))
            {
                return Err(XtaskError::InvalidHistory);
            }
            let scanner_source_only = object_paths.len() == 1
                && object_paths
                    .iter()
                    .next()
                    .is_some_and(|path| path == Path::new(SCANNER_SOURCE));
            let binary = bytes.iter().take(BINARY_PROBE_BYTES).any(|byte| *byte == 0);
            let private_marker_found = if scanner_source_only && !binary {
                contains_private_marker_outside_scanner_fixtures(&bytes)
            } else {
                contains_historical_private_marker(&bytes, binary)
            };
            if private_marker_found {
                return Err(XtaskError::InvalidHistory);
            }
            if binary {
                binary_blob_count += 1;
                binary_bytes = binary_bytes
                    .checked_add(size)
                    .ok_or(XtaskError::InvalidHistory)?;
                continue;
            }
            text_blob_count += 1;
        } else {
            if contains_private_marker(&bytes) {
                return Err(XtaskError::InvalidHistory);
            }
            metadata_object_count += 1;
        }
    }
    drop(reader);
    writer
        .join()
        .map_err(|_| XtaskError::InvalidHistory)?
        .map_err(|_| XtaskError::InvalidHistory)?;
    if !child
        .wait()
        .map_err(|_| XtaskError::InvalidHistory)?
        .success()
    {
        return Err(XtaskError::InvalidHistory);
    }
    Ok((
        text_blob_count,
        metadata_object_count,
        binary_blob_count,
        scanned_bytes,
        binary_bytes,
    ))
}

pub fn verify(root: &Path) -> Result<Value, XtaskError> {
    let root = root
        .canonicalize()
        .map_err(|_| XtaskError::InvalidHistory)?;
    let historical_paths = reachable_paths(&root)?;
    let object_paths = reachable_objects(&root)?;
    let inventory = object_inventory(&root, &object_paths)?;
    let (text_blobs, metadata, binary_blobs, scanned_bytes, binary_bytes) =
        scan_objects(&root, &object_paths, &inventory)?;
    Ok(json!({
        "kind":"groundline-public-history-verification",
        "schema":1,
        "status":"PASS",
        "reachable_object_count":inventory.len(),
        "historical_path_count":historical_paths.len(),
        "text_blob_count":text_blobs,
        "metadata_object_count":metadata,
        "binary_blob_count":binary_blobs,
        "scanned_object_bytes":scanned_bytes,
        "binary_blob_bytes":binary_bytes,
        "binary_markers_scanned":true,
        "generic_ci_build_roots_allowlisted":true,
        "private_marker_count":0,
        "private_filename_count":0,
        "scanner_fixture_regions_normalized":true,
        "scanner_source_excluded":false,
        "mutation_performed":false,
        "private_paths_emitted":false,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use tempfile::tempdir;

    use super::verify;

    fn run(root: &Path, arguments: &[&str]) {
        let status = Command::new("git")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .arg("-C")
            .arg(root)
            .args(arguments)
            .status()
            .expect("execute git fixture");
        assert!(status.success());
    }

    fn repository() -> tempfile::TempDir {
        let root = tempdir().expect("temporary repository");
        run(root.path(), &["init", "-q"]);
        run(root.path(), &["config", "user.name", "GroundLine Test"]);
        run(
            root.path(),
            &[
                "config",
                "user.email",
                "groundline-test@users.noreply.github.com",
            ],
        );
        root
    }

    #[test]
    fn clean_reachable_history_passes() {
        let root = repository();
        fs::write(root.path().join("safe.txt"), "public fixture\n").unwrap();
        run(root.path(), &["add", "safe.txt"]);
        run(root.path(), &["commit", "-qm", "test: add safe fixture"]);
        let result = verify(root.path()).expect("clean history");
        assert_eq!(result["status"], "PASS");
        assert_eq!(result["private_marker_count"], 0);
    }

    #[test]
    fn deleted_private_blob_still_fails_history_gate() {
        let root = repository();
        let private_path = format!("/{}/example/private", "Users");
        fs::write(root.path().join("leak.txt"), private_path).unwrap();
        run(root.path(), &["add", "leak.txt"]);
        run(root.path(), &["commit", "-qm", "test: add fixture"]);
        fs::remove_file(root.path().join("leak.txt")).unwrap();
        run(root.path(), &["add", "-u"]);
        run(root.path(), &["commit", "-qm", "test: remove fixture"]);
        assert!(verify(root.path()).is_err());
    }

    #[test]
    fn binary_private_marker_fails_history_gate() {
        let root = repository();
        let private_path = format!("\0/{}/example/private", "Users");
        fs::write(root.path().join("artifact.bin"), private_path).unwrap();
        run(root.path(), &["add", "artifact.bin"]);
        run(root.path(), &["commit", "-qm", "test: add binary fixture"]);
        assert!(verify(root.path()).is_err());
    }

    #[test]
    fn binary_public_ci_build_root_is_not_personal_data() {
        let root = repository();
        let public_runner_path = format!("\0/{}/runner/work/groundline", "Users");
        fs::write(root.path().join("artifact.bin"), public_runner_path).unwrap();
        run(root.path(), &["add", "artifact.bin"]);
        run(
            root.path(),
            &["commit", "-qm", "test: add CI binary fixture"],
        );
        assert_eq!(verify(root.path()).unwrap()["status"], "PASS");
    }

    #[test]
    fn scanner_source_declaration_does_not_hide_a_separate_leak() {
        let root = repository();
        let source = root.path().join("xtask/src/package.rs");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(
            &source,
            format!(
                "const PRIVATE_MARKERS: &[&[u8]] = &[b\"/{}/\"];\nconst LEAK: &str = \"/{}/private\";\n",
                "Users", "Users"
            ),
        )
        .unwrap();
        run(root.path(), &["add", "xtask/src/package.rs"]);
        run(root.path(), &["commit", "-qm", "test: add scanner fixture"]);
        assert!(verify(root.path()).is_err());
    }

    #[test]
    fn deleted_private_filename_still_fails_history_gate() {
        let root = repository();
        fs::write(root.path().join(".env"), "public-placeholder=true\n").unwrap();
        run(root.path(), &["add", "-f", ".env"]);
        run(
            root.path(),
            &["commit", "-qm", "test: add forbidden filename"],
        );
        fs::remove_file(root.path().join(".env")).unwrap();
        run(root.path(), &["add", "-u"]);
        run(
            root.path(),
            &["commit", "-qm", "test: remove forbidden filename"],
        );
        assert!(verify(root.path()).is_err());
    }
}
