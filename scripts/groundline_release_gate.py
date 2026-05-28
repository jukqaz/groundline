#!/usr/bin/env python3
"""Run or print the GroundLine release verification gate list."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VERSION_MANIFESTS = [
    "plugin.json",
    ".codex-plugin/plugin.json",
    ".claude-plugin/plugin.json",
    "plugins/groundline/plugin.json",
    "plugins/groundline/.codex-plugin/plugin.json",
    "plugins/groundline/.claude-plugin/plugin.json",
]
PLAIN_SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


@dataclass(frozen=True)
class Gate:
    gate_id: str
    label: str
    command: list[str]
    cwd: Path


def python_command(*args: str) -> list[str]:
    return ["python3", *args]


def relative_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def sanitize_text(value: str) -> str:
    home = str(Path.home())
    if home and value.startswith(home):
        return "~" + value[len(home) :]
    return value.replace(home + "/", "~/") if home else value


def build_gates(include_docker_execution: bool, actionlint_bin: str | None) -> list[Gate]:
    lint_command = python_command("scripts/lint.py", "--json", "--require-actionlint")
    if actionlint_bin:
        lint_command.extend(["--actionlint-bin", actionlint_bin])

    gates = [
        Gate("source-validation", "Validate source package", python_command("scripts/validate_pack.py", "--json"), ROOT),
        Gate("packaged-validation", "Validate packaged plugin", python_command("scripts/validate_pack.py", "--json"), ROOT / "plugins/groundline"),
        Gate("lint", "Run lint and actionlint", lint_command, ROOT),
        Gate("runtime-layout", "Check provider runtime layout", python_command("scripts/check_runtime_layout.py", "--json"), ROOT),
        Gate("provider-native-validation", "Run provider-native validators", python_command("scripts/groundline_provider_validate.py", "--json"), ROOT),
        Gate("unit-tests", "Run unit tests", python_command("-m", "unittest", "discover", "-s", "tests", "-v"), ROOT),
        Gate("offline-doctor", "Run offline doctor", python_command("scripts/groundline_doctor.py", "--json", "--offline", "--probe-tools"), ROOT),
        Gate("offline-radar", "Run offline radar", python_command("scripts/groundline_radar.py", "--json", "--offline", "--command-sources"), ROOT),
        Gate("safety-eval", "Run safety eval", python_command("scripts/groundline_safety_eval.py", "--json"), ROOT),
        Gate("privacy-scan", "Run privacy scan", python_command("scripts/groundline_privacy_scan.py", "--json"), ROOT),
        Gate(
            "provider-smoke",
            "Run provider smoke",
            python_command("scripts/groundline_provider_smoke.py", "--json", "--require-installed"),
            ROOT,
        ),
        Gate(
            "staged-dogfood",
            "Run staged dogfood",
            python_command("scripts/groundline_dogfood.py", "--stage-package", "--probe-runtimes", "--json"),
            ROOT,
        ),
        Gate(
            "macos-local-scenario",
            "Run macOS local scenario",
            python_command("scripts/run_scenarios.py", "--platform", "macos", "--sandbox", "local", "--json"),
            ROOT,
        ),
        Gate(
            "linux-docker-dry-run",
            "Run Linux Docker dry-run scenario",
            python_command("scripts/run_scenarios.py", "--platform", "linux", "--sandbox", "docker", "--dry-run", "--json"),
            ROOT,
        ),
    ]
    if include_docker_execution:
        gates.append(
            Gate(
                "linux-docker-execution",
                "Run Linux Docker execution scenario",
                python_command("scripts/run_scenarios.py", "--platform", "linux", "--sandbox", "docker", "--json"),
                ROOT,
            )
        )
    return gates


def output_tail(value: str | bytes | None, line_limit: int = 30) -> str:
    if not value:
        return ""
    if isinstance(value, bytes):
        value = value.decode("utf-8", errors="replace")
    text = "\n".join(value.splitlines()[-line_limit:])
    return sanitize_text(text)


SUMMARY_KEYS = [
    "status",
    "install_doctor_status",
    "next_actions",
    "install_issues",
    "missing_manifests",
    "mutation_performed",
    "publishing_performed",
    "real_home_touched",
    "secret_value_printed",
    "fake_home_used",
    "network",
    "case_count",
    "finding_count",
    "scanned_file_count",
]


def sanitized_json_value(value):
    if isinstance(value, str):
        return sanitize_text(value)
    if isinstance(value, list):
        return [sanitized_json_value(item) for item in value]
    if isinstance(value, dict):
        return {key: sanitized_json_value(item) for key, item in value.items()}
    return value


def manifest_version(path: Path) -> str | None:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    version = data.get("version") if isinstance(data, dict) else None
    return version if isinstance(version, str) else None


def release_version_check(expected_version: str) -> dict:
    issues = []
    if not PLAIN_SEMVER_RE.fullmatch(expected_version):
        issues.append("invalid_release_version")
    versions = {rel: manifest_version(ROOT / rel) for rel in VERSION_MANIFESTS}
    mismatches = [
        {"path": rel, "version": version}
        for rel, version in versions.items()
        if version != expected_version
    ]
    next_actions = []
    if "invalid_release_version" in issues:
        next_actions.append("use plain semver like 0.3.3 without a v prefix")
    if mismatches and not issues:
        next_actions.append(f"set source manifests to {expected_version} and run sync_provider_package.py")
    return {
        "id": "release-version",
        "label": "Check release manifest versions",
        "status": "PASS" if not issues and not mismatches else "FAIL",
        "expected_version": expected_version,
        "release_version_valid": not issues,
        "issues": issues,
        "manifest_versions": versions,
        "mismatches": mismatches,
        "next_actions": next_actions,
    }


def json_summary(stdout: str | None) -> dict | None:
    if not stdout:
        return None
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError:
        return None
    if not isinstance(payload, dict):
        return None
    summary = {key: sanitized_json_value(payload[key]) for key in SUMMARY_KEYS if key in payload}
    return summary or None


def gate_to_result(gate: Gate, executed: bool) -> dict:
    return {
        "id": gate.gate_id,
        "label": gate.label,
        "cwd": relative_path(gate.cwd),
        "command": gate.command,
        "executed": executed,
    }


def run_gate(gate: Gate, timeout: int) -> dict:
    result = gate_to_result(gate, executed=True)
    started = time.monotonic()
    try:
        completed = subprocess.run(
            gate.command,
            cwd=gate.cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        result.update(
            {
                "status": "PARTIAL",
                "exit_code": None,
                "duration_seconds": round(time.monotonic() - started, 3),
                "error": "gate timed out",
                "stdout_tail": output_tail(exc.stdout),
                "stderr_tail": output_tail(exc.stderr),
            }
        )
        return result
    except OSError as exc:
        result.update(
            {
                "status": "FAIL",
                "exit_code": None,
                "duration_seconds": round(time.monotonic() - started, 3),
                "error": type(exc).__name__,
            }
        )
        return result

    status = "PASS"
    if completed.returncode == 2:
        status = "PARTIAL"
    elif completed.returncode != 0:
        status = "FAIL"
    result.update(
        {
            "status": status,
            "exit_code": completed.returncode,
            "duration_seconds": round(time.monotonic() - started, 3),
            "stdout_tail": output_tail(completed.stdout),
            "stderr_tail": output_tail(completed.stderr),
        }
    )
    summary = json_summary(completed.stdout)
    if summary:
        result["json_summary"] = summary
    return result


def aggregate_status(gates: list[dict]) -> str:
    statuses = {gate.get("status") for gate in gates if gate.get("executed")}
    if "FAIL" in statuses:
        return "FAIL"
    if "PARTIAL" in statuses:
        return "PARTIAL"
    return "PASS"


def summarize_non_passing_gates(gates: list[dict]) -> list[dict]:
    summaries: list[dict] = []
    for gate in gates:
        status = gate.get("status")
        if status == "PASS" or not gate.get("executed"):
            continue
        summary = {
            "id": gate.get("id"),
            "label": gate.get("label"),
            "status": status,
        }
        if "exit_code" in gate:
            summary["exit_code"] = gate["exit_code"]
        if gate.get("error"):
            summary["error"] = gate["error"]
        if gate.get("json_summary"):
            summary["json_summary"] = gate["json_summary"]
        summaries.append(summary)
    return summaries


def collect_next_actions(gates: list[dict]) -> list[str]:
    actions: set[str] = set()
    for gate in gates:
        if gate.get("status") == "PASS" or not gate.get("executed"):
            continue
        json_actions = gate.get("json_summary", {}).get("next_actions")
        if isinstance(json_actions, list):
            actions.update(action for action in json_actions if isinstance(action, str))
        elif not gate.get("json_summary"):
            actions.add(f"inspect the {gate.get('id')} gate output")
    return sorted(actions)


def collect_preflight_actions(checks: list[dict]) -> list[str]:
    actions: set[str] = set()
    for check in checks:
        if check.get("status") == "PASS":
            continue
        actions.update(action for action in check.get("next_actions", []) if isinstance(action, str))
    return sorted(actions)


def build_result(args: argparse.Namespace) -> tuple[dict, int]:
    gates = build_gates(args.include_docker_execution, args.actionlint_bin)
    preflight_checks = [release_version_check(args.release_version)] if args.release_version else []
    result = {
        "name": "groundline",
        "suite": "release-gate",
        "mode": "plan" if args.plan else "run",
        "status": "PASS",
        "release_version": args.release_version,
        "mutation_performed": False,
        "publishing_performed": False,
        "real_home_touched": False,
        "include_docker_execution": args.include_docker_execution,
        "approval_required_commands_excluded": [
            'git tag "$TAG"',
            'git push origin "$TAG"',
            'gh release create "$TAG"',
        ],
        "non_passing_gates": [],
        "next_actions": [],
        "preflight_checks": preflight_checks,
        "gates": [],
    }

    if args.plan:
        result["gates"] = [gate_to_result(gate, executed=False) for gate in gates]
        failed_checks = [check for check in preflight_checks if check["status"] != "PASS"]
        if failed_checks:
            result["status"] = "FAIL"
            result["non_passing_gates"] = failed_checks
            result["next_actions"] = collect_preflight_actions(failed_checks)
            return result, 1
        return result, 0

    gate_results: list[dict] = []
    failed_checks = [check for check in preflight_checks if check["status"] != "PASS"]
    if failed_checks and not args.keep_going:
        result["status"] = "FAIL"
        result["non_passing_gates"] = failed_checks
        result["next_actions"] = collect_preflight_actions(failed_checks)
        return result, 1

    for gate in gates:
        gate_result = run_gate(gate, args.timeout)
        gate_results.append(gate_result)
        if gate_result["status"] != "PASS" and not args.keep_going:
            break
    result["gates"] = gate_results
    gate_status = aggregate_status(gate_results)
    result["status"] = "FAIL" if failed_checks else gate_status
    result["non_passing_gates"] = failed_checks + summarize_non_passing_gates(gate_results)
    result["next_actions"] = sorted(set(collect_preflight_actions(failed_checks) + collect_next_actions(gate_results)))

    if result["status"] == "PASS":
        return result, 0
    if result["status"] == "PARTIAL":
        return result, 2
    return result, 1


def main() -> int:
    parser = argparse.ArgumentParser(description="Run or print GroundLine release gates.")
    parser.add_argument("--json", action="store_true", help="emit JSON")
    parser.add_argument("--plan", action="store_true", help="print the gate list without running commands")
    parser.add_argument("--keep-going", action="store_true", help="run remaining gates after a failure")
    parser.add_argument("--include-docker-execution", action="store_true", help="include the real Linux Docker execution gate")
    parser.add_argument("--actionlint-bin", help="path to actionlint binary for the lint gate")
    parser.add_argument("--timeout", type=int, default=600, help="per-gate timeout in seconds")
    parser.add_argument("--release-version", help="expected semver for source and packaged manifests")
    args = parser.parse_args()

    result, code = build_result(args)
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    elif args.plan:
        print("GroundLine release gate plan")
        for gate in result["gates"]:
            print(f"- {gate['id']}: {' '.join(gate['command'])}")
    else:
        print(f"GroundLine release gate: {result['status']}")
        for gate in result["gates"]:
            print(f"- {gate['id']}: {gate['status']}")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
