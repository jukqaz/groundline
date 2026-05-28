#!/usr/bin/env python3
"""Stage the installable provider package under plugins/groundline."""

from __future__ import annotations

import argparse
import json
import shutil
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


def copy_file(src_rel: str, dest_rel: str) -> None:
    src = ROOT / src_rel
    dest = PACKAGE_ROOT / dest_rel
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)


def copy_dir(rel: str) -> None:
    src = ROOT / rel
    dest = PACKAGE_ROOT / rel
    ignored = shutil.ignore_patterns("__pycache__", "*.pyc")
    if rel == "docs":
        ignored = shutil.ignore_patterns("__pycache__", "*.pyc", "superpowers")
    shutil.copytree(src, dest, dirs_exist_ok=True, ignore=ignored)


def clean_package_root() -> None:
    if PACKAGE_ROOT == ROOT or ROOT not in PACKAGE_ROOT.parents:
        raise RuntimeError(f"refusing to clean unsafe package path: {PACKAGE_ROOT}")
    if PACKAGE_ROOT.exists():
        shutil.rmtree(PACKAGE_ROOT)
    PACKAGE_ROOT.mkdir(parents=True)


def sync_package() -> dict:
    clean_package_root()

    for src_rel, dest_rel in MANIFEST_FILES:
        copy_file(src_rel, dest_rel)
    for rel in TOP_LEVEL_FILES:
        copy_file(rel, rel)
    for rel in TOP_LEVEL_DIRS:
        copy_dir(rel)

    skill_count = len([path for path in (PACKAGE_ROOT / "skills").iterdir() if path.is_dir()])
    return {
        "status": "PASS",
        "mutation_performed": True,
        "package_path": str(PACKAGE_ROOT),
        "skill_count": skill_count,
        "marketplace_files": [
            ".agents/plugins/marketplace.json",
            ".claude-plugin/marketplace.json",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Stage the GroundLine provider package.")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    result = sync_package()
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"staged {result['package_path']} with {result['skill_count']} skills")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
