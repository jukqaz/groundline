import ast
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


PACK_ROOT = Path(__file__).resolve().parents[1]
SCRIPTS_DIR = PACK_ROOT / "scripts"


class GroundLineScriptContractTests(unittest.TestCase):
    def run_script(self, script_name: str, *args: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        script_path = SCRIPTS_DIR / script_name
        self.assertTrue(script_path.is_file(), f"missing script: scripts/{script_name}")
        command = [sys.executable, str(script_path), *args]
        completed = subprocess.run(
            command,
            cwd=PACK_ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            env=env,
        )
        return completed

    def run_script_json(self, script_name: str, *args: str) -> dict:
        completed = self.run_script(script_name, *args)
        self.assertEqual(
            completed.returncode,
            0,
            f"{script_name} failed\nstdout:\n{completed.stdout}\nstderr:\n{completed.stderr}",
        )
        try:
            return json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            self.fail(f"{script_name} did not emit JSON: {exc}\nstdout:\n{completed.stdout}")

    def make_fake_home(self, root: Path) -> Path:
        home = root / "home"
        (home / ".codex").mkdir(parents=True)
        (home / ".claude").mkdir(parents=True)
        (home / ".gemini/config/plugins/groundline").mkdir(parents=True)
        return home

    def write_fake_executable(self, bin_dir: Path, name: str, output: str, exit_code: int = 0) -> Path:
        path = bin_dir / name
        path.write_text(
            f"#!/bin/sh\nprintf '%s\\n' '{output}'\nexit {exit_code}\n",
            encoding="utf-8",
        )
        path.chmod(0o755)
        return path

    def test_all_groundline_scripts_exist_and_are_stdlib_only(self) -> None:
        expected_scripts = [
            "check_runtime_layout.py",
            "groundline_doctor.py",
            "groundline_dogfood.py",
            "groundline_plan_update.py",
            "groundline_provider_smoke.py",
            "groundline_radar.py",
            "lint.py",
            "run_scenarios.py",
            "sync_provider_package.py",
            "validate_pack.py",
        ]
        allowed_roots = {
            "__future__",
            "argparse",
            "ast",
            "dataclasses",
            "datetime",
            "hashlib",
            "json",
            "os",
            "pathlib",
            "platform",
            "re",
            "shlex",
            "shutil",
            "subprocess",
            "sys",
            "tempfile",
            "time",
            "typing",
            "urllib",
        }

        for script_name in expected_scripts:
            with self.subTest(script=script_name):
                script_path = SCRIPTS_DIR / script_name
                self.assertTrue(script_path.is_file(), f"missing script: scripts/{script_name}")
                module = ast.parse(script_path.read_text(encoding="utf-8"))
                imports: set[str] = set()
                for node in ast.walk(module):
                    if isinstance(node, ast.Import):
                        imports.update(alias.name.split(".", 1)[0] for alias in node.names)
                    elif isinstance(node, ast.ImportFrom) and node.module:
                        imports.add(node.module.split(".", 1)[0])
                unexpected = sorted(imports - allowed_roots)
                self.assertEqual(unexpected, [])

    def test_all_scripts_expose_help(self) -> None:
        script_names = [
            "check_runtime_layout.py",
            "groundline_doctor.py",
            "groundline_dogfood.py",
            "groundline_plan_update.py",
            "groundline_provider_smoke.py",
            "groundline_radar.py",
            "lint.py",
            "run_scenarios.py",
            "sync_provider_package.py",
            "validate_pack.py",
        ]

        for script_name in script_names:
            with self.subTest(script=script_name):
                completed = self.run_script(script_name, "--help")
                self.assertEqual(completed.returncode, 0)
                self.assertIn("usage:", completed.stdout.lower())

    def test_validate_pack_emits_json_success_contract(self) -> None:
        result = self.run_script_json("validate_pack.py", "--json")

        self.assertEqual(result["name"], "groundline")
        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["mutation_performed"])
        self.assertEqual(result["supported_runtimes"], ["codex", "claude_code", "antigravity"])
        self.assertEqual(result["supported_platforms"], ["macos-arm64", "linux"])

    def test_lint_emits_json_success_without_optional_actionlint(self) -> None:
        result = self.run_script_json("lint.py", "--json")

        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["mutation_performed"])
        self.assertTrue(result["checks"]["python_ast"])
        self.assertTrue(result["checks"]["json"])
        self.assertIn(result["checks"]["actionlint"], ["skipped", "passed"])

    def test_lint_requires_actionlint_when_requested(self) -> None:
        completed = self.run_script("lint.py", "--json", "--require-actionlint", "--actionlint-bin", "/missing/actionlint")

        self.assertNotEqual(completed.returncode, 0)
        result = json.loads(completed.stdout)
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["checks"]["actionlint"], "missing")

    def test_lint_runs_actionlint_when_binary_is_available(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-actionlint-") as temp:
            bin_dir = Path(temp)
            actionlint = self.write_fake_executable(bin_dir, "actionlint", "ok")
            result = self.run_script_json(
                "lint.py",
                "--json",
                "--require-actionlint",
                "--actionlint-bin",
                str(actionlint),
            )

        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["checks"]["actionlint"], "passed")

    def test_check_runtime_layout_reports_three_supported_manifests(self) -> None:
        result = self.run_script_json("check_runtime_layout.py", "--json")

        self.assertFalse(result["mutation_performed"])
        self.assertEqual(set(result["manifests"].keys()), {"codex", "claude_code", "antigravity"})
        self.assertTrue(result["manifests"]["codex"]["present"])
        self.assertTrue(result["manifests"]["claude_code"]["present"])
        self.assertTrue(result["manifests"]["antigravity"]["present"])
        self.assertNotIn("gemini_cli", result["manifests"])

    def test_doctor_never_uses_real_home_when_home_is_explicit(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-home-") as temp:
            home = self.make_fake_home(Path(temp))
            result = self.run_script_json(
                "groundline_doctor.py",
                "--home",
                str(home),
                "--json",
                "--offline",
            )

        self.assertEqual(result["home"], str(home))
        self.assertTrue(result["fake_home_used"])
        self.assertFalse(result["real_home_touched"])
        self.assertFalse(result["mutation_performed"])

    def test_doctor_reports_standard_tools_as_optional(self) -> None:
        tools = {
            "github": {"available": True},
            "context7": {"available": False},
            "exa": {"available": False},
        }

        with tempfile.TemporaryDirectory(prefix="groundline-tools-") as temp:
            home = self.make_fake_home(Path(temp))
            tools_path = Path(temp) / "tools.json"
            tools_path.write_text(json.dumps(tools), encoding="utf-8")
            result = self.run_script_json(
                "groundline_doctor.py",
                "--home",
                str(home),
                "--tools-fixture",
                str(tools_path),
                "--json",
                "--offline",
            )

        self.assertTrue(result["tools"]["github"]["available"])
        self.assertFalse(result["tools"]["context7"]["available"])
        self.assertFalse(result["tools"]["exa"]["available"])
        self.assertEqual(result["tools"]["context7"]["requirement"], "optional")
        self.assertEqual(result["tools"]["exa"]["requirement"], "optional")
        self.assertIn("context7", result["capability_gaps"])
        self.assertIn("exa", result["capability_gaps"])
        self.assertIn("setup_recommendations", result)

    def test_doctor_can_probe_external_tools_when_explicit(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-external-tools-") as temp:
            root = Path(temp)
            home = self.make_fake_home(root)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            self.write_fake_executable(bin_dir, "git", "git version 2.45.0")
            self.write_fake_executable(bin_dir, "gh", "gh version 2.50.0")
            self.write_fake_executable(bin_dir, "docker", "Docker version 26.1.0, build abc")
            self.write_fake_executable(bin_dir, "curl", "curl 8.7.1")

            result = self.run_script_json(
                "groundline_doctor.py",
                "--home",
                str(home),
                "--bin-dir",
                str(bin_dir),
                "--probe-tools",
                "--json",
                "--offline",
            )

        external_tools = result["external_tools"]
        self.assertEqual(result["network"], "disabled")
        self.assertFalse(result["mutation_performed"])
        self.assertTrue(external_tools["git"]["available"])
        self.assertTrue(external_tools["github_cli"]["available"])
        self.assertTrue(external_tools["docker"]["available"])
        self.assertTrue(external_tools["curl"]["available"])
        self.assertEqual(external_tools["git"]["command"], ["git", "--version"])
        self.assertEqual(external_tools["github_cli"]["command"], ["gh", "--version"])
        self.assertEqual(external_tools["git"]["resolved_path"], str(bin_dir / "git"))
        self.assertEqual(external_tools["github_cli"]["version"], "gh version 2.50.0")

    def test_doctor_redacts_secret_like_external_tool_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-external-secret-") as temp:
            root = Path(temp)
            home = self.make_fake_home(root)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            self.write_fake_executable(bin_dir, "gh", "sk-test-secret-value")

            result = self.run_script_json(
                "groundline_doctor.py",
                "--home",
                str(home),
                "--bin-dir",
                str(bin_dir),
                "--probe-tools",
                "--json",
                "--offline",
            )

        serialized = json.dumps(result)
        self.assertNotIn("sk-test-secret-value", serialized)
        self.assertTrue(result["external_tools"]["github_cli"]["redacted"])
        self.assertEqual(result["external_tools"]["github_cli"]["version"], "[redacted]")

    def test_radar_requires_explicit_network_flag_for_remote_sources(self) -> None:
        registry = {
            "sources": [
                {
                    "id": "codex-docs",
                    "kind": "remote",
                    "url": "https://openai.com/academy/codex-plugins-and-skills/",
                    "last_seen_version": "old",
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

        self.assertEqual(result["network"], "disabled")
        self.assertEqual(result["skipped_sources"][0]["id"], "codex-docs")
        self.assertEqual(result["skipped_sources"][0]["reason"], "network disabled")

    def test_radar_defaults_to_offline_without_network_flag(self) -> None:
        registry = {
            "sources": [
                {
                    "id": "claude-code-docs",
                    "kind": "remote",
                    "url": "https://code.claude.com/docs/en/plugins",
                    "last_seen_version": "old",
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
            )

        self.assertEqual(result["network"], "disabled")
        self.assertEqual(result["skipped_sources"][0]["reason"], "network disabled")

    def test_radar_network_includes_remote_sources_as_research_seeds(self) -> None:
        registry = {
            "sources": [
                {
                    "id": "spec-kit",
                    "kind": "remote",
                    "url": "https://github.com/github/spec-kit",
                    "owner": "GitHub",
                },
                {
                    "id": "agent-os",
                    "kind": "remote",
                    "url": "https://github.com/buildermethods/agent-os",
                    "owner": "Builder Methods",
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
                "--network",
            )

        self.assertEqual(result["network"], "enabled")
        self.assertEqual(result["skipped_sources"], [])
        self.assertEqual(result["research_packet"]["mode"], "ecosystem-scan")
        self.assertEqual(result["research_packet"]["sources"], ["spec-kit", "agent-os"])
        self.assertEqual(result["upgrade_task_candidates"], [])

    def test_radar_runs_local_command_sources_without_network(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-radar-command-") as temp:
            root = Path(temp)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            version_tool = self.write_fake_executable(bin_dir, "source-version", "v2")
            registry = {
                "sources": [
                    {
                        "id": "codex-release-feed",
                        "owner": "Codex",
                        "kind": "command",
                        "command": [str(version_tool)],
                        "last_seen_version": "v1",
                        "url": "local://codex-release-feed",
                    }
                ]
            }
            registry_path = root / "source-registry.json"
            registry_path.write_text(json.dumps(registry), encoding="utf-8")

            result = self.run_script_json(
                "groundline_radar.py",
                "--registry",
                str(registry_path),
                "--command-sources",
                "--json",
                "--offline",
            )

        self.assertEqual(result["network"], "disabled")
        self.assertEqual(result["changed_sources"][0]["id"], "codex-release-feed")
        self.assertEqual(result["changed_sources"][0]["old_version"], "v1")
        self.assertEqual(result["changed_sources"][0]["new_version"], "v2")
        self.assertEqual(result["skipped_sources"], [])

    def test_radar_rejects_shell_string_command_sources(self) -> None:
        registry = {
            "sources": [
                {
                    "id": "unsafe-command",
                    "owner": "Local",
                    "kind": "command",
                    "command": "echo v2",
                    "last_seen_version": "v1",
                }
            ]
        }

        with tempfile.TemporaryDirectory(prefix="groundline-radar-command-") as temp:
            registry_path = Path(temp) / "source-registry.json"
            registry_path.write_text(json.dumps(registry), encoding="utf-8")
            result = self.run_script_json(
                "groundline_radar.py",
                "--registry",
                str(registry_path),
                "--command-sources",
                "--json",
                "--offline",
            )

        self.assertEqual(result["changed_sources"], [])
        self.assertEqual(result["skipped_sources"][0]["id"], "unsafe-command")
        self.assertEqual(result["skipped_sources"][0]["reason"], "command must be a list")

    def test_radar_skips_command_sources_without_explicit_flag(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-radar-command-") as temp:
            root = Path(temp)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            version_tool = self.write_fake_executable(bin_dir, "source-version", "v2")
            registry = {
                "sources": [
                    {
                        "id": "disabled-command",
                        "owner": "Local",
                        "kind": "command",
                        "command": [str(version_tool)],
                        "last_seen_version": "v1",
                    }
                ]
            }
            registry_path = root / "source-registry.json"
            registry_path.write_text(json.dumps(registry), encoding="utf-8")

            result = self.run_script_json(
                "groundline_radar.py",
                "--registry",
                str(registry_path),
                "--json",
                "--offline",
            )

        self.assertEqual(result["changed_sources"], [])
        self.assertEqual(result["skipped_sources"][0]["id"], "disabled-command")
        self.assertEqual(result["skipped_sources"][0]["reason"], "command sources disabled")

    def test_run_scenarios_rejects_unsupported_platform(self) -> None:
        completed = self.run_script(
            "run_scenarios.py",
            "--platform",
            "windows",
            "--sandbox",
            "local",
            "--json",
        )

        self.assertNotEqual(completed.returncode, 0)
        payload = json.loads(completed.stdout)
        self.assertEqual(payload["status"], "FAIL")
        self.assertEqual(payload["error"], "unsupported platform")

    def test_run_scenarios_rejects_unsupported_sandbox_pair(self) -> None:
        completed = self.run_script(
            "run_scenarios.py",
            "--platform",
            "macos",
            "--sandbox",
            "docker",
            "--json",
        )

        self.assertNotEqual(completed.returncode, 0)
        payload = json.loads(completed.stdout)
        self.assertEqual(payload["status"], "FAIL")
        self.assertEqual(payload["error"], "unsupported sandbox for platform")

    def test_scripts_do_not_print_secret_like_values(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-secret-") as temp:
            home = self.make_fake_home(Path(temp))
            secret_file = home / ".codex/auth.json"
            secret_file.write_text('{"token":"sk-test-secret-value"}', encoding="utf-8")
            result = self.run_script(
                "groundline_doctor.py",
                "--home",
                str(home),
                "--json",
                "--offline",
            )

        combined_output = result.stdout + result.stderr
        self.assertNotIn("sk-test-secret-value", combined_output)
        self.assertNotIn("token", combined_output.lower())

    def test_provider_smoke_reports_manifest_and_target_plan_without_mutation(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-smoke-") as temp:
            home = self.make_fake_home(Path(temp))
            result = self.run_script_json(
                "groundline_provider_smoke.py",
                "--home",
                str(home),
                "--json",
            )

        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["mutation_performed"])
        self.assertTrue(result["fake_home_used"])
        self.assertIn("source_package", result)
        self.assertGreaterEqual(result["source_package"]["skill_count"], 12)
        self.assertTrue(result["source_package"]["skill_index_present"])
        self.assertTrue(result["source_package"]["skill_index_consistent"])
        self.assertTrue(result["source_package"]["human_portfolio_present"])
        self.assertEqual(set(result["providers"].keys()), {"codex", "claude_code", "antigravity"})
        self.assertTrue(result["providers"]["codex"]["manifest_present"])
        self.assertIn("install_target", result["providers"]["codex"])
        self.assertIn("runtime_probe", result["providers"]["codex"])
        self.assertFalse(result["providers"]["codex"]["runtime_probe"]["target_exists"])
        self.assertIn("update_command", result)

    def test_provider_smoke_detects_staged_install_target_contents(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-installed-") as temp:
            home = self.make_fake_home(Path(temp))
            target = home / ".codex/plugins/groundline"
            target.mkdir(parents=True)
            shutil.copytree(PACK_ROOT / ".codex-plugin", target / ".codex-plugin")
            shutil.copytree(PACK_ROOT / "skills", target / "skills")

            result = self.run_script_json(
                "groundline_provider_smoke.py",
                "--home",
                str(home),
                "--json",
            )

        probe = result["providers"]["codex"]["runtime_probe"]
        self.assertTrue(probe["target_exists"])
        self.assertTrue(probe["target_manifest_present"])
        self.assertTrue(probe["target_skills_present"])
        self.assertEqual(probe["target_skill_count"], result["source_package"]["skill_count"])
        self.assertTrue(probe["target_skill_count_matches_source"])

    def test_dogfood_harness_runs_staged_provider_suite(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-dogfood-") as temp:
            root = Path(temp)
            home = self.make_fake_home(root)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            self.write_fake_executable(bin_dir, "codex", "codex-cli 0.134.0")
            self.write_fake_executable(bin_dir, "claude", "2.1.152 (Claude Code)")
            self.write_fake_executable(bin_dir, "agy", "1.0.2")

            result = self.run_script_json(
                "groundline_dogfood.py",
                "--home",
                str(home),
                "--bin-dir",
                str(bin_dir),
                "--stage-package",
                "--probe-runtimes",
                "--json",
            )

        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["suite"], "provider-dogfood")
        self.assertEqual(result["scenario_count"], 3)
        self.assertTrue(result["stage_package"])
        self.assertTrue(result["fake_home_used"])
        self.assertFalse(result["real_home_touched"])
        self.assertFalse(result["mutation_performed"])
        self.assertEqual(set(result["providers"].keys()), {"codex", "claude_code", "antigravity"})
        for provider in result["providers"].values():
            with self.subTest(provider=provider["name"]):
                self.assertEqual(provider["status"], "PASS")
                self.assertTrue(provider["runtime"]["available"])
                self.assertTrue(provider["install"]["target_exists"])
                self.assertTrue(provider["install"]["target_manifest_present"])
                self.assertTrue(provider["install"]["target_skills_present"])
                self.assertEqual(provider["install"]["target_skill_count"], result["source_package"]["skill_count"])
                self.assertTrue(provider["install"]["target_skill_count_matches_source"])
                self.assertEqual(len(provider["scenario_results"]), 3)
                self.assertTrue(all(item["status"] == "PASS" for item in provider["scenario_results"]))

    def test_dogfood_harness_refuses_real_home_staging(self) -> None:
        completed = self.run_script(
            "groundline_dogfood.py",
            "--home",
            str(Path.home()),
            "--stage-package",
            "--json",
        )

        self.assertNotEqual(completed.returncode, 0)
        result = json.loads(completed.stdout)
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["suite"], "provider-dogfood")
        self.assertEqual(result["home"], "~")
        self.assertFalse(result["real_home_touched"])
        self.assertFalse(result["mutation_performed"])
        self.assertIn("refusing to stage package", result["error"])


if __name__ == "__main__":
    unittest.main()
