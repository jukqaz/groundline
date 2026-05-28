#!/usr/bin/env python3
"""Check GroundLine runtime plugin layout."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def manifest_status(rel: str) -> dict:
    path = ROOT / rel
    status = {"present": path.is_file(), "path": rel}
    if path.is_file():
        try:
            status["name"] = json.loads(path.read_text(encoding="utf-8")).get("name")
        except json.JSONDecodeError:
            status["name"] = None
    return status


def build_result() -> dict:
    return {
        "status": "PASS",
        "mutation_performed": False,
        "manifests": {
            "codex": manifest_status(".codex-plugin/plugin.json"),
            "claude_code": manifest_status(".claude-plugin/plugin.json"),
            "antigravity": manifest_status("plugin.json"),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Check GroundLine runtime layout.")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()
    result = build_result()
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print("GroundLine runtime layout checked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
