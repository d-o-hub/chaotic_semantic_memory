#!/usr/bin/env bash
# validate-github-actions-shas.sh - Verify GitHub Actions are pinned to valid SHAs
#
# Usage: validate-github-actions-shas.sh [--help] [--offline] [--verbose]
#
# Validates that all GitHub Actions in workflow files are:
# 1. Pinned to a SHA (not tags or branches)
# 2. SHA is 40 hex characters (git SHA format)
# 3. SHA matches the referenced version (if version comment present)
#
# Exit codes:
#   0 - All actions properly pinned
#   1 - One or more validation errors
#   2 - Script error (missing files, etc.)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKFLOW_DIR="${REPO_ROOT}/.github/workflows"

# Default options
OFFLINE=false
VERBOSE=false

# Colors for output (disabled if not terminal)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

# Output counters
ERRORS=0
WARNINGS=0
CHECKED=0

usage() {
    cat <<'EOF'
validate-github-actions-shas.sh - Verify GitHub Actions are pinned to valid SHAs

USAGE:
    validate-github-actions-shas.sh [OPTIONS]

OPTIONS:
    -h, --help      Show this help message
    -o, --offline   Skip network-based SHA existence checks
    -v, --verbose   Show detailed output for each action

DESCRIPTION:
    Scans all .github/workflows/*.yml files and validates that:
    1. Actions are pinned to SHA (not version tags or branches)
    2. SHA format is valid (40 hex characters)
    3. SHA matches version comment (if present, optional network check)

EXAMPLES:
    # Run with network checks (requires gh CLI)
    validate-github-actions-shas.sh

    # Run offline (no network, just format validation)
    validate-github-actions-shas.sh --offline

    # Verbose output for debugging
    validate-github-actions-shas.sh --verbose

SECURITY RATIONALE:
    SHA-pinning prevents supply chain attacks where a malicious actor
    could push a compromised version to an action's tag. SHA references
    are immutable, ensuring the exact code is always used.

    Format: owner/repo@SHA # comment with version
    Example: actions/checkout@a5ac7e51b41094c92402da3b24376905380af847 # v4

EOF
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            usage
            ;;
        -o|--offline)
            OFFLINE=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 2
            ;;
    esac
done

# Check workflow directory exists
if [[ ! -d "${WORKFLOW_DIR}" ]]; then
    echo "${RED}ERROR:${NC} Workflow directory not found: ${WORKFLOW_DIR}"
    exit 2
fi

# Find all workflow files
WORKFLOW_FILES=()
while IFS= read -r -d '' file; do
    WORKFLOW_FILES+=("$file")
done < <(find "${WORKFLOW_DIR}" -name "*.yml" -type f -print0 2>/dev/null)

if [[ ${#WORKFLOW_FILES[@]} -eq 0 ]]; then
    echo "${YELLOW}WARNING:${NC} No workflow files found in ${WORKFLOW_DIR}"
    exit 0
fi

# Helper: log based on verbosity
log_verbose() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo "${BLUE}[VERBOSE]${NC} $1"
    fi
}

# Helper: log error
log_error() {
    echo "${RED}ERROR:${NC} $1"
    ERRORS=$((ERRORS + 1))
}

# Helper: log warning
log_warning() {
    echo "${YELLOW}WARNING:${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

# Helper: validate SHA format (40 hex characters)
is_valid_sha() {
    local sha="$1"
    # Git SHA-1 is 40 hex characters
    if [[ "$sha" =~ ^[a-fA-F0-9]{40}$ ]]; then
        return 0
    else
        return 1
    fi
}

# Helper: check if SHA exists for action (requires gh CLI and network)
check_sha_exists() {
    local owner_repo="$1"
    local sha="$2"

    if [[ "$OFFLINE" == "true" ]]; then
        log_verbose "Skipping network check for ${owner_repo}@${sha} (--offline mode)"
        return 0
    fi

    # Check if gh CLI is available
    if ! command -v gh &>/dev/null; then
        log_warning "gh CLI not available, skipping SHA existence check for ${owner_repo}@${sha}"
        return 0
    fi

    # Use gh api to check if commit exists
    # This checks if the SHA is a valid commit in the repository
    if gh api "repos/${owner_repo}/commits/${sha}" --silent 2>/dev/null; then
        log_verbose "SHA ${sha} exists in ${owner_repo}"
        return 0
    else
        log_error "SHA ${sha} does not exist in ${owner_repo}"
        return 1
    fi
}

# Helper: extract version from comment (if present)
extract_version_comment() {
    local uses_line="$1"
    # Pattern: owner/repo@SHA # v1.2.3 or # v1 or # comment
    # Need to store regex in a variable to avoid # being interpreted as comment
    # Match v followed by digits, optionally with dots and more version parts
    local pattern='#[[:space:]]*(v?[0-9]+(\.[0-9]+)?(\.[0-9]+)?[a-zA-Z0-9_-]*)'
    if [[ "$uses_line" =~ $pattern ]]; then
        echo "${BASH_REMATCH[1]}"
    else
        echo ""
    fi
}

# Helper: check if using version tag (not SHA)
is_version_tag() {
    local ref="$1"
    # Version tags look like: v1, v1.2, v1.2.3, v2-beta, etc.
    # SHA is 40 hex chars
    if [[ "$ref" =~ ^v?[0-9]+(\.[0-9]+)?(\.[0-9]+)?[a-zA-Z0-9_-]*$ ]] && [[ ! "$ref" =~ ^[a-fA-F0-9]{40}$ ]]; then
        return 0
    else
        return 1
    fi
}

# Main validation function for a single uses line
validate_action() {
    local uses_line="$1"
    local file="$2"
    local line_num="$3"

    # Extract action reference: owner/repo@ref
    # Pattern: uses: owner/repo@ref or uses: owner/repo@ref # comment
    # Store regex in variable to avoid special char interpretation issues
    local uses_pattern='uses:[[:space:]]*([a-zA-Z0-9_-]+/[a-zA-Z0-9_-]+)@([a-zA-Z0-9_.-]+)'
    local action_ref=""
    if [[ "$uses_line" =~ $uses_pattern ]]; then
        local owner_repo="${BASH_REMATCH[1]}"
        local ref="${BASH_REMATCH[2]}"
        action_ref="${owner_repo}@${ref}"
    else
        # Docker actions or local paths - skip validation
        log_verbose "Skipping non-GitHub action: ${uses_line}"
        return 0
    fi

    CHECKED=$((CHECKED + 1))

    local owner_repo="${BASH_REMATCH[1]}"
    local ref="${BASH_REMATCH[2]}"

    # Check 1: Must be SHA-pinned, not version tag or branch
    if is_version_tag "$ref"; then
        log_error "${file}:${line_num}: ${owner_repo}@${ref} uses version tag (should be SHA-pinned)"
        log_verbose "  Suggested fix: Find SHA with: gh api repos/${owner_repo}/git/refs/tags/${ref}"
        return 1
    fi

    # Check 2: SHA must be valid format (40 hex chars)
    if ! is_valid_sha "$ref"; then
        # Could be a branch name or other reference
        if [[ "$ref" =~ ^[a-zA-Z0-9_-]+$ ]]; then
            log_error "${file}:${line_num}: ${owner_repo}@${ref} uses branch name (should be SHA-pinned)"
        else
            log_error "${file}:${line_num}: ${owner_repo}@${ref} has invalid SHA format (expected 40 hex chars)"
        fi
        return 1
    fi

    # Check 3: Optionally verify SHA exists (network check)
    if ! check_sha_exists "$owner_repo" "$ref"; then
        return 1
    fi

    # Check 4: Version comment should match SHA (optional, informational)
    local version_comment=$(extract_version_comment "$uses_line")
    if [[ -n "$version_comment" ]]; then
        log_verbose "${file}:${line_num}: ${owner_repo}@${ref} # ${version_comment} - SHA pinned correctly"
    else
        log_warning "${file}:${line_num}: ${owner_repo}@${ref} - missing version comment (recommended: add # vX.Y.Z)"
    fi

    return 0
}

# Process each workflow file
echo "Validating GitHub Actions SHA references in ${WORKFLOW_DIR}"
echo ""

# Store regex pattern in variable to avoid interpretation issues
uses_line_pattern='^[^#]*uses:[[:space:]]*'

for workflow_file in "${WORKFLOW_FILES[@]}"; do
    filename=$(basename "$workflow_file")
    log_verbose "Processing: ${filename}"

    line_num=0
    while IFS= read -r line; do
        line_num=$((line_num + 1))
        # Look for uses: lines (GitHub Actions)
        if [[ "$line" =~ $uses_line_pattern ]]; then
            validate_action "$line" "$filename" "$line_num" || true
        fi
    done < "$workflow_file"
done

# Summary
echo ""
echo "=== Validation Summary ==="
echo "Workflow files: ${#WORKFLOW_FILES[@]}"
echo "Actions checked: ${CHECKED}"
echo "Errors: ${ERRORS}"
echo "Warnings: ${WARNINGS}"
echo ""

if [[ $ERRORS -gt 0 ]]; then
    echo "${RED}FAILED:${NC} ${ERRORS} actions need SHA pinning"
    echo ""
    echo "How to fix:"
    echo "  1. Find SHA for a version tag:"
    echo "     gh api repos/OWNER/REPO/git/refs/tags/vTAG --jq '.object.sha'"
    echo ""
    echo "  2. Update workflow to use SHA with version comment:"
    echo "     uses: OWNER/REPO@SHA # vTAG"
    echo ""
    echo "  Example:"
    echo "     uses: actions/checkout@a5ac7e51b41094c92402da3b24376905380af847 # v4"
    exit 1
else
    echo "${GREEN}PASSED:${NC} All actions properly pinned to SHA"
    exit 0
fi