import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


PACK_ROOT = Path(__file__).resolve().parents[1]
PLAN_UPDATE = PACK_ROOT / "scripts/groundline_plan_update.py"


class GroundLinePlanUpdateContractTests(unittest.TestCase):
    def run_plan_update(self, payload: dict) -> dict:
        self.assertTrue(PLAN_UPDATE.is_file(), "missing script: scripts/groundline_plan_update.py")
        with tempfile.TemporaryDirectory(prefix="groundline-plan-") as temp:
            input_path = Path(temp) / "input.json"
            input_path.write_text(json.dumps(payload), encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(PLAN_UPDATE), "--input", str(input_path), "--json"],
                cwd=PACK_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

        self.assertEqual(
            completed.returncode,
            0,
            f"plan update failed\nstdout:\n{completed.stdout}\nstderr:\n{completed.stderr}",
        )
        try:
            return json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            self.fail(f"groundline_plan_update.py did not emit JSON: {exc}\nstdout:\n{completed.stdout}")

    def test_converts_doctor_gap_into_upgrade_packet(self) -> None:
        payload = {
            "kind": "doctor",
            "recommended_mode": "standalone-groundline",
            "capability_gaps": [
                {
                    "id": "context7-missing",
                    "capability": "current docs research",
                    "recommended_source": "references/tool-profiles.md",
                }
            ],
            "runtimes": {
                "codex": {"scope": "supported", "present": True},
                "claude_code": {"scope": "supported", "present": True},
                "antigravity": {"scope": "supported", "present": True},
            },
            "mutation_performed": False,
            "network": "disabled",
        }

        result = self.run_plan_update(payload)

        self.assertEqual(result["kind"], "upgrade_packet")
        self.assertEqual(result["source_kind"], "doctor")
        self.assertFalse(result["mutation_performed"])
        self.assertIn("current_evidence", result)
        self.assertIn("suspected_drift", result)
        self.assertIn("docs_to_verify", result)
        self.assertIn("affected_groundline_files", result)
        self.assertIn("proposed_tasks", result)
        self.assertIn("verification_checklist", result)
        self.assertIn("references/tool-profiles.md", result["affected_groundline_files"])
        self.assertGreaterEqual(len(result["proposed_tasks"]), 1)

    def test_converts_radar_change_into_research_packet(self) -> None:
        payload = {
            "kind": "radar",
            "changed_sources": [
                {
                    "id": "claude-code-docs",
                    "owner": "Claude Code",
                    "old_version": "2026-05-26",
                    "new_version": "2026-05-27",
                    "url": "fixture://claude-code-docs",
                }
            ],
            "new_sources": [],
            "removed_sources": [],
            "mutation_performed": False,
            "network": "disabled",
        }

        result = self.run_plan_update(payload)

        self.assertEqual(result["kind"], "upgrade_packet")
        self.assertEqual(result["source_kind"], "radar")
        self.assertFalse(result["mutation_performed"])
        self.assertIn("claude-code-docs", json.dumps(result))
        self.assertIn("research_packet", result)
        self.assertIn("docs_to_verify", result)
        self.assertIn("verification_checklist", result)
        self.assertGreaterEqual(len(result["docs_to_verify"]), 1)

    def test_plan_update_rejects_secret_values(self) -> None:
        payload = {
            "kind": "doctor",
            "capability_gaps": [],
            "diagnostics": {
                "accidental_secret": "sk-test-secret-value",
            },
            "mutation_performed": False,
            "network": "disabled",
        }

        self.assertTrue(PLAN_UPDATE.is_file(), "missing script: scripts/groundline_plan_update.py")
        with tempfile.TemporaryDirectory(prefix="groundline-plan-") as temp:
            input_path = Path(temp) / "input.json"
            input_path.write_text(json.dumps(payload), encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(PLAN_UPDATE), "--input", str(input_path), "--json"],
                cwd=PACK_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

        self.assertNotEqual(completed.returncode, 0)
        self.assertNotIn("sk-test-secret-value", completed.stdout + completed.stderr)
        payload = json.loads(completed.stdout)
        self.assertEqual(payload["status"], "FAIL")
        self.assertEqual(payload["error"], "secret-like input rejected")

    def test_plan_update_rejects_unknown_input_kind(self) -> None:
        payload = {
            "kind": "unknown",
            "mutation_performed": False,
            "network": "disabled",
        }

        self.assertTrue(PLAN_UPDATE.is_file(), "missing script: scripts/groundline_plan_update.py")
        with tempfile.TemporaryDirectory(prefix="groundline-plan-") as temp:
            input_path = Path(temp) / "input.json"
            input_path.write_text(json.dumps(payload), encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(PLAN_UPDATE), "--input", str(input_path), "--json"],
                cwd=PACK_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

        self.assertNotEqual(completed.returncode, 0)
        result = json.loads(completed.stdout)
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["error"], "unsupported input kind")


if __name__ == "__main__":
    unittest.main()
