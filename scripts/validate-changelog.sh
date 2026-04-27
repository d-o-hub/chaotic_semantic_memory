#!/usr/bin/env bash
# validate-changelog.sh - Comprehensive CHANGELOG validation
# Production-ready validation with defensive error handling
#
# Usage: ./scripts/validate-changelog.sh [VERSION]
#   If VERSION is not provided, extracts from Cargo.toml
#
# Validates:
#   1. Version header exists (## [VERSION] format)
#   2. No duplicate headers
#   3. Header has date (Keep a Changelog format: YYYY-MM-DD)
#
# Exit codes:
#   0 - Validation passed
#   1 - Validation failed with actionable error message

set -euo pipefail

VERSION="${1:-}"

# Extract version from Cargo.toml if not provided
if [[ -z "$VERSION" ]]; then
  VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d'"' -f2)
  if [[ -z "$VERSION" ]]; then
    echo "::error::Could not extract version from Cargo.toml"
    exit 1
  fi
fi

echo "Validating CHANGELOG.md for version: ${VERSION}"

# 1. Check version header exists
# Pattern: ## [VERSION] - matches Keep a Changelog format
# Note: The ## prefix is required (markdown heading), not [VERSION] at line start
HEADER_PATTERN="^## \\[${VERSION}\\]"

if ! grep -q "${HEADER_PATTERN}" CHANGELOG.md; then
  echo "::error::No CHANGELOG entry for version ${VERSION}"
  echo "   Expected format: ## [${VERSION}] - YYYY-MM-DD"
  echo "   Current headers in CHANGELOG.md:"
  grep "^## \\[" CHANGELOG.md | head -5 || echo "   (none found)"
  exit 1
fi

# 2. Guardrail: Check for duplicate headers
# Multiple headers break release note extraction (awk stops at first match)
HEADER_COUNT=$(grep -c "${HEADER_PATTERN}" CHANGELOG.md || true)

if [[ "${HEADER_COUNT}" -gt 1 ]]; then
  echo "::error::Duplicate CHANGELOG header for ${VERSION} (${HEADER_COUNT} occurrences)"
  echo "   This breaks release note extraction."
  echo "   Fix: Keep only one '## [${VERSION}] - YYYY-MM-DD'"
  echo "   Found at:"
  grep -n "${HEADER_PATTERN}" CHANGELOG.md
  exit 1
fi

# 3. Guardrail: Verify header has date (Keep a Changelog format)
# Pattern: ## [VERSION] - YYYY-MM-DD
DATE_PATTERN="^## \\[${VERSION}\\] - [0-9]\\{4\\}-[0-9]\\{2\\}-[0-9]\\{2\\}"

if ! grep -q "${DATE_PATTERN}" CHANGELOG.md; then
  echo "::error::CHANGELOG header for ${VERSION} missing date"
  echo "   Expected format: ## [${VERSION}] - YYYY-MM-DD"
  echo "   Found header:"
  grep "${HEADER_PATTERN}" CHANGELOG.md
  exit 1
fi

# Extract the date for confirmation
HEADER_DATE=$(grep "${DATE_PATTERN}" CHANGELOG.md | sed 's/.*- //' | head -1)
echo "✅ CHANGELOG validation passed for ${VERSION} (date: ${HEADER_DATE})"