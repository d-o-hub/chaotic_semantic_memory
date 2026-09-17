#!/usr/bin/env bash
# scripts/scale-evidence.sh
#
# ADR-0095 Tier-2 scale evidence: runs the `scale_evidence` example for one
# mode and writes the machine-readable artifact plus an evidence manifest that
# records every field the ADR requires (commit, dirty state, corpus version
# and checksum, seed, feature set, command, toolchain, hardware, sample count,
# baseline, variance, result schema version).
#
# Usage:
#   scripts/scale-evidence.sh <ann|persistence|memory> [--out DIR] [extra args...]
#
# Example:
#   scripts/scale-evidence.sh ann --out plans/evidence/scale_2026_09_17
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <ann|persistence|memory> [--out DIR] [extra args...]" >&2
  exit 2
fi

MODE="$1"
shift

OUT_DIR="target/scale-evidence"
EXTRA=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      OUT_DIR="$2"
      shift 2
      ;;
    *)
      EXTRA+=("$1")
      shift
      ;;
  esac
done

FEATURES="persistence,ann-hnsw,ann-lsh"
case "${MODE}" in
  ann) ARTIFACT="ann_scale.json" ;;
  persistence) ARTIFACT="persistence_scale.json" ;;
  memory) ARTIFACT="memory_model.json" ;;
  *)
    echo "unknown mode: ${MODE}" >&2
    exit 2
    ;;
esac

mkdir -p "${OUT_DIR}"

COMMAND="cargo run --release --example scale_evidence --features ${FEATURES} -- ${MODE} --out ${OUT_DIR} ${EXTRA[*]:-}"
echo "==> ${COMMAND}"

export CARGO_TERM_PROGRESS_WHEN=never
# shellcheck disable=SC2086
cargo run --release --example scale_evidence --features "${FEATURES}" -- \
  "${MODE}" --out "${OUT_DIR}" "${EXTRA[@]}"

# --- Evidence manifest (ADR-0095) -------------------------------------------
# Values are exported and read by the generator below; no shell interpolation
# happens inside the Python source.
export EVIDENCE_ARTIFACT="${ARTIFACT}"
export EVIDENCE_COMMAND="${COMMAND}"
export EVIDENCE_FEATURES="${FEATURES}"
export GIT_COMMIT
GIT_COMMIT="$(git rev-parse HEAD)"
export GIT_DIRTY="false"
if [[ -n "$(git status --porcelain)" ]]; then
  export GIT_DIRTY="true"
fi
export RUSTC_VERSION
RUSTC_VERSION="$(rustc --version)"
export CARGO_VERSION
CARGO_VERSION="$(cargo --version)"
export CPU_MODEL
CPU_MODEL="$(awk -F': ' '/^model name/ {print $2; exit}' /proc/cpuinfo)"
export CPU_CORES
CPU_CORES="$(nproc)"
export MEM_TOTAL_KB
MEM_TOTAL_KB="$(awk '/^MemTotal:/ {print $2; exit}' /proc/meminfo)"
export OS_NAME
OS_NAME="$(uname -s)"
export ARCH
ARCH="$(uname -m)"
export KERNEL
KERNEL="$(uname -r)"

python3 - "${OUT_DIR}" <<'PY'
import json
import os
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
artifact_name = os.environ["EVIDENCE_ARTIFACT"]
doc = json.loads((out / artifact_name).read_text())

if doc.get("kind") == "ann_scale":
    seed = doc.get("seed", 42)
    checksum = doc["scales"][0].get("corpus_checksum", "n/a")
    samples = sum(scale.get("queries", 0) for scale in doc.get("scales", []))
    corpus_kind = doc.get("corpus_version", "synthetic-clustered-v1")
elif doc.get("kind") == "persistence_scale":
    seed = 42
    checksum = "synthetic-clustered-v1:seed42"
    samples = sum(scale.get("concepts", 0) for scale in doc.get("scales", []))
    corpus_kind = "synthetic-clustered-v1"
else:
    seed = 42
    checksum = "synthetic-clustered-v1:seed42"
    samples = len(doc.get("points", []))
    corpus_kind = doc.get("corpus_version", "synthetic-clustered-v1")

manifest = {
    "schema_version": 1,
    "artifact": artifact_name,
    "artifact_kind": doc.get("kind"),
    "commit": os.environ["GIT_COMMIT"],
    "dirty": os.environ["GIT_DIRTY"] == "true",
    "command": os.environ["EVIDENCE_COMMAND"],
    "features": os.environ["EVIDENCE_FEATURES"],
    "profile": "release",
    "dataset": {
        "kind": corpus_kind,
        "seed": seed,
        "checksum": checksum,
    },
    "toolchain": {
        "rustc": os.environ["RUSTC_VERSION"],
        "cargo": os.environ["CARGO_VERSION"],
    },
    "hardware": {
        "os": os.environ["OS_NAME"],
        "kernel": os.environ["KERNEL"],
        "arch": os.environ["ARCH"],
        "cpu": os.environ["CPU_MODEL"],
        "cores": int(os.environ["CPU_CORES"]),
        "mem_total_kb": int(os.environ["MEM_TOTAL_KB"]),
    },
    "samples": samples,
    "baseline": {
        "ref": "none",
        "note": "absolute ceilings and distribution statistics only; no A/B baseline",
    },
    "variance": {
        "kind": "percentiles",
        "reported": ["p50_us", "p95_us", "p99_us", "min_us", "max_us", "mean_us"],
    },
}

(out / "evidence.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"wrote {out / 'evidence.json'}")
PY
