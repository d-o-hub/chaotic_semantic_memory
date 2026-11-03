#!/usr/bin/env python3
"""Enumerate, verify and rewrite the intra-workspace dependency version pins.

Issue #964. `scripts/sync-version.sh` updated a member's own `version =` and the
root -> member pins, and nothing else. Measured 2026-11-01 on `cae70539`: 26 path
edges to workspace members, 18 of them stale. Seventeen of the 18 are a *member ->
anything* edge still declaring `0.3.8`; the 18th is `fuzz/Cargo.toml` declaring
`0.3.7`, two releases behind. The issue enumerated 16 because it read single-line
`{ path = …, version = … }` forms only — `fuzz/Cargo.toml` writes its pin as a
multi-line `[dependencies.chaotic_semantic_memory]` table — and a tomllib pass over
`crates/*` plus the root reached 17 of 25 edges because `benchmarks/Cargo.toml` is a
declared root member no `crates/*` pattern reaches. `verify-version-sync.sh` printed
"✓ All local versions synchronized: 0.3.9" over all 18 and exited 0, so the drift had
no sensor at all.

Why it survived three releases: `version = "0.3.8"` means `^0.3.8`, which 0.3.9
satisfies, so Cargo resolves correctly and nothing breaks. The cost lands at the next
major — `csm-duckdb@0.4.0` would publish a manifest claiming compatibility with root
`^0.3.8`, false the moment 0.4.0 exists — and 7 of the 9 crates.io artifacts ship that
metadata today.

Three modes, one parser, so the tool and the gate can never disagree (the class of bug
this file exists to kill):

    workspace-version-edges.py --list                [repo-root]
    workspace-version-edges.py --check               [repo-root]
    workspace-version-edges.py --sync <version>      [repo-root] [--dry-run]

Exit: 0 as requested, 1 `--check` found a stale pin, 2 the audit could not run (no
manifests, no tomllib, a bad version argument, a root outside the trusted directories, a
manifest resolving outside the root, a declared `[workspace] members` entry the walk did
not reach, or the text parser disagreeing with the TOML parser about which edges exist).

The exit-2 rule is load-bearing. tomllib cross-checks the edge set the line parser
found, so a manifest form the parser does not understand reports "could not audit"
rather than "no drift" — the difference between a sensor and a shrug. Discovery is a
pruned filesystem walk, cross-checked the same way: `[workspace] members` is asked which
manifests must have been read.
"""

from __future__ import annotations

import argparse
import glob
import os
import re
import sys
import tempfile
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # python < 3.11: no cross-check, so no verdict
    tomllib = None

DEP_LEAVES = ("dependencies", "dev-dependencies", "build-dependencies")
MANIFEST_NAME = "Cargo.toml"
# Directories that cannot hold a workspace member. `target` is where `cargo package`
# unpacks a copy of every published crate — each with its own Cargo.toml and its own
# stale pins — and `.qoder/worktrees/*` holds full checkout copies of this repository,
# which is exactly how a stale worktree turned `validate.sh` red (AGENTS.md rule 12).
# The prune is not trusted to be right: `check_member_coverage` refuses if a member the
# root manifest *declares* is missing from what the walk found, so pruning a real member
# is a loud exit 2 rather than the silent green this file exists to kill.
PRUNED_DIRS = frozenset({".git", ".qoder", "node_modules", "target", "vendor"})
HEADER = re.compile(r"^\[([^\]]+)\]\s*(?:#.*)?$")
VERSION_VALUE = re.compile(r'(version\s*=\s*")([^"]*)(")')
INLINE_ENTRY = re.compile(r"([A-Za-z0-9_.+-]+)\s*=\s*\{")
PATH_VALUE = re.compile(r'path\s*=\s*"([^"]*)"')
WORKSPACE_TRUE = re.compile(r"\bworkspace\s*=\s*true")


class ParseError(Exception):
    """A manifest shape this parser must not be trusted on."""


def refuse(message):
    """Exit 2: the audit could not run. Never 1, which is a verdict of drift.

    `SystemExit(str)` exits 1, so every "could not audit" path goes through here or the
    sensor would report as a stale pin — and a #903-class red release job with the wrong
    reason is the failure mode this file exists to prevent.
    """
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(2)


