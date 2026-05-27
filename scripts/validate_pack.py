#!/usr/bin/env python3
"""Validate the GroundLine repository layout."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

EXPECTED_SKILLS = {
    "reconcile-current-state",
    "audit-agent-history",
    "close-live-work",
    "guard-side-effects",
    "align-agent-home",
    "recover-worktree-branch",
}

REQUIRED_FILES = [
    ".codex-plugin/plugin.json",
    ".claude-plugin/plugin.json",
    ".github/workflows/radar.yml",
    ".github/workflows/test.yml",
    "plugin.json",
    "CHANGELOG.md",
    "README.md",
    "docs/install.md",
    "docs/update.md",
    "docs/provider-smoke.md",
    "docs/runtime-support.md",
    "docs/examples.md",
    "docs/release-checklist.md",
    "references/capability-blueprint.md",
    "references/config-sync-boundary.md",
    "references/output-contracts.md",
    "references/platform-support.md",
    "references/runtime-matrix.md",
    "references/source-registry.json",
    "references/superpowers-interop.md",
    "references/tool-profiles.md",
    "references/workflow-modes.md",
    "scripts/groundline_provider_smoke.py",
    "LICENSE",
]

FORBIDDEN_PATTERNS = [
    r"\b" + "State" + "First" + r"\b",
    r"\b" + "state-first" + "-pack" + r"\b",
    r"\b" + "Gem" + "ini" + r"\b",
    r"\b" + "lega" + "cy" + r"\b",
    r"\b" + "compat" + "ibility" + r"\b",
    r"\b" + "Win" + "dows" + r"\b",
    r"\b" + "W" + "SL" + r"\b",
    r"\b" + "Ru" + "st" + r"\b",
]


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def parse_frontmatter(path: Path) -> dict[str, str]:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        raise ValueError(f"{path.relative_to(ROOT)} missing opening frontmatter delimiter")
    end = text.find("\n---\n", 4)
    if end == -1:
        raise ValueError(f"{path.relative_to(ROOT)} missing closing frontmatter delimiter")
    data: dict[str, str] = {}
    for line in text[4:end].splitlines():
        if not line.strip():
            continue
        if ":" not in line:
            raise ValueError(f"{path.relative_to(ROOT)} invalid frontmatter line: {line}")
        key, value = line.split(":", 1)
        data[key.strip()] = value.strip()
    return data


def collect_errors() -> list[str]:
    errors: list[str] = []

    for rel in REQUIRED_FILES:
        if not (ROOT / rel).is_file():
            errors.append(f"missing required file: {rel}")

    for label, rel in {
        "codex": ".codex-plugin/plugin.json",
        "claude_code": ".claude-plugin/plugin.json",
        "antigravity": "plugin.json",
    }.items():
        path = ROOT / rel
        if path.is_file():
            try:
                manifest = load_json(path)
            except Exception as exc:
                errors.append(f"{rel} invalid JSON: {exc}")
                continue
            if manifest.get("name") != "groundline":
                errors.append(f"{label} manifest name must be groundline")

    codex_manifest = ROOT / ".codex-plugin/plugin.json"
    if codex_manifest.is_file():
        codex = load_json(codex_manifest)
        if codex.get("skills") != "./skills/":
            errors.append("Codex manifest must point skills to ./skills/")
        interface = codex.get("interface", {})
        if interface.get("displayName") != "GroundLine":
            errors.append("Codex displayName must be GroundLine")

    skills_dir = ROOT / "skills"
    actual_skills = {path.name for path in skills_dir.iterdir() if path.is_dir()} if skills_dir.is_dir() else set()
    if actual_skills != EXPECTED_SKILLS:
        errors.append(f"skill surface mismatch: {sorted(actual_skills)}")

    for skill_name in sorted(EXPECTED_SKILLS):
        skill_dir = skills_dir / skill_name
        skill_file = skill_dir / "SKILL.md"
        if not skill_file.is_file():
            errors.append(f"{skill_name} missing SKILL.md")
            continue
        try:
            data = parse_frontmatter(skill_file)
        except ValueError as exc:
            errors.append(str(exc))
            continue
        if data.get("name") != skill_name:
            errors.append(f"{skill_name} frontmatter name mismatch")
        if not data.get("description", "").startswith("Use when"):
            errors.append(f"{skill_name} description must start with Use when")
        if not (skill_dir / "agents/openai.yaml").is_file():
            errors.append(f"{skill_name} missing agents/openai.yaml")

    for path in sorted(ROOT.rglob("*")):
        if not path.is_file() or "tests" in path.parts or ".git" in path.parts:
            continue
        if path.suffix.lower() not in {".md", ".json", ".yaml", ".yml", ".py"}:
            continue
        text = path.read_text(encoding="utf-8")
        for pattern in FORBIDDEN_PATTERNS:
            if re.search(pattern, text, flags=re.IGNORECASE):
                errors.append(f"{path.relative_to(ROOT)} contains forbidden text: {pattern}")

    return errors


def build_result(errors: list[str]) -> dict:
    return {
        "name": "groundline",
        "status": "PASS" if not errors else "FAIL",
        "errors": errors,
        "mutation_performed": False,
        "supported_runtimes": ["codex", "claude_code", "antigravity"],
        "supported_platforms": ["macos-arm64", "linux"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate the GroundLine repository layout.")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    result = build_result(collect_errors())
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    elif result["status"] == "PASS":
        print("GroundLine validation passed")
    else:
        for error in result["errors"]:
            print(f"FAIL: {error}", file=sys.stderr)
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
