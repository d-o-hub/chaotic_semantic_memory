#!/usr/bin/env python3
"""
LLM Context Understanding Validation + drift gate (issue #828).

Part 1 asserts that an LLM can answer key questions from
docs/architecture/context.yaml (constraints, gates, performance targets,
workflow, data flow).

Part 2 is the drift gate: every counter and path in that file must match a
live source, because the artifact drifted silently for months (skills 19 vs
33, ADRs 16 vs 95, active_wave 11 vs 33, tests 134 vs 1044, plus two dead
key-file paths) while nothing read it:

    skills.total_count            .agents/skills/**/SKILL.md on disk
    skills.categories             .agents/skills/CATALOG.md section headers
    current_status.swarm_groups   CATALOG.md `Swarm` count
    key_files.adrs[].count        plans/adr/*.md
    current_status.active_wave    plans/GOAP_STATE.md `active_wave`
    current_status.total_tests    plans/GOAP_STATE.md `tests_count`
    modules_under_loc_limit       src/**/*.rs count (the LOC gate keeps them
                                  all <= 500, so both numbers equal the count)
    architecture.modules[].file   path existence
    key_files.*[].path            path existence

Usage:
    python3 tests/validate_llm_context.py [--repo-root DIR]

Exit codes: 0 all checks pass, 1 any drift/failure, 2 missing input or
dependency (PyYAML).
"""

import argparse
import re
import sys
from pathlib import Path
from typing import Dict, List, Tuple

try:
    import yaml
except ImportError:  # pragma: no cover - exercised by the fixture's skip path
    print(
        "ERROR: PyYAML is not importable by python3 (apt package: python3-yaml).",
        file=sys.stderr,
    )
    sys.exit(2)


