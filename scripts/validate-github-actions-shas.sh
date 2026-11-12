#!/usr/bin/env bash
# validate-github-actions-shas.sh - Verify GitHub Actions are pinned to valid SHAs
#
# Usage: validate-github-actions-shas.sh [--help] [--offline] [--verbose]
#
# Validates GitHub Actions in every .yml/.yaml file under .github/workflows:
# 1. Remote actions use a full commit SHA, not a tag or branch
# 2. The SHA has the expected 40-hex format
# 3. The SHA resolves in the upstream repository when the GitHub API is available
#
# Exit codes:
#   0 - No confirmed invalid pins; unavailable checks are SKIP unless required
#   1 - An invalid pin, or required online verification was unavailable
#   2 - Script error (missing files, etc.)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKFLOW_DIR="${REPO_ROOT}/.github/workflows"

OFFLINE=false
VERBOSE=false
REQUIRED=false
if [[ "${CSM_VALIDATE_GITHUB_ACTIONS_SHAS_REQUIRED:-}" == "true" ]]; then
    REQUIRED=true
fi

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

ERRORS=0
WARNINGS=0
UNVERIFIED=0
CHECKED=0
API_UNAVAILABLE=false
API_UNAVAILABLE_REASON=""
declare -A SHA_LOOKUP_RESULT=()
declare -A SHA_LOOKUP_REASON=()

