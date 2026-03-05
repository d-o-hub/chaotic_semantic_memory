#!/usr/bin/env bash
# =============================================================================
# check-docs-links.sh - Validate links, commands, and references in docs
# =============================================================================
# Usage: ./scripts/check-docs-links.sh [--quick] [--check-urls] [--fix]
#
# Checks:
#   1. Internal file links (relative paths like @file.md or [text](./path.md))
#   2. External URLs (http/https links) - with --check-urls
#   3. Code block commands (bash commands in markdown code blocks)
#   4. Version references consistency
#
# Exit codes:
#   0 - All checks passed
#   1 - Issues found
#   2 - Error occurred
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

QUICK_MODE=false
CHECK_URLS=false
FIX_MODE=false

for arg in "$@"; do
    case $arg in
        --quick) QUICK_MODE=true ;;
        --check-urls) CHECK_URLS=true ;;
        --fix) FIX_MODE=true ;;
        --help|-h)
            echo "Usage: $0 [--quick] [--check-urls] [--fix]"
            echo ""
            echo "Options:"
            echo "  --quick       Skip URL checks (faster)"
            echo "  --check-urls  Check external URLs (slower, requires network)"
            echo "  --fix         Auto-fix what can be fixed"
            echo "  --help        Show this help"
            exit 0
            ;;
    esac
done

cd "${PROJECT_ROOT}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Counters
BROKEN_LINKS=0
BROKEN_URLS=0
INVALID_COMMANDS=0
VERSION_MISMATCH=0
TOTAL_CHECKED=0

