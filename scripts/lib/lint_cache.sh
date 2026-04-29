#!/usr/bin/env bash
# lint_cache.sh - Hash-based lint caching library
#
# Store cache in .git/lint-cache/ (survives clean but not hard reset)
# Hash file content + config file content together for cache invalidation
#
# Usage:
#   source scripts/lib/lint_cache.sh
#   if lint_cache_needs_check "src/main.rs"; then
#       # run linting...
#       lint_cache_mark_checked "src/main.rs"
#   fi

set -euo pipefail

# Cache directory (inside .git so it survives `git clean` but not `git reset --hard`)
LINT_CACHE_DIR="${LINT_CACHE_DIR:-.git/lint-cache}"

# Config files whose changes invalidate the cache
LINT_CONFIG_FILES=(
    "Cargo.toml"
    "Cargo.lock"
    ".clippy.toml"
    "clippy.toml"
    "rustfmt.toml"
    ".rustfmt.toml"
    "rust-toolchain.toml"
    ".cargo/config.toml"
    ".cargo/config"
)

# Associative array for config hash memoization
declare -gA _LINT_CONFIG_HASH_CACHE=()
declare -g _LINT_COMBINED_CONFIG_HASH=""

# ==============================================================================
# Internal helper: Get portable sha256 hash
# ==============================================================================
_lint_hash_string() {
    local input="$1"
    if command -v sha256sum &>/dev/null; then
        echo -n "$input" | sha256sum | cut -d' ' -f1
    elif command -v shasum &>/dev/null; then
        echo -n "$input" | shasum -a 256 | cut -d' ' -f1
    else
        echo "ERROR: Neither sha256sum nor shasum available" >&2
        return 1
    fi
}

_lint_hash_file() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        echo ""
        return 0
    fi
    if command -v sha256sum &>/dev/null; then
        sha256sum "$file" | cut -d' ' -f1
    elif command -v shasum &>/dev/null; then
        shasum -a 256 "$file" | cut -d' ' -f1
    else
        echo "ERROR: Neither sha256sum nor shasum available" >&2
        return 1
    fi
}

# ==============================================================================
# Function: lint_cache_compute_config_hash
# Compute combined hash of all config files (memoized)
# Globals:
#   _LINT_COMBINED_CONFIG_HASH - cached combined hash
#   _LINT_CONFIG_HASH_CACHE - per-file hash cache
# Returns:
#   Combined sha256 hash of all config files
# ==============================================================================
lint_cache_compute_config_hash() {
    # Return cached value if available
    if [[ -n "${_LINT_COMBINED_CONFIG_HASH:-}" ]]; then
        echo "$_LINT_COMBINED_CONFIG_HASH"
        return 0
    fi

    local combined=""
    local config_file
    local file_hash

    for config_file in "${LINT_CONFIG_FILES[@]}"; do
        if [[ -f "$config_file" ]]; then
            # Check memoization cache first
            if [[ -n "${_LINT_CONFIG_HASH_CACHE[$config_file]:-}" ]]; then
                file_hash="${_LINT_CONFIG_HASH_CACHE[$config_file]}"
            else
                file_hash="$(_lint_hash_file "$config_file")"
                _LINT_CONFIG_HASH_CACHE["$config_file"]="$file_hash"
            fi
            combined="${combined}${config_file}:${file_hash}:"
        fi
    done

    # Hash the combined string
    _LINT_COMBINED_CONFIG_HASH="$(_lint_hash_string "$combined")"
    echo "$_LINT_COMBINED_CONFIG_HASH"
}

# ==============================================================================
# Function: lint_cache_compute_file_hash
# Compute hash of a source file combined with config hash
# Arguments:
#   $1 - File path to hash
# Returns:
#   Combined sha256 hash for cache key
# ==============================================================================
lint_cache_compute_file_hash() {
    local file="$1"

    if [[ ! -f "$file" ]]; then
        echo ""
        return 1
    fi

    local file_hash
    local config_hash

    file_hash="$(_lint_hash_file "$file")"
    config_hash="$(lint_cache_compute_config_hash)"

    # Combine file hash with config hash
    _lint_hash_string "${file_hash}:${config_hash}"
}

# ==============================================================================
# Function: lint_cache_get_cache_path
# Get the cache file path for a given source file
# Arguments:
#   $1 - File path
# Returns:
#   Path to the cache entry file
# ==============================================================================
lint_cache_get_cache_path() {
    local file="$1"
    local file_hash

    file_hash="$(lint_cache_compute_file_hash "$file")"

    # Create a safe filename from the original path
    local safe_path
    safe_path="$(echo "$file" | tr '/\\' '_')"

    echo "${LINT_CACHE_DIR}/${safe_path}.${file_hash}"
}

