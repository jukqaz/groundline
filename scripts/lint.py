#!/usr/bin/env python3
"""Run GroundLine local lint gates without writing cache files."""

from __future__ import annotations

import argparse
import ast
import json
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def project_files(suffixes: set[str]) -> list[Path]:
    files: list[Path] = []
    for path in sorted(ROOT.rglob("*")):
        if not path.is_file() or ".git" in path.parts:
            continue
        if path.suffix in suffixes:
            files.append(path)
    return files


def check_python_ast() -> list[str]:
    errors: list[str] = []
    for path in project_files({".py"}):
        try:
            ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        except SyntaxError as exc:
            errors.append(f"{path.relative_to(ROOT)}: {exc.msg}")
    return errors


def check_json() -> list[str]:
    errors: list[str] = []
    for path in project_files({".json"}):
        try:
            json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            errors.append(f"{path.relative_to(ROOT)}: {exc}")
    return errors


def check_workflow_static() -> list[str]:
    errors: list[str] = []
    for path in [ROOT / ".github/workflows/test.yml", ROOT / ".github/workflows/radar.yml"]:
        if not path.is_file():
            errors.append(f"missing workflow: {path.relative_to(ROOT)}")
            continue
        text = path.read_text(encoding="utf-8")
        if 'PYTHONDONTWRITEBYTECODE: "1"' not in text:
            errors.append(f"{path.relative_to(ROOT)} missing bytecode guard")
        if 'FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: "true"' not in text:
            errors.append(f"{path.relative_to(ROOT)} missing Node 24 action guard")
    return errors


def resolve_actionlint(path: str | None) -> str | None:
    if path:
        candidate = Path(path).expanduser()
        return str(candidate) if candidate.is_file() else None
    return shutil.which("actionlint")


def run_actionlint(actionlint_bin: str | None, require_actionlint: bool) -> tuple[str, list[str]]:
    resolved = resolve_actionlint(actionlint_bin)
    if not resolved:
        if require_actionlint:
            return "missing", ["actionlint not found"]
        return "skipped", []

    workflow_files = [
        str(ROOT / ".github/workflows/test.yml"),
        str(ROOT / ".github/workflows/radar.yml"),
    ]
    completed = subprocess.run(
        [resolved, "-no-color", *workflow_files],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=30,
    )
    if completed.returncode != 0:
        return "failed", [f"actionlint exited {completed.returncode}"]
    return "passed", []


def build_result(actionlint_bin: str | None, require_actionlint: bool) -> dict:
    python_errors = check_python_ast()
    json_errors = check_json()
    workflow_errors = check_workflow_static()
    actionlint_status, actionlint_errors = run_actionlint(actionlint_bin, require_actionlint)
    errors = python_errors + json_errors + workflow_errors + actionlint_errors
    return {
        "status": "PASS" if not errors else "FAIL",
        "mutation_performed": False,
        "checks": {
            "python_ast": not python_errors,
            "json": not json_errors,
            "workflow_static": not workflow_errors,
            "actionlint": actionlint_status,
        },
        "errors": errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Run GroundLine lint gates.")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    parser.add_argument("--actionlint-bin", help="path to actionlint binary")
    parser.add_argument("--require-actionlint", action="store_true", help="fail when actionlint is unavailable")
    args = parser.parse_args()

    result = build_result(args.actionlint_bin, args.require_actionlint)
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    elif result["status"] == "PASS":
        print("GroundLine lint passed")
    else:
        for error in result["errors"]:
            print(f"FAIL: {error}")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
