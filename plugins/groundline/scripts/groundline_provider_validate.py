#!/usr/bin/env python3
"""Run read-only provider-native package validators when available."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SECRET_PATTERN = re.compile(
    r"((?<![A-Za-z0-9])sk-[A-Za-z0-9_-]+|(?<![A-Za-z0-9])xox[baprs]-[A-Za-z0-9_-]+|(?i:api[_-]?key|token|secret|password))"
)


@dataclass(frozen=True)
class Validator:
    provider: str
    label: str
    command: list[str]
    cwd: Path


def relative_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def sanitize_text(value: str | None) -> tuple[str, bool]:
    if not value:
        return "", False
    home = str(Path.home())
    text = value
    if home:
        text = text.replace(home + "/", "~/")
        if text == home:
            text = "~"
    redacted = SECRET_PATTERN.search(text) is not None
    if redacted:
        text = SECRET_PATTERN.sub("[redacted]", text)
    return text, redacted


def output_tail(value: str | bytes | None, line_limit: int = 20) -> tuple[str, bool]:
    if not value:
        return "", False
    if isinstance(value, bytes):
        value = value.decode("utf-8", errors="replace")
    return sanitize_text("\n".join(value.splitlines()[-line_limit:]))


def package_root() -> Path:
    packaged = ROOT / "plugins/groundline"
    return packaged if packaged.is_dir() else ROOT


def build_validators() -> list[Validator]:
    package = package_root()
    package_arg = "plugins/groundline" if package != ROOT else "."
    validators = [
        Validator(
            "claude_code",
            "Validate Claude Code plugin manifest",
            ["claude", "plugin", "validate", package_arg, "--strict"],
            ROOT,
        ),
        Validator(
            "antigravity",
            "Validate Antigravity source package",
            ["agy", "plugin", "validate", "."],
            ROOT,
        ),
    ]
    if package != ROOT:
        validators.append(
            Validator(
                "antigravity",
                "Validate Antigravity packaged plugin",
                ["agy", "plugin", "validate", package_arg],
                ROOT,
            )
        )
    return validators


def run_validator(validator: Validator, timeout: int) -> dict:
    binary = validator.command[0]
    result = {
        "provider": validator.provider,
        "label": validator.label,
        "command": validator.command,
        "cwd": relative_path(validator.cwd),
    }
    if shutil.which(binary) is None:
        result.update(
            {
                "status": "PARTIAL",
                "available": False,
                "next_action": f"install or expose {binary} to run {validator.label}",
            }
        )
        return result

    try:
        completed = subprocess.run(
            validator.command,
            cwd=validator.cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        stdout_tail, stdout_redacted = output_tail(exc.stdout)
        stderr_tail, stderr_redacted = output_tail(exc.stderr)
        result.update(
            {
                "status": "PARTIAL",
                "available": True,
                "exit_code": None,
                "error": "validator timed out",
                "stdout_tail": stdout_tail,
                "stderr_tail": stderr_tail,
                "redacted": stdout_redacted or stderr_redacted,
                "next_action": f"rerun {validator.label} with a longer timeout",
            }
        )
        return result
    except OSError as exc:
        result.update(
            {
                "status": "FAIL",
                "available": True,
                "exit_code": None,
                "error": type(exc).__name__,
                "next_action": f"inspect {validator.label} execution",
            }
        )
        return result

    status = "PASS" if completed.returncode == 0 else "FAIL"
    stdout_tail, stdout_redacted = output_tail(completed.stdout)
    stderr_tail, stderr_redacted = output_tail(completed.stderr)
    result.update(
        {
            "status": status,
            "available": True,
            "exit_code": completed.returncode,
            "stdout_tail": stdout_tail,
            "stderr_tail": stderr_tail,
            "redacted": stdout_redacted or stderr_redacted,
        }
    )
    if status != "PASS":
        result["next_action"] = f"inspect {validator.label} output"
    return result


def aggregate_status(validations: list[dict]) -> str:
    statuses = {item["status"] for item in validations}
    if "FAIL" in statuses:
        return "FAIL"
    if "PARTIAL" in statuses:
        return "PARTIAL"
    return "PASS"


def build_result(timeout: int) -> dict:
    validations = [run_validator(validator, timeout) for validator in build_validators()]
    next_actions = sorted(
        item["next_action"] for item in validations if item.get("status") != "PASS" and item.get("next_action")
    )
    return {
        "name": "groundline",
        "suite": "provider-native-validation",
        "status": aggregate_status(validations),
        "package_mode": "source" if (ROOT / "plugins/groundline").is_dir() else "provider_package",
        "mutation_performed": False,
        "real_home_touched": False,
        "secret_value_printed": False,
        "validations": validations,
        "next_actions": next_actions,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Run provider-native GroundLine validators.")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    parser.add_argument("--timeout", type=int, default=120, help="per-validator timeout in seconds")
    args = parser.parse_args()

    result = build_result(args.timeout)
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"GroundLine provider-native validation: {result['status']}")
        for validation in result["validations"]:
            print(f"- {validation['label']}: {validation['status']}")

    if result["status"] == "PASS":
        return 0
    if result["status"] == "PARTIAL":
        return 2
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
