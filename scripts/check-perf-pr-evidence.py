#!/usr/bin/env python3
"""Fail-closed evidence gate for `perf(...)` PRs (ADR-0095).

The #737 / #739 / #740 / #754 Jules class: four `perf(...)` PRs were opened with
rewrites (`with_capacity` + push, iterator-closure removal, preallocation) and a
percentage claim, but no Criterion output, no flamegraph and no baseline
comparison. Reviewing them needed an allocator count run per PR, and every claim
turned out to be a no-op. This gate makes that failure mode visible in CI before
a reviewer spends the time: a `perf(...)` PR must document, in its body:

  * the baseline — ``plans/evidence/bench/canonical.json`` (a benchmark id from
    its ``benchmarks`` map) or, when the file has no entry for the affected
    benchmark, the nearest canonical comparator named plus the new benchmark
    output;
  * the measurement artifact — a Criterion output path or a flamegraph
    reference;
  * a numeric before/after pair for the affected benchmark.

It is a syntactic presence/shape check. It does not run benchmarks, does not
validate provenance, and does not assert an improvement — a human still judges
measurement quality. Scope validity of the ``perf(<scope>)`` title stays with
commitlint's ``scope-enum`` and is deliberately not reimplemented here.

Usage: GitHub Actions passes the PR title/body through the environment (never
through shell interpolation) and may also pass them as positional arguments;
environment variables are the fallback so a manual run needs only the env:

    python3 scripts/check-perf-pr-evidence.py "$PR_TITLE" "$PR_BODY"
    PR_TITLE='perf(retrieval): drop clones' PR_BODY="$(cat body.md)" \\
        python3 scripts/check-perf-pr-evidence.py

Exit code 0: title is not ``perf(...)`` (out of scope, including bot titles), or
all requirements are present. Exit code 1: a ``perf(...)`` title is missing
evidence; every failure is printed as a GitHub Actions ``::error::`` line.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

# Anchored at the repository root; the script lives in scripts/ and is invoked
# from the root by the CI step, so resolve relative to __file__ for local runs.
REPO_ROOT = Path(__file__).resolve().parent.parent
CANONICAL_BASELINE = "plans/evidence/bench/canonical.json"
CANONICAL_PATH = REPO_ROOT / CANONICAL_BASELINE

MAX_BODY_BYTES = 64 * 1024
SECTION_HEADING = "performance evidence"

# Markdown heading, e.g. "## Performance Evidence" / "### perf evidence".
HEADING_RE = re.compile(r"^(#{1,6})[ \t]*([^#\n][^\n]*?)[ \t]*#*[ \t]*$", re.MULTILINE)
COMMENT_RE = re.compile(r"<!--.*?-->", re.DOTALL)
ANGLE_PLACEHOLDER_RE = re.compile(r"<[^>\n]{1,80}>")
PLACEHOLDER_WORD_RE = re.compile(r"\b(N/A|TODO|TBD|FIXME|XXX|PLACEHOLDER)\b", re.IGNORECASE)

# Numeric pair: "100 us -> 80 us", "4.2 Mbps → 3.1 Mbps", "12 ms to 8 ms".
ARROW = r"(?:->|=>|\bbefore\b|\bafter\b|\bfrom\b|\bto\b)"
NUM = r"\d+(?:[.,]\d+)?(?:\s*[eE][+-]?\d+)?"
UNIT = r"(?:ns|us|µs|μs|ms|sec|secs|seconds?|s|ops?/s|qps|req/s|B|KB|Kb|MB|Mb|GB|Gb|bytes?|%|x)"
PAIR_RE = re.compile(
    rf"(?<![A-Za-z0-9_]){NUM}\s*{UNIT}?\s*{ARROW}\s*{NUM}\s*{UNIT}?(?![A-Za-z0-9_%])",
    re.IGNORECASE,
)
BEFORE_LABEL_RE = re.compile(rf"\bbefore\b[^.\n]{{0,40}}?{NUM}", re.IGNORECASE)
AFTER_LABEL_RE = re.compile(rf"\bafter\b[^.\n]{{0,40}}?{NUM}", re.IGNORECASE)
NUMBER_RE = re.compile(NUM)

# Baseline tokens: literal path or a benchmark id taken from canonical.json.
TOKEN_RE = re.compile(r"[^\s,;:()\[\]{}\"']+")

# Whole URL, matched before TOKEN_RE splits it (the token class excludes ":").
URL_RE = re.compile(r"https?://[^\s<>\"')\]]+", re.IGNORECASE)


def load_canonical_benchmark_ids() -> set[str]:
    """Benchmark ids from the committed canonical baseline.

    Failure to read the file degrades to an empty set: the literal canonical
    path still satisfies requirement (a), so a malformed baseline cannot block
    every perf PR — it just cannot be used as the named-comparator shortcut.
    """
    try:
        data = json.loads(CANONICAL_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return set()
    benchmarks = data.get("benchmarks")
    return set(benchmarks) if isinstance(benchmarks, dict) else set()


def find_evidence_section(body: str) -> str | None:
    """Text of the `## Performance Evidence` section, minus its heading.

    The section runs from its heading to the next heading of the same or higher
    level (or the end of the body). The heading is matched case-insensitively,
    at any level, so a `### Performance Evidence` subsection is accepted too.
    """
    lines = body.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    heading_at: int | None = None
    heading_level = 0
    for index, line in enumerate(lines):
        match = HEADING_RE.match(line)
        if match is None:
            continue
        level = len(match.group(1))
        title = match.group(2).strip().lower().rstrip(":").strip()
        if title == SECTION_HEADING:
            heading_at = index
            heading_level = level
            break

    if heading_at is None:
        return None

    end = len(lines)
    for index in range(heading_at + 1, len(lines)):
        match = HEADING_RE.match(lines[index])
        if match is not None and len(match.group(1)) <= heading_level:
            end = index
            break

    return "\n".join(lines[heading_at + 1 : end])


def mask_placeholders(text: str) -> tuple[str, str, str]:
    """Return three views of the section for the "not filled in" checks:

    * comment-free text — empty here means the author left only the template
      comment (or nothing);
    * comment-free, angle-bracket-free text — empty here means the author left
      only comments and `<placeholder>` slots;
    * the same text with inline N/A/TODO markers removed — empty here means the
      section carries no substance beyond placeholders.
    """
    without_comments = COMMENT_RE.sub("", text)
    without_angles = ANGLE_PLACEHOLDER_RE.sub("", without_comments)
    without_words = PLACEHOLDER_WORD_RE.sub("", without_angles)
    return without_comments, without_angles, without_words


def section_tokens(text: str) -> set[str]:
    """Whitespace/punctuation tokens for baseline and artifact matching.

    Markdown decorators and trailing punctuation are stripped; underscores and
    slashes are kept (benchmark ids contain both, e.g.
    `retrieval_baseline/exact_realistic_10000`).
    """
    tokens = set()
    for raw in TOKEN_RE.findall(text):
        # Single pass: stripping decorators and punctuation in two passes stops
        # at the first character outside the current set, leaving e.g. the
        # backtick in `` `path.json`. ``
        token = raw.strip("*_`~#<>|" + chr(92) + ".,;:!?)" + '"' + "'")
        # `<bench-id>` scaffolds are placeholders, not paths; the angle brackets
        # are stripped above, so drop any token that still carries them.
        if token and "<" not in token and ">" not in token:
            tokens.add(token)
    return tokens


def has_numeric_pair(text: str) -> bool:
    """A before/after pair for the affected benchmark.

    Accepts a directed pair (`100 us -> 80 us`, `12 ms to 8 ms`, `1.2 Mbps =>
    0.9 Mbps`) with a number on each side of the direction marker, or both a
    `before` and an `after` label each carrying its own number. A bare `0.5 ->
    0.6` (no unit) also qualifies: this is a presence/shape check, and the
    reviewer judges whether the numbers are meaningful.
    """
    if not NUMBER_RE.search(text):
        return False

    if _has_labeled_pair(text):
        return True

    return any(_arrow_sides_numbered(match.group(0)) for match in PAIR_RE.finditer(text))


def _has_labeled_pair(text: str) -> bool:
    """Both a `before` and an `after` label, each followed by a number."""
    for before in BEFORE_LABEL_RE.finditer(text):
        if AFTER_LABEL_RE.search(text, before.end()):
            return True
    return False


def _arrow_sides_numbered(region: str) -> bool:
    """True when both sides of a direction marker carry a number."""
    sides = re.split(r"->|=>|\bbefore\b|\bafter\b|\bfrom\b|\bto\b", region, maxsplit=1, flags=re.IGNORECASE)
    if len(sides) != 2:
        return False
    return NUMBER_RE.search(sides[0]) is not None and NUMBER_RE.search(sides[1]) is not None


def has_artifact(text: str) -> bool:
    """A Criterion output path/URL or a flamegraph reference.

    The artifact must be path- or URL-shaped: a bare `flamegraph` word (the PR
    template's own prompt line) and the baseline path
    `plans/evidence/bench/canonical.json` — which contains no `criterion/`
    segment — must not satisfy the gate. URLs are matched before whitespace
    tokenization because the token class excludes `:`, so an artifact URL would
    otherwise split at the scheme and lose the keyword/path pairing; a URL with
    no `criterion`/`flamegraph` segment is only accepted when it is a CI
    artifact download (`/artifacts/`), so an unrelated link never counts.
    """
    for url in URL_RE.findall(text):
        lowered = url.lower()
        if CANONICAL_BASELINE in lowered:
            continue
        if "criterion" in lowered or "flamegraph" in lowered or "/artifacts/" in lowered:
            return True

    for token in section_tokens(text):
        if CANONICAL_BASELINE in token:
            continue
        lowered = token.lower()
        if "flamegraph" in lowered and _path_shaped(token):
            return True
        # A Criterion output is a file (`target/criterion/.../estimates.json`);
        # the bare output directory is not an artifact.
        if "criterion/" in lowered and re.search(r"\.[a-z0-9]{1,6}$", lowered):
            return True
    return False


def _path_shaped(token: str) -> bool:
    """True when a token is a path or URL reference, not bare prose."""
    if "://" in token:
        return True
    return "/" in token or re.search(r"\.[a-z0-9]{1,6}$", token, re.IGNORECASE) is not None


def main(argv: list[str]) -> int:
    args = argv[1:]
    if len(args) > 2:
        print(
            "usage: check-perf-pr-evidence.py [PR_TITLE] [PR_BODY]  "
            "(positional arguments default to the PR_TITLE/PR_BODY environment variables)",
            file=sys.stderr,
        )
        return 2

    title = (args[0] if args else os.environ.get("PR_TITLE", "")).strip()
    body = args[1] if len(args) > 1 else os.environ.get("PR_BODY", "")

    # Only `perf(...)` titles are gated. Bot/dependency titles and every other
    # conventional-commit type pass through untouched.
    if not title.startswith("perf("):
        return 0

    failures: list[str] = []

    if not body.strip():
        failures.append(f"empty PR body; add a `## {SECTION_HEADING.title()}` section with baseline, artifact and before/after numbers")
    elif len(body.encode("utf-8", errors="replace")) > MAX_BODY_BYTES:
        failures.append(f"PR body larger than {MAX_BODY_BYTES} bytes; trim it and re-run")

    if failures:
        report(failures)
        return 1

    section = find_evidence_section(body)
    if section is None:
        report(["missing `## Performance Evidence` section in the PR body"])
        return 1

    without_comments, without_angles, without_placeholders = mask_placeholders(section)
    if not without_comments.strip():
        report(["`## Performance Evidence` section is empty; fill in the template"])
        return 1
    if not without_angles.strip():
        report(["`## Performance Evidence` section contains only the template comment/placeholders; fill it in"])
        return 1
    if not without_placeholders.strip():
        report(["`## Performance Evidence` section contains only placeholders (N/A/TODO/template text)"])
        return 1

    ids = load_canonical_benchmark_ids()
    tokens = section_tokens(without_angles)
    named_ids = sorted(token for token in tokens if token in ids)
    if CANONICAL_BASELINE not in without_angles and not named_ids:
        failures.append(
            f"missing baseline: name a benchmark id from `{CANONICAL_BASELINE}` "
            "(for example `retrieval_baseline/exact_realistic_10000`), or name the "
            "nearest canonical comparator plus the new benchmark output when the "
            "canonical baseline has no entry for the affected benchmark"
        )

    if not has_artifact(without_angles):
        failures.append(
            "missing measurement artifact: reference the Criterion output path "
            "(for example `target/criterion/<bench-id>/new/estimates.json`), a "
            "flamegraph file, or the Criterion/flamegraph CI run link (its "
            "`/artifacts/` download) — a bare mention without a path or link "
            "does not count"
        )

    if not has_numeric_pair(without_angles):
        failures.append(
            "missing numeric before/after pair for the affected benchmark "
            "(for example `before: 100 us; after: 80 us` or `100 us -> 80 us`)"
        )

    if failures:
        report(failures)
        return 1

    return 0


def report(failures: list[str]) -> None:
    for failure in failures:
        message = " ".join(failure.split())
        print(f"::error::perf PR evidence gate: {message}")


if __name__ == "__main__":
    sys.exit(main(sys.argv))