class LLMContextValidator:
    """Validates LLM understanding of the codebase context and its drift."""

    def __init__(self, repo_root: Path):
        self.repo_root = repo_root
        self.context_file = repo_root / "docs" / "architecture" / "context.yaml"
        if not self.context_file.is_file():
            print(f"ERROR: {self.context_file} is missing.", file=sys.stderr)
            sys.exit(2)
        with self.context_file.open() as f:
            self.context = yaml.safe_load(f)
        self.results: List[Tuple[str, bool, str]] = []

    # -- live sources ----------------------------------------------------
    def _live_skill_count(self) -> int:
        skills = self.repo_root / ".agents" / "skills"
        return len(list(skills.rglob("SKILL.md")))

    def _catalog_counts(self) -> Dict[str, int]:
        catalog = self.repo_root / ".agents" / "skills" / "CATALOG.md"
        counts: Dict[str, int] = {}
        if not catalog.is_file():
            return counts
        for line in catalog.read_text().splitlines():
            match = re.match(r"^##\s+(.+?)\s+\((\d+)\)\s*$", line)
            if match:
                counts[match.group(1).strip().lower()] = int(match.group(2))
        return counts

    def _goap_values(self) -> Dict[str, int]:
        state = self.repo_root / "plans" / "GOAP_STATE.md"
        values: Dict[str, int] = {}
        if not state.is_file():
            return values
        for line in state.read_text().splitlines():
            match = re.match(r"^\s{2}([a-z_]+):\s*(\d+)\b", line)
            if match:
                values[match.group(1)] = int(match.group(2))
        return values

    def _adr_count(self) -> int:
        return len(list((self.repo_root / "plans" / "adr").glob("*.md")))

    def _src_file_count(self) -> int:
        return len(list((self.repo_root / "src").rglob("*.rs")))

    def _key_file_paths(self) -> List[str]:
        paths: List[str] = []

        def walk(node):
            if isinstance(node, dict):
                if "path" in node and isinstance(node["path"], str):
                    paths.append(node["path"])
                else:
                    for value in node.values():
                        walk(value)
            elif isinstance(node, list):
                for item in node:
                    walk(item)

        walk(self.context.get("key_files", {}))
        return paths

    def _record(self, name: str, ok: bool, message: str) -> bool:
        self.results.append((name, ok, message))
        return ok

    # -- structural tests (LLM understanding) ----------------------------
    def test_constraint_retrieval(self) -> bool:
        hard = self.context["constraints"]["hard"]
        constraint = next((c for c in hard if c["name"] == "max_loc_per_file"), None)
        if constraint and constraint["value"] == 500:
            return self._record("Constraint Retrieval", True, "Found max_loc_per_file = 500")
        return self._record("Constraint Retrieval", False, "Could not find LOC constraint")

    def test_database_requirement(self) -> bool:
        hard = self.context["constraints"]["hard"]
        constraint = next((c for c in hard if c["name"] == "database_libsql"), None)
        if constraint and constraint.get("value") == "libsql":
            forbidden = constraint.get("forbidden", "")
            return self._record(
                "Database Requirement", True,
                f"Must use {constraint['value']}, never {forbidden}",
            )
        return self._record("Database Requirement", False, "Could not find database constraint")

    def test_validation_script(self) -> bool:
        script = self.context["validation"]["script"]
        gates = len(self.context["validation"]["gates"])
        if script == "scripts/validate.sh" and gates >= 5:
            return self._record("Validation Script", True, f"Script: {script}, Gates: {gates}")
        return self._record(
            "Validation Script", False, f"Unexpected: {script} with {gates} gates"
        )

    def test_performance_targets(self) -> bool:
        targets = self.context["validation"]["performance_targets"]
        target = next((t for t in targets if t["name"] == "reservoir_step_50k"), None)
        if target and target["target"] == "< 100μs":
            status = target.get("status", "unknown")
            measured = target.get("latest_measured", "N/A")
            return self._record(
                "Performance Targets", True,
                f"reservoir_step_50k: {target['target']} (measured: {measured}, status: {status})",
            )
        return self._record("Performance Targets", False, "Could not find reservoir target")

    def test_wasm_gating(self) -> bool:
        hard = self.context["constraints"]["hard"]
        constraint = next((c for c in hard if c["name"] == "wasm_threading"), None)
        if constraint and '#[cfg(not(target_arch = "wasm32"))]' in constraint.get("gate", ""):
            return self._record("WASM Gating", True, f"Gate: {constraint['gate']}")
        return self._record("WASM Gating", False, "Could not find WASM constraint")

    def test_workflow_steps(self) -> bool:
        loop = self.context["workflow"]["learning_loop"]
        if len(loop) == 5:
            steps = [s["action"] for s in loop]
            return self._record("Workflow Steps", True, f"Found {len(steps)} steps")
        return self._record(
            "Workflow Steps", False, f"Expected 5 steps, found {len(loop)}"
        )

    def test_data_flow(self) -> bool:
        flows = self.context["data_flow"]["flows"]
        flow = next((f for f in flows if f["name"] == "concept_injection"), None)
        if flow and len(flow["steps"]) >= 3:
            return self._record(
                "Data Flow", True, f"Concept injection has {len(flow['steps'])} steps"
            )
        return self._record("Data Flow", False, "Could not find concept injection flow")

    # -- drift tests (live sources) --------------------------------------
    def test_module_files(self) -> bool:
        modules = self.context["architecture"]["modules"]
        missing = [
            f"{m['name']}:{m['file']}"
            for m in modules
            if not (self.repo_root / m["file"]).exists()
        ]
        if missing:
            return self._record("Module Files", False, "Missing: " + ", ".join(missing))
        return self._record(
            "Module Files", True, f"All {len(modules)} module paths exist"
        )

    def test_skill_count(self) -> bool:
        declared = self.context["skills"]["total_count"]
        live = self._live_skill_count()
        if declared == live:
            return self._record("Skill Count", True, f"context.yaml {declared} == on-disk {live}")
        return self._record(
            "Skill Count", False, f"context.yaml says {declared}, disk has {live}"
        )

    def test_skill_categories(self) -> bool:
        categories = {c["name"]: c["count"] for c in self.context["skills"]["categories"]}
        total = self.context["skills"]["total_count"]
        catalog = self._catalog_counts()
        problems: List[str] = []
        if sum(categories.values()) != total:
            problems.append(f"category sum {sum(categories.values())} != total {total}")
        for name, count in categories.items():
            if name not in catalog:
                problems.append(f"{name}: no '## {name.title()} (n)' header in CATALOG.md")
            elif catalog[name] != count:
                problems.append(f"{name}: context.yaml {count} != CATALOG.md {catalog[name]}")
        for name, count in catalog.items():
            if name not in categories:
                problems.append(f"CATALOG.md section {name} ({count}) absent from context.yaml")
        swarm_groups = self.context["current_status"].get("swarm_groups")
        if "swarm" in categories and swarm_groups != categories["swarm"]:
            problems.append(
                f"swarm_groups {swarm_groups} != swarm category {categories['swarm']}"
            )
        if problems:
            return self._record("Skill Categories", False, "; ".join(problems))
        return self._record(
            "Skill Categories", True, "categories and swarm_groups match CATALOG.md"
        )

    def test_adr_count(self) -> bool:
        entries = self.context["key_files"]["adrs"]
        declared = next((e.get("count") for e in entries if "count" in e), None)
        live = self._adr_count()
        if declared == live:
            return self._record("ADR Count", True, f"context.yaml {declared} == plans/adr/*.md {live}")
        return self._record(
            "ADR Count", False, f"context.yaml says {declared}, plans/adr/*.md has {live}"
        )

    def test_active_wave(self) -> bool:
        declared = self.context["current_status"]["active_wave"]
        live = self._goap_values().get("active_wave")
        if live is None:
            return self._record("Active Wave", False, "plans/GOAP_STATE.md has no `active_wave:` key")
        if declared == live:
            return self._record("Active Wave", True, f"context.yaml {declared} == GOAP_STATE.md {live}")
        return self._record(
            "Active Wave", False, f"context.yaml says {declared}, GOAP_STATE.md says {live}"
        )

    def test_test_counts(self) -> bool:
        status = self.context["current_status"]
        live = self._goap_values().get("tests_count")
        if live is None:
            return self._record("Test Counts", False, "plans/GOAP_STATE.md has no `tests_count:` key")
        problems = []
        for key in ("total_tests", "tests_passing"):
            if status.get(key) != live:
                problems.append(f"{key} {status.get(key)} != GOAP_STATE tests_count {live}")
        if problems:
            return self._record("Test Counts", False, "; ".join(problems))
        return self._record("Test Counts", True, f"total_tests == tests_passing == {live}")

    def test_loc_coverage(self) -> bool:
        declared = str(self.context["current_status"]["modules_under_loc_limit"])
        match = re.match(r"^(\d+)\s*/\s*(\d+)$", declared.strip())
        if not match:
            return self._record(
                "LOC Coverage", False, f"unparsable modules_under_loc_limit: {declared!r}"
            )
        under, total = int(match.group(1)), int(match.group(2))
        live = self._src_file_count()
        if under == total == live:
            return self._record("LOC Coverage", True, f"src/**/*.rs {live} == {declared}")
        return self._record(
            "LOC Coverage",
            False,
            f"modules_under_loc_limit {declared} != live src/**/*.rs count {live}",
        )

    def test_key_files(self) -> bool:
        paths = self._key_file_paths()
        missing = [p for p in paths if not (self.repo_root / p).exists()]
        if missing:
            return self._record("Key Files", False, "Missing: " + ", ".join(missing))
        return self._record("Key Files", True, f"All {len(paths)} key-file paths exist")

    # -- runner ----------------------------------------------------------
    def run_all_tests(self) -> Dict[str, object]:
        tests = [
            self.test_constraint_retrieval,
            self.test_database_requirement,
            self.test_validation_script,
            self.test_performance_targets,
            self.test_wasm_gating,
            self.test_workflow_steps,
            self.test_data_flow,
            self.test_module_files,
            self.test_skill_count,
            self.test_skill_categories,
            self.test_adr_count,
            self.test_active_wave,
            self.test_test_counts,
            self.test_loc_coverage,
            self.test_key_files,
        ]
        passed = sum(1 for test in tests if test())
        total = len(tests)
        return {
            "total": total,
            "passed": passed,
            "failed": total - passed,
            "score": (passed / total) * 100,
            "results": self.results,
        }

    @staticmethod
    def print_report(results: Dict[str, object]) -> bool:
        print("=" * 70)
        print("LLM CONTEXT UNDERSTANDING VALIDATION")
        print("=" * 70)
        print()
        for test_name, passed, message in results["results"]:
            status = "✓ PASS" if passed else "✗ FAIL"
            print(f"{status:8} | {test_name:25} | {message}")
        print()
        print("-" * 70)
        print(
            f"Results: {results['passed']}/{results['total']} tests passed "
            f"({results['score']:.1f}%)"
        )
        print("-" * 70)
        return results["failed"] == 0


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        default=str(Path(__file__).resolve().parents[1]),
        help="repository root that holds docs/architecture/context.yaml (default: this repo)",
    )
    args = parser.parse_args()
    repo_root = Path(args.repo_root).resolve()

    print("Validating LLM context understanding and drift...\n")
    validator = LLMContextValidator(repo_root)
    results = validator.run_all_tests()
    success = validator.print_report(results)

    print("\n" + "=" * 70)
    print("CONTEXT FILE STRUCTURE")
    print("=" * 70)
    required_sections = [
        "metadata", "mission", "constraints", "architecture",
        "data_flow", "key_files", "skills", "validation",
        "workflow", "current_status", "guardrails",
    ]
    for section in required_sections:
        if section in validator.context:
            print(f"✓ {section}")
        else:
            print(f"✗ {section} - MISSING")
            success = False

    print("\n" + "=" * 70)
    print("TOKEN EFFICIENCY ANALYSIS")
    print("=" * 70)
    yaml_size = validator.context_file.stat().st_size
    drawio = repo_root / "docs" / "architecture" / "agents-context.drawio"
    agents = repo_root / "AGENTS.md"
    drawio_size = drawio.stat().st_size if drawio.is_file() else 0
    agents_size = agents.stat().st_size if agents.is_file() else 0
    print(f"context.yaml:    {yaml_size:6} bytes (LLM-optimized)")
    if agents_size:
        print(f"AGENTS.md:       {agents_size:6} bytes (text reference)")
        print(f"\nYAML is {agents_size / yaml_size:.1f}x more compact than AGENTS.md")
    if drawio_size:
        print(f"agents-context.drawio: {drawio_size:6} bytes (visual only)")
        print(f"YAML is {drawio_size / yaml_size:.1f}x more compact than draw.io XML")

    if success:
        print("\n✅ LLM context is drift-free and fully understandable!")
    else:
        print("\n❌ LLM context drifted from live sources (see FAIL rows above)")
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
