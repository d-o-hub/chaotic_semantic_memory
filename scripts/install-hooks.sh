#!/bin/bash
# Install git hooks that mirror CI checks locally
set -e

HOOKS_DIR="$(git rev-parse --git-dir)/hooks"

cp scripts/hooks/pre-push "$HOOKS_DIR/pre-push"
chmod +x "$HOOKS_DIR/pre-push"

echo "✅ Git hooks installed"
