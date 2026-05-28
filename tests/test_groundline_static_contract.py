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

        root_manifest = json.loads((PACK_ROOT / "plugin.json").read_text(encoding="utf-8"))
        codex = json.loads((PACK_ROOT / ".codex-plugin/plugin.json").read_text(encoding="utf-8"))
        claude = json.loads((PACK_ROOT / ".claude-plugin/plugin.json").read_text(encoding="utf-8"))
        interface = codex.get("interface", {})
        self.assertEqual(codex.get("version"), root_manifest.get("version"))
        self.assertEqual(claude.get("version"), root_manifest.get("version"))
        self.assertEqual(interface.get("displayName"), "GroundLine")
        self.assertIn("control plane", interface.get("longDescription", "").lower())
        self.assertEqual(interface.get("composerIcon"), "./assets/groundline-icon.svg")
        self.assertEqual(interface.get("logo"), "./assets/groundline-logo.svg")
        self.assertEqual(codex.get("homepage"), "https://github.com/jukqaz/groundline")
        self.assertEqual(codex.get("repository"), "https://github.com/jukqaz/groundline")
        self.assertEqual(codex.get("license"), "MIT")

    def test_manifest_versions_match_canonical_manifest(self) -> None:
        canonical = json.loads((PACK_ROOT / "plugin.json").read_text(encoding="utf-8")).get("version")
        manifest_paths = [
            ".codex-plugin/plugin.json",
            ".claude-plugin/plugin.json",
            "plugin.json",
            "plugins/groundline/.codex-plugin/plugin.json",
            "plugins/groundline/.claude-plugin/plugin.json",
            "plugins/groundline/plugin.json",
        ]

        for rel in manifest_paths:
            with self.subTest(manifest=rel):
                data = json.loads((PACK_ROOT / rel).read_text(encoding="utf-8"))
                self.assertEqual(data.get("version"), canonical)

        validator = (PACK_ROOT / "scripts/validate_pack.py").read_text(encoding="utf-8")
        self.assertIn("canonical_version", validator)
        self.assertNotIn("manifest version must be 0.3.2", validator)

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
            "docs/provider-activation-matrix.md",
            "docs/provider-guardrails.md",
            "docs/mcp-recipes.md",
            "docs/maturity-assessment.md",
            "docs/next-work.md",
            "docs/next-version.md",
            "docs/privacy.md",
            "docs/terms.md",
            "docs/provider-packaging.md",
            "docs/public-release.md",
            "docs/runtime-support.md",
            "docs/examples.md",
            "docs/workflow-cookbook.md",
            "docs/artifact-lifecycle.md",
            "docs/dogfood.md",
            "docs/skill-portfolio.md",
            "docs/skill-graduation-plan.md",
            "docs/release-checklist.md",
            "docs/ko/index.md",
            "docs/ko/human-guide.md",
            "docs/ko/install.md",
            "docs/ko/update.md",
            "docs/ko/examples.md",
            "docs/ko/workflow-cookbook.md",
            "docs/ko/artifact-lifecycle.md",
            "docs/ko/skill-portfolio.md",
            "docs/ko/skill-graduation-plan.md",
            "docs/ko/privacy.md",
            "docs/ko/terms.md",
            "docs/ko/provider-packaging.md",
            "docs/ko/provider-activation-matrix.md",
            "docs/ko/provider-guardrails.md",
            "docs/ko/mcp-recipes.md",
            "docs/ko/maturity-assessment.md",
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
            "references/optional-mcp-profiles.md",
            "references/superpowers-interop.md",
            "references/tool-profiles.md",
            "references/workflow-modes.md",
            "scripts/check_runtime_layout.py",
            "scripts/groundline_doctor.py",
            "scripts/groundline_dogfood.py",
            "scripts/groundline_plan_update.py",
            "scripts/groundline_provider_smoke.py",
            "scripts/groundline_radar.py",
            "scripts/groundline_release_gate.py",
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
        human_guide = (PACK_ROOT / "docs/human-guide.md").read_text(encoding="utf-8")
        korean_human_guide = (PACK_ROOT / "docs/ko/human-guide.md").read_text(encoding="utf-8")
        install = (PACK_ROOT / "docs/install.md").read_text(encoding="utf-8")
        korean_install = (PACK_ROOT / "docs/ko/install.md").read_text(encoding="utf-8")
        language_policy = (PACK_ROOT / "docs/language-policy.md").read_text(encoding="utf-8")
        korean_index = (PACK_ROOT / "docs/ko/index.md").read_text(encoding="utf-8")

        self.assertIn("Start Here", readme)
        self.assertIn("When To Use It", readme)
        self.assertIn("English is the default and canonical language", readme)
        self.assertIn("README.ko.md", readme)
        self.assertIn("docs/ko/index.md", readme)
        self.assertIn("docs/ko/provider-packaging.md", readme)
        self.assertIn("docs/provider-guardrails.md", readme)
        self.assertIn("docs/mcp-recipes.md", readme)
        self.assertIn("Choose The Right Request", human_guide)
        self.assertIn("Copyable Prompts", human_guide)
        self.assertIn("Install has two phases", install)
        self.assertIn("English is the default and canonical language", language_policy)
        self.assertIn("LLM-readable references", language_policy)
        self.assertIn("영어 문서가 기본", korean_index)
        self.assertIn("한국어 companion", korean_readme)
        self.assertIn("첫 요청 예시", korean_readme)
        self.assertIn("그대로 써도 되는 prompt", korean_human_guide)
        self.assertIn("설치는 두 단계", korean_install)

    def test_maturity_assessment_defines_score_and_next_work(self) -> None:
        assessment = (PACK_ROOT / "docs/maturity-assessment.md").read_text(encoding="utf-8")
        korean_assessment = (PACK_ROOT / "docs/ko/maturity-assessment.md").read_text(encoding="utf-8")
        next_work = (PACK_ROOT / "docs/next-work.md").read_text(encoding="utf-8")
        korean_index = (PACK_ROOT / "docs/ko/index.md").read_text(encoding="utf-8")
        skill_count = len([path for path in (PACK_ROOT / "skills").iterdir() if path.is_dir()])
        reference_count = len([path for path in (PACK_ROOT / "references").iterdir() if path.is_file()])
        packaged_doc_count = len([path for path in (PACK_ROOT / "plugins/groundline/docs").rglob("*") if path.is_file()])
        test_count = unittest.defaultTestLoader.discover(str(PACK_ROOT / "tests")).countTestCases()

        for term in [
            "Overall maturity: 85/100",
            "Public beta",
            "1.0 gap",
            "Provider install posture",
            "Real provider activation proof",
            "Skill graduation",
            "Version drift control",
            "Next Work Created",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, assessment)

        for term in [
            "P0: Version-aware install doctor",
            "P1: Real provider activation matrix",
            "P1: Skill graduation plan",
            "remaining accepted defer is Antigravity",
            "default CI release gate",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, next_work)

        self.assertNotIn("reduce the two accepted partials", next_work)
        self.assertNotIn("Decide whether Claude Code contract naming", next_work)
        self.assertIn(f"Package surface: {skill_count} skills, {reference_count} references, {packaged_doc_count} packaged docs", assessment)
        self.assertIn(f"returned {test_count} tests OK", assessment)
        self.assertIn("성숙도 평가", korean_assessment)
        self.assertIn("maturity-assessment.md", korean_index)

    def test_changelog_tracks_unreleased_patch_scope(self) -> None:
        changelog = (PACK_ROOT / "CHANGELOG.md").read_text(encoding="utf-8")

        for term in [
            "## Unreleased",
            "Version-aware provider smoke",
            "single-source version control",
            "provider activation matrix",
            "skill graduation plan",
            "workflow cookbook",
            "empty conflict-copy directories",
            "No lifecycle values are promoted",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, changelog)

        self.assertNotIn("- No unreleased changes.", changelog)

    def test_release_docs_cover_ci_external_probes_and_clean_python_runs(self) -> None:
        readme = (PACK_ROOT / "README.md").read_text(encoding="utf-8")
        checklist = (PACK_ROOT / "docs/release-checklist.md").read_text(encoding="utf-8")
        korean_checklist = (PACK_ROOT / "docs/ko/release-checklist.md").read_text(encoding="utf-8")
        next_version = (PACK_ROOT / "docs/next-version.md").read_text(encoding="utf-8")
        korean_next_version = (PACK_ROOT / "docs/ko/next-version.md").read_text(encoding="utf-8")

        for text in [readme, checklist]:
            with self.subTest(document=text[:20]):
                self.assertIn("PYTHONDONTWRITEBYTECODE=1", text)
                self.assertIn("--probe-tools", text)
                self.assertIn("--command-sources", text)

        self.assertIn(".github/workflows/test.yml", checklist)
        self.assertIn("(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)", readme)
        self.assertIn("scripts/groundline_release_gate.py --plan --json", readme)
        self.assertIn("scripts/groundline_release_gate.py --json --keep-going --include-docker-execution", checklist)
        self.assertIn("scripts/groundline_safety_eval.py --json", readme)
        self.assertIn("scripts/groundline_provider_smoke.py --json", readme)
        self.assertIn("scripts/groundline_release_gate.py --plan --json", checklist)
        self.assertIn("scripts/groundline_safety_eval.py --json", checklist)
        self.assertIn("scripts/groundline_dogfood.py --stage-package --probe-runtimes --json", checklist)
        self.assertIn("stale installed cache after a version bump", checklist)
        self.assertIn("Version Bump Sequence", checklist)
        self.assertIn("RELEASE_VERSION=X.Y.Z", checklist)
        self.assertIn('TAG="v$RELEASE_VERSION"', checklist)
        self.assertIn("Update source manifest versions only", checklist)
        self.assertIn("Do not edit `plugins/groundline` manifests directly", checklist)
        self.assertIn("scripts/sync_provider_package.py --json", checklist)
        self.assertIn("Keep the entries under `Unreleased` while the ship decision is `hold`", checklist)
        self.assertIn("plugins/groundline/.claude-plugin/plugin.json", checklist)
        self.assertIn("Approval-required Publishing Commands", checklist)
        self.assertIn("Do not run publish commands unless the user approved publishing in this", checklist)
        self.assertIn("ship decision is `hold`", checklist)
        self.assertIn('git tag "$TAG"', checklist)
        self.assertIn('git push origin "$TAG"', checklist)
        self.assertIn('gh release create "$TAG"', checklist)
        self.assertIn("Version bump 순서", korean_checklist)
        self.assertIn('TAG="v$RELEASE_VERSION"', korean_checklist)
        self.assertIn("승인 필요 배포 명령", korean_checklist)
        self.assertIn("명시 승인을 받기 전에는 실행하지 않습니다", korean_checklist)
        self.assertIn("safety eval", next_version)
        self.assertIn("macOS local scenario", next_version)
        self.assertIn("Linux Docker dry-run", next_version)
        self.assertIn("Linux Docker execution", next_version)
        next_work = (PACK_ROOT / "docs/next-work.md").read_text(encoding="utf-8")
        self.assertIn("post-publish provider install refresh", next_work)
        self.assertIn("next_actions", next_work)
        self.assertNotIn("provider smoke, staged dogfood, macOS scenario", next_work)
        self.assertIn("safety eval", korean_next_version)
        self.assertIn("Linux Docker dry-run", korean_next_version)
        self.assertIn("Linux Docker execution", korean_next_version)

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
        self.assertIn("working-directory: plugins/groundline", text)
        self.assertIn("python3 scripts/check_runtime_layout.py --json", text)
        self.assertIn("python3 scripts/groundline_doctor.py --json --offline --probe-tools", text)
        self.assertIn("python3 scripts/groundline_radar.py --json --offline --command-sources", text)
        self.assertIn("python3 scripts/groundline_release_gate.py --plan --json", text)
        self.assertIn("python3 scripts/run_scenarios.py --platform macos --sandbox local --json", text)
        self.assertIn("python3 scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json", text)
        self.assertIn("python3 scripts/groundline_safety_eval.py --json", text)
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
        self.assertIn('"docs/provider-guardrails.md"', validator)
        self.assertIn('"docs/mcp-recipes.md"', validator)
        self.assertIn('"docs/next-work.md"', validator)
        self.assertIn('"docs/next-version.md"', validator)
        self.assertIn('"docs/privacy.md"', validator)
        self.assertIn('"docs/terms.md"', validator)
        self.assertIn('"docs/provider-packaging.md"', validator)
        self.assertIn('"docs/public-release.md"', validator)
        self.assertIn('"docs/workflow-cookbook.md"', validator)
        self.assertIn('"docs/artifact-lifecycle.md"', validator)
        self.assertIn('"docs/dogfood.md"', validator)
        self.assertIn('"docs/skill-portfolio.md"', validator)
        self.assertIn('"docs/skill-graduation-plan.md"', validator)
        self.assertIn('"docs/ko/index.md"', validator)
        self.assertIn('"docs/ko/human-guide.md"', validator)
        self.assertIn('"docs/ko/install.md"', validator)
        self.assertIn('"docs/ko/update.md"', validator)
        self.assertIn('"docs/ko/examples.md"', validator)
        self.assertIn('"docs/ko/workflow-cookbook.md"', validator)
        self.assertIn('"docs/ko/artifact-lifecycle.md"', validator)
        self.assertIn('"docs/ko/skill-portfolio.md"', validator)
        self.assertIn('"docs/ko/skill-graduation-plan.md"', validator)
        self.assertIn('"docs/ko/privacy.md"', validator)
        self.assertIn('"docs/ko/terms.md"', validator)
        self.assertIn('"docs/ko/provider-packaging.md"', validator)
        self.assertIn('"docs/ko/provider-guardrails.md"', validator)
        self.assertIn('"docs/ko/mcp-recipes.md"', validator)
        self.assertIn('"docs/ko/release-checklist.md"', validator)
        self.assertIn('"docs/ko/next-version.md"', validator)
        self.assertIn('"references/skill-index.json"', validator)
        self.assertIn('"references/optional-mcp-profiles.md"', validator)
        self.assertIn('"references/skill-lifecycle.md"', validator)
        self.assertIn('"CONTRIBUTING.md"', validator)
        self.assertIn('"SECURITY.md"', validator)
        self.assertIn('"scripts/groundline_dogfood.py"', validator)
        self.assertIn('"scripts/groundline_provider_smoke.py"', validator)
        self.assertIn('"scripts/groundline_release_gate.py"', validator)
        self.assertIn('"scripts/lint.py"', validator)
        self.assertIn('"scripts/sync_provider_package.py"', validator)
        self.assertIn('"plugins/groundline/.codex-plugin/plugin.json"', validator)

    def test_install_and_update_docs_cover_public_repo_flow(self) -> None:
        install = (PACK_ROOT / "docs/install.md").read_text(encoding="utf-8")
        update = (PACK_ROOT / "docs/update.md").read_text(encoding="utf-8")
        smoke = (PACK_ROOT / "docs/provider-smoke.md").read_text(encoding="utf-8")
        dogfood = (PACK_ROOT / "docs/provider-dogfood.md").read_text(encoding="utf-8")
        activation = (PACK_ROOT / "docs/provider-activation-matrix.md").read_text(encoding="utf-8")
        provider_packaging = (PACK_ROOT / "docs/provider-packaging.md").read_text(encoding="utf-8")
        korean_provider_packaging = (PACK_ROOT / "docs/ko/provider-packaging.md").read_text(encoding="utf-8")

        self.assertIn("gh repo clone jukqaz/groundline", install)
        self.assertIn("git clone https://github.com/jukqaz/groundline.git", install)
        self.assertIn("public plugin package", install)
        self.assertIn("python3 scripts/groundline_provider_smoke.py --json", install)
        self.assertIn("package validation returns `status=PASS`", install)
        self.assertIn("provider smoke returns `status=PASS` for a matching install, or `PARTIAL`", install)
        self.assertIn("Treat provider smoke `FAIL` as an install blocker", install)
        self.assertIn("top-level `next_actions`", install)
        self.assertIn("git pull --ff-only", update)
        self.assertIn("python3 scripts/validate_pack.py --json", update)
        self.assertIn("(cd plugins/groundline && PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json)", update)
        self.assertIn("python3 scripts/groundline_safety_eval.py --json", update)
        self.assertIn("python3 scripts/groundline_doctor.py --json --offline --probe-tools", update)
        self.assertIn("python3 scripts/groundline_radar.py --json --offline --command-sources", update)
        self.assertIn("python3 scripts/groundline_provider_smoke.py --json", smoke)
        self.assertIn("python3 scripts/groundline_dogfood.py --stage-package --probe-runtimes --json", dogfood)
        self.assertIn("docs/superpowers/", provider_packaging)
        self.assertIn("intentionally", provider_packaging)
        self.assertIn("docs/superpowers/", korean_provider_packaging)
        self.assertIn("의도적으로 제외", korean_provider_packaging)
        self.assertIn("Provider Activation Matrix", activation)
        for family in [
            "handoff",
            "side-effect-guard",
            "release-cut",
            "ecosystem-evaluation",
            "ai-usage-maturity",
        ]:
            with self.subTest(prompt_family=family):
                self.assertIn(family, activation)
                self.assertIn(family, dogfood)
        self.assertIn("codex plugin marketplace add jukqaz/groundline --ref main", install)
        self.assertIn("claude plugin marketplace add jukqaz/groundline", install)
        self.assertIn("agy plugin install https://github.com/jukqaz/groundline", install)
        self.assertIn("mutation_performed=false", smoke)
        self.assertIn("secret_value_printed=false", smoke)
        self.assertIn("install_doctor_status", smoke)
        self.assertIn("version_matches_source", smoke)
        self.assertIn("install_source", smoke)
        self.assertIn("stale_cache_version", smoke)
        self.assertIn("skill_count_mismatch", smoke)
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
            "scenario_count=6",
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

    def test_workflow_cookbook_maps_prompt_to_skill_contract_and_stop_condition(self) -> None:
        cookbook = (PACK_ROOT / "docs/workflow-cookbook.md").read_text(encoding="utf-8")
        korean_cookbook = (PACK_ROOT / "docs/ko/workflow-cookbook.md").read_text(encoding="utf-8")
        korean_index = (PACK_ROOT / "docs/ko/index.md").read_text(encoding="utf-8")

        expected_rows = {
            "handoff recovery": ("package-agent-task", "GroundLine Task Packet"),
            "risky operation": ("guard-side-effects", "Boundary"),
            "release cut": ("stabilize-release-cut", "GroundLine Release Cut"),
            "ecosystem radar": ("agent-ecosystem-radar", "GroundLine Research"),
            "AI usage maturity": ("audit-agent-history -> evaluate-ai-usage-maturity", "GroundLine AI Usage Maturity"),
        }

        for workflow, (skill, contract) in expected_rows.items():
            with self.subTest(workflow=workflow):
                self.assertIn(workflow, cookbook)
                self.assertIn(skill, cookbook)
                self.assertIn(contract, cookbook)

        for term in ["Prompt", "Selected skill", "Output contract", "Verification evidence", "Stop condition"]:
            with self.subTest(term=term):
                self.assertIn(term, cookbook)

        self.assertIn("workflow-cookbook.md", korean_index)
        self.assertIn("Stop condition", korean_cookbook)
        self.assertIn("Provider Evidence Packet", korean_cookbook)

    def test_artifact_lifecycle_maps_templates_to_skills_and_contracts(self) -> None:
        lifecycle = (PACK_ROOT / "docs/artifact-lifecycle.md").read_text(encoding="utf-8")
        korean_lifecycle = (PACK_ROOT / "docs/ko/artifact-lifecycle.md").read_text(encoding="utf-8")
        korean_index = (PACK_ROOT / "docs/ko/index.md").read_text(encoding="utf-8")

        expected = {
            "research packet": ("research-agent-ecosystem", "GroundLine Research"),
            "comparison report": ("compare-agent-workflows", "GroundLine Comparison"),
            "upgrade decision": ("recommend-groundline-upgrades", "GroundLine Recommendation"),
            "release cut": ("stabilize-release-cut", "GroundLine Release Cut"),
            "release delta": ("compare-release-delta", "GroundLine Release Delta"),
        }

        for artifact, (skill, contract) in expected.items():
            with self.subTest(artifact=artifact):
                self.assertIn(artifact, lifecycle)
                self.assertIn(skill, lifecycle)
                self.assertIn(contract, lifecycle)
                self.assertIn(artifact, korean_lifecycle)

        for term in [
            "research -> compare -> recommend -> implement -> dogfood -> release -> post-release review",
            "provider-native feature duplication",
            "Do not create a new artifact type",
            "Stop condition",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, lifecycle)

        self.assertIn("artifact-lifecycle.md", korean_index)

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

    def test_provider_activation_matrix_defines_live_proof_runbook(self) -> None:
        activation = (PACK_ROOT / "docs/provider-activation-matrix.md").read_text(encoding="utf-8")
        korean_activation = (PACK_ROOT / "docs/ko/provider-activation-matrix.md").read_text(encoding="utf-8")

        for term in [
            "Collection Runbook",
            "explicit user approval",
            "Row Update Checklist",
            "Do not count these as live provider activation proof",
            "groundline_dogfood.py",
            "groundline_provider_smoke.py",
            "mutation_status",
            "raw_transcript_stored=false",
            "provider_home_dumped=false",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, activation)

        for family in [
            "handoff",
            "side-effect-guard",
            "release-cut",
            "ecosystem-evaluation",
            "ai-usage-maturity",
        ]:
            with self.subTest(prompt_family=family):
                self.assertIn(family, activation)
                self.assertIn(family, korean_activation)

        self.assertIn("수집 Runbook", korean_activation)
        self.assertIn("사용자 승인을 받습니다", korean_activation)
        self.assertIn("GroundLine AI Usage Maturity", activation)
        self.assertNotIn("GroundLine AI Usage Maturity Assessment", activation)

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
        self.assertIn("scripts/groundline_safety_eval.py --json", public_release)
        self.assertIn("scripts/groundline_doctor.py --json --offline --probe-tools", public_release)
        self.assertIn("scripts/groundline_radar.py --json --offline --command-sources", public_release)
        self.assertIn("scripts/run_scenarios.py --platform linux --sandbox docker --dry-run --json", public_release)
        self.assertIn("Do not tag, push, create a GitHub Release", public_release)
        self.assertIn("Approval-required Publishing Commands", public_release)
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

    def test_experimental_skills_have_graduation_decisions(self) -> None:
        index = json.loads((PACK_ROOT / "references/skill-index.json").read_text(encoding="utf-8"))
        portfolio = (PACK_ROOT / "docs/skill-portfolio.md").read_text(encoding="utf-8")
        graduation = (PACK_ROOT / "docs/skill-graduation-plan.md").read_text(encoding="utf-8")
        korean_graduation = (PACK_ROOT / "docs/ko/skill-graduation-plan.md").read_text(encoding="utf-8")
        allowed = {"graduate", "keep experimental", "merge", "defer"}

        experimental = [skill for skill in index["skills"] if skill["lifecycle"] == "experimental"]
        self.assertEqual(len(experimental), 12)

        for skill in experimental:
            name = skill["name"]
            with self.subTest(skill=name):
                self.assertIn(skill.get("graduation_decision"), allowed)
                self.assertIsInstance(skill.get("graduation_rationale"), str)
                self.assertGreater(len(skill["graduation_rationale"]), 20)
                self.assertIn(name, portfolio)
                self.assertIn(name, graduation)
                self.assertIn(name, korean_graduation)

        for decision in allowed:
            with self.subTest(decision=decision):
                self.assertIn(decision, graduation)

        self.assertIn("Graduation Decisions", portfolio)
        self.assertIn("examples", graduation)
        self.assertIn("output contracts", graduation)
        self.assertIn("dogfood evidence", graduation)

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

    def test_agent_ecosystem_routing_boundaries_are_explicit(self) -> None:
        skill_files = {
            name: (PACK_ROOT / f"skills/{name}/SKILL.md").read_text(encoding="utf-8")
            for name in [
                "agent-ecosystem-radar",
                "research-agent-ecosystem",
                "evaluate-agent-capability",
                "compare-agent-workflows",
                "recommend-groundline-upgrades",
            ]
        }
        llm_guide = (PACK_ROOT / "docs/llm-guide.md").read_text(encoding="utf-8")

        expected_terms = {
            "agent-ecosystem-radar": [
                "Use this when the user asks for research, comparison, and recommendation in one pass.",
                "Return all four sections in order",
            ],
            "research-agent-ecosystem": [
                "Use this only for source gathering.",
                "If the user asks for comparison or recommendation in the same request, use `agent-ecosystem-radar`.",
            ],
            "evaluate-agent-capability": [
                "Use this for one candidate at a time.",
                "If there are two or more candidates to rank, use `compare-agent-workflows`.",
            ],
            "compare-agent-workflows": [
                "Use this after research has produced two or more candidates, or one candidate plus a clear GroundLine baseline.",
                "Use `evaluate-agent-capability` first when the user asks whether one artifact is any good.",
            ],
            "recommend-groundline-upgrades": [
                "Use this after research or comparison findings already exist.",
                "If current sources still need to be gathered, use `research-agent-ecosystem` or `agent-ecosystem-radar` first.",
            ],
        }

        for skill_name, terms in expected_terms.items():
            for term in terms:
                with self.subTest(skill=skill_name, term=term):
                    self.assertIn(term, skill_files[skill_name])

        for term in [
            "single candidate evaluation",
            "two or more researched candidates",
            "research, comparison, and recommendation in one pass",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, llm_guide)

    def test_history_assessment_flow_links_inventory_to_maturity(self) -> None:
        audit = (PACK_ROOT / "skills/audit-agent-history/SKILL.md").read_text(encoding="utf-8")
        maturity = (PACK_ROOT / "skills/evaluate-ai-usage-maturity/SKILL.md").read_text(encoding="utf-8")
        llm_guide = (PACK_ROOT / "docs/llm-guide.md").read_text(encoding="utf-8")

        for text in [audit, maturity, llm_guide]:
            with self.subTest(document=text[:20]):
                self.assertIn("audit-agent-history -> evaluate-ai-usage-maturity", text)
                self.assertIn("Provider Evidence Packet", text)

    def test_release_skill_priority_is_explicit(self) -> None:
        hold = (PACK_ROOT / "skills/hold-the-line/SKILL.md").read_text(encoding="utf-8")
        polish = (PACK_ROOT / "skills/polish-release-candidate/SKILL.md").read_text(encoding="utf-8")
        stabilize = (PACK_ROOT / "skills/stabilize-release-cut/SKILL.md").read_text(encoding="utf-8")
        llm_guide = (PACK_ROOT / "docs/llm-guide.md").read_text(encoding="utf-8")

        self.assertIn("Use first when expansion pressure appears before the current work is closed.", hold)
        self.assertIn("Use after scope is already locked but before the ship decision.", polish)
        self.assertIn("Use when the active question is ship, hold, or continue.", stabilize)

        for term in [
            "hold-the-line before accepting expansion",
            "polish-release-candidate for locked pre-ship cleanup",
            "stabilize-release-cut for the final ship decision",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, llm_guide)

    def test_provider_dogfood_separates_contract_harness_from_invocation_proof(self) -> None:
        provider_dogfood = (PACK_ROOT / "docs/provider-dogfood.md").read_text(encoding="utf-8")
        dogfood = (PACK_ROOT / "docs/dogfood.md").read_text(encoding="utf-8")

        for text in [provider_dogfood, dogfood]:
            with self.subTest(document=text[:20]):
                self.assertIn("staged contract harness", text)
                self.assertIn("does not prove live LLM skill selection", text)
                self.assertIn("real provider invocation proof", text)

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
