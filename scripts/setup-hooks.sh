#!/usr/bin/env bash
# Setup script to install Git hooks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
HOOKS_DIR="${PROJECT_ROOT}/.git/hooks"

# Install pre-commit hook
if [ -f "${SCRIPT_DIR}/pre-commit.sh" ]; then
  cp "${SCRIPT_DIR}/pre-commit.sh" "${HOOKS_DIR}/pre-commit"
  chmod +x "${HOOKS_DIR}/pre-commit"
  echo "✅ Pre-commit hook installed!"
else
  echo "❌ pre-commit.sh not found in scripts/"
  exit 1
fi

# Install post-commit hook
POST_COMMIT_HOOK="${HOOKS_DIR}/post-commit"
cat > "$POST_COMMIT_HOOK" << 'HOOK_EOF'
#!/bin/bash
# post-commit hook to auto-update architecture diagram
# Calls scripts/gen-agents-context.sh to regenerate the draw.io diagram

set -e

# Prevent infinite recursion when this hook modifies commits.
LOCK_FILE=".git/hooks/.post-commit-lock"
if [ -f "$LOCK_FILE" ]; then
    exit 0
fi

touch "$LOCK_FILE"
trap 'rm -f "$LOCK_FILE"' EXIT

# Check if relevant files changed
CHANGED_FILES=$(git diff-tree --no-commit-id --name-only -r HEAD)

# Only update diagram if source files, skills, or plans changed
if echo "$CHANGED_FILES" | grep -qE "^(src/|\.agents/skills/|plans/|Cargo\.toml)"; then
    echo "📊 Updating architecture diagram..."
    "$(git rev-parse --show-toplevel)/scripts/gen-agents-context.sh"
    echo "   Run 'git add docs/architecture/agents-context.drawio' to include in next commit"
fi

# Generate llms-full.txt when source files change
if echo "$CHANGED_FILES" | grep -qE "^(src/|Cargo\.toml)"; then
    echo "📝 Updating llms-full.txt..."
    "$(git rev-parse --show-toplevel)/scripts/gen-llms-txt.sh"
    echo "   Run 'git add llms-full.txt' to include in next commit"
fi
HOOK_EOF

chmod +x "$POST_COMMIT_HOOK"
echo "✅ Post-commit hook installed!"

echo ""
echo "Hooks installed. Run 'scripts/validate.sh' for full validation before pushing."
