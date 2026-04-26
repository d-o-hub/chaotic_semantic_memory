#!/usr/bin/env bash
# Setup script to install Git hooks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
HOOKS_DIR="${PROJECT_ROOT}/.git/hooks"

# Install pre-commit hook
if [ -f "${SCRIPT_DIR}/pre-commit.sh" ]; then
  cp "${SCRIPT_DIR}/pre-commit.sh" "${HOOKS_DIR}/pre-commit"
  chmod 755 "${HOOKS_DIR}/pre-commit"
  echo "✅ Pre-commit hook installed!"
else
  echo "❌ pre-commit.sh not found in scripts/"
  exit 1
fi

# Remove post-commit hook if present
rm -f "${HOOKS_DIR}/post-commit"
echo "✅ Post-commit hook removed (manual generation only)."

echo ""
echo "Hooks installed. Run 'scripts/validate.sh' for full validation before pushing."
