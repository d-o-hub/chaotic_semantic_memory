#!/usr/bin/env bash
# scripts/disk-reclaim.sh — Clean cargo targets across all known Rust projects
# Usage: ./scripts/disk-reclaim.sh [--dry-run]
set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN=true
fi

GIT_DIR="${HOME}/git"

# Find all directories containing Cargo.toml under ~/git
PROJECTS=()
while IFS= read -r cargo_toml; do
  dir=$(dirname "$cargo_toml")
  # Only top-level projects (have a target/ dir or are git roots)
  if [[ -d "${dir}/target" ]]; then
    PROJECTS+=("$dir")
  fi
done < <(find "$GIT_DIR" -maxdepth 2 -name "Cargo.toml" -type f 2>/dev/null)

if [[ ${#PROJECTS[@]} -eq 0 ]]; then
  echo "No Rust projects with target/ dirs found under ${GIT_DIR}"
  exit 0
fi

TOTAL_BEFORE=0
for dir in "${PROJECTS[@]}"; do
  size=$(du -sm "${dir}/target" 2>/dev/null | cut -f1)
  TOTAL_BEFORE=$((TOTAL_BEFORE + size))
  printf "%-50s %4dMB\n" "${dir/#$HOME/~}" "$size"
done

echo "---"
printf "Total reclaimable: %dMB\n\n" "$TOTAL_BEFORE"

if $DRY_RUN; then
  echo "(dry run — no changes made)"
  exit 0
fi

read -rp "Clean all targets? [y/N] " confirm
if [[ "$confirm" != [yY] ]]; then
  echo "Aborted."
  exit 0
fi

for dir in "${PROJECTS[@]}"; do
  echo "Cleaning ${dir/#$HOME/~}..."
  (cd "$dir" && cargo clean 2>/dev/null) || rm -rf "${dir}/target"
done

echo "Done. Reclaimed ~${TOTAL_BEFORE}MB."
