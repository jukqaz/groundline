#!/usr/bin/env python3
"""Create an LLM-ready GroundLine upgrade packet."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


SECRET_PATTERN = re.compile(r"(sk-[A-Za-z0-9_-]+|secret-value)", re.IGNORECASE)


def contains_secret_like_value(value: object) -> bool:
    if isinstance(value, str):
        return bool(SECRET_PATTERN.search(value))
    if isinstance(value, dict):
        return any(contains_secret_like_value(item) for item in value.values())
    if isinstance(value, list):
        return any(contains_secret_like_value(item) for item in value)
    return False


def fail(message: str) -> int:
    print(json.dumps({"status": "FAIL", "error": message, "mutation_performed": False}, sort_keys=True))
    return 1


def from_doctor(payload: dict) -> dict:
    gaps = payload.get("capability_gaps", [])
    affected = sorted({gap.get("recommended_source") for gap in gaps if gap.get("recommended_source")})
    return {
        "kind": "upgrade_packet",
        "source_kind": "doctor",
        "mutation_performed": False,
        "current_evidence": payload.get("runtimes", {}),
        "suspected_drift": gaps,
        "docs_to_verify": affected,
        "affected_groundline_files": affected,
        "proposed_tasks": [
            {
                "title": f"Close capability gap: {gap.get('id', 'unknown')}",
                "capability": gap.get("capability"),
            }
            for gap in gaps
        ],
        "verification_checklist": ["run doctor", "run validate_pack", "run scenario tests"],
    }


def from_radar(payload: dict) -> dict:
    changed = payload.get("changed_sources", [])
    docs = [source.get("url") for source in changed if source.get("url")]
    return {
        "kind": "upgrade_packet",
        "source_kind": "radar",
        "mutation_performed": False,
        "current_evidence": {"changed_sources": changed},
        "suspected_drift": changed,
        "research_packet": {
            "sources": [source.get("id") for source in changed],
            "prompt": "Verify source changes and decide whether GroundLine needs an update.",
        },
        "docs_to_verify": docs,
        "affected_groundline_files": ["references/runtime-matrix.md", "references/source-registry.json"],
        "proposed_tasks": [
            {
                "title": f"Review source change: {source.get('id')}",
                "url": source.get("url"),
            }
            for source in changed
        ],
        "verification_checklist": ["verify official source", "update references", "run validation"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Create GroundLine upgrade packet.")
    parser.add_argument("--input", required=True, help="doctor or radar JSON input")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    try:
        payload = json.loads(Path(args.input).read_text(encoding="utf-8"))
    except Exception as exc:
        return fail(f"invalid input: {exc}")

    if contains_secret_like_value(payload):
        return fail("secret-like input rejected")

    kind = payload.get("kind")
    if kind == "doctor":
        result = from_doctor(payload)
    elif kind == "radar":
        result = from_radar(payload)
    else:
        return fail("unsupported input kind")

    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
