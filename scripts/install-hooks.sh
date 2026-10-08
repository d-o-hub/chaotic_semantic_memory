#!/usr/bin/env bash
# =============================================================================
# install-hooks.sh - Canonical git hooks installer
# =============================================================================
# Configures core.hooksPath to .githooks and installs hooks into git common dir.
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

case "${1:-}" in
  ""|--link) ;;
  *) echo "Error: unknown argument: $1" >&2; exit 1 ;;
esac
if [[ $# -gt 1 ]]; then
  echo "Error: expected at most one argument (--link)" >&2
  exit 1
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "❌ Error: Not inside a git repository." >&2
  exit 1
fi

GIT_COMMON_DIR="$(git rev-parse --git-common-dir)"
HOOKS_TARGET_DIR="${GIT_COMMON_DIR}/hooks"
GITHOOKS_DIR="${REPO_ROOT}/.githooks"

if [[ ! -d "${GITHOOKS_DIR}" ]]; then
  echo "❌ Error: .githooks directory not found at ${GITHOOKS_DIR}" >&2
  exit 1
fi

# Check the entire set before modifying hooks or configuration.
for hook in pre-commit pre-push; do
  if [[ ! -f "${GITHOOKS_DIR}/${hook}" ]]; then
    echo "❌ Required hook ${hook} missing in ${GITHOOKS_DIR}" >&2
    exit 1
  fi
done

echo "Installing git hooks from .githooks/ ..."

# Copy hooks to git common dir as fallback
mkdir -p "${HOOKS_TARGET_DIR}"
for hook in pre-commit pre-push; do
  chmod 755 "${GITHOOKS_DIR}/${hook}"
  cp "${GITHOOKS_DIR}/${hook}" "${HOOKS_TARGET_DIR}/${hook}"
  chmod 755 "${HOOKS_TARGET_DIR}/${hook}"
  echo "  ✓ Installed ${hook} to ${HOOKS_TARGET_DIR}/${hook}"
done

# Local config is shared by linked worktrees. Relative .githooks resolves in
# the active worktree; an absolute sibling path redirects every other checkout.
# Keep --link as a compatibility argument with the same worktree-safe behavior.
git config --local core.hooksPath .githooks
echo "  ✓ Configured core.hooksPath=.githooks (repo-local, worktree-safe)"

echo "✅ Git hooks successfully installed and configured!"
