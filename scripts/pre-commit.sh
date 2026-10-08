#!/usr/bin/env bash
# Pre-commit hook shim: delegates to .githooks/pre-commit
# For full validation, run: scripts/validate.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [[ -x "${REPO_ROOT}/.githooks/pre-commit" ]]; then
  exec "${REPO_ROOT}/.githooks/pre-commit" "$@"
else
  echo "❌ ${REPO_ROOT}/.githooks/pre-commit missing or not executable!"
  exit 1
fi
