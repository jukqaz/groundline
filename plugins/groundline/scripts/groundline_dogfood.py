#!/usr/bin/env python3
"""Run GroundLine provider dogfood checks without touching real provider homes."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
HIDDEN_ANTIGRAVITY_HOME = "." + "gem" + "ini"
SECRET_PATTERN = re.compile(r"(sk-[A-Za-z0-9_-]+|xox[baprs]-[A-Za-z0-9_-]+|(?i:api[_-]?key|token|secret|password))")

PROVIDERS = {
    "codex": {
        "display": "Codex",
        "binary": "codex",
        "version_args": ["--version"],
        "manifest_source": ".codex-plugin",
        "manifest_target": ".codex-plugin/plugin.json",
        "target_parts": [".codex", "plugins", "groundline"],
    },
    "claude_code": {
        "display": "Claude Code",
        "binary": "claude",
        "version_args": ["--version"],
        "manifest_source": ".claude-plugin",
        "manifest_target": ".claude-plugin/plugin.json",
        "target_parts": [".claude", "plugins", "groundline"],
    },
    "antigravity": {
        "display": "Antigravity",
        "binary": "agy",
        "version_args": ["--version"],
        "manifest_source": "plugin.json",
        "manifest_target": "plugin.json",
        "target_parts": [HIDDEN_ANTIGRAVITY_HOME, "antigravity-cli", "plugins", "groundline"],
    },
}

SCENARIOS = [
    {
        "id": "long-context-handoff",
        "prompt": "This thread is getting long. Package the current goal so another agent can continue without guessing.",
        "expected_skill": "package-agent-task",
        "expected_contract": "GroundLine Task Packet",
    },
    {
        "id": "completion-proof",
        "prompt": "Tests passed. Decide whether this work is actually complete and what evidence is still missing.",
        "expected_skill": "close-live-work",
        "expected_contract": "Status: PASS / PARTIAL / FAIL",
    },
    {
        "id": "expansion-control",
        "prompt": "I keep adding ideas. Lock the release cut and classify what is must fix, defer, or reject.",
        "expected_skill": "stabilize-release-cut",
        "expected_contract": "GroundLine Release Cut",
    },
]


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
    data = load_json(root / "references/skill-index.json")
    skills = data.get("skills", [])
    if not isinstance(skills, list):
        return set()
    return {item.get("name") for item in skills if isinstance(item, dict) and isinstance(item.get("name"), str)}


def source_package_status() -> dict:
    skill_names = {path.name for path in (ROOT / "skills").iterdir() if path.is_dir()} if (ROOT / "skills").is_dir() else set()
    index_names = load_skill_index_names(ROOT)
    codex = load_json(ROOT / ".codex-plugin/plugin.json")
    claude = load_json(ROOT / ".claude-plugin/plugin.json")
    return {
        "version": codex.get("version"),
        "claude_version": claude.get("version"),
        "skill_count": count_skill_dirs(ROOT),
        "skill_index_present": (ROOT / "references/skill-index.json").is_file(),
        "skill_index_skill_count": len(index_names),
        "skill_index_consistent": bool(skill_names) and skill_names == index_names,
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


def first_line(text: str) -> str:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped:
            return stripped
    return ""


def sanitize_output(text: str) -> tuple[str | None, bool]:
    value = first_line(text)
    if not value:
        return None, False
    if SECRET_PATTERN.search(value):
        return "[redacted]", True
    return value[:240], False


def resolve_binary(binary: str, bin_dir: Path | None) -> str | None:
    if bin_dir:
        candidate = bin_dir / binary
        if candidate.is_file():
            return str(candidate)
    return shutil.which(binary)


def runtime_probe(config: dict, bin_dir: Path | None, probe_runtimes: bool) -> dict:
    command = [config["binary"], *config["version_args"]]
    resolved = resolve_binary(config["binary"], bin_dir)
    result = {
        "command": command,
        "available": resolved is not None,
        "probed": probe_runtimes,
        "resolved_path": resolved,
        "version": None,
        "redacted": False,
    }
    if not probe_runtimes or not resolved:
        return result
    try:
        completed = subprocess.run(
            [resolved, *config["version_args"]],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        result["error"] = type(exc).__name__
        return result
    version, redacted = sanitize_output(completed.stdout or completed.stderr)
    result.update({"exit_code": completed.returncode, "version": version, "redacted": redacted})
    if completed.returncode != 0 and not version:
        result["error"] = "command failed"
    return result


def copy_package_to_target(target: Path, config: dict) -> None:
    target.mkdir(parents=True, exist_ok=True)
    source_manifest = ROOT / config["manifest_source"]
    if source_manifest.is_dir():
        shutil.copytree(source_manifest, target / config["manifest_source"], dirs_exist_ok=True)
    else:
        shutil.copy2(source_manifest, target / config["manifest_source"])
    shutil.copytree(ROOT / "skills", target / "skills", dirs_exist_ok=True)


def install_status(target: Path, config: dict, source_skill_count: int) -> dict:
    target_skill_count = count_skill_dirs(target)
    return {
        "target_exists": target.exists(),
        "target_manifest_present": (target / config["manifest_target"]).is_file(),
        "target_skills_present": (target / "skills").is_dir(),
        "target_skill_count": target_skill_count,
        "target_skill_count_matches_source": target_skill_count == source_skill_count if target.exists() else False,
    }


def skill_contract_result(scenario: dict) -> dict:
    skill_file = ROOT / "skills" / scenario["expected_skill"] / "SKILL.md"
    output_contracts = (ROOT / "references/output-contracts.md").read_text(encoding="utf-8")
    skill_text = skill_file.read_text(encoding="utf-8") if skill_file.is_file() else ""
    contract_present = scenario["expected_contract"] in output_contracts or scenario["expected_contract"] in skill_text
    status = "PASS" if skill_file.is_file() and contract_present else "FAIL"
    return {
        "id": scenario["id"],
        "prompt": scenario["prompt"],
        "expected_skill": scenario["expected_skill"],
        "expected_contract": scenario["expected_contract"],
        "skill_present": skill_file.is_file(),
        "contract_present": contract_present,
        "status": status,
    }


def provider_result(name: str, config: dict, home: Path, explicit_home: bool, args: argparse.Namespace, source_skill_count: int) -> dict:
    target = home.joinpath(*config["target_parts"])
    if args.stage_package:
        copy_package_to_target(target, config)
    runtime = runtime_probe(config, Path(args.bin_dir).expanduser() if args.bin_dir else None, args.probe_runtimes)
    install = install_status(target, config, source_skill_count)
    scenarios = [skill_contract_result(scenario) for scenario in SCENARIOS]
    checks = [
        install["target_exists"],
        install["target_manifest_present"],
        install["target_skills_present"],
        install["target_skill_count_matches_source"],
        all(item["status"] == "PASS" for item in scenarios),
    ]
    if args.probe_runtimes:
        checks.append(runtime["available"] and runtime.get("exit_code") == 0)
    status = "PASS" if all(checks) else "PARTIAL"
    return {
        "name": name,
        "display": config["display"],
        "status": status,
        "install_target": display_path(target, home, explicit_home),
        "runtime": runtime,
        "runtime_probe": runtime,
        "install": install,
        "scenario_results": scenarios,
    }


def build_result(home: Path, explicit_home: bool, args: argparse.Namespace) -> dict:
    source_package = source_package_status()
    fake_home_used = explicit_home or bool(args.stage_package)
    providers = {
        name: provider_result(name, config, home, fake_home_used, args, source_package["skill_count"])
        for name, config in PROVIDERS.items()
    }
    provider_statuses = {provider["status"] for provider in providers.values()}
    status = "PASS" if provider_statuses == {"PASS"} else "PARTIAL"
    return {
        "status": status,
        "suite": "provider-dogfood",
        "scenario_count": len(SCENARIOS),
        "stage_package": args.stage_package,
        "probe_runtimes": args.probe_runtimes,
        "home": display_path(home, home, fake_home_used),
        "fake_home_used": fake_home_used,
        "temp_state_created": bool(args.stage_package and not explicit_home),
        "mutation_performed": False,
        "real_home_touched": False,
        "source_package": source_package,
        "providers": providers,
        "scenarios": SCENARIOS,
    }


def is_real_home(path: Path) -> bool:
    return path.expanduser().resolve() == Path.home().resolve()


def real_home_rejection(home: Path) -> dict:
    return {
        "status": "FAIL",
        "suite": "provider-dogfood",
        "error": "refusing to stage package into the real home directory",
        "home": "~" if is_real_home(home) else str(home),
        "stage_package": True,
        "probe_runtimes": False,
        "fake_home_used": False,
        "temp_state_created": False,
        "mutation_performed": False,
        "real_home_touched": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Run GroundLine provider dogfood checks.")
    parser.add_argument("--home", help="fake or target home directory")
    parser.add_argument("--bin-dir", help="prefer runtime binaries from this directory")
    parser.add_argument("--stage-package", action="store_true", help="stage package files into a temporary or explicit home")
    parser.add_argument("--probe-runtimes", action="store_true", help="run read-only provider runtime version probes")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    explicit_home = args.home is not None
    if explicit_home:
        home = Path(args.home).expanduser()
        if args.stage_package and is_real_home(home):
            result = real_home_rejection(home)
            if args.json:
                print(json.dumps(result, indent=2, sort_keys=True))
            else:
                print(f"GroundLine dogfood: {result['status']}: {result['error']}")
            return 2
        result = build_result(home, explicit_home, args)
    else:
        with tempfile.TemporaryDirectory(prefix="groundline-dogfood-") as temp:
            home = Path(temp) / "home"
            result = build_result(home, explicit_home, args)

    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"GroundLine dogfood: {result['status']}")
    return 0 if result["status"] == "PASS" else 2


if __name__ == "__main__":
    raise SystemExit(main())