def resolve_root(raw):
    """Absolute, symlink-free repository root, or exit 2.

    Every path this tool reads or rewrites is derived from here, so containment is
    decided once at the boundary instead of at each `open()`. The root arrives on the
    command line (`scripts/validate.sh` passes its own parent, the fixture passes a
    temp tree), and a root that is not a directory is "could not run" — never a verdict
    of no drift.

    That argument is also the tool's only untrusted input, so it is confined to the
    directories this audit has business in: the checkout the script ships in, the tree
    it was invoked from, and the system temp dir (the fixture builds its trees there).
    `--sync` writes, which makes the boundary load-bearing rather than advisory: a root
    outside all three is exit 2, not a partial rewrite somewhere nobody pointed at. The
    check is `Path.is_relative_to` against resolved bases, the shape static analysis
    recognises as containment; `commonpath` is equivalent but reads as unvalidated.
    """
    root = Path(raw).resolve()
    if not root.is_dir():
        refuse(f"not a repository directory: {raw}")
    if not (
        root.is_relative_to(Path(__file__).resolve().parent.parent)
        or root.is_relative_to(Path.cwd().resolve())
        or root.is_relative_to(Path(tempfile.gettempdir()).resolve())
    ):
        refuse(
            f"refusing to audit {root}: it is outside this checkout, the current working "
            "directory and the system temp directory"
        )
    return root


def escapes(root, path):
    """True when `path` resolves outside `root`.

    Discovery is a walk that follows symlinked directories, so a tree can hand back a
    manifest living somewhere else entirely. Reading it would let one symlink smuggle in a
    file this audit was never pointed at, and `--sync` would write back through it.
    Escaping is refused rather than silently dropped — a filter that discards an edge
    reports clean.
    """
    return not Path(path).resolve().is_relative_to(root)


def walk_manifests(root):
    """Every Cargo.toml under `root`, by filesystem rather than by git.

    A git index records what someone chose to track; #964's drift lived in the manifests
    themselves, so a new member is audited before it is `git add`ed and the audit needs no
    git at all. Symlinked directories are followed — each directory's realpath is entered
    once, so a link back into the tree (`.blackbox/skills` here) cannot loop — and results
    are keyed by realpath so a file reachable along two paths is still one edge.

    It also reaches members no glob list names: `benchmarks/Cargo.toml` holds one of this
    repository's 26 edges and matches no `crates/*` pattern, which is how it shipped three
    releases behind at `0.3.6`.
    """
    found = {}
    seen_dirs = {root}
    for dirpath, dirnames, filenames in os.walk(root, followlinks=True):
        kept = []
        for name in sorted(dirnames):
            if name in PRUNED_DIRS:
                continue
            real = Path(dirpath, name).resolve()
            if real in seen_dirs:
                continue
            seen_dirs.add(real)
            kept.append(name)
        dirnames[:] = kept
        if MANIFEST_NAME in filenames:
            path = Path(dirpath, MANIFEST_NAME)
            found.setdefault(path.resolve(), path)
    return sorted(found.values())


def keep_inside(root, paths):
    """Return `paths`, refusing if any of them resolves outside `root`.

    Refuse rather than filter: dropping an escaping manifest quietly would make the
    audit report "every pin matches" about a tree it did not read.
    """
    outside = sorted(p for p in paths if escapes(root, p))
    if outside:
        refuse(
            "manifest(s) resolve outside the repository root; refusing to audit or "
            f"rewrite them: {', '.join(str(p) for p in outside)}"
        )
    return paths


def manifest_paths(root):
    """The workspace's manifest set: discovered by walk, deduped, and contained."""
    return keep_inside(root, walk_manifests(root))


def member_manifest_paths(root):
    """The manifests `[workspace] members` names, expanded the way cargo expands it.

    Read from the root manifest rather than hard-coded here, because the list that gates
    a build is the list a version audit should trust — and it is the counter-check that
    makes a wrong `PRUNED_DIRS` entry impossible to miss.
    """
    root_manifest = root / MANIFEST_NAME
    if not root_manifest.is_file():
        return []
    with open(root_manifest, "rb") as handle:
        document = tomllib.load(handle)
    members = (document.get("workspace") or {}).get("members")
    if members is None:
        return []
    if not isinstance(members, list):
        refuse("[workspace] members is not an array; the coverage cross-check did not run")
    paths = []
    for member in members:
        if not isinstance(member, str):
            refuse(f"[workspace] members holds a non-string: {member!r}")
        # `members` is file content, so it is the second untrusted input: a glob such as
        # `../../*` would reach outside the tree before `keep_inside` ever sees a result.
        if os.path.isabs(member) or ".." in Path(member).parts:
            refuse(f"[workspace] members entry escapes the root: {member!r}")
        if glob.has_magic(member):
            hits = sorted(h for h in glob.glob(str(root / member)) if os.path.isdir(h))
            paths += [Path(hit, MANIFEST_NAME) for hit in hits]
        else:
            paths.append(root / member / MANIFEST_NAME)
    return keep_inside(root, paths)


