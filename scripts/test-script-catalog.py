#!/usr/bin/env python3
"""Behavior fixtures for script catalog classifications."""

from __future__ import annotations

import pathlib
import tempfile
import unittest

from gen_script_catalog import render, wiring_violations


class ScriptCatalogTests(unittest.TestCase):
    def test_classifies_owner_caller_fixture_and_mutations(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            scripts = root / "scripts"
            workflows = root / ".github" / "workflows"
            scripts.mkdir()
            workflows.mkdir(parents=True)
            (scripts / "deploy.sh").write_text(
                "#!/usr/bin/env bash\n# Deploy the package.\nmkdir -p release\ngit add release\ngit push origin release\n",
                encoding="utf-8",
            )
            (scripts / "test-deploy.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")
            (workflows / "release.yml").write_text("run: scripts/deploy.sh\n", encoding="utf-8")

            catalog = render(root)
            self.assertIn("| `scripts/deploy.sh` | repository automation | Deploy the package.", catalog)
            self.assertIn("| detected | local + remote | `.github/workflows/release.yml` |", catalog)
            self.assertIn("scripts/test-deploy.sh", catalog)
            self.assertIn("called + fixture", catalog)

    def test_marks_unreferenced_script_operator_only(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            scripts = root / "scripts"
            scripts.mkdir()
            (scripts / "inspect.py").write_text(
                "#!/usr/bin/env python3\n\"\"\"Inspect state.\"\"\"\nprint('read-only')\n",
                encoding="utf-8",
            )

            catalog = render(root)
            self.assertIn("operator-only (no caller/fixture detected)", catalog)
            self.assertIn("none detected | none detected | none detected | none found", catalog)

    def test_resolves_hyphen_underscore_fixture_variants(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            scripts = root / "scripts"
            workflows = root / ".github" / "workflows"
            scripts.mkdir()
            workflows.mkdir(parents=True)

            cases = {
                "mutation_union.py": "test-mutation-union.sh",
                "mutation-inventory.py": "test-mutation_inventory.sh",
                "action_case_name.py": "test-action_case_name.sh",
            }
            for script_name, fixture_name in cases.items():
                (scripts / script_name).write_text("#!/usr/bin/env python3\n", encoding="utf-8")
                (scripts / fixture_name).write_text("#!/usr/bin/env bash\n", encoding="utf-8")
            (scripts / "test-action-case-name.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")
            workflow_calls = "".join(f"run: python3 scripts/{name}\n" for name in cases)
            (workflows / "ci.yml").write_text(workflow_calls, encoding="utf-8")

            catalog = render(root)
            for script_name, fixture_name in cases.items():
                row = next(
                    line for line in catalog.splitlines() if line.startswith(f"| `scripts/{script_name}` |")
                )
                columns = [column.strip() for column in row.split("|")]
                self.assertEqual(columns[7], f"scripts/{fixture_name}")
                self.assertEqual(columns[8], "called + fixture")


    def test_wiring_check_fails_unasserted_orphan(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            scripts = root / "scripts"
            scripts.mkdir()
            (scripts / "orphan.sh").write_text(
                "#!/usr/bin/env bash\n# Orphaned helper.\n", encoding="utf-8"
            )

            violations = wiring_violations(root)
            self.assertTrue(any("orphaned script" in item and "scripts/orphan.sh" in item for item in violations))

    def test_wiring_check_passes_asserted_orphan(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            scripts = root / "scripts"
            scripts.mkdir()
            (scripts / "orphan.sh").write_text(
                "#!/usr/bin/env bash\n# Orphaned helper.\n", encoding="utf-8"
            )
            (scripts / "SCRIPT_WIRING.md").write_text(
                "# Wiring\n\n| Script | Wired by |\n|---|---|\n"
                "| `scripts/orphan.sh` | documented operator command - quick-reference |\n",
                encoding="utf-8",
            )

            self.assertEqual(wiring_violations(root), [])

    def test_wiring_check_flags_stale_assertion_for_wired_script(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            scripts = root / "scripts"
            workflows = root / ".github" / "workflows"
            scripts.mkdir()
            workshops = workflows
            workshops.mkdir(parents=True)
            (scripts / "wired.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")
            (workflows / "ci.yml").write_text("run: scripts/wired.sh\n", encoding="utf-8")
            (scripts / "SCRIPT_WIRING.md").write_text(
                "# Wiring\n\n| Script | Wired by |\n|---|---|\n"
                "| `scripts/wired.sh` | operator-only claim that no longer holds |\n",
                encoding="utf-8",
            )

            violations = wiring_violations(root)
            self.assertTrue(any("stale assertion" in item and "wired.sh" in item for item in violations))

    def test_wiring_check_flags_assertion_for_missing_script(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            scripts = root / "scripts"
            scripts.mkdir()
            (scripts / "SCRIPT_WIRING.md").write_text(
                "# Wiring\n\n| Script | Wired by |\n|---|---|\n"
                "| `scripts/ghost.sh` | removed but the row stayed |\n",
                encoding="utf-8",
            )

            violations = wiring_violations(root)
            self.assertTrue(any("ghost.sh" in item and "no longer exists" in item for item in violations))


if __name__ == "__main__":
    unittest.main()
