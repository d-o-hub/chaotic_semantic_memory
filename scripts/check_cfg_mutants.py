#!/usr/bin/env python3
"""Reject cfg-split native/wasm declarations that the native mutant profile cannot see.

Issue #952: `cargo-mutants` discovers mutants by parsing source text, then prices
each one by whether the mutant tree builds and whether tests pass. A mutation
inside `#[cfg(target_arch = "wasm32")]` is byte-for-byte the same program on the
native runner: it builds trivially and the tests pass trivially, so it is
classified viable-then-MISSED forever. No test can ever kill it, and the required
`mutation-test` threshold pays for it in the denominator.

The concrete shape was a clock pair:

    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();
    #[cfg(target_arch = "wasm32")]
    let start = js_sys::Date::now();

The fix is to move the platform split into one helper (see `src/clock.rs`) so the
arithmetic site compiles natively. This checker reports a same-file pair when the
diff *adds* one of its lines, so a new pair cannot re-enter the PR-gate mutation
surface; pre-existing pairs in untouched files are not a reason to block an
unrelated PR. `--all` lists every pair for a one-off audit.

A `mod` pair is deliberately not a finding: module declarations carry no
arithmetic mutants (the clock split uses two child files behind `mod imp`).

Usage:
    git diff origin/main...HEAD | check_cfg_mutants.py <repo-root> --diff -
    check_cfg_mutants.py <repo-root> --all

Exit: 0 no findings, 1 finding(s), 2 unreadable input.
Fixture: scripts/test-check-cfg-mutants.sh
"""

from __future__ import annotations

import pathlib
import re
import sys

CFG_NATIVE = re.compile(r'^\s*#\[cfg\(not\(target_arch = "wasm32"\)\)\]\s*$')
CFG_WASM = re.compile(r'^\s*#\[cfg\(target_arch = "wasm32"\)\]\s*$')
DECL = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?"
    r"(fn|let|const|static|struct|enum|impl)\s+([A-Za-z_][A-Za-z0-9_]*)"
)
SKIP = re.compile(r"^\s*(#|//|/\*|\*|\*/|$)")
HUNK = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@")


def parse_diff(stream) -> set[tuple[str, int]]:
    """Parse a unified diff from stdin.

    Deliberately not a filename: the diff is operator input, and accepting an
    argv path here was a path-traversal sink (Sonar S8707) even though the
    caller is trusted. The mutation gate pipes the diff in.
    """
    added: set[tuple[str, int]] = set()
    current = ""
    lineno = 0
    for line in stream.read().splitlines():
        if line.startswith("+++ b/"):
            current = line[6:].strip()
            continue
        if line.startswith("+++ "):
            current = ""
            continue
        match = HUNK.match(line)
        if match:
            lineno = int(match.group(1))
            continue
        if not current:
            continue
        if line.startswith("+") and not line.startswith("+++"):
            added.add((current, lineno))
            lineno += 1
        elif line.startswith(" "):
            lineno += 1
        # '-' lines do not advance the new-file line number.
    return added


def next_decl(lines: list[str], start: int, limit: int = 6) -> tuple[str, str, int] | None:
    for index in range(start, min(start + limit, len(lines))):
        line = lines[index]
        if SKIP.match(line):
            continue
        match = DECL.match(line)
        if match:
            return match.group(1), match.group(2), index
        if not line.lstrip().startswith("#["):
            return None
    return None


def find_wasm_arm(lines: list[str], start: int, kind: str, name: str) -> int | None:
    for wasm_index in range(start, min(start + 12, len(lines))):
        candidate = lines[wasm_index]
        if not CFG_WASM.match(candidate):
            if (
                candidate.strip()
                and not SKIP.match(candidate)
                and not candidate.startswith((" ", "\t"))
                and candidate.strip() not in ("}", "};")
            ):
                return None
            continue
        wasm = next_decl(lines, wasm_index + 1)
        if wasm is not None and wasm[0] == kind and wasm[1] == name:
            return wasm_index
        return None
    return None


def scan_file(path: pathlib.Path) -> list[tuple[int, int, int, str, str]]:
    """Return (native_cfg_line, native_decl_line, wasm_cfg_line, kind, name)."""
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    findings: list[tuple[int, int, int, str, str]] = []
    for index, line in enumerate(lines):
        if not CFG_NATIVE.match(line):
            continue
        native = next_decl(lines, index + 1)
        if native is None:
            continue
        kind, name, native_line = native
        wasm_index = find_wasm_arm(lines, native_line + 1, kind, name)
        if wasm_index is not None:
            findings.append((index + 1, native_line + 1, wasm_index + 1, kind, name))
    return findings


def parse_cli(args: list[str]) -> tuple[pathlib.Path, bool, bool]:
    show_all = False
    use_stdin = False
    root: pathlib.Path | None = None
    index = 0
    while index < len(args):
        arg = args[index]
        if arg == "--all":
            show_all = True
            index += 1
        elif arg == "--diff":
            if index + 1 >= len(args):
                raise ValueError("--diff needs '-' and a redirected diff on stdin")
            if args[index + 1] != "-":
                raise ValueError("--diff only accepts '-' (pipe the diff in)")
            use_stdin = True
            index += 2
        else:
            root = pathlib.Path(arg)
            index += 1
    if not show_all and not use_stdin:
        raise ValueError("pass --diff - for a PR-scoped check or --all for an audit")
    return (root or pathlib.Path.cwd()).resolve(), show_all, use_stdin


def collect_findings(root: pathlib.Path, added: set[tuple[str, int]], show_all: bool) -> list[str]:
    roots = [root / "src"]
    roots.extend(sorted((root / "crates").glob("*/src")))
    findings: list[str] = []
    for base in roots:
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            if path.name.endswith("_tests.rs") or "tests" in path.parts:
                continue
            rel = path.relative_to(root).as_posix()
            for native_cfg, native_decl, wasm_cfg, kind, name in scan_file(path):
                touched = any(
                    (rel, line) in added for line in (native_cfg, native_decl, wasm_cfg)
                )
                if show_all or touched:
                    findings.append(
                        f"{rel}:{native_cfg}: `{kind} {name}` is declared on both cfg arms; "
                        "the wasm arm is unkillable under the native --lib mutation profile "
                        f"(native decl {native_decl}, wasm cfg {wasm_cfg})"
                    )
    return findings


def main(argv: list[str]) -> int:
    try:
        root, show_all, use_stdin = parse_cli(argv[1:])
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    if not root.is_dir():
        print(f"error: {root} is not a directory", file=sys.stderr)
        return 2
    added = parse_diff(sys.stdin) if use_stdin else set()
    findings = collect_findings(root, added, show_all)
    if findings:
        for finding in findings:
            print(f"error: {finding}", file=sys.stderr)
        print(
            f"error: {len(findings)} cfg-split declaration(s) in the mutation surface; "
            "route the platform difference through one helper so the arithmetic compiles natively",
            file=sys.stderr,
        )
        return 1
    scope = "every source root" if show_all else "lines added by this diff"
    print(f"ok: no cfg-split declarations in {scope}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
