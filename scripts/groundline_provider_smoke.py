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


def count_skill_dirs(root: Path) -> int:
    skills_dir = root / "skills"
    if not skills_dir.is_dir():
        return 0
    return sum(1 for path in skills_dir.iterdir() if path.is_dir() and (path / "SKILL.md").is_file())


def load_skill_index_names(root: Path) -> set[str]:
    index_path = root / "references/skill-index.json"
    if not index_path.is_file():
        return set()
    try:
        data = json.loads(index_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return set()
    skills = data.get("skills", [])
    if not isinstance(skills, list):
        return set()
    return {item.get("name") for item in skills if isinstance(item, dict) and isinstance(item.get("name"), str)}


def source_package_status(root: Path) -> dict:
    skill_names = {path.name for path in (root / "skills").iterdir() if path.is_dir()} if (root / "skills").is_dir() else set()
    index_names = load_skill_index_names(root)
    return {
        "skill_count": count_skill_dirs(root),
        "skill_index_present": (root / "references/skill-index.json").is_file(),
        "skill_index_skill_count": len(index_names),
        "skill_index_consistent": bool(skill_names) and skill_names == index_names,
        "human_portfolio_present": (root / "docs/skill-portfolio.md").is_file(),
        "runtime_docs_present": (root / "docs/runtime-support.md").is_file(),
    }


def display_path(path: Path, home: Path, explicit_home: bool) -> str:
    if explicit_home:
        return str(path)
    try:
        relative = path.relative_to(home)
    except ValueError:
        return str(path)
    if str(relative) == ".":
        return "~"
    return str(Path("~") / relative)


def runtime_probe(install_target: Path, manifest: str, source_skill_count: int) -> dict:
    target_skill_count = count_skill_dirs(install_target)
    return {
        "target_exists": install_target.exists(),
        "target_manifest_present": (install_target / manifest).is_file(),
        "target_skills_present": (install_target / "skills").is_dir(),
        "target_skill_count": target_skill_count,
        "target_skill_count_matches_source": target_skill_count == source_skill_count if install_target.exists() else False,
        "read_only": True,
    }


def provider_status(home: Path, explicit_home: bool, source_skill_count: int) -> dict:
    providers: dict[str, dict] = {}
    for name, config in PROVIDERS.items():
        manifest_path = ROOT / config["manifest"]
        install_target = home.joinpath(*config["target_parts"])
        providers[name] = {
            "manifest": config["manifest"],
            "manifest_present": manifest_path.is_file(),
            "install_target": display_path(install_target, home, explicit_home),
            "target_exists": install_target.exists(),
            "ready_for_manual_install": manifest_path.is_file(),
            "runtime_probe": runtime_probe(install_target, config["manifest"], source_skill_count),
        }
    return providers


def build_result(home: Path, explicit_home: bool) -> dict:
    source_package = source_package_status(ROOT)
    providers = provider_status(home, explicit_home, source_package["skill_count"])
    missing = [name for name, item in providers.items() if not item["manifest_present"]]
    return {
        "status": "PASS" if not missing else "FAIL",
        "home": display_path(home, home, explicit_home),
        "fake_home_used": explicit_home,
        "mutation_performed": False,
        "real_home_touched": False,
        "source_package": source_package,
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