# ==============================================================================
# Function: lint_cache_needs_check
# Check if a file needs to be linted (cache miss or changed)
# Arguments:
#   $1 - File path to check
# Returns:
#   0 if file needs linting (cache miss or stale)
#   1 if file is up to date (cache hit)
# ==============================================================================
lint_cache_needs_check() {
    local file="$1"

    # File must exist
    if [[ ! -f "$file" ]]; then
        return 1
    fi

    local cache_path
    cache_path="$(lint_cache_get_cache_path "$file")"

    # Cache hit if the file exists
    if [[ -f "$cache_path" ]]; then
        return 1  # No need to check
    fi

    return 0  # Needs checking
}

# ==============================================================================
# Function: lint_cache_mark_checked
# Mark a file as having been linted (store in cache)
# Arguments:
#   $1 - File path to mark
#   $2 - Optional: lint result (pass/fail/skip), defaults to "pass"
# Returns:
#   0 on success
# ==============================================================================
lint_cache_mark_checked() {
    local file="$1"
    local result="${2:-pass}"

    if [[ ! -f "$file" ]]; then
        return 1
    fi

    # Ensure cache directory exists
    mkdir -p "$LINT_CACHE_DIR"

    local cache_path
    cache_path="$(lint_cache_get_cache_path "$file")"

    # Store timestamp and result
    {
        echo "# Lint cache for: $file"
        echo "# Timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
        echo "# Result: $result"
        echo "# Config hash: $(lint_cache_compute_config_hash)"
    } > "$cache_path"

    return 0
}

# ==============================================================================
# Function: lint_cache_clear
# Clear the entire lint cache
# Returns:
#   0 on success
# ==============================================================================
lint_cache_clear() {
    if [[ -d "$LINT_CACHE_DIR" ]]; then
        rm -rf "$LINT_CACHE_DIR"
    fi

    # Clear memoization cache
    _LINT_CONFIG_HASH_CACHE=()
    _LINT_COMBINED_CONFIG_HASH=""

    return 0
}

# ==============================================================================
# Function: lint_cache_stats
# Print statistics about the cache
# Returns:
#   0 on success
# ==============================================================================
lint_cache_stats() {
    if [[ ! -d "$LINT_CACHE_DIR" ]]; then
        echo "Cache directory does not exist: $LINT_CACHE_DIR"
        echo "Total cached files: 0"
        return 0
    fi

    local count
    count="$(find "$LINT_CACHE_DIR" -type f | wc -l)"

    echo "Cache directory: $LINT_CACHE_DIR"
    echo "Total cached files: $count"

    if [[ $count -gt 0 ]]; then
        echo "Cache size: $(du -sh "$LINT_CACHE_DIR" | cut -f1)"
    fi

    return 0
}

# ==============================================================================
# Function: lint_cache_list
# List all cached files
# Returns:
#   0 on success
# ==============================================================================
lint_cache_list() {
    if [[ ! -d "$LINT_CACHE_DIR" ]]; then
        echo "No cache directory found"
        return 0
    fi

    local cache_file
    local source_file

    for cache_file in "$LINT_CACHE_DIR"/*; do
        if [[ -f "$cache_file" ]]; then
            # Extract source file from cache file header
            source_file="$(grep "^# Lint cache for:" "$cache_file" | cut -d: -f2- | sed 's/^ //')"
            local result
            result="$(grep "^# Result:" "$cache_file" | cut -d: -f2- | sed 's/^ //')"
            local timestamp
            timestamp="$(grep "^# Timestamp:" "$cache_file" | cut -d: -f2- | sed 's/^ //')"
            echo "$source_file [$result] @ $timestamp"
        fi
    done

    return 0
}

# ==============================================================================
# Function: lint_cache_prune_removed
# Remove cache entries for files that no longer exist
# Returns:
#   0 on success
# ==============================================================================
lint_cache_prune_removed() {
    if [[ ! -d "$LINT_CACHE_DIR" ]]; then
        return 0
    fi

    local cache_file
    local source_file
    local pruned=0

    for cache_file in "$LINT_CACHE_DIR"/*; do
        if [[ -f "$cache_file" ]]; then
            source_file="$(grep "^# Lint cache for:" "$cache_file" | cut -d: -f2- | sed 's/^ //')"
            if [[ -n "$source_file" && ! -f "$source_file" ]]; then
                rm "$cache_file"
                ((pruned++)) || true
            fi
        fi
    done

    if [[ $pruned -gt 0 ]]; then
        echo "Pruned $pruned stale cache entries"
    fi

    return 0
}