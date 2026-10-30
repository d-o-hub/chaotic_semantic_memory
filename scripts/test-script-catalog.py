#!/usr/bin/env python3
"""Behavior fixtures for script catalog classifications."""

from __future__ import annotations

import pathlib
import tempfile
import unittest

from gen_script_catalog import render


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


if __name__ == "__main__":
    unittest.main()
