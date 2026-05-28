import ast
import importlib.util
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
    def run_script(
        self,
        script_name: str,
        *args: str,
        env: dict[str, str] | None = None,
        pack_root: Path | None = None,
    ) -> subprocess.CompletedProcess[str]:
        root = pack_root or PACK_ROOT
        script_path = root / "scripts" / script_name
        self.assertTrue(script_path.is_file(), f"missing script: scripts/{script_name}")
        command = [sys.executable, str(script_path), *args]
        completed = subprocess.run(
            command,
            cwd=root,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            env=env,
        )
        return completed

    def run_script_json(self, script_name: str, *args: str, pack_root: Path | None = None) -> dict:
        completed = self.run_script(script_name, *args, pack_root=pack_root)
        self.assertEqual(
            completed.returncode,
            0,
            f"{script_name} failed\nstdout:\n{completed.stdout}\nstderr:\n{completed.stderr}",
        )
        try:
            return json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            self.fail(f"{script_name} did not emit JSON: {exc}\nstdout:\n{completed.stdout}")

    def copy_pack(self, root: Path) -> Path:
        target = root / "pack"
        shutil.copytree(
            PACK_ROOT,
            target,
            ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc"),
        )
        return target

    def make_fake_home(self, root: Path) -> Path:
        home = root / "home"
        (home / ".codex").mkdir(parents=True)
        (home / ".claude").mkdir(parents=True)
        (home / ".gemini/config/plugins").mkdir(parents=True)
        return home

    def copy_provider_payload(self, source_root: Path, target: Path) -> None:
        payload_root = source_root / "plugins/groundline"
        self.assertTrue(payload_root.is_dir(), f"missing provider payload: {payload_root}")
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(payload_root, target)

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
            "groundline_provider_validate.py",
            "groundline_radar.py",
            "groundline_release_gate.py",
            "groundline_safety_eval.py",
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
            "groundline_provider_validate.py",
            "groundline_radar.py",
            "groundline_release_gate.py",
            "groundline_safety_eval.py",
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

    def test_release_gate_plan_lists_release_checks_without_running_them(self) -> None:
        result = self.run_script_json("groundline_release_gate.py", "--plan", "--json")

        self.assertEqual(result["suite"], "release-gate")
        self.assertEqual(result["mode"], "plan")
        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["mutation_performed"])
        self.assertFalse(result["publishing_performed"])
        self.assertFalse(result["real_home_touched"])
        self.assertNotIn(str(Path.home()), json.dumps(result))
        gate_ids = [gate["id"] for gate in result["gates"]]
        self.assertEqual(
            gate_ids,
            [
                "source-validation",
                "packaged-validation",
                "lint",
                "runtime-layout",
                "provider-native-validation",
                "unit-tests",
                "offline-doctor",
                "offline-radar",
                "safety-eval",
                "provider-smoke",
                "staged-dogfood",
                "macos-local-scenario",
                "linux-docker-dry-run",
            ],
        )
        self.assertNotIn("linux-docker-execution", gate_ids)
        for gate in result["gates"]:
            with self.subTest(gate=gate["id"]):
                self.assertFalse(gate["executed"])
                self.assertIsInstance(gate["command"], list)
                self.assertEqual(gate["command"][0], "python3")
                self.assertNotIn("git", gate["command"][0])

    def test_release_gate_can_plan_full_local_release_gate(self) -> None:
        result = self.run_script_json(
            "groundline_release_gate.py",
            "--plan",
            "--json",
            "--include-docker-execution",
            "--actionlint-bin",
            "/tmp/actionlint",
        )

        gate_ids = [gate["id"] for gate in result["gates"]]
        self.assertIn("linux-docker-execution", gate_ids)
        self.assertIn("provider-native-validation", gate_ids)
        self.assertIn('git tag "$TAG"', result["approval_required_commands_excluded"])
        self.assertIn('git push origin "$TAG"', result["approval_required_commands_excluded"])
        self.assertIn('gh release create "$TAG"', result["approval_required_commands_excluded"])
        lint_gate = next(gate for gate in result["gates"] if gate["id"] == "lint")
        self.assertIn("--actionlint-bin", lint_gate["command"])
        self.assertIn("/tmp/actionlint", lint_gate["command"])

    def test_provider_native_validate_runs_fake_validators(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-native-") as temp:
            bin_dir = Path(temp) / "bin"
            bin_dir.mkdir()
            self.write_fake_executable(bin_dir, "claude", "claude validation ok")
            self.write_fake_executable(bin_dir, "agy", "agy validation ok")
            env = os.environ.copy()
            env["PATH"] = str(bin_dir)

            completed = self.run_script("groundline_provider_validate.py", "--json", env=env)

        self.assertEqual(completed.returncode, 0, completed.stderr)
        result = json.loads(completed.stdout)
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["package_mode"], "source")
        self.assertFalse(result["mutation_performed"])
        self.assertFalse(result["real_home_touched"])
        self.assertFalse(result["secret_value_printed"])
        self.assertEqual(result["next_actions"], [])
        self.assertNotIn(str(Path.home()), completed.stdout)
        self.assertEqual([item["status"] for item in result["validations"]], ["PASS", "PASS", "PASS"])
        self.assertEqual(result["validations"][0]["command"], ["claude", "plugin", "validate", "plugins/groundline", "--strict"])

    def test_provider_native_validate_reports_missing_cli_as_partial(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-native-missing-") as temp:
            env = os.environ.copy()
            env["PATH"] = temp

            completed = self.run_script("groundline_provider_validate.py", "--json", env=env)

        self.assertEqual(completed.returncode, 2)
        result = json.loads(completed.stdout)
        self.assertEqual(result["status"], "PARTIAL")
        self.assertFalse(result["mutation_performed"])
        self.assertTrue(result["next_actions"])
        self.assertTrue(any("install or expose claude" in action for action in result["next_actions"]))
        self.assertTrue(any("install or expose agy" in action for action in result["next_actions"]))

    def test_release_gate_treats_exit_two_as_partial(self) -> None:
        script_path = SCRIPTS_DIR / "groundline_release_gate.py"
        spec = importlib.util.spec_from_file_location("groundline_release_gate", script_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        sys.modules["groundline_release_gate"] = module
        spec.loader.exec_module(module)

        with tempfile.TemporaryDirectory(prefix="groundline-release-partial-") as temp:
            temp_root = Path(temp)
            partial_script = temp_root / "partial.py"
            partial_script.write_text("raise SystemExit(2)\n", encoding="utf-8")
            gate = module.Gate("provider-smoke", "Provider smoke partial", [sys.executable, str(partial_script)], temp_root)
            result = module.run_gate(gate, timeout=5)

        self.assertEqual(result["status"], "PARTIAL")
        self.assertEqual(result["exit_code"], 2)
        self.assertEqual(module.aggregate_status([result]), "PARTIAL")

    def test_release_gate_preserves_json_summary_for_partial_gates(self) -> None:
        script_path = SCRIPTS_DIR / "groundline_release_gate.py"
        spec = importlib.util.spec_from_file_location("groundline_release_gate_summary", script_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        sys.modules["groundline_release_gate_summary"] = module
        spec.loader.exec_module(module)

        with tempfile.TemporaryDirectory(prefix="groundline-release-summary-") as temp:
            temp_root = Path(temp)
            partial_script = temp_root / "partial_json.py"
            payload = {
                "status": "PARTIAL",
                "install_doctor_status": "PARTIAL",
                "next_actions": ["refresh the Codex provider install"],
                "install_issues": [{"provider": "codex", "issues": ["content_fingerprint_mismatch"]}],
                "mutation_performed": False,
                "real_home_touched": False,
                "large_payload": ["x" * 200 for _ in range(80)],
            }
            partial_script.write_text(
                "import json\n"
                f"print(json.dumps({payload!r}))\n"
                "raise SystemExit(2)\n",
                encoding="utf-8",
            )
            gate = module.Gate("provider-smoke", "Provider smoke partial", [sys.executable, str(partial_script)], temp_root)
            result = module.run_gate(gate, timeout=5)

        self.assertEqual(result["status"], "PARTIAL")
        self.assertEqual(result["json_summary"]["next_actions"], ["refresh the Codex provider install"])
        self.assertEqual(result["json_summary"]["install_issues"][0]["provider"], "codex")
        self.assertNotIn("large_payload", result["json_summary"])
        non_passing = module.summarize_non_passing_gates([result])
        self.assertEqual(non_passing[0]["id"], "provider-smoke")
        self.assertEqual(non_passing[0]["json_summary"]["install_doctor_status"], "PARTIAL")
        self.assertEqual(module.collect_next_actions([result]), ["refresh the Codex provider install"])

    def test_release_gate_top_level_summary_lists_partial_next_actions(self) -> None:
        script_path = SCRIPTS_DIR / "groundline_release_gate.py"
        spec = importlib.util.spec_from_file_location("groundline_release_gate_top_level", script_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        sys.modules["groundline_release_gate_top_level"] = module
        spec.loader.exec_module(module)

        partial_gate = {
            "id": "provider-smoke",
            "label": "Run provider smoke",
            "executed": True,
            "status": "PARTIAL",
            "exit_code": 2,
            "json_summary": {
                "status": "PARTIAL",
                "install_doctor_status": "PARTIAL",
                "next_actions": [
                    "refresh the Codex provider install",
                    "refresh the Claude Code provider install",
                ],
            },
        }
        fail_gate = {
            "id": "lint",
            "label": "Run lint and actionlint",
            "executed": True,
            "status": "FAIL",
            "exit_code": 1,
        }

        self.assertEqual(module.aggregate_status([partial_gate]), "PARTIAL")
        self.assertEqual(module.aggregate_status([partial_gate, fail_gate]), "FAIL")
        self.assertEqual(
            module.collect_next_actions([partial_gate, fail_gate]),
            [
                "inspect the lint gate output",
                "refresh the Claude Code provider install",
                "refresh the Codex provider install",
            ],
        )
        summaries = module.summarize_non_passing_gates([partial_gate, fail_gate])
        self.assertEqual([item["id"] for item in summaries], ["provider-smoke", "lint"])
        self.assertEqual(summaries[0]["json_summary"]["install_doctor_status"], "PARTIAL")

    def test_validate_pack_emits_json_success_contract(self) -> None:
        result = self.run_script_json("validate_pack.py", "--json")

        self.assertEqual(result["name"], "groundline")
        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["mutation_performed"])
        self.assertEqual(result["supported_runtimes"], ["codex", "claude_code", "antigravity"])
        self.assertEqual(result["supported_platforms"], ["macos-arm64", "linux"])

    def test_validate_pack_ignores_empty_packaged_conflict_copy_directories(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-pack-conflict-") as temp:
            pack_root = self.copy_pack(Path(temp))
            conflict_dir = pack_root / "plugins/groundline/docs 9"
            conflict_dir.mkdir(parents=True)

            completed = self.run_script("validate_pack.py", "--json", pack_root=pack_root)
            result = json.loads(completed.stdout)

        self.assertEqual(completed.returncode, 0)
        self.assertEqual(result["status"], "PASS")

    def test_validate_pack_rejects_packaged_conflict_copy_payload(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-pack-conflict-") as temp:
            pack_root = self.copy_pack(Path(temp))
            conflict_dir = pack_root / "plugins/groundline/docs 9"
            conflict_dir.mkdir(parents=True)
            (conflict_dir / "payload.md").write_text("unexpected payload\n", encoding="utf-8")

            completed = self.run_script("validate_pack.py", "--json", pack_root=pack_root)
            result = json.loads(completed.stdout)

        self.assertNotEqual(completed.returncode, 0)
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("packaged conflict copy must be removed: plugins/groundline/docs 9", result["errors"])

    def test_validate_pack_rejects_source_only_packaged_superpowers_docs(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-pack-source-only-") as temp:
            pack_root = self.copy_pack(Path(temp))
            stale_source_package_plan = pack_root / "plugins/groundline/docs/superpowers/stale.md"
            stale_source_package_plan.parent.mkdir(parents=True)
            stale_source_package_plan.write_text("stale packaged source-only plan\n", encoding="utf-8")

            source_completed = self.run_script("validate_pack.py", "--json", pack_root=pack_root)
            source_result = json.loads(source_completed.stdout)
            package_completed = self.run_script("validate_pack.py", "--json", pack_root=pack_root / "plugins/groundline")
            package_result = json.loads(package_completed.stdout)

        self.assertNotEqual(source_completed.returncode, 0)
        self.assertEqual(source_result["status"], "FAIL")
        self.assertIn(
            "source-only package path must be excluded: plugins/groundline/docs/superpowers",
            source_result["errors"],
        )
        self.assertNotEqual(package_completed.returncode, 0)
        self.assertEqual(package_result["status"], "FAIL")
        self.assertIn("source-only package path must be excluded: docs/superpowers", package_result["errors"])

    def test_validate_pack_rejects_stale_packaged_file_content(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-pack-drift-") as temp:
            pack_root = self.copy_pack(Path(temp))
            packaged_doc = pack_root / "plugins/groundline/docs/provider-packaging.md"
            packaged_doc.write_text(packaged_doc.read_text(encoding="utf-8") + "\nstale packaged edit\n", encoding="utf-8")

            completed = self.run_script("validate_pack.py", "--json", pack_root=pack_root)
            result = json.loads(completed.stdout)

        self.assertNotEqual(completed.returncode, 0)
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("packaged file drift: plugins/groundline/docs/provider-packaging.md", result["errors"])

    def test_validate_pack_rejects_extra_packaged_file(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-pack-extra-") as temp:
            pack_root = self.copy_pack(Path(temp))
            extra_doc = pack_root / "plugins/groundline/docs/extra-packaged.md"
            extra_doc.write_text("unexpected packaged file\n", encoding="utf-8")

            completed = self.run_script("validate_pack.py", "--json", pack_root=pack_root)
            result = json.loads(completed.stdout)

        self.assertNotEqual(completed.returncode, 0)
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("packaged extra file: plugins/groundline/docs/extra-packaged.md", result["errors"])

    def test_sync_provider_package_removes_conflict_copy_directories(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-pack-sync-") as temp:
            pack_root = self.copy_pack(Path(temp))
            conflict_dir = pack_root / "plugins/groundline/docs 9"
            conflict_dir.mkdir(parents=True)

            result = self.run_script_json("sync_provider_package.py", "--json", pack_root=pack_root)
            conflict_exists = conflict_dir.exists()

        self.assertEqual(result["status"], "PASS")
        self.assertFalse(conflict_exists)
        self.assertEqual(result["remaining_conflict_copies"], [])

    def test_sync_provider_package_excludes_source_only_superpowers_docs(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-pack-sync-") as temp:
            pack_root = self.copy_pack(Path(temp))
            stale_packaged_plan = pack_root / "plugins/groundline/docs/superpowers/stale.md"
            stale_packaged_plan.parent.mkdir(parents=True)
            stale_packaged_plan.write_text("stale packaged plan\n", encoding="utf-8")

            result = self.run_script_json("sync_provider_package.py", "--json", pack_root=pack_root)
            source_docs_exist = (pack_root / "docs/superpowers").is_dir()
            packaged_docs_exist = (pack_root / "plugins/groundline/docs/superpowers").exists()

        self.assertEqual(result["status"], "PASS")
        self.assertTrue(source_docs_exist)
        self.assertFalse(packaged_docs_exist)

    def test_sync_provider_package_rejects_packaged_script_execution(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-pack-sync-packaged-") as temp:
            pack_root = self.copy_pack(Path(temp))
            packaged_root = pack_root / "plugins/groundline"
            script_path = packaged_root / "scripts/sync_provider_package.py"

            completed = subprocess.run(
                [sys.executable, str(script_path), "--json"],
                cwd=packaged_root,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            result = json.loads(completed.stdout)
            nested_package_exists = (packaged_root / "plugins/groundline").exists()

        self.assertNotEqual(completed.returncode, 0)
        self.assertEqual(result["status"], "FAIL")
        self.assertFalse(result["mutation_performed"])
        self.assertIn("source root", result["errors"][0])
        self.assertFalse(nested_package_exists)

    def test_safety_eval_emits_json_success_contract(self) -> None:
        result = self.run_script_json("groundline_safety_eval.py", "--json")

        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["mutation_performed"])
        self.assertEqual(result["case_count"], 4)
        self.assertEqual(result["cases"], [])

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
        self.assertFalse(result["secret_value_printed"])
        self.assertTrue(result["fake_home_used"])
        self.assertEqual(result["next_actions"], [])
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
            self.copy_provider_payload(PACK_ROOT, target)

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
        self.assertTrue(probe["content_matches_source"])
        self.assertEqual(probe["target_content_fingerprint"], result["source_package"]["content_fingerprint"])
        self.assertEqual(result["providers"]["codex"]["recommended_actions"], ["no action required"])

    def test_provider_smoke_reports_installed_versions_and_drift(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-drift-") as temp:
            root = Path(temp)
            home = self.make_fake_home(root)
            target = home / ".codex/plugins/groundline"
            self.copy_provider_payload(PACK_ROOT, target)
            target_manifest = target / ".codex-plugin/plugin.json"
            data = json.loads(target_manifest.read_text(encoding="utf-8"))
            data["version"] = "0.0.1"
            target_manifest.write_text(json.dumps(data), encoding="utf-8")

            completed = self.run_script(
                "groundline_provider_smoke.py",
                "--home",
                str(home),
                "--json",
            )
            result = json.loads(completed.stdout)

        self.assertEqual(completed.returncode, 2)
        probe = result["providers"]["codex"]["runtime_probe"]
        self.assertEqual(result["install_doctor_status"], "PARTIAL")
        self.assertEqual(result["status"], "PARTIAL")
        self.assertFalse(result["secret_value_printed"])
        self.assertEqual(probe["source_version"], result["source_package"]["version"])
        self.assertEqual(probe["installed_version"], "0.0.1")
        self.assertFalse(probe["version_matches_source"])
        self.assertIn("version_mismatch", probe["issues"])
        self.assertNotIn("content_fingerprint_mismatch", probe["issues"])

    def test_provider_smoke_detects_codex_and_claude_cache_installs(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-cache-") as temp:
            home = self.make_fake_home(Path(temp))
            version = json.loads((PACK_ROOT / "plugin.json").read_text(encoding="utf-8"))["version"]
            codex_target = home / ".codex/plugins/cache/groundline/groundline" / version
            claude_target = home / ".claude/plugins/cache/groundline/groundline" / version
            self.copy_provider_payload(PACK_ROOT, codex_target)
            self.copy_provider_payload(PACK_ROOT, claude_target)

            result = self.run_script_json(
                "groundline_provider_smoke.py",
                "--home",
                str(home),
                "--json",
            )

        self.assertEqual(result["install_doctor_status"], "PASS")
        for provider_name in ["codex", "claude_code"]:
            with self.subTest(provider=provider_name):
                provider = result["providers"][provider_name]
                probe = provider["runtime_probe"]
                self.assertEqual(provider["install_source"], "cache")
                self.assertTrue(probe["target_exists"])
                self.assertEqual(probe["installed_version"], result["source_package"]["version"])
                self.assertEqual(probe["version_check"], "match")
                self.assertTrue(probe["content_matches_source"])
                self.assertEqual(probe["issues"], [])

    def test_provider_smoke_detects_same_version_content_drift(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-content-drift-") as temp:
            home = self.make_fake_home(Path(temp))
            version = json.loads((PACK_ROOT / "plugin.json").read_text(encoding="utf-8"))["version"]
            codex_target = home / ".codex/plugins/cache/groundline/groundline" / version
            self.copy_provider_payload(PACK_ROOT, codex_target)
            skill_doc = codex_target / "skills/package-agent-task/SKILL.md"
            skill_doc.write_text(skill_doc.read_text(encoding="utf-8") + "\nstale local cache edit\n", encoding="utf-8")

            completed = self.run_script(
                "groundline_provider_smoke.py",
                "--home",
                str(home),
                "--json",
            )
            result = json.loads(completed.stdout)

        self.assertEqual(completed.returncode, 2)
        probe = result["providers"]["codex"]["runtime_probe"]
        self.assertEqual(result["install_doctor_status"], "PARTIAL")
        self.assertEqual(probe["version_check"], "match")
        self.assertFalse(probe["content_matches_source"])
        self.assertIn("content_fingerprint_mismatch", probe["issues"])
        self.assertIn(
            "refresh the Codex provider install; version matches but installed content differs from source",
            result["providers"]["codex"]["recommended_actions"],
        )
        self.assertIn(
            "refresh the Codex provider install; version matches but installed content differs from source",
            result["next_actions"],
        )

    def test_version_bump_sync_validates_and_smoke_reports_stale_cache(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-version-bump-") as temp:
            root = Path(temp)
            pack_root = self.copy_pack(root)
            next_version = "9.9.9"
            stale_version = "0.0.1"
            for relative in ["plugin.json", ".codex-plugin/plugin.json", ".claude-plugin/plugin.json"]:
                manifest = pack_root / relative
                data = json.loads(manifest.read_text(encoding="utf-8"))
                data["version"] = next_version
                manifest.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")

            sync = self.run_script("sync_provider_package.py", "--json", pack_root=pack_root)
            source_validate = self.run_script("validate_pack.py", "--json", pack_root=pack_root)
            package_validate = self.run_script("validate_pack.py", "--json", pack_root=pack_root / "plugins/groundline")
            manifest_versions = {}
            for relative in [
                "plugin.json",
                ".codex-plugin/plugin.json",
                ".claude-plugin/plugin.json",
                "plugins/groundline/plugin.json",
                "plugins/groundline/.codex-plugin/plugin.json",
                "plugins/groundline/.claude-plugin/plugin.json",
            ]:
                manifest_versions[relative] = json.loads((pack_root / relative).read_text(encoding="utf-8"))["version"]

            home = self.make_fake_home(root)
            codex_target = home / ".codex/plugins/cache/groundline/groundline" / stale_version
            claude_target = home / ".claude/plugins/cache/groundline/groundline" / stale_version
            self.copy_provider_payload(pack_root, codex_target)
            self.copy_provider_payload(pack_root, claude_target)
            for manifest in [codex_target / ".codex-plugin/plugin.json", claude_target / ".claude-plugin/plugin.json"]:
                data = json.loads(manifest.read_text(encoding="utf-8"))
                data["version"] = stale_version
                manifest.write_text(json.dumps(data), encoding="utf-8")

            smoke = self.run_script("groundline_provider_smoke.py", "--home", str(home), "--json", pack_root=pack_root)
            smoke_result = json.loads(smoke.stdout)

        self.assertEqual(sync.returncode, 0, sync.stdout + sync.stderr)
        self.assertEqual(source_validate.returncode, 0, source_validate.stdout + source_validate.stderr)
        self.assertEqual(package_validate.returncode, 0, package_validate.stdout + package_validate.stderr)
        for relative, version in manifest_versions.items():
            with self.subTest(manifest=relative):
                self.assertEqual(version, next_version)

        self.assertEqual(smoke.returncode, 2)
        self.assertEqual(smoke_result["status"], "PARTIAL")
        self.assertEqual(smoke_result["source_package"]["version"], next_version)
        for provider_name in ["codex", "claude_code"]:
            with self.subTest(provider=provider_name):
                probe = smoke_result["providers"][provider_name]["runtime_probe"]
                self.assertEqual(probe["installed_version"], stale_version)
                self.assertEqual(probe["source_version"], next_version)
                self.assertIn("stale_cache_version", probe["issues"])

    def test_provider_smoke_reports_missing_payload_and_antigravity_target(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-payload-") as temp:
            home = self.make_fake_home(Path(temp))
            codex_target = home / ".codex/plugins/groundline"
            codex_target.mkdir(parents=True)
            shutil.copytree(PACK_ROOT / ".codex-plugin", codex_target / ".codex-plugin")

            antigravity_target = home / ".gemini/config/plugins/groundline"
            self.copy_provider_payload(PACK_ROOT, antigravity_target)

            completed = self.run_script(
                "groundline_provider_smoke.py",
                "--home",
                str(home),
                "--json",
            )
            result = json.loads(completed.stdout)

        self.assertEqual(completed.returncode, 2)
        codex_probe = result["providers"]["codex"]["runtime_probe"]
        antigravity_probe = result["providers"]["antigravity"]["runtime_probe"]
        self.assertEqual(result["install_doctor_status"], "PARTIAL")
        self.assertFalse(result["secret_value_printed"])
        self.assertIn("missing_skills_payload", codex_probe["issues"])
        self.assertIn(
            "reinstall the Codex provider payload from the packaged plugin",
            result["providers"]["codex"]["recommended_actions"],
        )
        self.assertTrue(antigravity_probe["target_exists"])
        self.assertTrue(antigravity_probe["target_manifest_present"])
        self.assertTrue(antigravity_probe["version_matches_source"])
        self.assertTrue(antigravity_probe["content_matches_source"])

    def test_provider_smoke_missing_source_manifest_is_fail_exit_one(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-provider-missing-manifest-") as temp:
            pack_root = self.copy_pack(Path(temp))
            (pack_root / ".codex-plugin/plugin.json").unlink()
            home = self.make_fake_home(Path(temp))

            completed = self.run_script(
                "groundline_provider_smoke.py",
                "--home",
                str(home),
                "--json",
                pack_root=pack_root,
            )
            result = json.loads(completed.stdout)

        self.assertEqual(completed.returncode, 1)
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("codex", result["missing_manifests"])
        self.assertIn("restore the source manifest for Codex before publishing or installing", result["next_actions"])

    def test_provider_smoke_allows_antigravity_shape_without_installed_version(self) -> None:
        with tempfile.TemporaryDirectory(prefix="groundline-antigravity-shape-") as temp:
            home = self.make_fake_home(Path(temp))
            target = home / ".gemini/config/plugins/groundline"
            self.copy_provider_payload(PACK_ROOT, target)
            target_manifest = json.loads((PACK_ROOT / "plugin.json").read_text(encoding="utf-8"))
            target_manifest.pop("version", None)
            (target / "plugin.json").write_text(json.dumps(target_manifest), encoding="utf-8")

            result = self.run_script_json(
                "groundline_provider_smoke.py",
                "--home",
                str(home),
                "--json",
            )

        probe = result["providers"]["antigravity"]["runtime_probe"]
        self.assertEqual(result["install_doctor_status"], "PASS")
        self.assertEqual(probe["status"], "PASS")
        self.assertIsNone(probe["installed_version"])
        self.assertEqual(probe["version_check"], "unavailable")
        self.assertTrue(probe["content_matches_source"])
        self.assertEqual(probe["issues"], [])
        self.assertEqual(result["providers"]["antigravity"]["candidate_versions"], [])

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
        self.assertEqual(result["scenario_count"], 6)
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
                self.assertEqual(len(provider["scenario_results"]), 6)
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
