#!/usr/bin/env python3
"""Validate GroundLine safety evaluation fixtures."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURE = ROOT / "scenarios/fixtures/safety-eval.json"
SYNTHETIC_SECRET = "sk-test-secret-value"
SECRET_VALUE_PATTERN = re.compile(
    r"(sk-[A-Za-z0-9_-]+|xox[baprs]-[A-Za-z0-9_-]+|gh[pousr]_[A-Za-z0-9_]+)"
)


def load_fixture(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def secret_values(text: str) -> list[str]:
    values = []
    for match in SECRET_VALUE_PATTERN.findall(text):
        if match != SYNTHETIC_SECRET:
            values.append(match)
    return values


def validate_case(item: object) -> list[str]:
    errors: list[str] = []
    if not isinstance(item, dict):
        return ["case must be an object"]
    for field in ["id", "prompt", "expected_boundary"]:
        value = item.get(field)
        if not isinstance(value, str) or not value.strip():
            errors.append(f"{field} must be a non-empty string")
    prompt = item.get("prompt", "")
    boundary = item.get("expected_boundary", "")
    leaked_values = secret_values(f"{prompt}\n{boundary}")
    if leaked_values:
        errors.append("case contains non-synthetic secret-like value")
    return errors


def build_result(fixture_path: Path) -> dict:
    failures = []
    try:
        fixture = load_fixture(fixture_path)
    except (OSError, json.JSONDecodeError) as exc:
        return {
            "status": "FAIL",
            "mutation_performed": False,
            "case_count": 0,
            "cases": [{"id": "fixture", "errors": [type(exc).__name__]}],
        }

    case_items = fixture.get("cases", [])
    if not isinstance(case_items, list):
        return {
            "status": "FAIL",
            "mutation_performed": False,
            "case_count": 0,
            "cases": [{"id": "fixture", "errors": ["cases must be a list"]}],
        }

    for index, item in enumerate(case_items):
        errors = validate_case(item)
        if errors:
            case_id = item.get("id") if isinstance(item, dict) else f"case-{index}"
            failures.append({"id": case_id, "errors": errors})

    return {
        "status": "PASS" if not failures else "FAIL",
        "mutation_performed": False,
        "case_count": len(case_items),
        "cases": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate GroundLine safety evaluation fixtures.")
    parser.add_argument("--fixture", default=str(DEFAULT_FIXTURE), help="path to safety evaluation fixture")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    args = parser.parse_args()

    result = build_result(Path(args.fixture).expanduser())
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    elif result["status"] == "PASS":
        print(f"GroundLine safety eval passed ({result['case_count']} cases)")
    else:
        for case in result["cases"]:
            print(f"FAIL: {case['id']}: {', '.join(case['errors'])}", file=sys.stderr)
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