def check_member_coverage(root, manifests):
    """Exit 2 unless every declared member's manifest is among `manifests`.

    Comparing by realpath is what keeps a symlinked tree from reading as "missing", and
    the direction matters: a discovered manifest nobody declared is merely out of scope,
    while a *declared* one nobody discovered means discovery was blind.
    """
    seen = {os.path.realpath(path) for path in manifests}
    missing = sorted(
        {
            os.path.relpath(path, root)
            for path in member_manifest_paths(root)
            if os.path.realpath(path) not in seen
        }
    )
    if missing:
        refuse(
            "a declared workspace member has no manifest among those discovered: "
            + ", ".join(missing)
            + " — discovery was blind, so this is not a verdict"
        )


def classify_header(parts):
    """(section, entry_or_None) for a TOML header, or (None, None) if not a dependency table.

    Understands `[dependencies]`, `[dev-dependencies.foo]`, `[workspace.dependencies]`
    and `[target.'cfg(…)'.dependencies(.foo)]`. Anything deeper than the entry name is
    refused rather than guessed at.
    """
    if parts[0] == "workspace":
        if len(parts) == 2 and parts[1] in DEP_LEAVES:
            return parts[1], None
        return None, None
    if parts[0] == "target":
        if len(parts) == 3 and parts[2] in DEP_LEAVES:
            return f"target.{parts[1]}.{parts[2]}", None
        if len(parts) == 4 and parts[2] in DEP_LEAVES:
            return f"target.{parts[1]}.{parts[2]}", parts[3]
        return None, None
    if parts[0] in DEP_LEAVES:
        if len(parts) == 1:
            return parts[0], None
        if len(parts) == 2:
            return parts[0], parts[1]
        joined = ".".join(parts)
        raise ParseError(f"dependency table nested too deep: [{joined}]")
    return None, None


def new_edge(path, section, name):
    return {
        "file": path,
        "section": section,
        "dep": name,
        "path": None,
        "declared": None,
        "workspace_flag": False,
        "edit": None,  # (line index, value start column, value end column)
    }


def version_span(body, start_line):
    """Locate the `version = "x"` value inside a possibly multi-line inline table."""
    match = VERSION_VALUE.search(body)
    if not match:
        return None
    newline_count = body[: match.start(2)].count("\n")
    line_no = start_line + newline_count
    offset = 0 if newline_count == 0 else body.rfind("\n", 0, match.start(2)) + 1
    start = match.start(2) - offset
    return (line_no, start, start + len(match.group(2)))


def fill_edge(edge, body, start_line):
    found = PATH_VALUE.search(body)
    if found:
        edge["path"] = found.group(1)
    version = VERSION_VALUE.search(body)
    if version:
        edge["declared"] = version.group(2)
        edge["edit"] = version_span(body, start_line)
    if WORKSPACE_TRUE.search(body):
        edge["workspace_flag"] = True


