#!/usr/bin/env python3
"""Run a read-only GroundLine doctor check."""

from __future__ import annotations

import argparse
import json
import platform
import re
import shutil
import subprocess
from pathlib import Path


SECRET_PATTERN = re.compile(
    r"((?<![A-Za-z0-9])sk-[A-Za-z0-9_-]+|(?<![A-Za-z0-9])xox[baprs]-[A-Za-z0-9_-]+|(?i:api[_-]?key|token|secret|password))"
)
EXTERNAL_TOOL_PROBES = {
    "git": ("git", ["--version"]),
    "github_cli": ("gh", ["--version"]),
    "docker": ("docker", ["--version"]),
    "curl": ("curl", ["--version"]),
}


def load_tools_fixture(path: str | None) -> dict:
    if not path:
        return {}
    return json.loads(Path(path).read_text(encoding="utf-8"))


def resolve_binary(binary: str, bin_dir: Path | None) -> str | None:
    if bin_dir:
        candidate = bin_dir / binary
        if candidate.is_file():
            return str(candidate)
    return shutil.which(binary)


def first_line(text: str) -> str:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped:
            return stripped
    return ""


def sanitize_output(text: str) -> tuple[str, bool]:
    value = first_line(text)
    if not value:
        return "", False
    if SECRET_PATTERN.search(value):
        return "[redacted]", True
    return value[:240], False


def probe_external_tool(key: str, binary: str, args: list[str], bin_dir: Path | None, probe_tools: bool) -> dict:
    resolved_path = resolve_binary(binary, bin_dir)
    status = {
        "available": resolved_path is not None,
        "requirement": "optional",
        "command": [binary, *args],
        "resolved_path": resolved_path,
        "probed": probe_tools,
        "version": None,
        "redacted": False,
    }
    if not probe_tools or not resolved_path:
        return status

    try:
        completed = subprocess.run(
            [resolved_path, *args],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        status["error"] = type(exc).__name__
        return status

    version, redacted = sanitize_output(completed.stdout or completed.stderr)
    status["exit_code"] = completed.returncode
    status["version"] = version or None
    status["redacted"] = redacted
    if completed.returncode != 0 and not status["version"]:
        status["error"] = "command failed"
    return status


def external_tool_status(bin_dir: Path | None, probe_tools: bool) -> dict:
    return {
        key: probe_external_tool(key, binary, args, bin_dir, probe_tools)
        for key, (binary, args) in EXTERNAL_TOOL_PROBES.items()
    }


def tool_status(name: str, fixture: dict, bin_dir: Path | None) -> dict:
    if name in fixture:
        available = bool(fixture[name].get("available"))
    elif name == "github":
        available = resolve_binary("gh", bin_dir) is not None
    else:
        available = False
    return {"available": available, "requirement": "optional"}


def tool_recommendations(tools: dict) -> tuple[list[str], list[dict]]:
    gaps: list[str] = []
    recommendations: list[dict] = []
    for name in ["github", "context7", "exa"]:
        if tools[name]["available"]:
            continue
        gaps.append(name)
        recommendations.append(
            {
                "tool": name,
                "reason": "standard profile tool is not available",
                "action": "configure the tool if this workflow needs repository evidence or current external research",
            }
        )
    return gaps, recommendations


def display_home_path(home: Path, explicit_home: bool) -> str:
    if explicit_home:
        return str(home)
    return "~"


def build_result(home: Path, explicit_home: bool, tools_fixture: dict, bin_dir: Path | None, probe_tools: bool) -> dict:
    superpowers_present = (home / ".codex/plugins/cache/openai-curated/superpowers").exists()
    antigravity_home = home / ("." + "gem" + "ini")
    mode = "companion-superpowers" if superpowers_present else "standalone-groundline"
    tools = {
        "github": tool_status("github", tools_fixture, bin_dir),
        "context7": tool_status("context7", tools_fixture, bin_dir),
        "exa": tool_status("exa", tools_fixture, bin_dir),
    }
    gaps, recommendations = tool_recommendations(tools)
    return {
        "status": "PASS",
        "home": display_home_path(home, explicit_home),
        "platform": {"system": platform.system(), "machine": platform.machine()},
        "recommended_mode": mode,
        "mutation_performed": False,
        "network": "disabled",
        "fake_home_used": explicit_home,
        "real_home_touched": False,
        "runtimes": {
            "codex": {"scope": "supported", "present": (home / ".codex").exists()},
            "claude_code": {"scope": "supported", "present": (home / ".claude").exists()},
            "antigravity": {"scope": "supported", "present": antigravity_home.exists()},
        },
        "superpowers": {"present": superpowers_present, "requirement": "recommended"},
        "tools": tools,
        "external_tools": external_tool_status(bin_dir, probe_tools),
        "capability_gaps": gaps,
        "setup_recommendations": recommendations,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Run GroundLine doctor.")
    parser.add_argument("--home", help="fake or target home directory")
    parser.add_argument("--tools-fixture", help="tool availability fixture JSON")
    parser.add_argument("--bin-dir", help="prefer executables from this directory for tool probes")
    parser.add_argument("--probe-tools", action="store_true", help="run read-only external tool version probes")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    parser.add_argument("--offline", action="store_true", help="disable network checks")
    args = parser.parse_args()

    explicit_home = args.home is not None
    home = Path(args.home).expanduser() if args.home else Path.home()
    bin_dir = Path(args.bin_dir).expanduser() if args.bin_dir else None
    result = build_result(home, explicit_home, load_tools_fixture(args.tools_fixture), bin_dir, args.probe_tools)
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"GroundLine recommended mode: {result['recommended_mode']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
