#!/usr/bin/env python3
"""Emit the module-level mutation inventory on stdout.

`scripts/mutation_test.sh` appends this output to its report:

    python3 "${SCRIPT_DIR}/mutation_inventory.py" >> "${REPORT_FILE}"

Reads `mutants.out/outcomes.json` — cargo-mutants' own machine-readable output
file, a fixed path relative to the repository root. No filesystem path is taken
from argv and there is no log-regex fallback: a missing or schema-mismatched
outcomes file is a hard error. The first version of this block lowercased the
`summary` value and compared it against `caught`/`missed`/..., which silently
produced all-zero caught/missed rows for files whose mutants were caught
(cargo-mutants 27.1.0 serializes the variant names `CaughtMutant` /
`MissedMutant`), and the non-empty `stats` suppressed the fallback, so a bad
parse still printed a plausible table. The review that found it is
plans/PR_ROAST_2026_10_09.md.

Schema (cargo-mutants 27.1.0, field `cargo_mutants_version`):
- each outcome: `scenario` is the string "Baseline" or {"Mutant": {...}},
  `summary` is one of CaughtMutant/MissedMutant/Timeout/Unviable/Success
  (Success = baseline);
- top level: `caught`/`missed`/`timeout`/`unviable`/`total_mutants` aggregates.

The aggregates are cross-checked against the per-file sums; the mismatch a
lowercase compare produces fails loudly here.

Fixture: scripts/test-mutation-inventory.sh
"""

from __future__ import annotations

import json
import pathlib
from collections import defaultdict

SUMMARY_MAP = {
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
}
AGGREGATE_KEYS = ("caught", "missed", "timeout", "unviable")
OUTCOMES_PATH = pathlib.Path("mutants.out/outcomes.json")


def empty_stats() -> dict[str, dict[str, int]]:
    return defaultdict(lambda: {"caught": 0, "missed": 0, "timeout": 0, "unviable": 0, "total": 0})


def build_stats(outcomes_path: pathlib.Path) -> dict[str, dict[str, int]]:
    data = json.loads(outcomes_path.read_text(encoding="utf-8"))
    stats = empty_stats()
    seen = 0
    for outcome in data.get("outcomes", []):
        scenario = outcome.get("scenario")
        summary = outcome.get("summary")
        if scenario == "Baseline":
            continue
        if not isinstance(scenario, dict) or "Mutant" not in scenario:
            raise SystemExit("unrecognized scenario in outcomes file")
        if not isinstance(summary, str) or summary not in SUMMARY_MAP:
            raise SystemExit(
                "unrecognized summary in outcomes file; "
                "cargo-mutants changed its SummaryOutcome schema — update SUMMARY_MAP"
            )
        file_path = scenario["Mutant"].get("file", "unknown")
        stats[file_path][SUMMARY_MAP[summary]] += 1
        stats[file_path]["total"] += 1
        seen += 1

    for key in AGGREGATE_KEYS:
        expected = data.get(key)
        if expected is None:
            continue
        got = sum(s[key] for s in stats.values())
        if expected != got:
            raise SystemExit(
                f"outcomes.json aggregate mismatch for {key!r}: file says {expected}, mapped {got}"
            )
    total_mutants = data.get("total_mutants")
    if total_mutants is not None and total_mutants != seen:
        raise SystemExit(
            f"outcomes.json total_mutants {total_mutants} != mapped mutant outcomes {seen}"
        )
    return stats


def render_inventory(stats: dict[str, dict[str, int]]) -> list[str]:
    lines = [
        "",
        "## Module-Level Mutation Inventory",
        "",
        f"- Source: `{OUTCOMES_PATH}` (cargo-mutants schema)",
        "",
        "| Module / File | Caught | Missed | Timeout | Unviable | Total | Score |",
        "|---|---|---|---|---|---|---|",
    ]
    for file_path in sorted(stats.keys()):
        s = stats[file_path]
        viable = s["total"] - s["unviable"]
        score = f"{(s['caught'] * 100.0 / viable):.1f}%" if viable > 0 else "N/A"
        lines.append(
            f"| `{file_path}` | {s['caught']} | {s['missed']} | {s['timeout']} | {s['unviable']} | {s['total']} | {score} |"
        )
    if not stats:
        lines.append("| _(no mutants)_ | 0 | 0 | 0 | 0 | 0 | N/A |")
    lines.append("")
    return lines


def main() -> int:
    if not OUTCOMES_PATH.exists():
        raise SystemExit(f"{OUTCOMES_PATH} is missing — nothing to inventory (no silent fallback)")
    stats = build_stats(OUTCOMES_PATH)
    print("\n".join(render_inventory(stats)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
