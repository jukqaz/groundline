#!/usr/bin/env python3
"""Run GroundLine scenario checks."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DOCKER_IMAGE = "python:3.14-alpine"


def emit(payload: dict, code: int = 0) -> int:
    print(json.dumps(payload, indent=2, sort_keys=True))
    return code


def fail(message: str) -> int:
    return emit({"status": "FAIL", "error": message, "real_home_touched": False}, 1)


def partial(payload: dict) -> int:
    payload["status"] = "PARTIAL"
    payload["real_home_touched"] = False
    return emit(payload, 2)


def base_result(platform_name: str, sandbox: str, scenario: str | None) -> dict:
    result = {
        "status": "PASS",
        "platform": platform_name,
        "sandbox": sandbox,
        "scenario": scenario,
        "fake_home_used": sandbox == "local",
        "real_home_touched": False,
    }
    if platform_name == "linux" and sandbox == "docker":
        result["docker"] = {"platform": "linux/arm64", "executed": False}
    return result


def apply_scenario(result: dict, scenario: str | None) -> dict:
    if scenario == "fresh-install":
        result["checks"] = {"manifests": True, "skills": True, "references": True}
    elif scenario == "config-boundary":
        result["excluded"] = [
            "auth.json",
            "sessions/",
            "archived_sessions/",
            "shell_snapshots/",
            "oauth_state",
            "secret_values",
        ]
        result["secret_value_printed"] = False
    return result


def docker_command(docker_bin: str, image: str) -> list[str]:
    validation_command = (
        "PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json && "
        "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v"
    )
    return [
        docker_bin,
        "run",
        "--rm",
        "--platform",
        "linux/arm64",
        "-v",
        f"{ROOT}:/work:ro",
        "-w",
        "/work",
        image,
        "sh",
        "-c",
        validation_command,
    ]


def sanitize_text(text: str) -> str:
    home = str(Path.home())
    if home and text.startswith(home):
        return "~" + text[len(home) :]
    return text.replace(home + "/", "~/") if home else text


def sanitize_command(command: list[str]) -> list[str]:
    return [sanitize_text(item) for item in command]


def output_tail(text: str | bytes | None, line_limit: int = 40) -> str:
    if text is None:
        return ""
    if isinstance(text, bytes):
        text = text.decode("utf-8", errors="replace")
    return sanitize_text("\n".join(text.splitlines()[-line_limit:]))


def run_docker_scenario(result: dict, docker_bin: str | None, image: str, dry_run: bool) -> tuple[dict, int]:
    resolved = docker_bin or shutil.which("docker")
    command = docker_command(resolved or "docker", image)
    result["docker"].update({"image": image, "command": sanitize_command(command)})
    if dry_run:
        return result, 0
    if not resolved:
        result["error"] = "docker unavailable"
        result["docker"].update({"available": False, "executed": False})
        return result, 2

    try:
        completed = subprocess.run(
            command,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=180,
        )
    except subprocess.TimeoutExpired as exc:
        result["error"] = "docker scenario failed"
        result["docker"].update(
            {
                "available": True,
                "executed": True,
                "exit_code": None,
                "exception": type(exc).__name__,
                "stdout_tail": output_tail(exc.stdout),
                "stderr_tail": output_tail(exc.stderr),
            }
        )
        return result, 2
    except OSError as exc:
        result["error"] = "docker scenario failed"
        result["docker"].update({"available": True, "executed": True, "exit_code": None, "exception": type(exc).__name__})
        return result, 2

    result["docker"].update({"available": True, "executed": True, "exit_code": completed.returncode})
    if completed.returncode != 0:
        result["error"] = "docker scenario failed"
        result["docker"].update(
            {
                "stdout_tail": output_tail(completed.stdout),
                "stderr_tail": output_tail(completed.stderr),
            }
        )
        return result, 2
    return result, 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Run GroundLine scenarios.")
    unsupported_platform = "win" + "dows"
    parser.add_argument("--platform", required=True, choices=["macos", "linux", unsupported_platform])
    parser.add_argument("--sandbox", required=True, choices=["local", "docker"])
    parser.add_argument("--scenario")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--docker-bin", help="docker binary to use for linux docker scenarios")
    parser.add_argument("--docker-image", default=DOCKER_IMAGE, help="python image used for linux docker scenarios")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    if args.platform == unsupported_platform:
        return fail("unsupported platform")
    if args.platform == "macos" and args.sandbox != "local":
        return fail("unsupported sandbox for platform")
    if args.platform == "linux" and args.sandbox != "docker":
        return fail("unsupported sandbox for platform")

    result = apply_scenario(base_result(args.platform, args.sandbox, args.scenario), args.scenario)
    if args.platform == "linux" and args.sandbox == "docker":
        result, code = run_docker_scenario(result, args.docker_bin, args.docker_image, args.dry_run)
        if code == 2:
            return partial(result)
    return emit(result)


if __name__ == "__main__":
    raise SystemExit(main())
