#!/usr/bin/env python3
"""Enumerate crate-version literals in tests, examples and documentation.

Issue #982: ``scripts/sync-version.sh`` substituted ``$CURRENT_VERSION`` — the
value in ``Cargo.toml`` *before* the release — so a literal older than that
could never be rewritten, and ``verify-version-sync.sh`` looked for a hardcoded
``0.2.[0-4]`` pattern that matched nothing. The result was seven ``"version":
"0.3.x"`` fixtures drifting in ``tests/`` and ``examples/`` while the release
gate stayed silent.

This checker does not depend on the previous version: it reads the current
version from the root manifest and compares every literal it can find.

A literal is accepted when it equals the current version, or when it is
deliberately pinned to an older value and marked within the previous three
lines with ``version-fixture: <reason>`` (a JSON/TOML/shell comment is fine as
long as the marker is present on the same or a nearby line).

Usage:
    check-version-literals.py [repo-root] [--list]

Exit codes:
    0  every literal is current or documented
    1  at least one literal drifted (details on stderr/stdout)
    2  the current version could not be read (not a verdict)
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SEMVER = re.compile(r'"version"\s*:\s*"(\d+\.\d+\.\d+)"')
MARKER = "version-fixture:"
MARKER_WINDOW = 3
SCAN_ROOTS = ("tests", "examples")
SCAN_FILES = ("README.md",)
SCAN_DIRS = ("book/src",)
EXCLUDED_DIRS = {".git", "target", "node_modules", ".qoder", ".opencode"}


def read_current_version(root: Path) -> str:
    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        print(f"error: {manifest} not found; cannot determine the current version", file=sys.stderr)
        raise SystemExit(2)
    in_package = False
    for line in manifest.read_text(encoding="utf-8", errors="replace").splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_package = stripped == "[package]"
            continue
        if in_package:
            match = re.match(r'version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"', stripped)
            if match:
                return match.group(1)
    print(f"error: no [package] version in {manifest}; cannot determine the current version", file=sys.stderr)
    raise SystemExit(2)


def files_to_scan(root: Path) -> list[Path]:
    paths: list[Path] = []
    for name in SCAN_FILES:
        candidate = root / name
        if candidate.is_file():
            paths.append(candidate)
    for name in SCAN_ROOTS + SCAN_DIRS:
        base = root / name
        if not base.exists():
            continue
        for path in base.rglob("*"):
            if not path.is_file():
                continue
            if any(part in EXCLUDED_DIRS for part in path.parts):
                continue
            if path.suffix in {".rs", ".sh", ".md", ".toml", ".json", ".yaml", ".yml"}:
                paths.append(path)
    return sorted(set(paths))


def main(argv: list[str]) -> int:
    args = [a for a in argv[1:] if not a.startswith("--")]
    list_only = "--list" in argv[1:]
    root = Path(args[0]).resolve() if args else Path.cwd()
    current = read_current_version(root)

    violations: list[str] = []
    listed: list[str] = []
    dangling_markers: list[str] = []

    for path in files_to_scan(root):
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        rel = path.relative_to(root)
        for index, line in enumerate(lines):
            match = SEMVER.search(line)
            window = lines[max(0, index - MARKER_WINDOW) : index + 1]
            marked = any(MARKER in nearby for nearby in window)
            if match:
                value = match.group(1)
                if value == current:
                    status = "current"
                elif marked:
                    status = "fixture"
                else:
                    status = "DRIFT"
                    violations.append(
                        f"{rel}:{index + 1}: \"version\": \"{value}\" is neither current "
                        f"({current}) nor marked with `{MARKER}`"
                    )
                listed.append(f"{rel}:{index + 1}: {value} [{status}]")
            elif MARKER in line:
                # A marker must document a literal nearby, otherwise it is stale
                # and would silently bless a future literal added on that line.
                next_window = lines[index : index + MARKER_WINDOW + 1]
                if not any(SEMVER.search(nearby) for nearby in next_window):
                    dangling_markers.append(f"{rel}:{index + 1}: `{MARKER}` documents no version literal")

    if list_only:
        for entry in listed:
            print(entry)
        return 0 if not violations and not dangling_markers else 1

    for violation in violations:
        print(f"error: {violation}", file=sys.stderr)
    for dangling in dangling_markers:
        print(f"error: {dangling}", file=sys.stderr)
    if violations or dangling_markers:
        print(
            f"error: {len(violations)} drifted literal(s) and {len(dangling_markers)} stray "
            f"marker(s); update them to {current} or annotate a deliberate fixture",
            file=sys.stderr,
        )
        return 1
    print(
        f"ok: {len(listed)} version literal(s) in tests/examples/docs are current ({current}) "
        f"or documented fixtures"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
