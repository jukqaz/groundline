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
    "agent-ecosystem-radar",
    "research-agent-ecosystem",
    "compare-agent-workflows",
    "recommend-groundline-upgrades",
    "evaluate-groundline-pack",
    "curate-groundline-skills",
    "evaluate-agent-capability",
    "evaluate-ai-usage-maturity",
    "package-agent-task",
    "hold-the-line",
    "polish-release-candidate",
    "stabilize-release-cut",
    "compare-release-delta",
}

BASE_REQUIRED_FILES = [
    ".codex-plugin/plugin.json",
    ".claude-plugin/plugin.json",
    "plugin.json",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "LICENSE",
    "README.md",
    "README.ko.md",
    "SECURITY.md",
    "assets/groundline-icon.svg",
    "assets/groundline-logo.svg",
    "docs/language-policy.md",
    "docs/install.md",
    "docs/git-history-privacy.md",
    "docs/human-guide.md",
    "docs/llm-guide.md",
    "docs/update.md",
    "docs/provider-smoke.md",
    "docs/provider-dogfood.md",
    "docs/provider-activation-matrix.md",
    "docs/provider-guardrails.md",
    "docs/mcp-recipes.md",
    "docs/maturity-assessment.md",
    "docs/next-work.md",
    "docs/next-version.md",
    "docs/privacy.md",
    "docs/terms.md",
    "docs/provider-packaging.md",
    "docs/public-release.md",
    "docs/runtime-support.md",
    "docs/examples.md",
    "docs/workflow-cookbook.md",
    "docs/artifact-lifecycle.md",
    "docs/dogfood.md",
    "docs/skill-portfolio.md",
    "docs/skill-graduation-plan.md",
    "docs/release-checklist.md",
    "docs/ko/index.md",
    "docs/ko/human-guide.md",
    "docs/ko/install.md",
    "docs/ko/update.md",
    "docs/ko/examples.md",
    "docs/ko/workflow-cookbook.md",
    "docs/ko/artifact-lifecycle.md",
    "docs/ko/skill-portfolio.md",
    "docs/ko/skill-graduation-plan.md",
    "docs/ko/privacy.md",
    "docs/ko/terms.md",
    "docs/ko/provider-packaging.md",
    "docs/ko/provider-activation-matrix.md",
    "docs/ko/provider-guardrails.md",
    "docs/ko/mcp-recipes.md",
    "docs/ko/maturity-assessment.md",
    "docs/ko/release-checklist.md",
    "docs/ko/next-version.md",
    "references/capability-blueprint.md",
    "references/config-sync-boundary.md",
    "references/output-contracts.md",
    "references/platform-support.md",
    "references/runtime-matrix.md",
    "references/external-workflow-interop.md",
    "references/capability-evaluation.md",
    "references/ai-usage-maturity.md",
    "references/multi-provider-fluency-boundary.md",
    "references/agent-task-packet.md",
    "references/release-stabilization.md",
    "references/skill-index.json",
    "references/skill-lifecycle.md",
    "references/source-registry.json",
    "references/optional-mcp-profiles.md",
    "references/superpowers-interop.md",
    "references/tool-profiles.md",
    "references/workflow-modes.md",
    "scenarios/fixtures/safety-eval.json",
    "scripts/groundline_dogfood.py",
    "scripts/groundline_provider_smoke.py",
    "scripts/groundline_release_gate.py",
    "scripts/groundline_safety_eval.py",
    "scripts/lint.py",
    "scripts/sync_provider_package.py",
]

SOURCE_ONLY_REQUIRED_FILES = [
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
    ".github/ISSUE_TEMPLATE/bug_report.md",
    ".github/ISSUE_TEMPLATE/feature_request.md",
    ".github/pull_request_template.md",
    ".github/workflows/radar.yml",
    ".github/workflows/test.yml",
    "plugins/groundline/.codex-plugin/plugin.json",
    "plugins/groundline/.claude-plugin/plugin.json",
    "plugins/groundline/plugin.json",
    "plugins/groundline/assets/groundline-icon.svg",
    "plugins/groundline/assets/groundline-logo.svg",
    "plugins/groundline/docs/provider-packaging.md",
    "plugins/groundline/docs/terms.md",
    "plugins/groundline/docs/ko/provider-packaging.md",
    "plugins/groundline/docs/ko/terms.md",
]

