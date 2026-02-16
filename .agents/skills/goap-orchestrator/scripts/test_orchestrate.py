#!/usr/bin/env python3
import json
import tempfile
import unittest
from pathlib import Path

import orchestrate


class OrchestrateTests(unittest.TestCase):
    def test_assign_specialist_prefers_implementation_for_generic_edge(self) -> None:
        task = "Implement remaining framework reliability edge cases"
        self.assertEqual(orchestrate.assign_specialist(task), "implementation-agent")

    def test_assign_specialist_for_wasm(self) -> None:
        task = "Verify wasm init/probe behavior"
        self.assertEqual(orchestrate.assign_specialist(task), "wasm-agent")

    def test_cli_outputs_board_files(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            tasks = root / "tasks.json"
            out_json = root / "board.json"
            out_md = root / "board.md"

            plan.write_text(json.dumps({"actions": [{"name": "analyze-repo"}]}), encoding="utf-8")
            tasks.write_text(json.dumps(["Add integration tests", "Prepare release commit"]), encoding="utf-8")

            code = orchestrate.main_cli(
                [
                    "--goal",
                    "release_ready",
                    "--plan",
                    str(plan),
                    "--tasks",
                    str(tasks),
                    "--out-json",
                    str(out_json),
                    "--out-md",
                    str(out_md),
                ]
            )
            self.assertEqual(code, 0)
            self.assertTrue(out_json.exists())
            self.assertTrue(out_md.exists())

            board = json.loads(out_json.read_text(encoding="utf-8"))
            self.assertEqual(board["goal"], "release_ready")
            self.assertIn("Add integration tests", board["specialists"]["test-agent"])
            self.assertIn("Prepare release commit", board["specialists"]["release-agent"])

    def test_cli_rejects_non_array_tasks(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            tasks = root / "tasks.json"
            out_json = root / "board.json"
            out_md = root / "board.md"

            plan.write_text(json.dumps({"actions": []}), encoding="utf-8")
            tasks.write_text(json.dumps({"task": "bad-shape"}), encoding="utf-8")

            with self.assertRaises(SystemExit) as raised:
                orchestrate.main_cli(
                    [
                        "--goal",
                        "release_ready",
                        "--plan",
                        str(plan),
                        "--tasks",
                        str(tasks),
                        "--out-json",
                        str(out_json),
                        "--out-md",
                        str(out_md),
                    ]
                )
            self.assertIn("tasks file must be a JSON array", str(raised.exception))

    def test_main_accepts_explicit_argv(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            plan = root / "plan.json"
            tasks = root / "tasks.json"
            out_json = root / "board.json"
            out_md = root / "board.md"

            plan.write_text(json.dumps({"actions": [{"name": "verify"}]}), encoding="utf-8")
            tasks.write_text(json.dumps(["Add test assertions"]), encoding="utf-8")

            code = orchestrate.main(
                [
                    "--goal",
                    "release_ready",
                    "--plan",
                    str(plan),
                    "--tasks",
                    str(tasks),
                    "--out-json",
                    str(out_json),
                    "--out-md",
                    str(out_md),
                ]
            )

            self.assertEqual(code, 0)
            self.assertTrue(out_json.exists())
            self.assertTrue(out_md.exists())


if __name__ == "__main__":
    unittest.main()
