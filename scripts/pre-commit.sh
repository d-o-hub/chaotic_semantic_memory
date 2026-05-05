#!/usr/bin/env bash
# Pre-commit hook: Fast checks only (fmt + LOC gate + CHANGELOG + security)
# For full validation, run: scripts/validate.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

MAX_SRC_LOC=500

echo "Running pre-commit checks..."

# Check formatting
echo " → Checking formatting..."
cargo fmt -- --check

# LOC gate (fast)
echo " → Checking LOC limits (< ${MAX_SRC_LOC})..."
for file in $(find src -name '*.rs'); do
  loc="$(wc -l < "${file}")"
  if [ "${loc}" -gt "${MAX_SRC_LOC}" ]; then
    echo "❌ LOC gate failed: ${file} has ${loc} lines (max: ${MAX_SRC_LOC})"
    exit 1
  fi
done

# CHANGELOG duplicate header guardrail (prevents release workflow failures)
echo " → Checking CHANGELOG format..."
if [[ -f "CHANGELOG.md" ]]; then
  # Check for duplicate version headers (each version should appear exactly once)
  DUPLICATES=$(grep "^## \\[" CHANGELOG.md | cut -d'[' -f2 | cut -d']' -f1 | sort | uniq -d)
  if [[ -n "$DUPLICATES" ]]; then
    echo "❌ Duplicate CHANGELOG headers found: $DUPLICATES"
    echo "   Each version should have exactly one '## [VERSION] - YYYY-MM-DD' header"
    exit 1
  fi
  
  # Check that version headers have dates
  NO_DATE=$(grep "^## \\[" CHANGELOG.md | grep -v " - [0-9]\\{4\\}-[0-9]\\{2\\}-[0-9]\\{2\\}" | grep -v "\\[Unreleased\\]" || true)
  if [[ -n "$NO_DATE" ]]; then
    echo "❌ CHANGELOG headers missing dates:"
    echo "$NO_DATE"
    echo "   Format: ## [VERSION] - YYYY-MM-DD"
    exit 1
  fi
fi

# Docs sync: regenerate llms.txt files
echo " → Regenerating llms.txt files..."
if [[ -f "scripts/gen-llms-txt.sh" ]]; then
    bash scripts/gen-llms-txt.sh 2>/dev/null || true
    echo "   ✓ llms.txt files regenerated"
fi

# Docs version sync check (will fail if other docs need updates)
echo " → Checking docs version sync..."
bash scripts/sync-docs.sh --check

# Clippy lint (catches CI failures early)
echo " → Checking clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Gitleaks secret scanning (optional - only if installed)
if command -v gitleaks >/dev/null 2>&1; then
  echo " → Scanning for secrets (gitleaks)..."
  # Scan only staged files for speed
  if git diff --cached --quiet; then
    echo "   skip: no staged changes to scan"
  else
    # Create temporary file with staged content
    STAGED_TMP=$(mktemp)
    git diff --cached --name-only > "$STAGED_TMP"
    if [[ -s "$STAGED_TMP" ]]; then
      gitleaks protect --staged --verbose 2>/dev/null || {
        echo "❌ Gitleaks detected potential secrets in staged files!"
        echo "   Review and remove sensitive data before committing"
        rm -f "$STAGED_TMP"
        exit 1
      }
    fi
    rm -f "$STAGED_TMP"
    echo "   ✓ No secrets detected"
  fi
else
  echo " → Gitleaks not installed (optional)"
  echo "   Install with: brew install gitleaks || apt install gitleaks"
fi

# Skill symlink validation (optional - only if skills exist)
if [[ -d ".claude/skills" ]] && [[ -x "${SCRIPT_DIR}/validate-skills.sh" ]]; then
  echo " → Validating skill symlinks..."
  "${SCRIPT_DIR}/validate-skills.sh" --verbose 2>/dev/null || {
    echo "❌ Invalid skill symlinks detected!"
    echo "   Run: scripts/setup-skills.sh --force"
    exit 1
  }
fi

# GitHub Actions SHA validation (optional - opt-in via env var)
# Note: Disabled by default as existing workflows use version tags
# To enable: export CSM_VALIDATE_GITHUB_ACTIONS_SHAS=true
if [[ -x "${SCRIPT_DIR}/validate-github-actions-shas.sh" ]] && [[ "${CSM_VALIDATE_GITHUB_ACTIONS_SHAS:-}" == "true" ]]; then
  echo " → Validating GitHub Actions SHAs..."
  "${SCRIPT_DIR}/validate-github-actions-shas.sh" --offline || {
    echo "❌ GitHub Actions not properly pinned to SHA!"
    echo "   Run: scripts/validate-github-actions-shas.sh --verbose"
    exit 1
  }
else
  echo "skip: GitHub Actions SHA validation (use CSM_VALIDATE_GITHUB_ACTIONS_SHAS=true to enable)"
fi

# Update coverage metrics in README.md (only if src files changed)
STAGED_SRC=$(git diff --cached --name-only 2>/dev/null | grep -c "^src/" || echo "0")
if [[ "$STAGED_SRC" -gt 0 ]] && [[ -x "${SCRIPT_DIR}/update-coverage.sh" ]]; then
  echo " → Updating coverage metrics..."
  "${SCRIPT_DIR}/update-coverage.sh"
  # Re-add README.md if it was modified
  git diff --quiet README.md || git add README.md
fi

echo "✅ Pre-commit checks passed!"
