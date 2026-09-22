#!/usr/bin/env bash
# skills-suggest.sh — offline rank-then-read shortlist over skill frontmatter.
# Stands in for `do-harness skills suggest` without the dependency (issue #749).
# Reads ONLY name/description frontmatter — never skill bodies.
set -euo pipefail

QUERY="${1:?usage: skills-suggest.sh \"<query>\" [limit]}"
LIMIT="${2:-5}"
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/.agents/skills"

python3 - "$QUERY" "$LIMIT" "$DIR" <<'EOF'
import sys, re, pathlib
query, limit, root = sys.argv[1], int(sys.argv[2]), sys.argv[3]

def tokens(text):
    return set(t for t in re.split(r'\W+', text.lower()) if len(t) > 2)

ALIAS = {  # tiny singular/plural + stem alias set; keep minimal
    "skills": "skill", "benches": "bench", "releases": "release",
    "queries": "query", "vectors": "vector", "indices": "index",
    "indexes": "index", "candidates": "candidate",
    # domain synonyms: metadata vocab is narrower than task phrasing
    "benchmarking": "benchmark", "benchmarks": "benchmark",
    "performance": "perf", "optimizing": "optimize",
    "retrieval": "retrieval", "scale": "scale", "scaling": "scale",
}
qtoks = tokens(query)
qtoks = {ALIAS.get(t, t) for t in qtoks}
results = []
for skill_dir in sorted(pathlib.Path(root).iterdir()):
    md = skill_dir / "SKILL.md"
    if not md.is_file():
        continue
    # frontmatter only: stream lines, stop at the closing ---; never read the body
    lines = []
    with md.open() as f:
        first = f.readline()
        if first.rstrip() != "---":
            continue  # no frontmatter block; not rankable
        for line in f:
            if line.rstrip() == "---":
                break
            lines.append(line)
    fm = "".join(lines)
    name = re.search(r'^name:\s*(.+)$', fm, re.M)
    desc = re.search(r'^description:\s*(.+)$', fm, re.M)
    name = name.group(1).strip() if name else skill_dir.name
    desc = desc.group(1).strip() if desc else ""
    ntoks = tokens(name)
    dtoks = tokens(desc)
    ntoks = {ALIAS.get(t, t) for t in ntoks}
    dtoks = {ALIAS.get(t, t) for t in dtoks}
    score = 2 * len(qtoks & ntoks) + len(qtoks & dtoks)
    if score:
        # path relative to the repo root, directly readable from where the
        # documented invocation runs
        path = f".agents/skills/{skill_dir.name}/SKILL.md"
        results.append((score, name, desc, path))
for score, name, desc, path in sorted(results, reverse=True)[:limit]:
    try:
        print(f"{score:>3}  {name:<28} {path}")
        print(f"     {desc[:110]}")
    except BrokenPipeError:
        break
EOF
