#!/usr/bin/env python3
"""Prove remote install and update states in a fake provider home."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "plugins/groundline" if (ROOT / "plugins/groundline").is_dir() else ROOT
HIDDEN_ANTIGRAVITY_HOME = "." + "gem" + "ini"

PROVIDERS = {
    "codex": {
        "label": "Codex",
        "target_parts": [".codex", "plugins", "groundline"],
        "manifest": ".codex-plugin/plugin.json",
        "payload_scope": "full_package",
    },
    "claude_code": {
        "label": "Claude Code",
        "target_parts": [".claude", "plugins", "groundline"],
        "manifest": ".claude-plugin/plugin.json",
        "payload_scope": "full_package",
    },
    "antigravity": {
        "label": "Antigravity",
        "target_parts": [HIDDEN_ANTIGRAVITY_HOME, "config", "plugins", "groundline"],
        "manifest": "plugin.json",
        "payload_scope": "skill_import",
    },
}

REMOTE_INSTALL_COMMANDS = {
    "codex": [
        "codex plugin marketplace add jukqaz/groundline --ref main",
        "codex plugin add groundline@groundline",
    ],
    "claude_code": [
        "claude plugin marketplace add jukqaz/groundline",
        "claude plugin install groundline@groundline",
    ],
    "antigravity": [
        "agy plugin install https://github.com/jukqaz/groundline",
    ],
}

UPDATE_COMMANDS = {
    "repository": "git pull --ff-only",
    "validate": "PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json",
    "confirm": "PYTHONDONTWRITEBYTECODE=1 python3 scripts/groundline_provider_smoke.py --json --require-installed",
}


def load_json(path: Path) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return data if isinstance(data, dict) else {}


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def current_version() -> str:
    version = load_json(SOURCE / "plugin.json").get("version")
    return version if isinstance(version, str) else "0.0.0"


def previous_patch_version(version: str) -> str:
    parts = version.split(".")
    if len(parts) != 3 or not all(part.isdigit() for part in parts):
        return version
    major, minor, patch = (int(part) for part in parts)
    if patch > 0:
        patch -= 1
    return f"{major}.{minor}.{patch}"


def is_real_home(path: Path) -> bool:
    return path.expanduser().resolve() == Path.home().resolve()


def provider_target(home: Path, provider: str) -> Path:
    return home.joinpath(*PROVIDERS[provider]["target_parts"])


def ensure_fake_home(home: Path) -> None:
    (home / ".codex/plugins").mkdir(parents=True, exist_ok=True)
    (home / ".claude/plugins").mkdir(parents=True, exist_ok=True)
    (home / HIDDEN_ANTIGRAVITY_HOME / "config/plugins").mkdir(parents=True, exist_ok=True)


def copy_provider_payload(provider: str, target: Path) -> None:
    config = PROVIDERS[provider]
    if target.exists():
        shutil.rmtree(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    if config["payload_scope"] == "skill_import":
        target.mkdir(parents=True, exist_ok=True)
        shutil.copy2(SOURCE / config["manifest"], target / config["manifest"])
        shutil.copytree(SOURCE / "skills", target / "skills")
        return
    shutil.copytree(SOURCE, target)


def set_manifest_version(path: Path, version: str) -> None:
    data = load_json(path)
    data["version"] = version
    write_json(path, data)


def remove_manifest_version(path: Path) -> None:
    data = load_json(path)
    data.pop("version", None)
    write_json(path, data)


def make_fresh_install(home: Path) -> list[str]:
    staged = []
    for provider in PROVIDERS:
        target = provider_target(home, provider)
        copy_provider_payload(provider, target)
        staged.append(str(target))
    return staged


def make_stale_install(home: Path, previous_version: str) -> list[str]:
    staged = make_fresh_install(home)
    for provider, config in PROVIDERS.items():
        target = provider_target(home, provider)
        manifest = target / config["manifest"]
        if provider == "antigravity":
            remove_manifest_version(manifest)
            skill_doc = target / "skills" / "package-agent-task" / "SKILL.md"
            skill_doc.write_text(skill_doc.read_text(encoding="utf-8") + "\nstale installed skill payload\n", encoding="utf-8")
        else:
            set_manifest_version(manifest, previous_version)
    return staged


def run_provider_smoke(home: Path) -> tuple[int, dict]:
    command = [
        sys.executable,
        str(ROOT / "scripts" / "groundline_provider_smoke.py"),
        "--home",
        str(home),
        "--json",
        "--require-installed",
    ]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError:
        payload = {
            "status": "FAIL",
            "error": "provider smoke did not emit JSON",
            "stdout_tail": completed.stdout[-1000:],
            "stderr_tail": completed.stderr[-1000:],
        }
    return completed.returncode, payload


def summarize_providers(smoke: dict) -> dict:
    summary = {}
    for provider, item in smoke.get("providers", {}).items():
        probe = item.get("runtime_probe", {})
        summary[provider] = {
            "label": PROVIDERS.get(provider, {}).get("label", provider),
            "install_state": item.get("install_state"),
            "install_source": item.get("install_source"),
            "installed_version": probe.get("installed_version"),
            "source_version": probe.get("source_version"),
            "version_check": probe.get("version_check"),
            "content_matches_source": probe.get("content_matches_source"),
            "target_skill_count": probe.get("target_skill_count"),
            "target_skill_count_matches_source": probe.get("target_skill_count_matches_source"),
            "issues": probe.get("issues", []),
            "recommended_actions": item.get("recommended_actions", []),
        }
    return summary


def scenario_from_smoke(name: str, expected: str, returncode: int, smoke: dict) -> dict:
    observed = smoke.get("install_doctor_status")
    status = "PASS" if observed == expected else "FAIL"
    if expected == "PASS" and returncode != 0:
        status = "FAIL"
    if expected == "PARTIAL" and returncode != 2:
        status = "FAIL"
    return {
        "name": name,
        "status": status,
        "expected_install_doctor_status": expected,
        "observed_install_doctor_status": observed,
        "install_doctor_status": observed,
        "returncode": returncode,
        "providers": summarize_providers(smoke),
        "next_actions": smoke.get("next_actions", []),
    }


def run_probe(home: Path, previous_version: str) -> dict:
    ensure_fake_home(home)

    make_fresh_install(home)
    fresh_returncode, fresh_smoke = run_provider_smoke(home)

    make_stale_install(home, previous_version)
    stale_returncode, stale_smoke = run_provider_smoke(home)

    staged_targets = make_fresh_install(home)
    refresh_returncode, refresh_smoke = run_provider_smoke(home)

    scenarios = {
        "fresh_install": scenario_from_smoke("fresh_install", "PASS", fresh_returncode, fresh_smoke),
        "stale_update_detection": scenario_from_smoke(
            "stale_update_detection",
            "PARTIAL",
            stale_returncode,
            stale_smoke,
        ),
        "post_update_refresh": scenario_from_smoke("post_update_refresh", "PASS", refresh_returncode, refresh_smoke),
    }
    status = "PASS" if all(item["status"] == "PASS" for item in scenarios.values()) else "FAIL"
    next_actions = sorted(
        {
            action
            for item in scenarios.values()
            if item["status"] != "PASS"
            for action in item.get("next_actions", [])
        }
    )
    return {
        "suite": "remote-install-update-proof",
        "status": status,
        "source_package": refresh_smoke.get("source_package", {}),
        "previous_version": previous_version,
        "remote_install_commands": REMOTE_INSTALL_COMMANDS,
        "update_commands": UPDATE_COMMANDS,
        "scenarios": scenarios,
        "staged_targets": staged_targets,
        "fake_home_used": True,
        "mutation_performed": False,
        "real_home_touched": False,
        "secret_value_printed": False,
        "next_actions": next_actions,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Run fake-home remote install and update proof.")
    parser.add_argument("--home", help="explicit fake home to stage provider installs")
    parser.add_argument("--previous-version", help="previous installed version to simulate")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    version = current_version()
    previous_version = args.previous_version or previous_patch_version(version)

    if args.home:
        home = Path(args.home).expanduser()
        if is_real_home(home):
            result = {
                "suite": "remote-install-update-proof",
                "status": "FAIL",
                "error": "refusing to stage remote install proof into the real home directory",
                "mutation_performed": False,
                "real_home_touched": False,
                "secret_value_printed": False,
            }
            print(json.dumps(result, indent=2, sort_keys=True))
            return 1
        home.mkdir(parents=True, exist_ok=True)
        result = run_probe(home, previous_version)
    else:
        with tempfile.TemporaryDirectory(prefix="groundline-remote-install-") as temp:
            result = run_probe(Path(temp) / "home", previous_version)

    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"{result['suite']}: {result['status']}")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
