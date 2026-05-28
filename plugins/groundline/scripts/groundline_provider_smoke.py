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
        "cache_root_parts": [".codex", "plugins", "cache", "groundline", "groundline"],
        "version_required": True,
    },
    "claude_code": {
        "manifest": ".claude-plugin/plugin.json",
        "target_parts": [".claude", "plugins", "groundline"],
        "cache_root_parts": [".claude", "plugins", "cache", "groundline", "groundline"],
        "version_required": True,
    },
    "antigravity": {
        "manifest": "plugin.json",
        "target_parts": [HIDDEN_ANTIGRAVITY_HOME, "config", "plugins", "groundline"],
        "cache_root_parts": [],
        "version_required": False,
    },
}


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}


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
    manifest = load_json(root / "plugin.json")
    return {
        "version": manifest.get("version"),
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


def version_sort_key(value: str) -> tuple[int, ...]:
    parts: list[int] = []
    for part in value.split("."):
        parts.append(int(part) if part.isdigit() else -1)
    return tuple(parts)


def installed_version_at(path: Path, manifest: str) -> str | None:
    version = load_json(path / manifest).get("version")
    return version if isinstance(version, str) else None


def select_install_target(home: Path, config: dict, source_version: str | None) -> tuple[Path, str, list[str]]:
    direct_target = home.joinpath(*config["target_parts"])
    candidates: list[tuple[Path, str]] = []
    if direct_target.exists():
        candidates.append((direct_target, "direct"))

    cache_root_parts = config.get("cache_root_parts", [])
    if cache_root_parts:
        cache_root = home.joinpath(*cache_root_parts)
        if cache_root.is_dir():
            for child in sorted(cache_root.iterdir(), key=lambda path: version_sort_key(path.name)):
                if child.is_dir():
                    candidates.append((child, "cache"))

    candidate_versions = []
    for path, source in candidates:
        installed_version = installed_version_at(path, config["manifest"])
        if installed_version:
            candidate_versions.append(installed_version)
        elif source == "cache":
            candidate_versions.append(path.name)
    if source_version:
        for path, source in candidates:
            if path.name == source_version or installed_version_at(path, config["manifest"]) == source_version:
                return path, source, candidate_versions
    if candidates:
        return candidates[-1][0], candidates[-1][1], candidate_versions
    return direct_target, "direct", candidate_versions


def runtime_probe(
    install_target: Path,
    manifest: str,
    source_skill_count: int,
    source_version: str | None,
    version_required: bool,
    install_source: str,
) -> dict:
    target_skill_count = count_skill_dirs(install_target)
    manifest_path = install_target / manifest
    manifest_present = manifest_path.is_file()
    target_exists = install_target.exists()
    target_skills_present = (install_target / "skills").is_dir()
    installed_version = load_json(manifest_path).get("version") if manifest_present else None
    skill_count_matches_source = target_skill_count == source_skill_count if target_exists else False
    version_matches_source = installed_version == source_version if manifest_present else False
    version_check = "not_installed"
    if manifest_present:
        if installed_version is None and not version_required:
            version_check = "unavailable"
        elif installed_version == source_version:
            version_check = "match"
        else:
            version_check = "mismatch"
    issues: list[str] = []
    if target_exists and not manifest_present:
        issues.append("missing_manifest_payload")
    if target_exists and not target_skills_present:
        issues.append("missing_skills_payload")
    if target_exists and target_skills_present and not skill_count_matches_source:
        issues.append("skill_count_mismatch")
    if manifest_present and version_check == "mismatch":
        issues.append("version_mismatch")
        if install_source == "cache":
            issues.append("stale_cache_version")
    status = "NOT_INSTALLED"
    if target_exists:
        status = "PASS" if not issues else "PARTIAL"
    return {
        "status": status,
        "target_exists": target_exists,
        "target_manifest_present": manifest_present,
        "target_skills_present": target_skills_present,
        "target_skill_count": target_skill_count,
        "target_skill_count_matches_source": skill_count_matches_source,
        "source_version": source_version,
        "installed_version": installed_version,
        "version_matches_source": version_matches_source,
        "version_check": version_check,
        "issues": issues,
        "read_only": True,
    }


def provider_status(home: Path, explicit_home: bool, source_skill_count: int, source_version: str | None) -> dict:
    providers: dict[str, dict] = {}
    for name, config in PROVIDERS.items():
        manifest_path = ROOT / config["manifest"]
        install_target, install_source, candidate_versions = select_install_target(home, config, source_version)
        probe = runtime_probe(
            install_target,
            config["manifest"],
            source_skill_count,
            source_version,
            config["version_required"],
            install_source,
        )
        providers[name] = {
            "manifest": config["manifest"],
            "manifest_present": manifest_path.is_file(),
            "install_target": display_path(install_target, home, explicit_home),
            "install_source": install_source,
            "candidate_versions": candidate_versions,
            "target_exists": install_target.exists(),
            "ready_for_manual_install": manifest_path.is_file(),
            "install_state": probe["status"],
            "runtime_probe": probe,
        }
    return providers


def build_result(home: Path, explicit_home: bool) -> dict:
    source_package = source_package_status(ROOT)
    providers = provider_status(home, explicit_home, source_package["skill_count"], source_package["version"])
    missing = [name for name, item in providers.items() if not item["manifest_present"]]
    drift = [
        {"provider": name, "issues": item["runtime_probe"]["issues"]}
        for name, item in providers.items()
        if item["runtime_probe"]["status"] == "PARTIAL"
    ]
    install_doctor_status = "FAIL" if missing else "PARTIAL" if drift else "PASS"
    return {
        "status": install_doctor_status,
        "home": display_path(home, home, explicit_home),
        "fake_home_used": explicit_home,
        "mutation_performed": False,
        "secret_value_printed": False,
        "real_home_touched": False,
        "install_doctor_status": install_doctor_status,
        "source_package": source_package,
        "providers": providers,
        "missing_manifests": missing,
        "install_issues": drift,
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
    if result["status"] == "PASS":
        return 0
    if result["status"] == "PARTIAL":
        return 2
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
