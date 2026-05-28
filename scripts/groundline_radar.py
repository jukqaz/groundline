#!/usr/bin/env python3
"""Detect source registry changes without mutating runtime state."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SECRET_PATTERN = re.compile(
    r"((?<![A-Za-z0-9])sk-[A-Za-z0-9_-]+|(?<![A-Za-z0-9])xox[baprs]-[A-Za-z0-9_-]+|(?i:api[_-]?key|token|secret|password))"
)


def first_line(text: str) -> str:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped:
            return stripped
    return ""


def command_source_version(source: dict) -> tuple[str | None, dict | None]:
    source_id = source.get("id")
    command = source.get("command")
    if not isinstance(command, list):
        return None, {"id": source_id, "reason": "command must be a list"}
    if not command or not all(isinstance(item, str) and item for item in command):
        return None, {"id": source_id, "reason": "command must contain string tokens"}

    try:
        completed = subprocess.run(
            command,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=10,
        )
    except FileNotFoundError:
        return None, {"id": source_id, "reason": "command not found"}
    except (OSError, subprocess.TimeoutExpired):
        return None, {"id": source_id, "reason": "command failed"}

    output = first_line(completed.stdout or completed.stderr)
    if completed.returncode != 0:
        return None, {"id": source_id, "reason": "command failed"}
    if not output:
        return None, {"id": source_id, "reason": "command produced no output"}
    if SECRET_PATTERN.search(output):
        return None, {"id": source_id, "reason": "command output redacted"}
    return output[:240], None


def classify_sources(sources: list[dict], network_enabled: bool, command_sources_enabled: bool) -> dict:
    changed: list[dict] = []
    new: list[dict] = []
    removed: list[dict] = []
    skipped: list[dict] = []

    for source in sources:
        source_id = source.get("id")
        if source.get("kind") == "remote" and not network_enabled:
            skipped.append({"id": source_id, "reason": "network disabled"})
            continue
        if source.get("kind") == "command":
            if not command_sources_enabled:
                skipped.append({"id": source_id, "reason": "command sources disabled"})
                continue
            current_version, skip_reason = command_source_version(source)
            if skip_reason:
                skipped.append(skip_reason)
                continue
            source = dict(source)
            source["current_version"] = current_version
        if source.get("removed"):
            removed.append({"id": source_id, "url": source.get("url")})
            continue
        last_seen = source.get("last_seen_version")
        current = source.get("current_version")
        if current and not last_seen:
            new.append({"id": source_id, "url": source.get("url")})
        elif current and last_seen and current != last_seen:
            changed.append(
                {
                    "id": source_id,
                    "owner": source.get("owner"),
                    "old_version": last_seen,
                    "new_version": current,
                    "url": source.get("url"),
                }
            )

    return {
        "changed_sources": changed,
        "new_sources": new,
        "removed_sources": removed,
        "skipped_sources": skipped,
    }


def build_result(registry_path: Path, network_enabled: bool, command_sources_enabled: bool) -> dict:
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    sources = registry.get("sources", [])
    classified = classify_sources(sources, network_enabled, command_sources_enabled)
    impacted_ids = [item["id"] for key in ["changed_sources", "new_sources", "removed_sources"] for item in classified[key]]
    skipped_ids = {item.get("id") for item in classified["skipped_sources"]}
    removed_ids = {item.get("id") for item in classified["removed_sources"]}
    review_ids = [
        source["id"]
        for source in sources
        if isinstance(source.get("id"), str) and source.get("id") not in skipped_ids and source.get("id") not in removed_ids
    ]
    research_ids = impacted_ids if impacted_ids else review_ids
    research_mode = "change-review" if impacted_ids else "ecosystem-scan"
    tasks = [
        {
            "title": f"Investigate {source_id}",
            "action": "verify source, update references, and run validation",
        }
        for source_id in impacted_ids
    ]
    result = {
        "kind": "radar",
        "status": "PASS",
        "mutation_performed": False,
        "network": "enabled" if network_enabled else "disabled",
        "research_packet": {
            "mode": research_mode,
            "sources": research_ids,
            "changed_or_new_sources": impacted_ids,
            "prompt": (
                "Verify changed sources and propose GroundLine updates."
                if impacted_ids
                else "Review tracked sources and propose GroundLine updates only when evidence supports it."
            ),
        },
        "upgrade_task_candidates": tasks,
        "verification_checklist": ["validate references", "run tests"],
    }
    result.update(classified)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Run GroundLine radar.")
    parser.add_argument("--registry", default=str(ROOT / "references/source-registry.json"))
    parser.add_argument("--json", action="store_true", help="emit JSON")
    parser.add_argument("--offline", action="store_true", help="disable network checks")
    parser.add_argument("--network", action="store_true", help="enable network checks")
    parser.add_argument("--command-sources", action="store_true", help="run local command sources from the registry")
    args = parser.parse_args()

    result = build_result(
        Path(args.registry),
        network_enabled=args.network and not args.offline,
        command_sources_enabled=args.command_sources,
    )
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print("GroundLine radar checked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
