#!/usr/bin/env bash
# scripts/rebuild.sh — Clean rebuild from scratch (release mode by default)
# Usage: ./scripts/rebuild.sh [--debug]
set -euo pipefail

MODE="--release"
if [[ "${1:-}" == "--debug" ]]; then
  MODE=""
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> cargo clean"
cargo clean

echo "==> cargo build ${MODE:-debug}"
# shellcheck disable=SC2086
cargo build $MODE

echo "Done. $(du -sh target/ | cut -f1) in target/"
