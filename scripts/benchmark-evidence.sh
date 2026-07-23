#!/usr/bin/env bash
# Generate benchmark evidence manifest with commit, dataset, seed, features,
# command, hardware, samples, variance, and baseline.
set -euo pipefail

OUT_DIR="${1:-benchmarks/results/ci}"
MANIFEST="$OUT_DIR/evidence.json"

# Gather metadata
COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
DATASET="benchmarks/datasets/v1/small"
FEATURES="default"
COMMAND="cargo run --manifest-path benchmarks/Cargo.toml -- --dataset-dir $DATASET --mode retrieval-only --out-dir $OUT_DIR"

# Hardware info
OS=$(uname -s)
ARCH=$(uname -m)
CPUS=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "unknown")
MEMORY_KB=$(grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2}' || echo "unknown")

# Load summary for samples and variance
SUMMARY="$OUT_DIR/summary.json"
SAMPLES=$(python3 -c "import json; print(json.load(open('$SUMMARY')).get('cases', 0))" 2>/dev/null || echo "0")

# Baseline (previous commit or empty)
BASELINE=$(git rev-parse HEAD~1 2>/dev/null || echo "")

cat > "$MANIFEST" <<EOF
{
  "commit": "$COMMIT",
  "timestamp": "$TIMESTAMP",
  "dataset": "$DATASET",
  "seed": 42,
  "features": "$FEATURES",
  "command": "$COMMAND",
  "hardware": {
    "os": "$OS",
    "arch": "$ARCH",
    "cpus": $CPUS,
    "memory_kb": ${MEMORY_KB:-0}
  },
  "samples": $SAMPLES,
  "baseline": "$BASELINE"
}
EOF

echo "Wrote evidence manifest: $MANIFEST"
