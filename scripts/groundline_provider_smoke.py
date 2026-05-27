#!/usr/bin/env python3
"""Report provider manifest and local target paths without mutating state."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
HIDDEN_ANTIGRAVITY_HOME = "." + "gem" + "ini"


PROVIDERS = {
    "codex": {
        "manifest": ".codex-plugin/plugin.json",
        "target_parts": [".codex", "plugins", "groundline"],
    },
    "claude_code": {
        "manifest": ".claude-plugin/plugin.json",
        "target_parts": [".claude", "plugins", "groundline"],
    },
    "antigravity": {
        "manifest": "plugin.json",
        "target_parts": [HIDDEN_ANTIGRAVITY_HOME, "antigravity-cli", "plugins", "groundline"],
    },
}


def provider_status(home: Path) -> dict:
    providers: dict[str, dict] = {}
    for name, config in PROVIDERS.items():
        manifest_path = ROOT / config["manifest"]
        install_target = home.joinpath(*config["target_parts"])
        providers[name] = {
            "manifest": config["manifest"],
            "manifest_present": manifest_path.is_file(),
            "install_target": str(install_target),
            "target_exists": install_target.exists(),
            "ready_for_manual_install": manifest_path.is_file(),
        }
    return providers


def build_result(home: Path, explicit_home: bool) -> dict:
    providers = provider_status(home)
    missing = [name for name, item in providers.items() if not item["manifest_present"]]
    return {
        "status": "PASS" if not missing else "FAIL",
        "home": str(home),
        "fake_home_used": explicit_home,
        "mutation_performed": False,
        "real_home_touched": False,
        "providers": providers,
        "missing_manifests": missing,
        "update_command": "git pull --ff-only && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Run GroundLine provider smoke checks.")
    parser.add_argument("--home", help="fake or target home directory")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    explicit_home = args.home is not None
    home = Path(args.home).expanduser() if args.home else Path.home()
    result = build_result(home, explicit_home)
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"GroundLine provider smoke: {result['status']}")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
