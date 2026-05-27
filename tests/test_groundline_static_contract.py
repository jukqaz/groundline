import json
import re
import unittest
from pathlib import Path


PACK_ROOT = Path(__file__).resolve().parents[1]


class GroundLineStaticContractTests(unittest.TestCase):
    def test_manifest_names_are_groundline(self) -> None:
        manifest_paths = [
            PACK_ROOT / ".codex-plugin/plugin.json",
            PACK_ROOT / ".claude-plugin/plugin.json",
            PACK_ROOT / "plugin.json",
        ]

        for manifest_path in manifest_paths:
            with self.subTest(manifest=manifest_path.relative_to(PACK_ROOT)):
                data = json.loads(manifest_path.read_text(encoding="utf-8"))
                self.assertEqual(data.get("name"), "groundline")

        codex = json.loads((PACK_ROOT / ".codex-plugin/plugin.json").read_text(encoding="utf-8"))
        interface = codex.get("interface", {})
        self.assertEqual(interface.get("displayName"), "GroundLine")
        self.assertIn("control plane", interface.get("longDescription", "").lower())

    def test_required_groundline_files_exist(self) -> None:
        required_files = [
            ".codex-plugin/plugin.json",
            ".claude-plugin/plugin.json",
            "plugin.json",
            "CHANGELOG.md",
            "README.md",
            ".github/workflows/test.yml",
            ".github/workflows/radar.yml",
            "docs/install.md",
            "docs/update.md",
            "docs/provider-smoke.md",
            "docs/runtime-support.md",
            "docs/examples.md",
            "docs/release-checklist.md",
            "references/capability-blueprint.md",
            "references/config-sync-boundary.md",
            "references/output-contracts.md",
            "references/platform-support.md",
            "references/runtime-matrix.md",
            "references/source-registry.json",
            "references/superpowers-interop.md",
            "references/tool-profiles.md",
            "references/workflow-modes.md",
            "scripts/check_runtime_layout.py",
            "scripts/groundline_doctor.py",
            "scripts/groundline_plan_update.py",
            "scripts/groundline_provider_smoke.py",
            "scripts/groundline_radar.py",
            "scripts/run_scenarios.py",
            "scripts/validate_pack.py",
            "scenarios/fixtures/fresh-install.json",
            "scenarios/expected/standalone-groundline.json",
        ]

        missing = [path for path in required_files if not (PACK_ROOT / path).is_file()]
        self.assertEqual(missing, [])

    def test_release_docs_cover_ci_external_probes_and_clean_python_runs(self) -> None:
        readme = (PACK_ROOT / "README.md").read_text(encoding="utf-8")
        checklist = (PACK_ROOT / "docs/release-checklist.md").read_text(encoding="utf-8")

        for text in [readme, checklist]:
            with self.subTest(document=text[:20]):
                self.assertIn("PYTHONDONTWRITEBYTECODE=1", text)
                self.assertIn("--probe-tools", text)
                self.assertIn("--command-sources", text)

        self.assertIn(".github/workflows/test.yml", checklist)

    def test_ci_workflow_runs_offline_validation_gates(self) -> None:
        workflow = PACK_ROOT / ".github/workflows/test.yml"
        self.assertTrue(workflow.is_file(), "missing GitHub Actions workflow")
        text = workflow.read_text(encoding="utf-8")

        self.assertIn("PYTHONDONTWRITEBYTECODE: \"1\"", text)
        self.assertIn("python3 -m unittest discover -s tests -v", text)
        self.assertIn("python3 scripts/validate_pack.py --json", text)
        self.assertIn("python3 scripts/check_runtime_layout.py --json", text)
        self.assertIn("python3 scripts/run_scenarios.py --platform macos --sandbox local --json", text)
        self.assertIn("python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json", text)

    def test_radar_workflow_runs_on_schedule_and_uploads_artifact(self) -> None:
        workflow = PACK_ROOT / ".github/workflows/radar.yml"
        self.assertTrue(workflow.is_file(), "missing radar workflow")
        text = workflow.read_text(encoding="utf-8")

        self.assertIn("schedule:", text)
        self.assertIn("workflow_dispatch:", text)
        self.assertIn("python3 scripts/groundline_radar.py --json --network", text)
        self.assertIn("actions/upload-artifact@v4", text)

    def test_validate_pack_requires_ci_workflow(self) -> None:
        validator = (PACK_ROOT / "scripts/validate_pack.py").read_text(encoding="utf-8")

        self.assertIn('".github/workflows/test.yml"', validator)
        self.assertIn('".github/workflows/radar.yml"', validator)
        self.assertIn('"docs/install.md"', validator)
        self.assertIn('"docs/update.md"', validator)
        self.assertIn('"docs/provider-smoke.md"', validator)
        self.assertIn('"scripts/groundline_provider_smoke.py"', validator)

    def test_install_and_update_docs_cover_private_repo_flow(self) -> None:
        install = (PACK_ROOT / "docs/install.md").read_text(encoding="utf-8")
        update = (PACK_ROOT / "docs/update.md").read_text(encoding="utf-8")
        smoke = (PACK_ROOT / "docs/provider-smoke.md").read_text(encoding="utf-8")

        self.assertIn("gh repo clone jukqaz/groundline", install)
        self.assertIn("python3 scripts/groundline_provider_smoke.py --json", install)
        self.assertIn("git pull --ff-only", update)
        self.assertIn("python3 scripts/validate_pack.py --json", update)
        self.assertIn("python3 scripts/groundline_provider_smoke.py --json", smoke)
        self.assertIn("mutation_performed=false", smoke)

    def test_skill_surface_matches_v0_1_design(self) -> None:
        expected_skills = {
            "reconcile-current-state",
            "audit-agent-history",
            "guard-side-effects",
            "close-live-work",
            "align-agent-home",
            "recover-worktree-branch",
        }

        skills_dir = PACK_ROOT / "skills"
        actual_skills = {path.name for path in skills_dir.iterdir() if path.is_dir()}
        self.assertEqual(actual_skills, expected_skills)

        for skill_name in expected_skills:
            with self.subTest(skill=skill_name):
                skill_dir = skills_dir / skill_name
                self.assertTrue((skill_dir / "SKILL.md").is_file())
                self.assertTrue((skill_dir / "agents/openai.yaml").is_file())

    def test_product_text_has_no_old_brand_or_unsupported_scope(self) -> None:
        forbidden_patterns = {
            r"\bStateFirst\b": "old brand",
            r"\bstate-first-pack\b": "old slug",
            r"\bGemini\b": "unsupported provider",
            r"\blegacy\b": "old-provider framing",
            r"\bcompatibility\b": "broad provider-adapter framing",
            r"\bWindows\b": "unsupported platform",
            r"\bWSL\b": "unsupported platform",
            r"\bRust\b": "compiled-tool framing",
        }

        checked_suffixes = {".md", ".json", ".yaml", ".yml", ".py"}
        violations: list[str] = []
        for path in sorted(PACK_ROOT.rglob("*")):
            if not path.is_file() or path.suffix not in checked_suffixes:
                continue
            if "tests" in path.parts:
                continue
            text = path.read_text(encoding="utf-8")
            for pattern, reason in forbidden_patterns.items():
                if re.search(pattern, text, flags=re.IGNORECASE):
                    rel = path.relative_to(PACK_ROOT)
                    violations.append(f"{rel}: {reason}: {pattern}")

        self.assertEqual(violations, [])


if __name__ == "__main__":
    unittest.main()