SOURCE_ONLY_PACKAGE_EXCLUSIONS = [
    "docs/superpowers",
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


def canonical_version() -> str | None:
    manifest = load_json(ROOT / "plugin.json")
    version = manifest.get("version")
    return version if isinstance(version, str) else None


def collect_conflict_copies(root: Path) -> list[Path]:
    if not root.is_dir():
        return []
    return sorted(path for path in root.rglob("*") if re.search(r" \d+$", path.name))


def conflict_copy_has_payload(path: Path) -> bool:
    if path.is_file():
        return True
    if not path.is_dir():
        return False
    return any(child.is_file() for child in path.rglob("*"))


def reject_source_only_package_paths(root: Path, prefix: str = "") -> list[str]:
    errors: list[str] = []
    for rel in SOURCE_ONLY_PACKAGE_EXCLUSIONS:
        path = root / rel
        if path.exists():
            display = f"{prefix}{rel}"
            errors.append(f"source-only package path must be excluded: {display}")
    return errors


def collect_errors() -> list[str]:
    errors: list[str] = []
    source_root = (ROOT / ".agents/plugins/marketplace.json").is_file()
    required_files = BASE_REQUIRED_FILES + (SOURCE_ONLY_REQUIRED_FILES if source_root else [])
    expected_version = canonical_version()

    for rel in required_files:
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
            if expected_version and manifest.get("version") != expected_version:
                errors.append(f"{label} manifest version must match plugin.json {expected_version}")

    codex_manifest = ROOT / ".codex-plugin/plugin.json"
    if codex_manifest.is_file():
        codex = load_json(codex_manifest)
        if codex.get("skills") != "./skills/":
            errors.append("Codex manifest must point skills to ./skills/")
        interface = codex.get("interface", {})
        if interface.get("displayName") != "GroundLine":
            errors.append("Codex displayName must be GroundLine")
        if interface.get("composerIcon") != "./assets/groundline-icon.svg":
            errors.append("Codex composerIcon must point to packaged icon")
        if interface.get("logo") != "./assets/groundline-logo.svg":
            errors.append("Codex logo must point to packaged logo")

    codex_marketplace = ROOT / ".agents/plugins/marketplace.json"
    if source_root and codex_marketplace.is_file():
        marketplace = load_json(codex_marketplace)
        if marketplace.get("name") != "groundline":
            errors.append("Codex marketplace name must be groundline")
        plugins = marketplace.get("plugins", [])
        if len(plugins) != 1:
            errors.append("Codex marketplace must contain exactly one plugin entry")
        elif plugins[0].get("source", {}).get("path") != "./plugins/groundline":
            errors.append("Codex marketplace must point to ./plugins/groundline")

    claude_marketplace = ROOT / ".claude-plugin/marketplace.json"
    if source_root and claude_marketplace.is_file():
        marketplace = load_json(claude_marketplace)
        if marketplace.get("name") != "groundline":
            errors.append("Claude Code marketplace name must be groundline")
        plugins = marketplace.get("plugins", [])
        if len(plugins) != 1:
            errors.append("Claude Code marketplace must contain exactly one plugin entry")
        elif plugins[0].get("source") != "./plugins/groundline":
            errors.append("Claude Code marketplace must point to ./plugins/groundline")

    skills_dir = ROOT / "skills"
    actual_skills = {path.name for path in skills_dir.iterdir() if path.is_dir()} if skills_dir.is_dir() else set()
    if actual_skills != EXPECTED_SKILLS:
        errors.append(f"skill surface mismatch: {sorted(actual_skills)}")

    if source_root:
        package_dir = ROOT / "plugins/groundline"
        if package_dir.is_dir():
            errors.extend(reject_source_only_package_paths(package_dir, "plugins/groundline/"))
            for path in collect_conflict_copies(package_dir):
                if conflict_copy_has_payload(path):
                    errors.append(f"packaged conflict copy must be removed: {path.relative_to(ROOT)}")
        package_skills = (
            {path.name for path in (package_dir / "skills").iterdir() if path.is_dir()}
            if (package_dir / "skills").is_dir()
            else set()
        )
        if package_skills != EXPECTED_SKILLS:
            errors.append(f"packaged skill surface mismatch: {sorted(package_skills)}")

        for rel in [
            "plugins/groundline/.codex-plugin/plugin.json",
            "plugins/groundline/.claude-plugin/plugin.json",
            "plugins/groundline/plugin.json",
        ]:
            path = ROOT / rel
            if path.is_file():
                try:
                    manifest = load_json(path)
                except Exception as exc:
                    errors.append(f"{rel} invalid JSON: {exc}")
                    continue
                if manifest.get("name") != "groundline":
                    errors.append(f"{rel} manifest name must be groundline")
                if expected_version and manifest.get("version") != expected_version:
                    errors.append(f"{rel} manifest version must match plugin.json {expected_version}")
    else:
        errors.extend(reject_source_only_package_paths(ROOT))
        for path in collect_conflict_copies(ROOT):
            if conflict_copy_has_payload(path):
                errors.append(f"packaged conflict copy must be removed: {path.relative_to(ROOT)}")

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
    source_root = (ROOT / ".agents/plugins/marketplace.json").is_file()
    return {
        "name": "groundline",
        "status": "PASS" if not errors else "FAIL",
        "errors": errors,
        "mutation_performed": False,
        "package_mode": "source" if source_root else "provider_package",
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
