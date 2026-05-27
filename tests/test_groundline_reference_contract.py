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

    def test_output_contracts_define_assessment_radar_and_boundary(self) -> None:
        text = self.read_reference("output-contracts.md")

        required_terms = [
            "GroundLine Assessment",
            "GroundLine Radar",
            "Boundary",
            "current conclusion",
            "verified state",
            "changed sources",
            "upgrade task candidates",
            "secret value printed: false",
        ]
        for term in required_terms:
            with self.subTest(term=term):
                self.assertIn(term, text)

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


if __name__ == "__main__":
    unittest.main()