# Get current version from Cargo.toml
CARGO_VERSION=$(grep -m1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
MAJOR_MINOR=$(echo "$CARGO_VERSION" | cut -d. -f1,2)

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Documentation Link & Command Checker${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Cargo.toml version: ${CARGO_VERSION}"
echo "Major.minor:        ${MAJOR_MINOR}"
echo ""

# =============================================================================
# 1. Check internal file links
# =============================================================================
echo -e "${CYAN}→ Checking internal file links...${NC}"

# Find all markdown files
MD_FILES=$(find . -name "*.md" -not -path "./target/*" -not -path "./.git/*" 2>/dev/null | sort)

check_file_link() {
    local link="$1"
    local source_file="$2"
    local source_dir
    source_dir=$(dirname "$source_file")
    
    # Handle different link formats
    local target_path
    
    # @file.md format (AGENTS.md style)
    if [[ "$link" =~ ^@(.*)$ ]]; then
        target_path="${BASH_REMATCH[1]}"
    # ./path or ../path format
    elif [[ "$link" =~ ^\.\.?/ ]]; then
        target_path="${source_dir}/${link}"
    # path.md format (relative to repo root in some contexts)
    elif [[ "$link" =~ ^[a-zA-Z] ]]; then
        target_path="${source_dir}/${link}"
    else
        return 0  # Skip non-file links
    fi
    
    # Clean up the path
    target_path=$(cd "$(dirname "$target_path")" 2>/dev/null && pwd)/$(basename "$target_path") 2>/dev/null || return 1
    
    # Check if file exists
    if [[ ! -f "$target_path" ]]; then
        return 1
    fi
    return 0
}

# Extract and check links from markdown files
while IFS= read -r md_file; do
    # Extract @file references (AGENTS.md style)
    # Skip npm packages (@scope/package) and GitHub mentions (@user)
    while IFS= read -r link; do
        # Skip npm package names (contain / like @d-o-hub/package)
        if [[ "$link" =~ ^@[^/]+/ ]]; then
            continue
        fi
        # Skip npm package names with underscore (@scope_package)
        if [[ "$link" =~ ^@.+_ ]]; then
            continue
        fi
        # Skip version tags (@v1.0.0)
        if [[ "$link" =~ ^@v[0-9] ]]; then
            continue
        fi
        # Skip email addresses (@domain.com)
        if [[ "$link" =~ ^@.*\.(com|io|org|net|dev) ]]; then
            continue
        fi
        # Skip GitHub mentions (single word, no extension, short)
        after_at="${link#@}"
        if [[ ! "$after_at" =~ \. ]] && [[ ${#after_at} -lt 20 ]]; then
            continue
        fi
        # Skip @-mentions pattern
        if [[ "$link" == "@-mentions" ]]; then
            continue
        fi
        # Skip example placeholders
        if [[ "$link" == "@file.md" ]] || [[ "$link" == "./path.md" ]]; then
            continue
        fi
        ((TOTAL_CHECKED++)) || true
        if ! check_file_link "$link" "$md_file"; then
            echo -e "${RED}✗${NC} $md_file: broken link '$link'"
            ((BROKEN_LINKS++)) || true
        fi
    done < <(grep -oE '@[a-zA-Z0-9_./-]+' "$md_file" 2>/dev/null | sort -u || true)
    
    # Extract [text](./path) style links
    while IFS= read -r link; do
        # Skip external URLs and anchors
        if [[ "$link" =~ ^https?:// ]] || [[ "$link" =~ ^# ]]; then
            continue
        fi
        # Skip example placeholders
        if [[ "$link" == "./path.md" ]] || [[ "$link" == "path.md" ]]; then
            continue
        fi
        ((TOTAL_CHECKED++)) || true
        if ! check_file_link "$link" "$md_file"; then
            echo -e "${RED}✗${NC} $md_file: broken link '$link'"
            ((BROKEN_LINKS++)) || true
        fi
    done < <(grep -oE '\]\([^)]+\)' "$md_file" 2>/dev/null | sed 's/\](\(.*\))/\1/' | sort -u || true)
done <<< "$MD_FILES"

if [[ "$BROKEN_LINKS" -eq 0 ]]; then
    echo -e "${GREEN}✓${NC} All internal file links valid"
fi

# =============================================================================
# 2. Check external URLs (optional, slower)
# =============================================================================
echo ""
echo -e "${CYAN}→ Checking external URLs...${NC}"

if $CHECK_URLS && ! $QUICK_MODE; then
    # Extract unique URLs from markdown files
    URLS=$(grep -rhoE 'https?://[^)>\s]+' . --include="*.md" 2>/dev/null | sort -u | head -50)
    
    for url in $URLS; do
        ((TOTAL_CHECKED++)) || true
        # Skip known stable URLs
        if [[ "$url" =~ shields\.io ]] || [[ "$url" =~ img\.shields\.io ]]; then
            continue
        fi
        
        # Check URL with timeout
        if curl --silent --head --max-time 5 "$url" > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC} $url"
        else
            echo -e "${RED}✗${NC} $url (failed or timeout)"
            ((BROKEN_URLS++)) || true
        fi
    done
    
    if [[ "$BROKEN_URLS" -eq 0 ]]; then
        echo -e "${GREEN}✓${NC} All external URLs valid"
    fi
else
    echo -e "${YELLOW}⊘${NC} Skipped (use --check-urls to enable)"
fi

# =============================================================================
# 3. Check code block commands
# =============================================================================
echo ""
echo -e "${CYAN}→ Checking code block commands...${NC}"

check_command() {
    local cmd="$1"
    local source_file="$2"
    
    # Skip commands that are clearly examples or placeholders
    if [[ "$cmd" =~ ^\# ]] || [[ "$cmd" =~ \<.*\> ]] || [[ "$cmd" =~ \.\.\. ]]; then
        return 0
    fi
    
    # Check if command exists
    local base_cmd
    base_cmd=$(echo "$cmd" | awk '{print $1}')
    
    # Skip common non-command patterns
    case "$base_cmd" in
        cargo|rustc|rustup|git|gh|npm|node|python|pip)
            if ! command -v "$base_cmd" &> /dev/null; then
                return 1
            fi
            ;;
        csm)
            # Check if csm binary exists
            if [[ ! -x "./target/release/csm" ]] && [[ ! -x "./target/debug/csm" ]]; then
                echo -e "${YELLOW}!${NC} $source_file: 'csm' binary not built (run: cargo build --release)"
                return 0  # Don't fail, just warn
            fi
            ;;
        ./scripts/*|scripts/*)
            local script_path="${base_cmd#./}"
            if [[ ! -f "$script_path" ]]; then
                return 1
            fi
            ;;
    esac
    return 0
}

# Check bash/shell code blocks
while IFS= read -r md_file; do
    # Extract commands from bash/sh code blocks
    in_bash_block=false
    while IFS= read -r line; do
        if [[ "$line" =~ ^\`\`\`(bash|sh|shell) ]]; then
            in_bash_block=true
            continue
        fi
        if [[ "$line" =~ ^\`\`\`$ ]] && $in_bash_block; then
            in_bash_block=false
            continue
        fi
        if $in_bash_block && [[ -n "$line" ]] && [[ ! "$line" =~ ^\# ]]; then
            ((TOTAL_CHECKED++)) || true
            # Extract the command (first word)
            base_cmd=$(echo "$line" | awk '{print $1}')
            if ! check_command "$base_cmd" "$md_file"; then
                echo -e "${RED}✗${NC} $md_file: command '$base_cmd' not found"
                ((INVALID_COMMANDS++)) || true
            fi
        fi
    done < "$md_file"
done <<< "$MD_FILES"

if [[ "$INVALID_COMMANDS" -eq 0 ]]; then
    echo -e "${GREEN}✓${NC} All commands valid"
fi

# =============================================================================
# 4. Check version references consistency
# =============================================================================
echo ""
echo -e "${CYAN}→ Checking version references...${NC}"

# Files that should reference the current version
VERSION_PATTERNS=(
    "wasm/package.json:\"version\": \"[0-9]+\.[0-9]+\.[0-9]+\""
    "README.md:chaotic_semantic_memory = { version = \"[0-9]+\.[0-9]+\""
    "book/src/getting-started.md:chaotic_semantic_memory = { version = \"[0-9]+\.[0-9]+\""
)

check_version_ref() {
    local file="$1"
    local pattern="$2"
    local expected="$3"
    
    if [[ ! -f "$file" ]]; then
        return 0  # Skip missing files
    fi
    
    local current
    current=$(grep -oE "$pattern" "$file" 2>/dev/null | head -1 || true)
    
    if [[ -z "$current" ]]; then
        return 0  # Pattern not found, skip
    fi
    
    if [[ ! "$current" =~ $expected ]]; then
        echo -e "${RED}✗${NC} $file: version mismatch (expected $expected, found: $current)"
        return 1
    fi
    return 0
}

# Check wasm/package.json has exact version
if [[ -f "wasm/package.json" ]]; then
    ((TOTAL_CHECKED++)) || true
    pkg_ver=$(grep '"version"' wasm/package.json | head -1 | sed 's/.*"\([0-9][^"]*\)".*/\1/')
    if [[ "$pkg_ver" != "$CARGO_VERSION" ]]; then
        echo -e "${RED}✗${NC} wasm/package.json: version $pkg_ver (expected $CARGO_VERSION)"
        ((VERSION_MISMATCH++)) || true
    else
        echo -e "${GREEN}✓${NC} wasm/package.json: version $pkg_ver"
    fi
fi

# Check README.md has major.minor
if [[ -f "README.md" ]]; then
    ((TOTAL_CHECKED++)) || true
    readme_ver=$(grep -oE 'chaotic_semantic_memory = \{ version = "[0-9]+\.[0-9]+"' README.md | head -1 | sed 's/.*"\([0-9]\+\.[0-9]\+\)".*/\1/' || true)
    if [[ -n "$readme_ver" ]] && [[ "$readme_ver" != "$MAJOR_MINOR" ]]; then
        echo -e "${RED}✗${NC} README.md: version $readme_ver (expected $MAJOR_MINOR)"
        ((VERSION_MISMATCH++)) || true
    elif [[ -n "$readme_ver" ]]; then
        echo -e "${GREEN}✓${NC} README.md: version $readme_ver"
    fi
fi

# Check book/src/getting-started.md has major.minor
if [[ -f "book/src/getting-started.md" ]]; then
    ((TOTAL_CHECKED++)) || true
    book_ver=$(grep -oE 'chaotic_semantic_memory = \{ version = "[0-9]+\.[0-9]+"' book/src/getting-started.md | head -1 | sed 's/.*"\([0-9]\+\.[0-9]\+\)".*/\1/' || true)
    if [[ -n "$book_ver" ]] && [[ "$book_ver" != "$MAJOR_MINOR" ]]; then
        echo -e "${RED}✗${NC} book/src/getting-started.md: version $book_ver (expected $MAJOR_MINOR)"
        ((VERSION_MISMATCH++)) || true
    elif [[ -n "$book_ver" ]]; then
        echo -e "${GREEN}✓${NC} book/src/getting-started.md: version $book_ver"
    fi
fi

# Check llms.txt has correct version
if [[ -f "llms.txt" ]]; then
    ((TOTAL_CHECKED++)) || true
    llms_ver=$(grep "^\*\*Version:\*\*" llms.txt | head -1 | sed 's/\*\*Version:\*\* *\([0-9]\+\.[0-9]\+\.[0-9]\+\).*/\1/' || true)
    if [[ -z "$llms_ver" ]]; then
        llms_ver=$(grep "Version:" llms.txt | head -1 | sed 's/.*: *\([0-9]\+\.[0-9]\+\.[0-9]\+\).*/\1/' || true)
    fi
    if [[ -n "$llms_ver" ]] && [[ "$llms_ver" != "$CARGO_VERSION" ]]; then
        echo -e "${RED}✗${NC} llms.txt: version $llms_ver (expected $CARGO_VERSION)"
        ((VERSION_MISMATCH++)) || true
    elif [[ -n "$llms_ver" ]]; then
        echo -e "${GREEN}✓${NC} llms.txt: version $llms_ver"
    fi
fi

if [[ "$VERSION_MISMATCH" -eq 0 ]]; then
    echo -e "${GREEN}✓${NC} All version references consistent"
fi

# =============================================================================
# Summary
# =============================================================================
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Summary${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Total checked:     ${TOTAL_CHECKED}"
echo "Broken links:      ${BROKEN_LINKS}"
echo "Broken URLs:       ${BROKEN_URLS}"
echo "Invalid commands:  ${INVALID_COMMANDS}"
echo "Version mismatches: ${VERSION_MISMATCH}"
echo ""

TOTAL_ISSUES=$((BROKEN_LINKS + BROKEN_URLS + INVALID_COMMANDS + VERSION_MISMATCH))

if [[ "$TOTAL_ISSUES" -gt 0 ]]; then
    echo -e "${RED}✗ Found ${TOTAL_ISSUES} issue(s)${NC}"
    echo ""
    echo "To fix version mismatches, run: ./scripts/sync-docs.sh"
    exit 1
else
    echo -e "${GREEN}✓ All checks passed${NC}"
    exit 0
fi