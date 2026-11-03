#!/usr/bin/env python3
"""Aggregate sharded cargo-mutants outcomes into one verdict (issue #951).

The required branch-protection context is literally `mutation-test`, so a matrix
cannot own it: `mutation-test (1/4)` would leave the required name missing. The
`mutation-shard` matrix each runs a disjoint `--shard i/N` slice and uploads its
`progress/mutation` directory; this script runs in the `mutation-test` job and
scores the **union**:

    score = sum(caught) / (sum(total) - sum(unviable)) * 100

Never an average of per-shard percentages: that would let an all-missed shard
pass on a sibling's score. Timeout budgets are also enforced over the union, so
an N-way split cannot grant N times the allowance.

Completeness is explicit: each shard writes `shard-complete` (it ran) or
`no-mutants` (its slice was empty by construction). A cancelled/timed-out shard
leaves no marker and the aggregator refuses to certify the run — a partial
artifact set must never read as a passing score.

Exit: 0 pass (including the 0-mutant case), 1 threshold/budget failure,
2 incomplete or unreadable evidence (not a verdict).

Fixture: scripts/test-mutation-union.sh
"""

from __future__ import annotations

import argparse
import json
import math
import os
import pathlib
import re
import sys

SUMMARY_MAP = {
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
}
COUNTERS = ("caught", "missed", "timeout", "unviable")
MARKER_RE = re.compile(r"^(\d+)/(\d+)$")


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(2)


def markers(root: pathlib.Path, name: str, expected: int) -> dict[int, pathlib.Path]:
    found: dict[int, pathlib.Path] = {}
    for path in sorted(root.rglob(name)):
        text = path.read_text(encoding="utf-8", errors="replace").strip()
        match = MARKER_RE.match(text)
        if not match:
            fail(f"{path}: marker {text!r} is not i/N")
        shard, total = int(match.group(1)), int(match.group(2))
        if total != expected:
            fail(f"{path}: marker says {shard}/{total}, expected i/{expected}")
        if not 1 <= shard <= expected:
            fail(f"{path}: shard {shard} is outside 1..{expected}")
        if shard in found:
            fail(f"shard {shard} has two {name} markers: {found[shard]} and {path}")
        found[shard] = path
    return found


def read_outcomes(path: pathlib.Path) -> dict[str, int]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"{path}: unreadable outcomes file ({exc})")
    totals = dict.fromkeys(COUNTERS, 0)
    seen = 0
    for outcome in data.get("outcomes", []):
        scenario = outcome.get("scenario")
        if scenario == "Baseline":
            continue
        summary = outcome.get("summary")
        if not isinstance(scenario, dict) or "Mutant" not in scenario:
            fail(f"{path}: unrecognized scenario")
        if summary not in SUMMARY_MAP:
            fail(f"{path}: unrecognized summary {summary!r}; cargo-mutants schema changed")
        totals[SUMMARY_MAP[summary]] += 1
        seen += 1
    for key in COUNTERS:
        expected = data.get(key)
        if expected is not None and expected != totals[key]:
            fail(f"{path}: aggregate {key} {expected} != mapped {totals[key]}")
    expected_total = data.get("total_mutants")
    if expected_total is not None and expected_total != seen:
        fail(f"{path}: total_mutants {expected_total} != mapped {seen}")
    return {**totals, "total": seen}


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", required=True, type=pathlib.Path)
    parser.add_argument("--expected-shards", required=True, type=int)
    parser.add_argument(
        "--threshold",
        type=float,
        default=float(os.environ.get("MUTATION_THRESHOLD", "85")),
    )
    parser.add_argument(
        "--timeout-budget",
        type=int,
        default=int(os.environ.get("MUTATION_TIMEOUT_BUDGET", "5")),
    )
    parser.add_argument(
        "--timeout-fraction",
        type=float,
        default=float(os.environ.get("MUTATION_TIMEOUT_FRACTION", "0.10")),
    )
    args = parser.parse_args(argv)

    root = args.artifacts
    if not root.is_dir():
        fail(f"artifact directory {root} does not exist (download step produced nothing)")

    expected = args.expected_shards
    if expected < 1:
        fail("--expected-shards must be >= 1")
    complete = markers(root, "shard-complete", expected)
    empty = markers(root, "no-mutants", expected)
    overlap = sorted(set(complete) & set(empty))
    if overlap:
        fail(f"shard(s) {overlap} have both shard-complete and no-mutants markers")
    present = set(complete) | set(empty)
    missing = sorted(set(range(1, expected + 1)) - present)
    if missing:
        fail(
            f"missing shard evidence for {missing} of 1..{expected}; a partial shard set "
            "cannot certify the required mutation-test context"
        )

    outcomes = sorted(root.rglob("mutants.out/outcomes.json"))
    totals = dict.fromkeys(COUNTERS, 0)
    totals["total"] = 0
    for path in outcomes:
        part = read_outcomes(path)
        for key in (*COUNTERS, "total"):
            totals[key] += part[key]

    print(
        f"mutation union: shards={len(complete)} complete, {len(empty)} empty, "
        f"outcomes files={len(outcomes)}"
    )
    if totals["total"] == 0:
        print("0 mutants in changed Rust sources, CI check skipped")
        return 0

    viable = totals["total"] - totals["unviable"]
    score = totals["caught"] * 100.0 / viable if viable > 0 else 100.0
    print(
        "mutation union: "
        f"total={totals['total']} caught={totals['caught']} timeout={totals['timeout']} "
        f"missed={totals['missed']} unviable={totals['unviable']} score={score:.4f}%"
    )
    if totals["timeout"] > args.timeout_budget:
        print(
            f"error: {totals['timeout']} mutation timeouts exceed union budget "
            f"{args.timeout_budget}",
            file=sys.stderr,
        )
        return 1
    if viable >= 10:
        limit = max(1, math.ceil(viable * args.timeout_fraction))
        if totals["timeout"] > limit:
            print(
                f"error: {totals['timeout']} mutation timeouts exceed "
                f"{args.timeout_fraction:.2f} of union viable={viable} (limit {limit})",
                file=sys.stderr,
            )
            return 1
    if score >= args.threshold:
        print(f"mutation union score {score:.4f}% >= {args.threshold:g}%, CI check passed")
        return 0
    print(
        f"mutation union score {score:.4f}% < {args.threshold:g}%, CI check failed",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
