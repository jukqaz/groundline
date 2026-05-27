import json
import platform
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


PACK_ROOT = Path(__file__).resolve().parents[1]
SCRIPTS_DIR = PACK_ROOT / "scripts"


class GroundLineScenarioContractTests(unittest.TestCase):
    def run_script(self, script_name: str, *args: str) -> subprocess.CompletedProcess[str]:
        script_path = SCRIPTS_DIR / script_name
        self.assertTrue(script_path.is_file(), f"missing script: scripts/{script_name}")

        return subprocess.run(
            [sys.executable, str(script_path), *args],
            cwd=PACK_ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def run_script_json(self, script_name: str, *args: str) -> dict:
        completed = self.run_script(script_name, *args)
        self.assertEqual(
            completed.returncode,
            0,
            f"script failed: {script_name}\nstdout:\n{completed.stdout}\nstderr:\n{completed.stderr}",
        )
        try:
            return json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            self.fail(f"{script_name} did not emit JSON: {exc}\nstdout:\n{completed.stdout}")

    def make_fake_home(self, root: Path, *, superpowers: bool = False) -> Path:
        home = root / "home"
        (home / ".codex").mkdir(parents=True)
        (home / ".claude").mkdir(parents=True)
        (home / ".gemini/config/plugins/groundline").mkdir(parents=True)
        (home / ".gemini/antigravity-cli/plugins/groundline").mkdir(parents=True)

        if superpowers:
            (home / ".codex/plugins/cache/openai-curated/superpowers").mkdir(parents=True)

        return home

    def test_doctor_recommends_standalone_mode_without_superpowers(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-home-") as temp:
            home = self.make_fake_home(Path(temp), superpowers=False)

            result = self.run_script_json(
                "groundline_doctor.py",
                "--home",
                str(home),
                "--json",
                "--offline",
            )

        self.assertEqual(result["recommended_mode"], "standalone-groundline")
        self.assertFalse(result["mutation_performed"])
        self.assertEqual(result["network"], "disabled")
        self.assertEqual(result["runtimes"]["codex"]["scope"], "supported")
        self.assertEqual(result["runtimes"]["claude_code"]["scope"], "supported")
        self.assertEqual(result["runtimes"]["antigravity"]["scope"], "supported")
        self.assertNotIn("gemini_cli", result["runtimes"])

    def test_doctor_recommends_companion_mode_with_superpowers(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-home-") as temp:
            home = self.make_fake_home(Path(temp), superpowers=True)

            result = self.run_script_json(
                "groundline_doctor.py",
                "--home",
                str(home),
                "--json",
                "--offline",
            )

        self.assertEqual(result["recommended_mode"], "companion-superpowers")
        self.assertTrue(result["superpowers"]["present"])
        self.assertFalse(result["mutation_performed"])

    def test_radar_reports_changed_fixture_source_without_network(self) -> None:
        registry = {
            "sources": [
                {
                    "id": "codex-docs",
                    "kind": "fixture",
                    "last_seen_version": "2026-05-26",
                    "current_version": "2026-05-27",
                    "url": "fixture://codex-docs",
                    "owner": "Codex",
                }
            ]
        }

        with tempfile.TemporaryDirectory(prefix="groundline-radar-") as temp:
            registry_path = Path(temp) / "source-registry.json"
            registry_path.write_text(json.dumps(registry), encoding="utf-8")

            result = self.run_script_json(
                "groundline_radar.py",
                "--registry",
                str(registry_path),
                "--json",
                "--offline",
            )

        self.assertFalse(result["mutation_performed"])
        self.assertEqual(result["network"], "disabled")
        self.assertEqual(result["changed_sources"][0]["id"], "codex-docs")
        self.assertIn("research_packet", result)
        self.assertGreaterEqual(len(result["upgrade_task_candidates"]), 1)

    def test_radar_reports_new_and_removed_fixture_sources(self) -> None:
        registry = {
            "sources": [
                {
                    "id": "antigravity-docs",
                    "kind": "fixture",
                    "current_version": "2026-05-27",
                    "url": "fixture://antigravity-docs",
                    "owner": "Antigravity",
                },
                {
                    "id": "old-agent-tool",
                    "kind": "fixture",
                    "last_seen_version": "2026-05-26",
                    "removed": True,
                    "url": "fixture://old-agent-tool",
                    "owner": "External",
                },
            ]
        }

        with tempfile.TemporaryDirectory(prefix="groundline-radar-") as temp:
            registry_path = Path(temp) / "source-registry.json"
            registry_path.write_text(json.dumps(registry), encoding="utf-8")

            result = self.run_script_json(
                "groundline_radar.py",
                "--registry",
                str(registry_path),
                "--json",
                "--offline",
            )

        self.assertEqual(result["new_sources"][0]["id"], "antigravity-docs")
        self.assertEqual(result["removed_sources"][0]["id"], "old-agent-tool")
        self.assertGreaterEqual(len(result["upgrade_task_candidates"]), 1)

    def test_run_scenarios_fresh_install_validation(self) -> None:
        result = self.run_script_json(
            "run_scenarios.py",
            "--scenario",
            "fresh-install",
            "--platform",
            "macos",
            "--sandbox",
            "local",
            "--json",
        )

        self.assertEqual(result["scenario"], "fresh-install")
        self.assertEqual(result["status"], "PASS")
        self.assertTrue(result["checks"]["manifests"])
        self.assertTrue(result["checks"]["skills"])
        self.assertTrue(result["checks"]["references"])
        self.assertFalse(result["real_home_touched"])

    def test_run_scenarios_config_boundary_excludes_runtime_state(self) -> None:
        result = self.run_script_json(
            "run_scenarios.py",
            "--scenario",
            "config-boundary",
            "--platform",
            "macos",
            "--sandbox",
            "local",
            "--json",
        )

        self.assertEqual(result["scenario"], "config-boundary")
        self.assertEqual(result["status"], "PASS")
        excluded = set(result["excluded"])
        self.assertIn("auth.json", excluded)
        self.assertIn("sessions/", excluded)
        self.assertIn("archived_sessions/", excluded)
        self.assertIn("shell_snapshots/", excluded)
        self.assertIn("oauth_state", excluded)
        self.assertIn("secret_values", excluded)
        self.assertFalse(result["secret_value_printed"])

    @unittest.skipUnless(
        sys.platform == "darwin" and platform.machine() == "arm64",
        "macOS scenario is only required on Apple Silicon",
    )
    def test_run_scenarios_macos_local_uses_fake_home_only(self) -> None:
        result = self.run_script_json(
            "run_scenarios.py",
            "--platform",
            "macos",
            "--sandbox",
            "local",
            "--json",
        )

        self.assertEqual(result["platform"], "macos")
        self.assertEqual(result["sandbox"], "local")
        self.assertTrue(result["fake_home_used"])
        self.assertFalse(result["real_home_touched"])

    def test_run_scenarios_linux_docker_dry_run_uses_arm64_container(self) -> None:
        result = self.run_script_json(
            "run_scenarios.py",
            "--platform",
            "linux",
            "--sandbox",
            "docker",
            "--dry-run",
            "--json",
        )

        self.assertEqual(result["platform"], "linux")
        self.assertEqual(result["sandbox"], "docker")
        self.assertEqual(result["docker"]["platform"], "linux/arm64")
        self.assertFalse(result["docker"]["executed"])
        self.assertFalse(result["real_home_touched"])

    def write_fake_docker(self, root: Path, *, exit_code: int = 0) -> tuple[Path, Path]:
        log_path = root / "docker-args.log"
        docker_path = root / "docker"
        docker_path.write_text(
            f"#!/bin/sh\nprintf '%s\\n' \"$@\" > '{log_path}'\nexit {exit_code}\n",
            encoding="utf-8",
        )
        docker_path.chmod(0o755)
        return docker_path, log_path

    def test_run_scenarios_linux_docker_executes_when_not_dry_run(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-docker-") as temp:
            docker_bin, log_path = self.write_fake_docker(Path(temp))
            result = self.run_script_json(
                "run_scenarios.py",
                "--platform",
                "linux",
                "--sandbox",
                "docker",
                "--docker-bin",
                str(docker_bin),
                "--json",
            )
            docker_args = log_path.read_text(encoding="utf-8").splitlines()

        self.assertEqual(result["status"], "PASS")
        self.assertTrue(result["docker"]["executed"])
        self.assertEqual(result["docker"]["exit_code"], 0)
        self.assertEqual(result["docker"]["platform"], "linux/arm64")
        self.assertEqual(result["docker"]["image"], "python:3.14-alpine")
        self.assertIn("run", docker_args)
        self.assertIn("--rm", docker_args)
        self.assertIn("--platform", docker_args)
        self.assertIn("linux/arm64", docker_args)
        self.assertIn("python:3.14-alpine", docker_args)

    def test_run_scenarios_linux_docker_failure_reports_partial(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-docker-") as temp:
            docker_bin, _ = self.write_fake_docker(Path(temp), exit_code=42)
            completed = self.run_script(
                "run_scenarios.py",
                "--platform",
                "linux",
                "--sandbox",
                "docker",
                "--docker-bin",
                str(docker_bin),
                "--json",
            )

        self.assertNotEqual(completed.returncode, 0)
        result = json.loads(completed.stdout)
        self.assertEqual(result["status"], "PARTIAL")
        self.assertEqual(result["error"], "docker scenario failed")
        self.assertTrue(result["docker"]["executed"])
        self.assertEqual(result["docker"]["exit_code"], 42)
        self.assertFalse(result["real_home_touched"])


if __name__ == "__main__":
    unittest.main()
