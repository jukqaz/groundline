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
            PACK_ROOT / "plugins/groundline/.codex-plugin/plugin.json",
            PACK_ROOT / "plugins/groundline/.claude-plugin/plugin.json",
            PACK_ROOT / "plugins/groundline/plugin.json",
        ]

        for manifest_path in manifest_paths:
            with self.subTest(manifest=manifest_path.relative_to(PACK_ROOT)):
                data = json.loads(manifest_path.read_text(encoding="utf-8"))
                self.assertEqual(data.get("name"), "groundline")

        codex = json.loads((PACK_ROOT / ".codex-plugin/plugin.json").read_text(encoding="utf-8"))
        claude = json.loads((PACK_ROOT / ".claude-plugin/plugin.json").read_text(encoding="utf-8"))
        interface = codex.get("interface", {})
        self.assertEqual(codex.get("version"), "0.3.0")
        self.assertEqual(claude.get("version"), "0.3.0")
        self.assertEqual(interface.get("displayName"), "GroundLine")
        self.assertIn("control plane", interface.get("longDescription", "").lower())
        self.assertEqual(interface.get("composerIcon"), "./assets/groundline-icon.svg")
        self.assertEqual(interface.get("logo"), "./assets/groundline-logo.svg")
        self.assertEqual(codex.get("homepage"), "https://github.com/jukqaz/groundline")
        self.assertEqual(codex.get("repository"), "https://github.com/jukqaz/groundline")
        self.assertEqual(codex.get("license"), "MIT")

    def test_required_groundline_files_exist(self) -> None:
        required_files = [
            ".codex-plugin/plugin.json",
            ".agents/plugins/marketplace.json",
            ".claude-plugin/plugin.json",
            ".claude-plugin/marketplace.json",
            "plugin.json",
            "CHANGELOG.md",
            "CONTRIBUTING.md",
            "LICENSE",
            "README.md",
            "README.ko.md",
            "SECURITY.md",
            "assets/groundline-icon.svg",
            "assets/groundline-logo.svg",
            "docs/language-policy.md",
            ".github/ISSUE_TEMPLATE/bug_report.md",
            ".github/ISSUE_TEMPLATE/feature_request.md",
            ".github/pull_request_template.md",
            ".github/workflows/test.yml",
            ".github/workflows/radar.yml",
            "docs/install.md",
            "docs/git-history-privacy.md",
            "docs/human-guide.md",
            "docs/llm-guide.md",
            "docs/update.md",
            "docs/provider-smoke.md",
            "docs/provider-dogfood.md",
            "docs/next-work.md",
            "docs/next-version.md",
            "docs/privacy.md",
            "docs/terms.md",
            "docs/provider-packaging.md",
            "docs/public-release.md",
            "docs/runtime-support.md",
            "docs/examples.md",
            "docs/dogfood.md",
            "docs/skill-portfolio.md",
            "docs/release-checklist.md",
            "docs/ko/index.md",
            "docs/ko/human-guide.md",
            "docs/ko/install.md",
            "docs/ko/update.md",
            "docs/ko/examples.md",
            "docs/ko/skill-portfolio.md",
            "docs/ko/privacy.md",
            "docs/ko/terms.md",
            "docs/ko/provider-packaging.md",
            "docs/ko/release-checklist.md",
            "docs/ko/next-version.md",
            "references/capability-blueprint.md",
            "references/config-sync-boundary.md",
            "references/output-contracts.md",
            "references/platform-support.md",
            "references/runtime-matrix.md",
            "references/external-workflow-interop.md",
            "references/capability-evaluation.md",
            "references/ai-usage-maturity.md",
            "references/multi-provider-fluency-boundary.md",
            "references/agent-task-packet.md",
            "references/release-stabilization.md",
            "references/skill-index.json",
            "references/skill-lifecycle.md",
            "references/source-registry.json",
            "references/superpowers-interop.md",
            "references/tool-profiles.md",
            "references/workflow-modes.md",
            "scripts/check_runtime_layout.py",
            "scripts/groundline_doctor.py",
            "scripts/groundline_dogfood.py",
            "scripts/groundline_plan_update.py",
            "scripts/groundline_provider_smoke.py",
            "scripts/groundline_radar.py",
            "scripts/lint.py",
            "scripts/run_scenarios.py",
            "scripts/sync_provider_package.py",
            "scripts/validate_pack.py",
            "plugins/groundline/.codex-plugin/plugin.json",
            "plugins/groundline/.claude-plugin/plugin.json",
            "plugins/groundline/plugin.json",
            "plugins/groundline/assets/groundline-icon.svg",
            "plugins/groundline/assets/groundline-logo.svg",
            "plugins/groundline/docs/provider-packaging.md",
            "plugins/groundline/docs/terms.md",
            "plugins/groundline/docs/ko/provider-packaging.md",
            "plugins/groundline/docs/ko/terms.md",
            "scenarios/fixtures/fresh-install.json",
            "scenarios/expected/standalone-groundline.json",
        ]

        missing = [path for path in required_files if not (PACK_ROOT / path).is_file()]
        self.assertEqual(missing, [])

    def test_human_docs_are_bilingual_with_english_default(self) -> None:
        readme = (PACK_ROOT / "README.md").read_text(encoding="utf-8")
        korean_readme = (PACK_ROOT / "README.ko.md").read_text(encoding="utf-8")
        language_policy = (PACK_ROOT / "docs/language-policy.md").read_text(encoding="utf-8")
        korean_index = (PACK_ROOT / "docs/ko/index.md").read_text(encoding="utf-8")

        self.assertIn("English is the default and canonical language", readme)
        self.assertIn("README.ko.md", readme)
        self.assertIn("docs/ko/index.md", readme)
        self.assertIn("docs/ko/provider-packaging.md", readme)
        self.assertIn("English is the default and canonical language", language_policy)
        self.assertIn("LLM-readable references", language_policy)
        self.assertIn("영어 문서가 기본", korean_index)
        self.assertIn("한국어 companion", korean_readme)

    def test_release_docs_cover_ci_external_probes_and_clean_python_runs(self) -> None:
        readme = (PACK_ROOT / "README.md").read_text(encoding="utf-8")
        checklist = (PACK_ROOT / "docs/release-checklist.md").read_text(encoding="utf-8")

        for text in [readme, checklist]:
            with self.subTest(document=text[:20]):
                self.assertIn("PYTHONDONTWRITEBYTECODE=1", text)
                self.assertIn("--probe-tools", text)
                self.assertIn("--command-sources", text)

        self.assertIn(".github/workflows/test.yml", checklist)
        self.assertIn("scripts/groundline_dogfood.py --stage-package --probe-runtimes --json", checklist)

    def test_ci_workflow_runs_offline_validation_gates(self) -> None:
        workflow = PACK_ROOT / ".github/workflows/test.yml"
        self.assertTrue(workflow.is_file(), "missing GitHub Actions workflow")
        text = workflow.read_text(encoding="utf-8")

        self.assertIn("PYTHONDONTWRITEBYTECODE: \"1\"", text)
        self.assertIn("FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: \"true\"", text)
        self.assertIn('ACTIONLINT_VERSION: "1.7.12"', text)
        self.assertIn('ACTIONLINT_SHA256: "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"', text)
        self.assertIn("actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz", text)
        self.assertIn("shasum -a 256 -c -", text)
        self.assertIn("python3 scripts/lint.py --json --require-actionlint", text)
        self.assertIn("python3 -m unittest discover -s tests -v", text)
        self.assertIn("python3 scripts/validate_pack.py --json", text)
        self.assertIn("python3 scripts/check_runtime_layout.py --json", text)
        self.assertIn("python3 scripts/run_scenarios.py --platform macos --sandbox local --json", text)
        self.assertIn("python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json", text)
        self.assertIn("python3 scripts/groundline_dogfood.py --stage-package --json", text)

    def test_radar_workflow_runs_on_schedule_and_uploads_artifact(self) -> None:
        workflow = PACK_ROOT / ".github/workflows/radar.yml"
        self.assertTrue(workflow.is_file(), "missing radar workflow")
        text = workflow.read_text(encoding="utf-8")

        self.assertIn("schedule:", text)
        self.assertIn("workflow_dispatch:", text)
        self.assertIn("python3 scripts/groundline_radar.py --json --network", text)
        self.assertIn("actions/upload-artifact@v7", text)
        self.assertIn("FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: \"true\"", text)

    def test_validate_pack_requires_ci_workflow(self) -> None:
        validator = (PACK_ROOT / "scripts/validate_pack.py").read_text(encoding="utf-8")

        self.assertIn('".github/workflows/test.yml"', validator)
        self.assertIn('".github/workflows/radar.yml"', validator)
        self.assertIn('"references/external-workflow-interop.md"', validator)
        self.assertIn('"references/capability-evaluation.md"', validator)
        self.assertIn('"references/ai-usage-maturity.md"', validator)
        self.assertIn('"references/multi-provider-fluency-boundary.md"', validator)
        self.assertIn('"references/agent-task-packet.md"', validator)
        self.assertIn('"references/release-stabilization.md"', validator)
        self.assertIn('"docs/install.md"', validator)
        self.assertIn('"README.ko.md"', validator)
        self.assertIn('".agents/plugins/marketplace.json"', validator)
        self.assertIn('".claude-plugin/marketplace.json"', validator)
        self.assertIn('"assets/groundline-icon.svg"', validator)
        self.assertIn('"assets/groundline-logo.svg"', validator)
        self.assertIn('"docs/language-policy.md"', validator)
        self.assertIn('"docs/git-history-privacy.md"', validator)
        self.assertIn('"docs/human-guide.md"', validator)
        self.assertIn('"docs/llm-guide.md"', validator)
        self.assertIn('"docs/update.md"', validator)
        self.assertIn('"docs/provider-smoke.md"', validator)
        self.assertIn('"docs/provider-dogfood.md"', validator)
        self.assertIn('"docs/next-work.md"', validator)
        self.assertIn('"docs/next-version.md"', validator)
        self.assertIn('"docs/privacy.md"', validator)
        self.assertIn('"docs/terms.md"', validator)
        self.assertIn('"docs/provider-packaging.md"', validator)
        self.assertIn('"docs/public-release.md"', validator)
        self.assertIn('"docs/dogfood.md"', validator)
        self.assertIn('"docs/skill-portfolio.md"', validator)
        self.assertIn('"docs/ko/index.md"', validator)
        self.assertIn('"docs/ko/human-guide.md"', validator)
        self.assertIn('"docs/ko/install.md"', validator)
        self.assertIn('"docs/ko/update.md"', validator)
        self.assertIn('"docs/ko/examples.md"', validator)
        self.assertIn('"docs/ko/skill-portfolio.md"', validator)
        self.assertIn('"docs/ko/privacy.md"', validator)
        self.assertIn('"docs/ko/terms.md"', validator)
        self.assertIn('"docs/ko/provider-packaging.md"', validator)
        self.assertIn('"docs/ko/release-checklist.md"', validator)
        self.assertIn('"docs/ko/next-version.md"', validator)
        self.assertIn('"references/skill-index.json"', validator)
        self.assertIn('"references/skill-lifecycle.md"', validator)
        self.assertIn('"CONTRIBUTING.md"', validator)
        self.assertIn('"SECURITY.md"', validator)
        self.assertIn('"scripts/groundline_dogfood.py"', validator)
        self.assertIn('"scripts/groundline_provider_smoke.py"', validator)
        self.assertIn('"scripts/lint.py"', validator)
        self.assertIn('"scripts/sync_provider_package.py"', validator)
        self.assertIn('"plugins/groundline/.codex-plugin/plugin.json"', validator)

    def test_install_and_update_docs_cover_public_repo_flow(self) -> None:
        install = (PACK_ROOT / "docs/install.md").read_text(encoding="utf-8")
        update = (PACK_ROOT / "docs/update.md").read_text(encoding="utf-8")
        smoke = (PACK_ROOT / "docs/provider-smoke.md").read_text(encoding="utf-8")
        dogfood = (PACK_ROOT / "docs/provider-dogfood.md").read_text(encoding="utf-8")

        self.assertIn("gh repo clone jukqaz/groundline", install)
        self.assertIn("git clone https://github.com/jukqaz/groundline.git", install)
        self.assertIn("public plugin package", install)
        self.assertIn("python3 scripts/groundline_provider_smoke.py --json", install)
        self.assertIn("git pull --ff-only", update)
        self.assertIn("python3 scripts/validate_pack.py --json", update)
        self.assertIn("python3 scripts/groundline_provider_smoke.py --json", smoke)
        self.assertIn("python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json", dogfood)
        self.assertIn("codex plugin marketplace add jukqaz/groundline --ref main", install)
        self.assertIn("claude plugin marketplace add jukqaz/groundline", install)
        self.assertIn("agy plugin install https://github.com/jukqaz/groundline", install)
        self.assertIn("mutation_performed=false", smoke)
        self.assertIn("displayed with `~`", smoke)

    def test_provider_marketplaces_point_to_packaged_plugin(self) -> None:
        codex_marketplace = json.loads((PACK_ROOT / ".agents/plugins/marketplace.json").read_text(encoding="utf-8"))
        claude_marketplace = json.loads((PACK_ROOT / ".claude-plugin/marketplace.json").read_text(encoding="utf-8"))
        package_root = PACK_ROOT / "plugins/groundline"

        self.assertEqual(codex_marketplace["name"], "groundline")
        self.assertEqual(codex_marketplace["plugins"][0]["name"], "groundline")
        self.assertEqual(codex_marketplace["plugins"][0]["source"]["path"], "./plugins/groundline")
        self.assertEqual(codex_marketplace["plugins"][0]["policy"]["installation"], "AVAILABLE")
        self.assertEqual(codex_marketplace["plugins"][0]["category"], "Coding")

        self.assertEqual(claude_marketplace["name"], "groundline")
        self.assertEqual(claude_marketplace["plugins"][0]["name"], "groundline")
        self.assertEqual(claude_marketplace["plugins"][0]["source"], "./plugins/groundline")

        packaged_skills = {path.name for path in (package_root / "skills").iterdir() if path.is_dir()}
        root_skills = {path.name for path in (PACK_ROOT / "skills").iterdir() if path.is_dir()}
        self.assertEqual(packaged_skills, root_skills)
        self.assertTrue((package_root / ".codex-plugin/plugin.json").is_file())
        self.assertTrue((package_root / ".claude-plugin/plugin.json").is_file())
        self.assertTrue((package_root / "plugin.json").is_file())

    def test_dogfood_doc_tracks_provider_matrix_and_results(self) -> None:
        text = (PACK_ROOT / "docs/dogfood.md").read_text(encoding="utf-8")

        for term in [
            "Dogfood Matrix",
            "Provider",
            "Scenario",
            "Expected skill",
            "Evidence",
            "Result",
            "Harness Evidence",
            "provider-dogfood",
            "Real home mutation: false",
            "status=PASS",
            "scenario_count=3",
            "runtime_probe",
            "fake_home_used=true",
            "target_exists=true",
            "target_skill_count=19",
            "target_skill_count_matches_source=true",
            "2026-05-28",
            "Accepted Defer",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_provider_dogfood_defines_sanitized_invocation_proof(self) -> None:
        provider_dogfood = (PACK_ROOT / "docs/provider-dogfood.md").read_text(encoding="utf-8")

        for term in [
            "Sanitized Invocation Proof",
            "raw_transcript_stored: false",
            "provider_home_dumped: false",
            "prompt_family",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, provider_dogfood)

    def test_public_docs_cover_security_privacy_license_and_identity(self) -> None:
        readme = (PACK_ROOT / "README.md").read_text(encoding="utf-8")
        security = (PACK_ROOT / "SECURITY.md").read_text(encoding="utf-8")
        contributing = (PACK_ROOT / "CONTRIBUTING.md").read_text(encoding="utf-8")
        history_privacy = (PACK_ROOT / "docs/git-history-privacy.md").read_text(encoding="utf-8")
        human_guide = (PACK_ROOT / "docs/human-guide.md").read_text(encoding="utf-8")
        llm_guide = (PACK_ROOT / "docs/llm-guide.md").read_text(encoding="utf-8")
        privacy = (PACK_ROOT / "docs/privacy.md").read_text(encoding="utf-8")
        public_release = (PACK_ROOT / "docs/public-release.md").read_text(encoding="utf-8")
        pr_template = (PACK_ROOT / ".github/pull_request_template.md").read_text(encoding="utf-8")
        bug_template = (PACK_ROOT / ".github/ISSUE_TEMPLATE/bug_report.md").read_text(encoding="utf-8")
        license_text = (PACK_ROOT / "LICENSE").read_text(encoding="utf-8")
        codex = json.loads((PACK_ROOT / ".codex-plugin/plugin.json").read_text(encoding="utf-8"))
        claude = json.loads((PACK_ROOT / ".claude-plugin/plugin.json").read_text(encoding="utf-8"))

        self.assertIn("License: MIT", readme)
        self.assertIn("GitHub Security Advisories", security)
        self.assertIn("Do not commit provider auth files", contributing)
        self.assertIn("current tree", history_privacy)
        self.assertIn("fresh public repository", history_privacy)
        self.assertIn("This guide is for a person", human_guide)
        self.assertIn("This guide is for LLM agents", llm_guide)
        self.assertIn("Default home paths should be shown as `~`", privacy)
        self.assertIn("author and committer metadata", public_release)
        self.assertIn("CI downloads pinned tools and verifies checksums", public_release)
        self.assertIn("No secrets", pr_template)
        self.assertIn("This report does not include secrets", bug_template)
        self.assertIn("GroundLine contributors", license_text)
        self.assertEqual(codex["author"]["name"], "GroundLine contributors")
        self.assertEqual(codex["interface"]["developerName"], "GroundLine")
        self.assertEqual(claude["author"]["name"], "GroundLine contributors")
        public_text = "\n".join(
            [readme, security, contributing, history_privacy, human_guide, llm_guide, privacy, public_release, license_text]
        ).lower()
        local_home_name = Path.home().name.lower()
        if local_home_name:
            self.assertNotIn(local_home_name, public_text)

    def test_skill_surface_matches_v0_1_design(self) -> None:
        expected_skills = {
            "reconcile-current-state",
            "audit-agent-history",
            "guard-side-effects",
            "close-live-work",
            "align-agent-home",
            "recover-worktree-branch",
            "agent-ecosystem-radar",
            "research-agent-ecosystem",
            "compare-agent-workflows",
            "recommend-groundline-upgrades",
            "evaluate-groundline-pack",
            "curate-groundline-skills",
            "evaluate-agent-capability",
            "evaluate-ai-usage-maturity",
            "package-agent-task",
            "hold-the-line",
            "polish-release-candidate",
            "stabilize-release-cut",
            "compare-release-delta",
        }

        skills_dir = PACK_ROOT / "skills"
        actual_skills = {path.name for path in skills_dir.iterdir() if path.is_dir()}
        self.assertEqual(actual_skills, expected_skills)

        for skill_name in expected_skills:
            with self.subTest(skill=skill_name):
                skill_dir = skills_dir / skill_name
                self.assertTrue((skill_dir / "SKILL.md").is_file())
                self.assertTrue((skill_dir / "agents/openai.yaml").is_file())

    def test_agent_ecosystem_skill_set_is_chained(self) -> None:
        orchestrator = (PACK_ROOT / "skills/agent-ecosystem-radar/SKILL.md").read_text(encoding="utf-8")
        outputs = (PACK_ROOT / "references/output-contracts.md").read_text(encoding="utf-8")
        interop = (PACK_ROOT / "references/external-workflow-interop.md").read_text(encoding="utf-8")

        for skill_name in [
            "research-agent-ecosystem",
            "evaluate-agent-capability",
            "compare-agent-workflows",
            "recommend-groundline-upgrades",
        ]:
            with self.subTest(skill=skill_name):
                self.assertIn(skill_name, orchestrator)

        for contract in [
            "GroundLine Research",
            "GroundLine Capability Evaluation",
            "GroundLine Comparison",
            "GroundLine Recommendation",
            "GroundLine Pack Evaluation",
        ]:
            with self.subTest(contract=contract):
                self.assertIn(contract, outputs)

        for source in ["Spec Kit", "Agent OS", "BMAD", "gstack", "grill-me", "Promptfoo"]:
            with self.subTest(source=source):
                self.assertIn(source, interop)

    def test_agent_ecosystem_orchestrator_uses_full_output_contract(self) -> None:
        orchestrator = (PACK_ROOT / "skills/agent-ecosystem-radar/SKILL.md").read_text(encoding="utf-8")

        required_fields = [
            "secondary sources",
            "confirmed facts",
            "unverified claims",
            "overlap with GroundLine",
            "context and setup cost",
            "comparison gaps",
            "side-effect boundary",
        ]

        for field in required_fields:
            with self.subTest(field=field):
                self.assertIn(field, orchestrator)

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
