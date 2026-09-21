#!/usr/bin/env bash
# scripts/bench-baseline.sh
#
# Canonical Criterion baseline for release claims (ADR-0095 Tier 3): "named
# reference runner and toolchain", "canonical Criterion baselines and confidence
# intervals".
#
# Criterion baselines normally live in `target/criterion/`, which is git-ignored
# and wiped by `cargo clean`, so `pre-release-validate.sh`'s
# `--baseline main` could never compare against anything in CI or on a fresh
# checkout. This script exports the measurements into a committed artifact
# instead.
#
# Usage:
#   scripts/bench-baseline.sh save    [--set core|all]   # (re)record the canonical baseline
#   scripts/bench-baseline.sh compare [--set core|all] [--tolerance 0.20]
#
# `core` (default) measures the claims named in the benchmarking skill table —
# `benches/benchmark.rs` plus `benches/persistence_benchmark.rs`. `all` adds the
# remaining targets and takes much longer.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

MODE="${1:-}"; shift || true
SET="core"
TOLERANCE="0.20"
ADVISORY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --set) SET="$2"; shift 2 ;;
    --set=*) SET="${1#*=}"; shift ;;
    --tolerance) TOLERANCE="$2"; shift 2 ;;
    --tolerance=*) TOLERANCE="${1#*=}"; shift ;;
    --advisory) ADVISORY=true; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

case "${MODE}" in
  save|compare) ;;
  *) echo "usage: $0 <save|compare> [--set core|all|<target>] [--tolerance 0.20] [--advisory]" >&2; exit 2 ;;
esac

ARTIFACT_DIR="plans/evidence/bench"
BASELINE_FILE="${ARTIFACT_DIR}/canonical.json"

case "${SET}" in
  core)
    benches=(benchmark persistence_benchmark)
    ;;
  all)
    benches=(benchmark persistence_benchmark binary_benchmark bm25_benchmark
             graph_candidates_benchmark hamming_benchmark hybrid_benchmark
             rerank_benchmark embedding_benchmark)
    ;;
  *)
    # explicit comma-separated bench targets, e.g. --set persistence_benchmark
    IFS=',' read -r -a benches <<< "${SET}"
    for bench in "${benches[@]}"; do
      if [[ ! -f "benches/${bench}.rs" ]]; then
        echo "unknown set or bench target: ${bench}" >&2
        exit 2
      fi
    done
    ;;
esac

mkdir -p "${ARTIFACT_DIR}"
echo "==> reference runner profile: plans/REFERENCE_RUNNER.md"

for bench in "${benches[@]}"; do
  echo "==> cargo bench --bench ${bench}"
  CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" cargo bench --bench "${bench}" -- \
    --warm-up-time 1 --measurement-time 3 >/dev/null
done

CANDIDATE_FILE="${ARTIFACT_DIR}/.candidate.json"
python3 - "${CANDIDATE_FILE}" "${BASELINE_FILE}" "${MODE}" "${TOLERANCE}" "${ADVISORY}" <<'PY'
import json
import pathlib
import subprocess
import sys

candidate_path = pathlib.Path(sys.argv[1])
baseline_path = pathlib.Path(sys.argv[2])
mode = sys.argv[3]
tolerance = float(sys.argv[4])
advisory = sys.argv[5] == "true"

root = pathlib.Path("target/criterion")
measurements: dict[str, dict[str, float]] = {}
for estimates in sorted(root.glob("**/new/estimates.json")):
    bench_id = str(estimates.parent.parent.relative_to(root))
    data = json.loads(estimates.read_text())
    median = data.get("median", {})
    measurements[bench_id] = {
        "median_ns": median.get("point_estimate"),
        "ci_lower_ns": (median.get("confidence_interval") or {}).get("lower_bound"),
        "ci_upper_ns": (median.get("confidence_interval") or {}).get("upper_bound"),
    }

if not measurements:
    print("no criterion estimates found under target/criterion", file=sys.stderr)
    sys.exit(1)


def git(*args: str) -> str:
    return subprocess.run(["git", *args], capture_output=True, text=True, check=False).stdout.strip()


manifest = {
    "schema_version": 1,
    "artifact": "criterion_baseline",
    "mode": mode,
    "commit": git("rev-parse", "HEAD"),
    "dirty": bool(git("status", "--porcelain", "--", ".", ":!plans/evidence")),
    "toolchain": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
    "cpu": next((l.split(":", 1)[1].strip() for l in pathlib.Path("/proc/cpuinfo").read_text().splitlines() if l.startswith("model name")), "unknown"),
    "os": f"{subprocess.run(['uname', '-s'], capture_output=True, text=True).stdout.strip()} {subprocess.run(['uname', '-r'], capture_output=True, text=True).stdout.strip()}",
    "command": f"scripts/bench-baseline.sh {mode}",
    "warm_up_seconds": 1,
    "measurement_seconds": 3,
    "sample_count_per_bench": None,
    "benchmarks": measurements,
}

if mode == "save":
    baseline_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(f"wrote {baseline_path} ({len(measurements)} benchmarks)")
    sys.exit(0)

candidate_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
if not baseline_path.exists():
    print(f"no canonical baseline at {baseline_path}; run `scripts/bench-baseline.sh save` first", file=sys.stderr)
    sys.exit(1)

baseline = json.loads(baseline_path.read_text())
base_benchmarks = baseline.get("benchmarks", {})
regressions = []
noise = []
missing = []
for name, current in sorted(measurements.items()):
    previous = base_benchmarks.get(name)
    if previous is None or previous.get("median_ns") in (None, 0) or current.get("median_ns") in (None, 0):
        missing.append(name)
        continue
    delta = current["median_ns"] / previous["median_ns"] - 1.0
    overlaps = (current.get("ci_lower_ns") or 0) <= (previous.get("ci_upper_ns") or float("inf"))
    if delta > tolerance and not overlaps:
        flag = "REGRESSION"
        regressions.append((name, delta))
    elif delta > tolerance:
        # above tolerance but the baseline's upper bound still covers the new
        # median: on a laptop this is usually load/thermal noise, so it is
        # reported as such instead of failing a release.
        flag = "noise?"
        noise.append((name, delta))
    else:
        flag = "ok" if delta > -tolerance else "faster"
    print(f"{flag:<11} {name:<60} {delta * 100:+7.2f}%  ci_overlap={overlaps}")

print()
print(f"baseline: {baseline_path} ({baseline.get('commit', '?')[:8]}, cpu={baseline.get('cpu', '?')})")
print(
    f"tolerance: ±{tolerance * 100:.0f}%; benchmarks compared: {len(measurements) - len(missing)}; "
    f"not in baseline: {len(missing)}; above tolerance but within baseline CI: {len(noise)}"
)

if regressions:
    print(f"REGRESSIONS beyond tolerance with non-overlapping confidence intervals: {len(regressions)}", file=sys.stderr)
    for name, delta in regressions:
        print(f"  {name}: {delta * 100:+.2f}%", file=sys.stderr)
    if advisory:
        print("advisory mode: not failing the caller; re-run to confirm before acting", file=sys.stderr)
        sys.exit(0)
    sys.exit(1)
print("PASS: no regression beyond tolerance with non-overlapping confidence intervals")
PY

rm -f "${CANDIDATE_FILE}"
