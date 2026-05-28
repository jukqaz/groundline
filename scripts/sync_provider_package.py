#!/usr/bin/env python3
"""Stage the installable provider package under plugins/groundline."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PACKAGE_ROOT = ROOT / "plugins/groundline"

TOP_LEVEL_FILES = [
    "README.md",
    "README.ko.md",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "LICENSE",
    "SECURITY.md",
    "plugin.json",
]

TOP_LEVEL_DIRS = [
    "assets",
    "docs",
    "references",
    "scenarios",
    "scripts",
    "skills",
]

MANIFEST_FILES = [
    (".codex-plugin/plugin.json", ".codex-plugin/plugin.json"),
    (".claude-plugin/plugin.json", ".claude-plugin/plugin.json"),
]
CONFLICT_COPY_PATTERN = re.compile(r" \d+$")

SOURCE_ROOT_MARKERS = [
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
]


def copy_file(src_rel: str, dest_rel: str) -> None:
    src = ROOT / src_rel
    dest = PACKAGE_ROOT / dest_rel
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        dest.unlink()
    dest.write_bytes(src.read_bytes())


def copy_dir(rel: str) -> None:
    src = ROOT / rel
    dest = PACKAGE_ROOT / rel
    expected_files: set[Path] = set()
    for path in sorted(src.rglob("*")):
        if path.is_dir():
            continue
        if "__pycache__" in path.parts or path.suffix == ".pyc":
            continue
        if rel == "docs" and "superpowers" in path.relative_to(src).parts:
            continue
        relative = path.relative_to(src)
        expected_files.add(relative)
        target = dest / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if target.exists():
            target.unlink()
        target.write_bytes(path.read_bytes())

    if not dest.is_dir():
        return
    for path in sorted(dest.rglob("*"), key=lambda item: len(item.parts), reverse=True):
        if CONFLICT_COPY_PATTERN.search(path.name):
            continue
        relative = path.relative_to(dest)
        if path.is_file() and relative not in expected_files:
            path.unlink()
        elif path.is_dir() and not any(path.iterdir()):
            path.rmdir()

def ensure_package_root() -> None:
    if PACKAGE_ROOT == ROOT or ROOT not in PACKAGE_ROOT.parents:
        raise RuntimeError(f"refusing to clean unsafe package path: {PACKAGE_ROOT}")
    PACKAGE_ROOT.mkdir(parents=True, exist_ok=True)


def ensure_source_root() -> None:
    missing = [marker for marker in SOURCE_ROOT_MARKERS if not (ROOT / marker).is_file()]
    if missing:
        raise RuntimeError(
            "sync_provider_package.py must run from the GroundLine source root; "
            f"missing source root marker(s): {', '.join(missing)}"
        )
    if ROOT.name == "groundline" and ROOT.parent.name == "plugins":
        raise RuntimeError(
            "sync_provider_package.py must not run from an installable provider package; "
            "run the source-root script instead"
        )


def conflict_copy_paths() -> list[Path]:
    if not PACKAGE_ROOT.is_dir():
        return []
    return sorted(
        (path for path in PACKAGE_ROOT.rglob("*") if CONFLICT_COPY_PATTERN.search(path.name)),
        key=lambda item: len(item.parts),
        reverse=True,
    )


def clean_conflict_copies(timeout_seconds: float = 8.0, interval_seconds: float = 0.25) -> dict[str, list[str]]:
    removed: set[str] = set()
    clean_scans = 0
    deadline = time.monotonic() + timeout_seconds
    paths: list[Path] = []

    while time.monotonic() < deadline:
        paths = conflict_copy_paths()
        if not paths:
            clean_scans += 1
            if clean_scans >= 8:
                return {"removed": sorted(removed), "remaining": []}
            time.sleep(interval_seconds)
            continue

        clean_scans = 0
        for path in paths:
            removed.add(str(path.relative_to(PACKAGE_ROOT)))
            if path.is_dir():
                shutil.rmtree(path)
            else:
                path.unlink()
        time.sleep(interval_seconds)

    return {"removed": sorted(removed), "remaining": [str(path.relative_to(PACKAGE_ROOT)) for path in conflict_copy_paths()]}


def sync_package() -> dict:
    ensure_source_root()
    ensure_package_root()

    for src_rel, dest_rel in MANIFEST_FILES:
        copy_file(src_rel, dest_rel)
    for rel in TOP_LEVEL_FILES:
        copy_file(rel, rel)
    for rel in TOP_LEVEL_DIRS:
        copy_dir(rel)
    conflict_cleanup = clean_conflict_copies()

    skill_count = len([path for path in (PACKAGE_ROOT / "skills").iterdir() if path.is_dir()])
    status = "PASS" if not conflict_cleanup["remaining"] else "FAIL"
    return {
        "status": status,
        "errors": [
            f"packaged conflict copy remained after cleanup: {path}" for path in conflict_cleanup["remaining"]
        ],
        "mutation_performed": True,
        "package_path": str(PACKAGE_ROOT),
        "skill_count": skill_count,
        "conflict_copies_removed": conflict_cleanup["removed"],
        "remaining_conflict_copies": conflict_cleanup["remaining"],
        "marketplace_files": [
            ".agents/plugins/marketplace.json",
            ".claude-plugin/marketplace.json",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Stage the GroundLine provider package.")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    try:
        result = sync_package()
    except RuntimeError as exc:
        result = {
            "status": "FAIL",
            "errors": [str(exc)],
            "mutation_performed": False,
            "package_path": str(PACKAGE_ROOT),
            "skill_count": 0,
            "conflict_copies_removed": [],
            "remaining_conflict_copies": [],
            "marketplace_files": [
                ".agents/plugins/marketplace.json",
                ".claude-plugin/marketplace.json",
            ],
        }
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        if result["status"] == "PASS":
            print(f"staged {result['package_path']} with {result['skill_count']} skills")
        else:
            print("sync_provider_package.py failed:")
            for error in result["errors"]:
                print(f"- {error}")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
