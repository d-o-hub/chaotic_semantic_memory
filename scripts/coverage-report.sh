#!/usr/bin/env bash
# scripts/coverage-report.sh
#
# Behavior-based test evidence (ADR-0095): raw test-attribute counts are
# inventory only, so this reports *unique compiled behavior* plus line/branch
# coverage where the toolchain can produce it.
#
# Usage:
#   scripts/coverage-report.sh inventory   # fast, grep + parse (default)
#   scripts/coverage-report.sh llvm-cov    # slow, instrumented workspace build
#
# `inventory` never runs tests. `llvm-cov` needs `llvm-tools-preview` for the
# active toolchain (`rustup component add llvm-tools-preview`).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

MODE="${1:-inventory}"

inventory() {
  python3 - <<'PY'
import collections
import pathlib
import re

FN_RE = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(", re.M)
ATTR_RE = re.compile(r"#\[(?:tokio::)?test\]")

LAYERS = {
    "tests/ (root integration)": sorted(pathlib.Path("tests").glob("*.rs")),
    "src/ (root facade)": sorted(pathlib.Path("src").rglob("*.rs")),
    "crates/*/src (owners)": sorted(p for p in pathlib.Path("crates").rglob("*.rs") if "src" in p.parts),
    "crates/*/tests": sorted(p for p in pathlib.Path("crates").rglob("*.rs") if "tests" in p.parts),
}

def tests_in(path: pathlib.Path) -> list[str]:
    src = path.read_text()
    names: list[str] = []
    lines = src.splitlines()
    for i, line in enumerate(lines):
        if not ATTR_RE.search(line):
            continue
        # the test fn starts on this line or shortly after (other attributes may follow)
        for probe in lines[i : i + 6]:
            match = FN_RE.search(probe)
            if match:
                names.append(match.group(1))
                break
    return names

by_layer: dict[str, list[tuple[pathlib.Path, str]]] = {}
for layer, files in LAYERS.items():
    entries = [(p, n) for p in files for n in tests_in(p)]
    by_layer[layer] = entries

print("layer                        files  tests  unique(file, fn)")
total = 0
all_entries: list[tuple[pathlib.Path, str]] = []
for layer, entries in by_layer.items():
    files = len({p for p, _ in entries}) if entries else len(LAYERS[layer])
    unique = len({(str(p), n) for p, n in entries})
    print(f"{layer:<28} {files:>5}  {len(entries):>5}  {unique:>5}")
    total += len(entries)
    all_entries.extend(entries)

unique_total = len({(str(p), n) for p, n in all_entries})
print(f"{'TOTAL':<28} {'':>5}  {total:>5}  {unique_total:>5}")
print()

names = collections.defaultdict(set)
for path, name in all_entries:
    names[name].add(str(path))
shared = {n: paths for n, paths in names.items() if len(paths) > 1}
print(f"distinct test names: {len(names)}; names present in more than one file: {len(shared)}")
if shared:
    print("(same name in two files is a duplicate *lead*, not proof — compare the bodies)")
    for name, paths in sorted(shared.items())[:10]:
        print(f"  {name}: {', '.join(sorted(paths))}")
PY
}

# Branch coverage (`-Z coverage-options=branch`) is nightly-only; line coverage
# works on any toolchain. Override with COVERAGE_TOOLCHAIN=stable (line only).
COVERAGE_TOOLCHAIN="${COVERAGE_TOOLCHAIN:-nightly}"
BRANCH_FLAG=(--branch)

if [[ "${COVERAGE_TOOLCHAIN}" != "nightly" && "${COVERAGE_TOOLCHAIN}" != *-nightly ]]; then
  BRANCH_FLAG=()
  echo "note: branch coverage needs a nightly toolchain; reporting line coverage only" >&2
fi

llvm_cov() {
  if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "cargo-llvm-cov is not installed: cargo install cargo-llvm-cov" >&2
    exit 1
  fi
  if ! cargo "+${COVERAGE_TOOLCHAIN}" llvm-cov --version >/dev/null 2>&1; then
    echo "cargo-llvm-cov cannot run on toolchain '${COVERAGE_TOOLCHAIN}'." >&2
    echo "Install it or pick another: rustup component add llvm-tools-preview --toolchain <name>" >&2
    exit 1
  fi

  local target_flags=(--lib)
  if [[ "${INCLUDE_TESTS:-0}" == "1" ]]; then
    target_flags=(--lib --tests)
  fi

  mkdir -p target/coverage
  echo "==> line + branch coverage (toolchain: ${COVERAGE_TOOLCHAIN}; targets: ${target_flags[*]})"
  CARGO_BUILD_JOBS=2 cargo "+${COVERAGE_TOOLCHAIN}" llvm-cov \
    --workspace \
    --exclude csm-duckdb \
    "${target_flags[@]}" \
    "${BRANCH_FLAG[@]}" \
    --summary-only

  echo
  echo "==> per-file JSON artifact"
  CARGO_BUILD_JOBS=2 cargo "+${COVERAGE_TOOLCHAIN}" llvm-cov \
    --workspace \
    --exclude csm-duckdb \
    "${target_flags[@]}" \
    "${BRANCH_FLAG[@]}" \
    --json \
    --output-path target/coverage/coverage.json \
    --summary-only >/dev/null
  echo "wrote target/coverage/coverage.json"
}

case "${MODE}" in
  inventory) inventory ;;
  llvm-cov) llvm_cov ;;
  llvm-cov-all)
    # Adds the integration targets (tests/*.rs, 71 binaries) to the measurement:
    # slower and heavier, but it is the only way unit-only numbers do not
    # understate behavior coverage.
    INCLUDE_TESTS=1 llvm_cov
    ;;
  *)
    echo "unknown mode: ${MODE} (expected inventory | llvm-cov | llvm-cov-all)" >&2
    exit 2
    ;;
esac
