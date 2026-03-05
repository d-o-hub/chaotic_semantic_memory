#!/usr/bin/env bash
# =============================================================================
# check-docs-links.sh - Validate links, commands, and versions in docs
# =============================================================================
# Usage: ./scripts/check-docs-links.sh [--check-urls] [--fix]
#
# Checks:
#   1. Internal file links (relative paths like @file.md or [text](./path.md))
#   2. External URLs (http/https links) - with --check-urls
#   3. Code block commands (bash commands in markdown code blocks)
#   4. Version references consistency across ALL files:
#      - Core: Cargo.toml, Cargo.lock, wasm/package.json
#      - Docs: README.md, book/src/getting-started.md, CHANGELOG.md, llms.txt
#      - Tests: examples/cli/*.sh, tests/*.rs
#      - Generated: export.json, csm_test.json (gitignored)
#
# Exit codes:
#   0 - All checks passed
#   1 - Issues found
#   2 - Error occurred
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CHECK_URLS=false
FIX_MODE=false

for arg in "$@"; do
    case $arg in
        --check-urls) CHECK_URLS=true ;;
        --fix) FIX_MODE=true ;;
        --help|-h)
            echo "Usage: $0 [--check-urls] [--fix]"
            echo ""
            echo "Options:"
            echo "  --check-urls  Check external URLs (slower, requires network)"
            echo "  --fix         Auto-fix version mismatches"
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
FIXED=0