usage() {
    cat <<'EOF'
validate-github-actions-shas.sh - Verify GitHub Actions are pinned to valid SHAs

USAGE:
    validate-github-actions-shas.sh [OPTIONS]

OPTIONS:
    -h, --help      Show this help message
    -o, --offline   Skip network-based SHA checks (reports SKIP — NOT A PASS)
    -v, --verbose   Show detailed output for each action

DESCRIPTION:
    Recursively scans .github/workflows for .yml and .yaml files and validates:
    1. Remote actions are pinned to a full commit SHA (not a tag or branch)
    2. SHA format is valid (40 hex characters)
    3. SHA resolves upstream when the GitHub API is available

    Set CSM_VALIDATE_GITHUB_ACTIONS_SHAS_REQUIRED=true to fail when online
    verification is unavailable. Without that flag, unavailable checks remain
    nonfatal but are explicitly reported as SKIP — NOT A PASS.

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

# Find all workflow files (both .yml and .yaml extensions)
WORKFLOW_FILES=()
while IFS= read -r -d '' file; do
    WORKFLOW_FILES+=("$file")
done < <(find "${WORKFLOW_DIR}" -type f \( -name "*.yml" -o -name "*.yaml" \) -print0 2>/dev/null)

if [[ ${#WORKFLOW_FILES[@]} -eq 0 ]]; then
    echo "${YELLOW}WARNING:${NC} No workflow files found in ${WORKFLOW_DIR}"
    if [[ "${REQUIRED}" == "true" ]]; then
        echo "${YELLOW}SKIP — NOT A PASS:${NC} required verification found no workflow files"
        exit 1
    fi
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
log_skip() {
    echo "${YELLOW}SKIP — NOT A PASS:${NC} $1"
    UNVERIFIED=$((UNVERIFIED + 1))
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

check_sha_exists() {
    local owner_repo="$1"
    local sha="$2"
    local file="$3"
    local line_num="$4"
    local endpoint="repos/${owner_repo}/commits/${sha}"
    local cache_key="${owner_repo}@${sha}"
    local cached_result="${SHA_LOOKUP_RESULT[${cache_key}]:-}"
    local response
    local status_line
    local status_code=""
    local reason

    case "${cached_result}" in
        verified)
            log_verbose "SHA ${sha} exists in ${owner_repo} (cached)"
            return 0
            ;;
        invalid)
            log_error "${file}:${line_num}: SHA ${sha} does not exist in ${owner_repo} (GitHub confirmed no commit)"
            return 1
            ;;
        unverified)
            log_skip "cached result for ${owner_repo}@${sha} is unverified: ${SHA_LOOKUP_REASON[${cache_key}]}"
            return 0
            ;;
    esac

    if [[ "${OFFLINE}" == "true" ]]; then
        reason="online verification disabled (--offline)"
        SHA_LOOKUP_RESULT["${cache_key}"]="unverified"
        SHA_LOOKUP_REASON["${cache_key}"]="${reason}"
        API_UNAVAILABLE=true
        API_UNAVAILABLE_REASON="${reason}"
        log_skip "${reason} for ${owner_repo}@${sha}"
        return 0
    fi

    if [[ "${API_UNAVAILABLE}" == "true" ]]; then
        reason="online verification stopped after ${API_UNAVAILABLE_REASON}"
        SHA_LOOKUP_RESULT["${cache_key}"]="unverified"
        SHA_LOOKUP_REASON["${cache_key}"]="${reason}"
        log_skip "${reason}; cannot verify ${owner_repo}@${sha}"
        return 0
    fi

    if ! command -v gh >/dev/null 2>&1; then
        reason="gh CLI is unavailable"
        SHA_LOOKUP_RESULT["${cache_key}"]="unverified"
        SHA_LOOKUP_REASON["${cache_key}"]="${reason}"
        API_UNAVAILABLE=true
        API_UNAVAILABLE_REASON="${reason}"
        log_skip "${reason}; cannot verify ${owner_repo}@${sha}"
        return 0
    fi

    # --include preserves the HTTP status. Only GitHub's explicit 422
    # "No commit found for SHA" proves a pin invalid; 404 can also mean the
    # repository is private or the token cannot access it.
    if response="$(gh api --include --jq '.sha // .message' "${endpoint}" 2>&1)"; then
        SHA_LOOKUP_RESULT["${cache_key}"]="verified"
        log_verbose "SHA ${sha} exists in ${owner_repo}"
        return 0
    fi

    status_line="${response%%$'\n'*}"
    if [[ "${status_line}" =~ ^HTTP/[0-9.]+[[:space:]]422[[:space:]] ]] \
        && [[ "${response}" == *"No commit found for SHA: ${sha}"* ]]; then
        SHA_LOOKUP_RESULT["${cache_key}"]="invalid"
        log_error "${file}:${line_num}: SHA ${sha} does not exist in ${owner_repo} (GitHub confirmed no commit)"
        return 1
    fi

    if [[ "${status_line}" =~ ^HTTP/[0-9.]+[[:space:]]([0-9]{3}) ]]; then
        status_code="${BASH_REMATCH[1]}"
    else
        status_line="network or gh API error"
    fi
    reason="${status_line}"
    case "${status_code}" in
        401|403|429|5??|"")
            API_UNAVAILABLE=true
            API_UNAVAILABLE_REASON="${reason}"
            ;;
    esac
    SHA_LOOKUP_RESULT["${cache_key}"]="unverified"
    SHA_LOOKUP_REASON["${cache_key}"]="${reason}"
    log_skip "could not verify ${owner_repo}@${sha}: ${reason}"
    return 0
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

validate_action() {
    local uses_line="$1"
    local file="$2"
    local line_num="$3"
    local uses_pattern='^([a-zA-Z0-9_.-]+)/([a-zA-Z0-9_.-]+)(/[^@[:space:]#]+)?@([^[:space:]#]+)$'
    local unpinned_pattern='^[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+(/[^@[:space:]#]+)?$'
    local action_value
    local owner_repo
    local ref

    action_value="${uses_line#*uses:}"
    action_value="${action_value%%#*}"
    action_value="${action_value#"${action_value%%[![:space:]]*}"}"
    action_value="${action_value%"${action_value##*[![:space:]]}"}"
    if [[ "${action_value}" == \"*\" ]] || [[ "${action_value}" == \'*\' ]]; then
        action_value="${action_value:1:${#action_value}-2}"
    fi

    # Local and Docker actions do not resolve to commits in an upstream repo.
    if [[ "${action_value}" == ./* || "${action_value}" == ../* \
        || "${action_value}" == docker://* ]]; then
        log_verbose "Skipping local or Docker action: ${action_value}"
        return 0
    fi

    if [[ "${action_value}" =~ ${uses_pattern} ]]; then
        # A nested action path still resolves its commit in owner/repo.
        owner_repo="${BASH_REMATCH[1]}/${BASH_REMATCH[2]}"
        ref="${BASH_REMATCH[4]}"
    else
        if [[ "${action_value}" == *"\${{"* ]]; then
            CHECKED=$((CHECKED + 1))
            log_error "${file}:${line_num}: dynamic action reference cannot be verified; use a literal SHA pin"
            return 1
        fi
        if [[ "${action_value}" =~ ${unpinned_pattern} ]]; then
            CHECKED=$((CHECKED + 1))
            log_error "${file}:${line_num}: ${action_value} has no @ref (remote actions must be SHA-pinned)"
            return 1
        fi
        if [[ "${action_value}" =~ ^[a-zA-Z0-9_.-]+/ ]]; then
            CHECKED=$((CHECKED + 1))
            log_error "${file}:${line_num}: malformed remote action reference: ${action_value}"
            return 1
        fi

        log_verbose "Skipping non-GitHub action: ${action_value}"
        return 0
    fi

    CHECKED=$((CHECKED + 1))

    # Check 1: Must be SHA-pinned, not a version tag or branch.
    if is_version_tag "${ref}"; then
        log_error "${file}:${line_num}: ${owner_repo}@${ref} uses version tag (should be SHA-pinned)"
        log_verbose "  Suggested fix: Find SHA with: gh api repos/${owner_repo}/git/refs/tags/${ref}"
        return 1
    fi

    # Check 2: SHA must be valid format (40 hex chars).
    if ! is_valid_sha "${ref}"; then
        if [[ "${ref}" =~ ^[a-zA-Z0-9_-]+$ ]]; then
            log_error "${file}:${line_num}: ${owner_repo}@${ref} uses branch name (should be SHA-pinned)"
        else
            log_error "${file}:${line_num}: ${owner_repo}@${ref} has invalid SHA format (expected 40 hex chars)"
        fi
        return 1
    fi

    # Check 3: Verify the SHA when the GitHub API is reachable.
    check_sha_exists "${owner_repo}" "${ref}" "${file}" "${line_num}" || return 1

    # Check 4: Version comments are informational.
    local version_comment
    version_comment="$(extract_version_comment "${uses_line}")"
    if [[ -n "${version_comment}" ]]; then
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

echo ""
echo "=== Validation Summary ==="
echo "Workflow files: ${#WORKFLOW_FILES[@]}"
echo "Actions checked: ${CHECKED}"
echo "Errors: ${ERRORS}"
echo "Warnings: ${WARNINGS}"
echo "Unverified: ${UNVERIFIED}"
echo ""

if [[ "${ERRORS}" -gt 0 ]]; then
    echo "${RED}FAILED:${NC} ${ERRORS} actions need SHA pinning or contain invalid SHAs"
    echo ""
    echo "How to fix:"
    echo "  1. Find the commit SHA for the intended action release."
    echo "  2. Update the workflow to use OWNER/REPO@SHA # version."
    echo "  3. Re-run this validator with GitHub API access."
    exit 1
elif [[ "${UNVERIFIED}" -gt 0 ]]; then
    echo "${YELLOW}SKIP — NOT A PASS:${NC} ${UNVERIFIED} action SHA reference(s) could not be verified"
    if [[ "${REQUIRED}" == "true" ]]; then
        echo "${RED}FAILED:${NC} CSM_VALIDATE_GITHUB_ACTIONS_SHAS_REQUIRED=true"
        exit 1
    fi
    echo "No invalid SHA was confirmed; online verification remains incomplete."
    exit 0
else
    echo "${GREEN}PASSED:${NC} All remote actions are pinned to existing SHAs"
    exit 0
fi