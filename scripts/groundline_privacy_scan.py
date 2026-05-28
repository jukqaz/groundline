#!/usr/bin/env python3
"""Scan committed GroundLine text surfaces for privacy and stale-proof leaks."""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ALLOWED_SECRET_FIXTURES = {"sk-test-secret-value"}
GENERIC_HOME_NAMES = {"admin", "home", "root", "runner", "sandbox", "user", "work"}
SCAN_TARGETS = [
    "README.md",
    "README.ko.md",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "docs",
    "references",
    "skills",
    "scripts",
    "tests",
    ".github",
    ".agents",
    ".claude-plugin",
    ".codex-plugin",
    "plugin.json",
    "plugins/groundline",
]
SKIP_PARTS = {".git", "__pycache__"}
SKIP_SUFFIXES = {".pyc"}
SECRET_PATTERNS = [
    re.compile(r"(?<![A-Za-z0-9])sk-[A-Za-z0-9_-]+"),
    re.compile(r"(?<![A-Za-z0-9])xox[baprs]-[A-Za-z0-9_-]+"),
    re.compile(r"ghp_[A-Za-z0-9_]+"),
    re.compile(r"github_pat_[A-Za-z0-9_]+"),
    re.compile(r"-----BEGIN (?:OPENSSH|RSA|EC|DSA) PRIVATE KEY-----"),
]
STALE_CLAIM_PATTERNS = [
    (re.compile(r"\b(?:105|106|107|108|109|110|111|112|113|114) tests OK\b"), "stale_test_count"),
    (re.compile(r"\b44 packaged docs\b"), "stale_packaged_doc_count"),
    (re.compile("local release gates " + "pass", re.IGNORECASE), "overstated_release_gate"),
    (re.compile("passes the local closeout " + "gates", re.IGNORECASE), "overstated_release_gate"),
    (re.compile(r"provider install posture \| PASS"), "overstated_provider_install_posture"),
    (
        re.compile("Provider install evidence: the read-only install doctor reports " + "PASS"),
        "overstated_provider_install_evidence",
    ),
]


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    code: str

    def as_dict(self) -> dict:
        return {"path": self.path, "line": self.line, "code": self.code}


def relative_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def iter_scan_files() -> list[Path]:
    files: list[Path] = []
    for rel in SCAN_TARGETS:
        target = ROOT / rel
        if not target.exists():
            continue
        candidates = [target] if target.is_file() else sorted(target.rglob("*"))
        for path in candidates:
            if not path.is_file():
                continue
            if SKIP_PARTS.intersection(path.parts) or path.suffix in SKIP_SUFFIXES:
                continue
            files.append(path)
    return sorted(set(files))


def has_disallowed_secret(line: str) -> bool:
    for pattern in SECRET_PATTERNS:
        for match in pattern.finditer(line):
            if match.group(0) not in ALLOWED_SECRET_FIXTURES:
                return True
    return False


def detectable_home_name(name: str) -> str:
    normalized = name.strip().lower()
    if len(normalized) < 5 or normalized in GENERIC_HOME_NAMES:
        return ""
    return name


def scan_line(path: Path, line_number: int, line: str, home: str, home_name: str) -> list[Finding]:
    findings: list[Finding] = []
    rel = relative_path(path)
    if home and home in line:
        findings.append(Finding(rel, line_number, "local_home_path"))
    if home_name and home_name.lower() in line.lower():
        findings.append(Finding(rel, line_number, "local_home_name"))
    if has_disallowed_secret(line):
        findings.append(Finding(rel, line_number, "secret_like_value"))
    for pattern, code in STALE_CLAIM_PATTERNS:
        if pattern.search(line):
            findings.append(Finding(rel, line_number, code))
    return findings


def build_result() -> dict:
    home = str(Path.home())
    home_name = detectable_home_name(Path.home().name)
    findings: list[Finding] = []
    scanned_files = 0
    skipped_binary_files = 0

    for path in iter_scan_files():
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            skipped_binary_files += 1
            continue
        scanned_files += 1
        for line_number, line in enumerate(text.splitlines(), start=1):
            findings.extend(scan_line(path, line_number, line, home, home_name))

    return {
        "status": "PASS" if not findings else "FAIL",
        "suite": "privacy-scan",
        "mutation_performed": False,
        "real_home_touched": False,
        "secret_value_printed": False,
        "scanned_file_count": scanned_files,
        "skipped_binary_file_count": skipped_binary_files,
        "finding_count": len(findings),
        "findings": [finding.as_dict() for finding in findings],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Scan GroundLine committed files for privacy and stale-proof leaks.")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    result = build_result()
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    elif result["status"] == "PASS":
        print("GroundLine privacy scan passed")
    else:
        print("GroundLine privacy scan failed")
        for finding in result["findings"]:
            print(f"- {finding['path']}:{finding['line']} {finding['code']}")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