# Get current version from Cargo.toml
CARGO_VERSION=$(grep -m1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
if [[ -z "$CARGO_VERSION" ]]; then
    echo -e "${RED}✗ Could not extract version from Cargo.toml${NC}"
    exit 2
fi
MAJOR_MINOR=$(echo "$CARGO_VERSION" | cut -d. -f1,2)

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Documentation Link & Command Checker${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Cargo.toml version: ${CARGO_VERSION}"
echo "Major.minor:        ${MAJOR_MINOR}"
echo ""

# =============================================================================
# Helper function to check a version reference
# =============================================================================
check_version() {
    local file="$1"
    local expected="$2"
    local pattern="$3"
    local description="$4"
    local extract_cmd="$5"

    if [[ ! -f "$file" ]]; then
        echo -e "${YELLOW}⊘${NC} $description: file not found"
        return 0
    fi

    ((TOTAL_CHECKED++)) || true

    local current
    current=$(eval "$extract_cmd" 2>/dev/null || echo "")

    if [[ -z "$current" ]]; then
        echo -e "${YELLOW}?${NC} $description: pattern not found"
        return 0
    fi

    if [[ "$current" == "$expected" ]]; then
        echo -e "${GREEN}✓${NC} $description: $current"
        return 0
    else
        echo -e "${RED}✗${NC} $description: $current (expected $expected)"
        ((VERSION_MISMATCH++)) || true
        
        if $FIX_MODE && [[ -n "$pattern" ]]; then
            sed -i "$pattern" "$file" 2>/dev/null || true
            echo -e "${CYAN}↻${NC} Fixed: $description"
            ((FIXED++)) || true
        fi
        return 1
    fi
}

# =============================================================================
# 1. Check internal file links
# =============================================================================
echo -e "${CYAN}→ Checking internal file links...${NC}"

MD_FILES=$(find . -name "*.md" -not -path "./target/*" -not -path "./.git/*" 2>/dev/null | sort)

check_file_link() {
    local link="$1"
    local source_file="$2"
    local source_dir
    source_dir=$(dirname "$source_file")
    local target_path
    
    if [[ "$link" =~ ^@(.*)$ ]]; then
        target_path="${BASH_REMATCH[1]}"
    elif [[ "$link" =~ ^\.\./? ]]; then
        target_path="${source_dir}/${link}"
    elif [[ "$link" =~ ^[a-zA-Z] ]]; then
        target_path="${source_dir}/${link}"
    else
        return 0
    fi
    
    target_path=$(cd "$(dirname "$target_path")" 2>/dev/null && pwd)/$(basename "$target_path") 2>/dev/null || return 1
    
    [[ -f "$target_path" ]]
}

while IFS= read -r md_file; do
    # Extract @file references
    while IFS= read -r link; do
        # Skip npm packages (@scope/package), version tags (@v1.0.0), emails, GitHub mentions
        if [[ "$link" =~ ^@[^/]+/ ]]; then continue; fi
        if [[ "$link" =~ ^@v[0-9] ]]; then continue; fi  # Skip version tags like @v2.0.0
        if [[ "$link" =~ ^@.+_ ]]; then continue; fi
        if [[ "$link" =~ ^@.*\.(com|io|org|net|dev) ]]; then continue; fi
        after_at="${link#@}"
        if [[ ! "$after_at" =~ \. ]] && [[ ${#after_at} -lt 20 ]]; then continue; fi
        if [[ "$link" == "@-mentions" ]] || [[ "$link" == "@file.md" ]]; then continue; fi
        ((TOTAL_CHECKED++)) || true
        if ! check_file_link "$link" "$md_file"; then
            echo -e "${RED}✗${NC} $md_file: broken link '$link'"
            ((BROKEN_LINKS++)) || true
        fi
    done < <(grep -oE '@[a-zA-Z0-9_./-]+' "$md_file" 2>/dev/null | sort -u || true)
    
    # Extract [text](./path) style links
    while IFS= read -r link; do
        if [[ "$link" =~ ^https?:// ]] || [[ "$link" =~ ^# ]]; then continue; fi
        if [[ "$link" == "./path.md" ]] || [[ "$link" == "path.md" ]]; then continue; fi
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

if $CHECK_URLS; then
    URLS=$(grep -rhoE 'https?://[^)>\s]+' . --include="*.md" 2>/dev/null | sort -u | head -50)
    
    for url in $URLS; do
        ((TOTAL_CHECKED++)) || true
        if [[ "$url" =~ shields\.io ]] || [[ "$url" =~ img\.shields\.io ]]; then continue; fi
        
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

while IFS= read -r md_file; do
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
        if $in_bash_block && [[ -n "$line" ]] && [[ ! "$line" =~ ^# ]]; then
            ((TOTAL_CHECKED++)) || true
            base_cmd=$(echo "$line" | awk '{print $1}')
            case "$base_cmd" in
                cargo|rustc|rustup|git|gh|npm|node|python|pip)
                    if ! command -v "$base_cmd" &> /dev/null; then
                        echo -e "${RED}✗${NC} $md_file: command '$base_cmd' not found"
                        ((INVALID_COMMANDS++)) || true
                    fi
                    ;;
                ./scripts/*|scripts/*)
                    script_path="${base_cmd#./}"
                    if [[ ! -f "$script_path" ]]; then
                        echo -e "${RED}✗${NC} $md_file: script '$script_path' not found"
                        ((INVALID_COMMANDS++)) || true
                    fi
                    ;;
            esac
        fi
    done < "$md_file"
done <<< "$MD_FILES"

if [[ "$INVALID_COMMANDS" -eq 0 ]]; then
    echo -e "${GREEN}✓${NC} All commands valid"
fi

# =============================================================================
# 4. Check version references consistency (comprehensive)
# =============================================================================
echo ""
echo -e "${CYAN}→ Checking version references...${NC}"

# Core package files
echo -e "${BLUE}  Core Package Files${NC}"
echo -e "${GREEN}✓${NC} Cargo.toml: $CARGO_VERSION (source of truth)"
((TOTAL_CHECKED++)) || true

check_version "wasm/package.json" "$CARGO_VERSION" \
    "s/\"version\": \"[0-9]\+\.[0-9]\+\.[0-9]\+\"/\"version\": \"${CARGO_VERSION}\"/" \
    "wasm/package.json" \
    'grep "\"version\"" wasm/package.json | head -1 | sed "s/.*: *\"\([0-9.]\+\)\".*/\1/"'

check_version "Cargo.lock" "$CARGO_VERSION" \
    "" \
    "Cargo.lock" \
    'grep "name = \"chaotic_semantic_memory\"" Cargo.lock -A1 | grep version | head -1 | sed "s/version = \"\(.*\)\"/\1/"'

# pkg/ is generated by wasm-pack, skip if not tracked
if git ls-files pkg/package.json 2>/dev/null | grep -q .; then
    check_version "pkg/package.json" "$CARGO_VERSION" \
        "s/\"version\": \"[0-9]\+\.[0-9]\+\.[0-9]\+\"/\"version\": \"${CARGO_VERSION}\"/" \
        "pkg/package.json" \
        'grep "\"version\"" pkg/package.json | head -1 | sed "s/.*: *\"\([0-9.]\+\)\".*/\1/"'
elif [[ -f "pkg/package.json" ]]; then
    echo -e "${YELLOW}⊘${NC} pkg/package.json: generated file (not tracked in git)"
fi

# Documentation files
echo ""
echo -e "${BLUE}  Documentation Files${NC}"

check_version "README.md" "$MAJOR_MINOR" \
    "s/chaotic_semantic_memory = { version = \"[0-9]\+\.[0-9]\+\"/chaotic_semantic_memory = { version = \"${MAJOR_MINOR}\"/g" \
    "README.md (installation)" \
    'grep "chaotic_semantic_memory = { version" README.md | head -1 | sed "s/.*version = \"\([0-9.]\+\)\".*/\1/"'

check_version "book/src/getting-started.md" "$MAJOR_MINOR" \
    "s/chaotic_semantic_memory = { version = \"[0-9]\+\.[0-9]\+\"/chaotic_semantic_memory = { version = \"${MAJOR_MINOR}\"/g" \
    "book/src/getting-started.md" \
    'grep "chaotic_semantic_memory = { version" book/src/getting-started.md | head -1 | sed "s/.*version = \"\([0-9.]\+\)\".*/\1/"'

# CHANGELOG.md - check for current version header
if [[ -f "CHANGELOG.md" ]]; then
    ((TOTAL_CHECKED++)) || true
    if grep -q "## \[$CARGO_VERSION\]" CHANGELOG.md; then
        echo -e "${GREEN}✓${NC} CHANGELOG.md: has [$CARGO_VERSION] section"
    else
        echo -e "${YELLOW}?${NC} CHANGELOG.md: no [$CARGO_VERSION] section (may be unreleased)"
    fi
fi

check_version "llms.txt" "$CARGO_VERSION" \
    "" \
    "llms.txt" \
    'grep -oE "[0-9]+\.[0-9]+\.[0-9]+" llms.txt | head -1'

check_version "llms-full.txt" "$CARGO_VERSION" \
    "" \
    "llms-full.txt" \
    'grep -oE "[0-9]+\.[0-9]+\.[0-9]+" llms-full.txt | head -1'

# Test & Example files
echo ""
echo -e "${BLUE}  Test & Example Files${NC}"

for f in examples/cli/*.sh; do
    if [[ -f "$f" ]]; then
        ver=$(grep -oE '"version"[[:space:]]*:[[:space:]]*"[0-9.]+"' "$f" 2>/dev/null | head -1 | sed 's/.*"\([0-9.]\+\)".*/\1/' || true)
        if [[ -n "$ver" ]]; then
            ((TOTAL_CHECKED++)) || true
            if [[ "$ver" == "$CARGO_VERSION" ]]; then
                echo -e "${GREEN}✓${NC} $f: $ver"
            else
                echo -e "${RED}✗${NC} $f: $ver (expected $CARGO_VERSION)"
                ((VERSION_MISMATCH++)) || true
                if $FIX_MODE; then
                    sed -i "s/\"version\"[[:space:]]*:[[:space:]]*\"[0-9.]\+\"/\"version\": \"${CARGO_VERSION}\"/g" "$f"
                    echo -e "${CYAN}↻${NC} Fixed: $f"
                    ((FIXED++)) || true
                fi
            fi
        fi
    fi
done

for f in tests/*.rs; do
    if [[ -f "$f" ]]; then
        ver=$(grep -oE '"version"[[:space:]]*:[[:space:]]*"[0-9.]+"' "$f" 2>/dev/null | head -1 | sed 's/.*"\([0-9.]\+\)".*/\1/' || true)
        if [[ -n "$ver" ]]; then
            ((TOTAL_CHECKED++)) || true
            if [[ "$ver" == "$CARGO_VERSION" ]]; then
                echo -e "${GREEN}✓${NC} $f: $ver"
            else
                echo -e "${RED}✗${NC} $f: $ver (expected $CARGO_VERSION)"
                ((VERSION_MISMATCH++)) || true
                if $FIX_MODE; then
                    sed -i "s/\"version\"[[:space:]]*:[[:space:]]*\"[0-9.]\+\"/\"version\": \"${CARGO_VERSION}\"/g" "$f"
                    echo -e "${CYAN}↻${NC} Fixed: $f"
                    ((FIXED++)) || true
                fi
            fi
        fi
    fi
done

# Generated/Test JSON files
echo ""
echo -e "${BLUE}  Generated/Test JSON Files${NC}"

if [[ -f "export.json" ]]; then
    check_version "export.json" "$CARGO_VERSION" \
        "s/\"version\": \"[0-9.]\+\"/\"version\": \"${CARGO_VERSION}\"/" \
        "export.json" \
        'grep "\"version\"" export.json | head -1 | sed "s/.*: *\"\([0-9.]\+\)\".*/\1/"'
fi

if [[ -f "csm_test.json" ]]; then
    check_version "csm_test.json" "$CARGO_VERSION" \
        "s/\"version\": \"[0-9.]\+\"/\"version\": \"${CARGO_VERSION}\"/" \
        "csm_test.json" \
        'grep "\"version\"" csm_test.json | head -1 | sed "s/.*: *\"\([0-9.]\+\)\".*/\1/"'
fi

# pkg/README.md (if tracked)
if git ls-files pkg/README.md 2>/dev/null | grep -q .; then
    ver=$(grep "| Version" pkg/README.md 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || true)
    if [[ -n "$ver" ]]; then
        ((TOTAL_CHECKED++)) || true
        if [[ "$ver" == "$CARGO_VERSION" ]]; then
            echo -e "${GREEN}✓${NC} pkg/README.md: $ver"
        else
            echo -e "${RED}✗${NC} pkg/README.md: $ver (expected $CARGO_VERSION)"
            ((VERSION_MISMATCH++)) || true
        fi
    fi
elif [[ -f "pkg/README.md" ]]; then
    echo -e "${YELLOW}⊘${NC} pkg/README.md: generated file (not tracked in git)"
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
if $FIX_MODE; then
    echo "Files fixed:       ${FIXED}"
fi
echo ""

TOTAL_ISSUES=$((BROKEN_LINKS + BROKEN_URLS + INVALID_COMMANDS + VERSION_MISMATCH))

if [[ "$TOTAL_ISSUES" -gt 0 ]]; then
    if $FIX_MODE; then
        echo -e "${YELLOW}⚠ Fixed ${FIXED} version mismatch(es)${NC}"
        echo ""
        echo "Run these commands to complete:"
        echo "  ./scripts/sync-docs.sh"
        echo "  git add -A && git commit -m 'fix: sync versions to ${CARGO_VERSION}'"
    else
        echo -e "${RED}✗ Found ${TOTAL_ISSUES} issue(s)${NC}"
        echo ""
        echo "Run with --fix to auto-fix version mismatches, or:"
        echo "  ./scripts/sync-docs.sh"
    fi
    exit 1
else
    echo -e "${GREEN}✓ All checks passed${NC}"
    exit 0
fi