def parse_manifest(path):
    """Return (package_version, {section, dep -> edge}) for one manifest.

    Line-oriented on purpose: tomllib answers "what does this mean" but not "where on
    which line", and `--sync` must rewrite one token without reformatting anyone else's
    manifest. tomllib is still consulted, as the cross-check in `collect()`.
    """
    with open(path, encoding="utf-8") as handle:
        lines = handle.readlines()
    package_version = None
    edges = {}
    section = None
    table_entry = None
    in_package = False
    number = 0
    while number < len(lines):
        line = lines[number]
        stripped = line.strip()
        number += 1
        if not stripped or stripped.startswith("#"):
            continue
        header = HEADER.match(stripped)
        if header:
            parts = [p.strip().strip("'\"") for p in header.group(1).split(".")]
            in_package = parts == ["package"]
            section, table_entry = (None, None) if in_package else classify_header(parts)
            continue
        if not line[:1].isspace():
            version_line = re.match(r'\s*version\s*=\s*"([^"]+)"', line)
            if in_package and version_line:
                package_version = version_line.group(1)
        if section is None:
            continue
        if table_entry is not None:
            edge = edges.setdefault(
                (section, table_entry), new_edge(path, section, table_entry)
            )
            fill_edge(edge, line, number - 1)
            continue
        inline = INLINE_ENTRY.match(line.strip())
        if not inline:
            continue
        name = inline.group(1)
        end = number - 1
        body = line
        while body.count("{") != body.count("}"):
            end += 1
            if end >= len(lines):
                raise ParseError(f"{path}:{number}: unbalanced inline dependency table")
            body += lines[end]
        number = end + 1
        fill_edge(edges.setdefault((section, name), new_edge(path, section, name)),
                  body, number - len(body.splitlines()))
    return package_version, edges


def toml_dep_tables(document):
    """Yield (section, {dep: spec}) using the same vocabulary `classify_header` returns."""
    for leaf in DEP_LEAVES:
        deps = document.get(leaf)
        if isinstance(deps, dict):
            yield leaf, deps
    for target, tables in (document.get("target") or {}).items():
        if not isinstance(tables, dict):
            continue
        for leaf in DEP_LEAVES:
            deps = tables.get(leaf)
            if isinstance(deps, dict):
                yield f"target.{target}.{leaf}", deps
    workspace = document.get("workspace") or {}
    for leaf in DEP_LEAVES:
        deps = workspace.get(leaf)
        if isinstance(deps, dict):
            yield leaf, deps


def collect(root):
    """(member_versions, edges_to_members) for the whole workspace graph."""
    member_versions = {}
    all_edges = []
    manifests = manifest_paths(root)
    check_member_coverage(root, manifests)
    for path in manifests:
        package_version, edges = parse_manifest(path)
        with open(path, "rb") as handle:
            document = tomllib.load(handle)
        name = (document.get("package") or {}).get("name")
        if name and package_version:
            member_versions[name] = package_version
        for edge in edges.values():
            if edge["path"] is not None:
                all_edges.append(edge)
    cross_check(root, manifests, all_edges)
    edges = [e for e in all_edges if e["dep"] in member_versions]
    for edge in edges:
        edge["target"] = member_versions[edge["dep"]]
    return member_versions, edges


def cross_check(root, manifests, edges):
    """Exit 2 unless the line parser and tomllib agree about every path edge *and its pin*.

    Comparing the version value too is what closes the false-green: `version = '0.3.8'`
    (single quotes) is valid TOML that a double-quote regex never sees, and an edge-set-only
    cross-check would call that "no pin to sync" and report clean.
    """
    if tomllib is None:  # pragma: no cover - collect() would already have refused
        refuse("python3 has no tomllib (3.11+); the version-edge audit did not run")
    expected = {}
    for path in manifests:
        with open(path, "rb") as handle:
            document = tomllib.load(handle)
        for section, deps in toml_dep_tables(document):
            for name, spec in deps.items():
                if isinstance(spec, dict) and "path" in spec:
                    expected[(os.path.relpath(path, root), section, name)] = spec.get(
                        "version"
                    )
    seen = {
        (os.path.relpath(e["file"], root), e["section"], e["dep"]): e["declared"]
        for e in edges
    }
    if set(seen) != set(expected):
        unparsed = ", ".join(sorted(f"{s}.{n}" for _, s, n in set(expected) - set(seen)))
        misparsed = ", ".join(sorted(f"{s}.{n}" for _, s, n in set(seen) - set(expected)))
        refuse(
            "the version-edge parser disagrees with tomllib about which path "
            f"edges exist (unparsed: {unparsed or 'none'}; misparsed: {misparsed or 'none'}); "
            "the audit did not run"
        )
    silent = [
        f"{key[0]} [{key[1]}] {key[2]}: tomllib sees \"{expected[key]}\", the parser sees none"
        for key in sorted(seen)
        if seen[key] != expected[key]
    ]
    if silent:
        refuse(
            "the version-edge parser could not read a pin tomllib can: "
            + "; ".join(silent)
            + " — refusing to report clean"
        )


def stale_edges(edges, expected):
    """Edges whose declared requirement is not the member's own version."""
    return [
        e
        for e in edges
        if e["declared"] is not None and e["declared"] != expected[e["dep"]]
    ]


