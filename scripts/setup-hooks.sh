#!/usr/bin/env bash
# Setup script to install Git hooks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOKS_DIR="${SCRIPT_DIR}/../.git/hooks"

# Install pre-commit hook
if [ -f "${SCRIPT_DIR}/pre-commit.sh" ]; then
  cp "${SCRIPT_DIR}/pre-commit.sh" "${HOOKS_DIR}/pre-commit"
  chmod +x "${HOOKS_DIR}/pre-commit"
  echo "✅ Pre-commit hook installed!"
else
  echo "❌ pre-commit.sh not found in scripts/"
  exit 1
fi

# Keep post-commit hook if it exists (for diagram updates)
if [ -f "${HOOKS_DIR}/post-commit" ]; then
  echo "✅ Post-commit hook already exists (kept for diagram updates)"
fi

echo ""
echo "Hooks installed. Run 'scripts/validate.sh' for full validation before pushing."
