import json
import unittest
from pathlib import Path


PACK_ROOT = Path(__file__).resolve().parents[1]
REFERENCES = PACK_ROOT / "references"


class GroundLineReferenceContractTests(unittest.TestCase):
    def read_reference(self, name: str) -> str:
        path = REFERENCES / name
        self.assertTrue(path.is_file(), f"missing reference: references/{name}")
        return path.read_text(encoding="utf-8")

    def test_runtime_matrix_supports_only_current_three_runtimes(self) -> None:
        text = self.read_reference("runtime-matrix.md")

        for required in ["Codex", "Claude Code", "Antigravity"]:
            self.assertIn(required, text)

        forbidden = ["Gemini CLI", "Cursor", "Roo Code", "Cline", "Copilot"]
        for item in forbidden:
            with self.subTest(item=item):
                self.assertNotIn(item, text)

        self.assertIn(".codex-plugin/plugin.json", text)
        self.assertIn(".claude-plugin/plugin.json", text)
        self.assertIn("plugin.json", text)

    def test_runtime_matrix_excludes_provider_native_reimplementation(self) -> None:
        text = self.read_reference("runtime-matrix.md")

        for term in [
            "provider-native",
            "do not reimplement",
            "capability boundary",
            "provider-neutral contracts",
            "runtime-specific affordances",
            "document and verify",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_platform_support_is_macos_apple_silicon_and_linux_only(self) -> None:
        text = self.read_reference("platform-support.md")

        self.assertIn("macOS", text)
        self.assertIn("Apple Silicon", text)
        self.assertIn("Linux", text)
        self.assertIn("linux/arm64", text)

        for forbidden in ["Windows", "WSL", "PowerShell", "CMD"]:
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, text)

    def test_config_sync_boundary_keeps_runtime_state_out(self) -> None:
        text = self.read_reference("config-sync-boundary.md")

        included = [
            "AGENTS.md",
            "skills/",
            "rules/",
            "tool profiles",
            "provider manifests",
        ]
        excluded = [
            "auth.json",
            "sessions/",
            "archived_sessions/",
            "shell_snapshots/",
            "plugin caches",
            "OAuth state",
            "secret values",
            "sqlite",
        ]

        for item in included:
            with self.subTest(include=item):
                self.assertIn(item, text)
        for item in excluded:
            with self.subTest(exclude=item):
                self.assertIn(item, text)

    def test_workflow_modes_cover_companion_standalone_and_external_stack(self) -> None:
        text = self.read_reference("workflow-modes.md")

        for mode in ["companion-superpowers", "standalone-groundline", "external-stack"]:
            with self.subTest(mode=mode):
                self.assertIn(mode, text)

        self.assertIn("orient -> bound -> act -> prove -> handoff", text)
        self.assertIn("handoff packet", text.lower())

    def test_superpowers_interop_separates_responsibilities(self) -> None:
        text = self.read_reference("superpowers-interop.md")

        self.assertIn("reconcile-current-state", text)
        self.assertIn("verification-before-completion", text)
        self.assertIn("close-live-work", text)
        self.assertIn("does not replace Superpowers", text)

    def test_tool_profiles_keep_mcp_optional(self) -> None:
        text = self.read_reference("tool-profiles.md")

        for tool in ["GitHub", "Context7", "Exa"]:
            with self.subTest(tool=tool):
                self.assertIn(tool, text)

        self.assertIn("Strict Local", text)
        self.assertIn("no external calls", text)
        self.assertIn("does not enable MCP", text)
        self.assertIn("provider-native", text)
        self.assertIn("setup recommendation", text)

    def test_output_contracts_define_assessment_radar_and_boundary(self) -> None:
        text = self.read_reference("output-contracts.md")

        required_terms = [
            "GroundLine Assessment",
            "GroundLine Radar",
            "Boundary",
            "GroundLine Research",
            "GroundLine Capability Evaluation",
            "GroundLine AI Usage Maturity",
            "GroundLine Task Packet",
            "GroundLine Scope Hold",
            "GroundLine Release Polish",
            "GroundLine Release Cut",
            "GroundLine Release Delta",
            "GroundLine Comparison",
            "GroundLine Recommendation",
            "GroundLine Pack Evaluation",
            "current conclusion",
            "verified state",
            "changed sources",
            "upgrade task candidates",
            "skill completeness",
            "trigger clarity",
            "verification strength",
            "source evidence",
            "context cost",
            "security risk",
            "overall score",
            "axis scores",
            "provider coverage",
            "evidence mode",
            "fluency overlay",
            "longitudinal comparison",
            "development edges",
            "evaluation method",
            "evidence-to-score map",
            "problem diagnosis",
            "improvement plan",
            "expansion trigger",
            "decision: finish_current|accept_with_budget|defer|watch|reject",
            "parked ideas",
            "polish scope",
            "cleanup findings",
            "commit split",
            "previous version",
            "deployed version",
            "delta checklist",
            "rollback note",
            "non-goals",
            "scope lock",
            "change budget",
            "release gates",
            "dogfood evidence",
            "next upgrades",
            "adopt",
            "adapt",
            "watch",
            "reject",
            "secret value printed: false",
        ]
        for term in required_terms:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_hold_the_line_defines_expansion_control_loop(self) -> None:
        skill = (PACK_ROOT / "skills/hold-the-line/SKILL.md").read_text(encoding="utf-8")
        index = json.loads((REFERENCES / "skill-index.json").read_text(encoding="utf-8"))
        portfolio = (PACK_ROOT / "docs/skill-portfolio.md").read_text(encoding="utf-8")
        llm_guide = (PACK_ROOT / "docs/llm-guide.md").read_text(encoding="utf-8")
        examples = (PACK_ROOT / "docs/examples.md").read_text(encoding="utf-8")

        required_terms = [
            "expansion pressure",
            "finish current is the default",
            "accept with budget",
            "one next action",
            "parked ideas",
            "do not start research",
            "new skill requires repeated failure",
            "GroundLine Scope Hold",
        ]
        for term in required_terms:
            with self.subTest(term=term):
                self.assertIn(term, skill)

        indexed = {item["name"]: item for item in index["skills"]}
        self.assertIn("hold-the-line", indexed)
        self.assertEqual(indexed["hold-the-line"]["workflow_stage"], "decide")
        self.assertIn("hold-the-line", portfolio)
        self.assertIn("hold-the-line", llm_guide)
        self.assertIn("hold-the-line", examples)

    def test_polish_release_candidate_defines_pre_ship_cleanup_loop(self) -> None:
        skill = (PACK_ROOT / "skills/polish-release-candidate/SKILL.md").read_text(encoding="utf-8")
        index = json.loads((REFERENCES / "skill-index.json").read_text(encoding="utf-8"))
        portfolio = (PACK_ROOT / "docs/skill-portfolio.md").read_text(encoding="utf-8")
        llm_guide = (PACK_ROOT / "docs/llm-guide.md").read_text(encoding="utf-8")
        examples = (PACK_ROOT / "docs/examples.md").read_text(encoding="utf-8")

        required_terms = [
            "docs polish",
            "duplicate cleanup",
            "stale references",
            "privacy sweep",
            "identity sweep",
            "gate order",
            "commit split",
            "public readiness",
            "no new capability",
            "GroundLine Release Polish",
        ]
        for term in required_terms:
            with self.subTest(term=term):
                self.assertIn(term, skill)

        indexed = {item["name"]: item for item in index["skills"]}
        self.assertIn("polish-release-candidate", indexed)
        self.assertEqual(indexed["polish-release-candidate"]["workflow_stage"], "verify")
        self.assertIn("polish-release-candidate", portfolio)
        self.assertIn("polish-release-candidate", llm_guide)
        self.assertIn("polish-release-candidate", examples)

    def test_compare_release_delta_defines_post_deploy_checklist(self) -> None:
        skill = (PACK_ROOT / "skills/compare-release-delta/SKILL.md").read_text(encoding="utf-8")
        index = json.loads((REFERENCES / "skill-index.json").read_text(encoding="utf-8"))
        portfolio = (PACK_ROOT / "docs/skill-portfolio.md").read_text(encoding="utf-8")
        llm_guide = (PACK_ROOT / "docs/llm-guide.md").read_text(encoding="utf-8")
        examples = (PACK_ROOT / "docs/examples.md").read_text(encoding="utf-8")

        required_terms = [
            "previous version",
            "deployed version",
            "delta checklist",
            "expected changes",
            "unexpected changes",
            "runtime evidence",
            "rollback note",
            "post-deploy",
            "GroundLine Release Delta",
        ]
        for term in required_terms:
            with self.subTest(term=term):
                self.assertIn(term, skill)

        indexed = {item["name"]: item for item in index["skills"]}
        self.assertIn("compare-release-delta", indexed)
        self.assertEqual(indexed["compare-release-delta"]["workflow_stage"], "verify")
        self.assertIn("compare-release-delta", portfolio)
        self.assertIn("compare-release-delta", llm_guide)
        self.assertIn("compare-release-delta", examples)

    def test_agent_task_packet_defines_llm_ready_context_packaging(self) -> None:
        text = self.read_reference("agent-task-packet.md")

        for term in [
            "task packet",
            "current conclusion",
            "goal",
            "context",
            "constraints",
            "non-goals",
            "mutation boundary",
            "verification",
            "handoff",
            "success criteria",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_release_stabilization_defines_scope_lock_and_dogfood_gates(self) -> None:
        text = self.read_reference("release-stabilization.md")

        for term in [
            "scope lock",
            "change budget",
            "release gates",
            "dogfood evidence",
            "provider smoke",
            "must fix",
            "defer",
            "ship decision",
            "regression check",
            "public release",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_release_discipline_surfaces_align_on_expansion_control(self) -> None:
        release = self.read_reference("release-stabilization.md")
        stabilize = (PACK_ROOT / "skills/stabilize-release-cut/SKILL.md").read_text(encoding="utf-8")
        recommend = (PACK_ROOT / "skills/recommend-groundline-upgrades/SKILL.md").read_text(encoding="utf-8")
        curate = (PACK_ROOT / "skills/curate-groundline-skills/SKILL.md").read_text(encoding="utf-8")
        dogfood = (PACK_ROOT / "docs/dogfood.md").read_text(encoding="utf-8")

        required_by_surface = {
            "release": [
                "Expansion Classifier",
                "release blocker evidence",
                "watch is the default",
                "must fix requires",
                "new idea cannot enter release scope without classification",
            ],
            "stabilize": [
                "Classify new ideas before accepting scope",
                "must fix requires release-blocking evidence",
                "watch is the default",
                "Do not add a new skill during stabilization",
            ],
            "recommend": [
                "watch is the default",
                "promising but unproven",
                "failure evidence",
            ],
            "curate": [
                "New Skill Gate",
                "repeated work or dogfood",
                "too broad",
                "output contract",
            ],
            "dogfood": [
                "new skills derived from dogfood results",
                "dogfood or repeated failure evidence",
                "release scope",
            ],
        }
        surfaces = {
            "release": release,
            "stabilize": stabilize,
            "recommend": recommend,
            "curate": curate,
            "dogfood": dogfood,
        }

        for surface, terms in required_by_surface.items():
            for term in terms:
                with self.subTest(surface=surface, term=term):
                    self.assertIn(term, surfaces[surface])

    def test_capability_blueprint_defines_desired_capabilities_not_full_config(self) -> None:
        text = self.read_reference("capability-blueprint.md")

        self.assertIn("capability blueprint", text.lower())
        self.assertIn("setup profile", text.lower())
        self.assertIn("not complete runtime config", text.lower())
        self.assertIn("doctor", text.lower())
        self.assertIn("radar", text.lower())

    def test_source_registry_tracks_only_groundline_sources(self) -> None:
        path = REFERENCES / "source-registry.json"
        self.assertTrue(path.is_file(), "missing reference: references/source-registry.json")

        data = json.loads(path.read_text(encoding="utf-8"))
        sources = data.get("sources")
        self.assertIsInstance(sources, list)
        source_ids = {source.get("id") for source in sources}

        expected = {
            "codex-docs",
            "claude-code-docs",
            "antigravity-docs",
            "superpowers",
            "spec-kit",
            "agent-os",
            "bmad-method",
            "gstack",
            "grill-me",
            "promptfoo-coding-agent-redteam",
            "ai-fluency-assessment-skill",
            "mcpmarket-ai-fluency-assessment",
            "paolomoz-ai-fluency",
            "github",
            "context7",
            "exa",
        }
        self.assertTrue(expected.issubset(source_ids))

        forbidden_fragments = ["gemini-cli", "cursor", "roo", "cline", "copilot"]
        for source in sources:
            serialized = json.dumps(source).lower()
            for fragment in forbidden_fragments:
                with self.subTest(source=source.get("id"), fragment=fragment):
                    self.assertNotIn(fragment, serialized)

    def test_external_workflow_interop_supports_research_compare_recommend(self) -> None:
        text = self.read_reference("external-workflow-interop.md")

        for section in ["Research", "Compare", "Recommend", "Adoption Rules"]:
            with self.subTest(section=section):
                self.assertIn(section, text)

        for source in ["Spec Kit", "Agent OS", "BMAD", "Superpowers", "gstack", "grill-me", "Promptfoo"]:
            with self.subTest(source=source):
                self.assertIn(source, text)

        for decision in ["adopt", "adapt", "watch", "reject"]:
            with self.subTest(decision=decision):
                self.assertIn(decision, text)

    def test_capability_evaluation_defines_existing_tool_and_skill_rubric(self) -> None:
        text = self.read_reference("capability-evaluation.md")

        for term in [
            "existing tools",
            "existing skills",
            "plugins",
            "MCP servers",
            "hooks",
            "agents",
            "source evidence",
            "context cost",
            "security risk",
            "maintenance signal",
            "GroundLine fit",
            "adopt",
            "adapt",
            "watch",
            "reject",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_ai_usage_maturity_defines_private_evidence_based_rubric(self) -> None:
        text = self.read_reference("ai-usage-maturity.md")

        for term in [
            "task framing",
            "context packaging",
            "tool orchestration",
            "agent delegation",
            "verification discipline",
            "reuse",
            "automation leverage",
            "safety",
            "impact",
            "agent-native operator",
            "raw transcripts",
            "evaluation method",
            "evidence-to-score map",
            "provider coverage",
            "evidence mode",
            "fluency overlay",
            "Delegation",
            "Description",
            "Discernment",
            "Diligence",
            "longitudinal comparison",
            "development edges",
            "artifact passivity",
            "quarterly",
            "problem diagnosis",
            "improvement plan",
            "priority order",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_multi_provider_fluency_boundary_separates_modes(self) -> None:
        text = self.read_reference("multi-provider-fluency-boundary.md")

        for term in [
            "artifact-backed maturity",
            "collector-backed fluency",
            "Codex",
            "Claude Code",
            "Antigravity",
            "redacted evidence packet",
            "raw transcript boundary",
            "no automatic collection",
            "explicit approval",
            "behavior extraction",
            "CEFR",
            "24 behaviors",
            "12 sub-competencies",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_multi_provider_fluency_boundary_defines_provider_evidence_packet(self) -> None:
        text = self.read_reference("multi-provider-fluency-boundary.md")

        for term in [
            "Provider Evidence Packet",
            "provider:",
            "time window:",
            "projects touched:",
            "task types:",
            "autonomy level:",
            "tools used:",
            "verification evidence:",
            "handoff evidence:",
            "repeated failure patterns:",
            "approved excerpts: none|redacted",
            "privacy boundary:",
            "inventory-only",
            "redacted summary",
            "no secret values",
            "full raw message scan",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_skill_lifecycle_defines_human_and_llm_taxonomy(self) -> None:
        text = self.read_reference("skill-lifecycle.md")

        for term in [
            "workflow_stage",
            "artifact_type",
            "risk_level",
            "provider_scope",
            "lifecycle",
            "Human-readable",
            "LLM-readable",
            "merge",
            "split",
            "deprecate",
        ]:
            with self.subTest(term=term):
                self.assertIn(term, text)

    def test_skill_index_matches_skill_directories_and_categories(self) -> None:
        path = REFERENCES / "skill-index.json"
        self.assertTrue(path.is_file(), "missing reference: references/skill-index.json")
        data = json.loads(path.read_text(encoding="utf-8"))
        skills = data.get("skills")
        self.assertIsInstance(skills, list)

        indexed_names = {item.get("name") for item in skills}
        actual_names = {path.name for path in (PACK_ROOT / "skills").iterdir() if path.is_dir()}
        self.assertEqual(indexed_names, actual_names)

        allowed = {
            "workflow_stage": {"orient", "research", "compare", "decide", "act", "verify", "handoff", "maintain"},
            "artifact_type": {"skill", "reference", "script", "agent", "hook", "mcp-recommendation", "docs"},
            "risk_level": {
                "read-only",
                "local-write",
                "provider-home-write",
                "remote-write",
                "production",
                "access-billing",
                "secret-sensitive",
            },
            "provider_scope": {"provider-neutral", "codex", "claude-code", "antigravity"},
            "lifecycle": {"candidate", "active", "experimental", "deprecated", "merged", "rejected"},
        }

        for item in skills:
            with self.subTest(skill=item.get("name")):
                for key, values in allowed.items():
                    self.assertIn(item.get(key), values)
                self.assertIsInstance(item.get("human_summary"), str)
                self.assertIsInstance(item.get("llm_trigger"), str)
                self.assertTrue(item["human_summary"])
                self.assertTrue(item["llm_trigger"].startswith("Use when"))

    def test_skill_portfolio_is_human_readable_and_lists_indexed_skills(self) -> None:
        text = (PACK_ROOT / "docs/skill-portfolio.md").read_text(encoding="utf-8")
        index = json.loads((REFERENCES / "skill-index.json").read_text(encoding="utf-8"))

        for heading in ["How To Read This", "Skill Portfolio", "Lifecycle Notes"]:
            with self.subTest(heading=heading):
                self.assertIn(heading, text)

        for item in index["skills"]:
            with self.subTest(skill=item["name"]):
                self.assertIn(f"`{item['name']}`", text)


if __name__ == "__main__":
    unittest.main()