def label(root, edge):
    return f"{os.path.relpath(edge['file'], root)} [{edge['section']}] {edge['dep']}"


def cmd_list(root):
    member_versions, edges = collect(root)
    print(f"{'FILE':34s} {'SECTION':26s} {'DEPENDENCY':26s} {'DECLARES':9s} {'TARGET':8s} PATH")
    for edge in sorted(edges, key=lambda e: (e["file"], e["section"], e["dep"])):
        print(
            f"{os.path.relpath(edge['file'], root):34s} {edge['section']:26s} "
            f"{edge['dep']:26s} {str(edge['declared']):9s} {edge['target']:8s} "
            f"{edge['path']}"
        )
    print(
        f"\n{len(edges)} path edge(s) to workspace members; "
        f"{len(stale_edges(edges, member_versions))} stale"
    )
    return 0


def cmd_check(root):
    member_versions, edges = collect(root)
    stale = stale_edges(edges, member_versions)
    if not stale:
        print(
            f"version edges: {len(edges)} path edge(s) to workspace members, every pin "
            "matches the member's own version"
        )
        return 0
    for edge in sorted(stale, key=lambda e: (e["file"], e["section"], e["dep"])):
        print(
            f"DRIFT: {label(root, edge)} declares \"{edge['declared']}\" but the "
            f"package is \"{edge['target']}\""
        )
    print(
        f"\nversion edges: {len(stale)} stale pin(s) of {len(edges)}; fix with "
        "python3 scripts/workspace-version-edges.py --sync <version>",
        file=sys.stderr,
    )
    return 1


def cmd_sync(root, version, dry_run):
    """Rewrite every member pin to `version`, or refuse. No verdict is returned: this
    either writes or exits 2, so a caller cannot mistake a return code for a finding."""
    _, edges = collect(root)
    wrong = [e for e in edges if e["declared"] is not None and e["declared"] != version]
    if not wrong:
        print(f"version edges: nothing to sync, all {len(edges)} pin(s) already {version}")
        return
    edits = {}
    for edge in wrong:
        if edge["edit"] is None:
            refuse(
                f"{label(root, edge)} has a version this parser cannot locate; "
                "refusing a partial rewrite"
            )
        edits.setdefault(edge["file"], []).append(edge["edit"])
    total = 0
    for path, spots in sorted(edits.items()):
        with open(path, encoding="utf-8") as handle:
            lines = handle.readlines()
        for line_no, start, end in sorted(spots, reverse=True):
            lines[line_no] = lines[line_no][:start] + version + lines[line_no][end:]
            total += 1
        verb = "would rewrite" if dry_run else "rewrote"
        print(f"{verb} {len(spots)} edge(s) in {os.path.relpath(path, root)}")
        if not dry_run:
            with open(path, "w", encoding="utf-8") as handle:
                handle.writelines(lines)
    print(f"{'would rewrite' if dry_run else 'rewrote'} {total} intra-workspace pin(s) to {version}")
    if not dry_run:
        moved = [e for e in wrong if e["target"] != version]
        if moved:
            print(
                f"note: {len(moved)} pin(s) now declare {version} while the member itself is "
                f"{sorted({e['target'] for e in moved})} — sync the member versions too, or "
                "--check will report drift",
                file=sys.stderr,
            )


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--list", action="store_true", help="print every path edge to a member")
    mode.add_argument("--check", action="store_true", help="exit 1 on a stale pin")
    mode.add_argument("--sync", metavar="VERSION", help="rewrite member pins to VERSION")
    parser.add_argument("root", nargs="?", default=".", help="repository root")
    parser.add_argument("--dry-run", action="store_true", help="with --sync")
    args = parser.parse_args(argv)

    if tomllib is None:
        refuse("python3 has no tomllib (3.11+); the version-edge audit did not run")
    if args.sync is not None and not re.fullmatch(r"\d+\.\d+\.\d+", args.sync):
        refuse(f"not a semver version: {args.sync}")
    root = resolve_root(args.root)
    if not manifest_paths(root):
        refuse(f"no Cargo.toml under {root}")
    try:
        if args.list:
            return cmd_list(root)
        if args.check:
            return cmd_check(root)
        cmd_sync(root, args.sync, args.dry_run)
        return 0
    except ParseError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
