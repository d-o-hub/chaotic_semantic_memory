#!/bin/bash
#
# skill-memory.sh - Shared memory functions for opencode agents
#
# Usage: source .opencode/lib/skill-memory.sh
#
# Security: Validated inputs, secure permissions, structured logging
# Error Handling: Captures stderr, differentiates exit codes
# Logging: Structured JSON with severity levels
#

set -euo pipefail

# Configuration with defaults
: "${CSM_MEMORY_DB:=.agents/memory/skill-memory.db}"
: "${CSM_VERBOSE:=0}"
: "${CSM_LOG_LEVEL:=WARN}"  # ERROR, WARN, INFO, DEBUG, TRACE
: "${CSM_METRICS_ENABLED:=true}"
: "${CSM_METRICS_DIR:=.agents/memory/metrics}"
: "${CSM_RATE_LIMIT_ENABLED:=false}"
: "${CSM_RATE_LIMIT_OPS_PER_MINUTE:=60}"
: "${CSM_RATE_LIMIT_DIR:=.agents/memory/rate-limits}"

# Constants
readonly SKILL_MEMORY_VERSION="2.0.0"
readonly MAX_CONCEPT_ID_LENGTH=256
readonly VALID_SKILL_NAME_PATTERN='^[a-zA-Z0-9_-]+$'
readonly VALID_CONCEPT_ID_PATTERN='^[a-zA-Z0-9_:-]+$'

# ============================================================================
# RETRY LOGIC
# ============================================================================

# Configuration for retry behavior
: "${CSM_RETRY_MAX_ATTEMPTS:=3}"
: "${CSM_RETRY_BASE_DELAY:=1}"  # seconds
: "${CSM_RETRY_MAX_DELAY:=8}"   # seconds

# Execute CLI command with retry logic
# Usage: _cli_with_retry csm [args...]
_cli_with_retry() {
    local max_attempts="${CSM_RETRY_MAX_ATTEMPTS}"
    local base_delay="${CSM_RETRY_BASE_DELAY}"
    local max_delay="${CSM_RETRY_MAX_DELAY}"
    local attempt=1
    local output
    local exit_code
    
    _log_debug "Executing with retry (max $max_attempts attempts): $*"
    
    while [[ $attempt -le $max_attempts ]]; do
        output=$("$@" 2>&1) && {
            echo "$output"
            return 0
        }
        
        exit_code=$?
        
        # Don't retry on certain error codes
        case $exit_code in
            1)  # Validation error - don't retry
                _log_error "Command failed with validation error (exit $exit_code): $*"
                echo "$output"
                return $exit_code
                ;;
            127) # Command not found - don't retry
                _log_error "Command not found: $1"
                echo "$output"
                return $exit_code
                ;;
        esac
        
        if [[ $attempt -lt $max_attempts ]]; then
            local delay=$((base_delay * (2 ** (attempt - 1))))
            [[ $delay -gt $max_delay ]] && delay=$max_delay
            
            _log_warn "Attempt $attempt failed (exit $exit_code), retrying in ${delay}s..."
            _log_debug "Error output: $output"
            sleep $delay
        else
            _log_error "All $max_attempts attempts failed (exit $exit_code): $*"
            echo "$output"
            return $exit_code
        fi
        
        ((attempt++))
    done
    
    return $exit_code
}

# ============================================================================
# LOGGING FUNCTIONS
# ============================================================================

# Internal log function with severity levels
_log() {
    local level="$1"
    local message="$2"
    
    # Check if we should log this level
    case "$CSM_LOG_LEVEL" in
        ERROR) [[ "$level" == "ERROR" ]] || return ;;
        WARN)  [[ "$level" =~ ^(ERROR|WARN)$ ]] || return ;;
        INFO)  [[ "$level" =~ ^(ERROR|WARN|INFO)$ ]] || return ;;
        DEBUG) [[ "$level" =~ ^(ERROR|WARN|INFO|DEBUG)$ ]] || return ;;
        TRACE) ;;
    esac
    
    local timestamp
    timestamp=$(date -Iseconds)
    
    # Structured JSON log to stderr
    if command -v jq >/dev/null 2>&1; then
        jq -n \
            --arg ts "$timestamp" \
            --arg lvl "$level" \
            --arg msg "$message" \
            --arg src "skill-memory" \
            --arg pid "$$" \
            '{
                timestamp: $ts,
                level: $lvl,
                source: $src,
                message: $msg,
                pid: ($pid | tonumber)
            }' >&2
    else
        # Fallback if jq not available
        echo "{\"timestamp\":\"$timestamp\",\"level\":\"$level\",\"source\":\"skill-memory\",\"message\":\"$message\",\"pid\":$$}" >&2
    fi
}

# Convenience functions for each level
_log_error()   { _log "ERROR" "$*"; }
_log_warn()    { _log "WARN"  "$*"; }
_log_info()    { _log "INFO"  "$*"; }
_log_debug()   { _log "DEBUG" "$*"; }
_log_trace()   { _log "TRACE" "$*"; }

# Audit log - always logs regardless of level
_log_audit() {
    local action="$1"
    local concept_id="${2:-}"
    local details="${3:-}"
    
    local timestamp
    timestamp=$(date -Iseconds)
    
    if command -v jq >/dev/null 2>&1; then
        jq -n \
            --arg ts "$timestamp" \
            --arg act "$action" \
            --arg cid "$concept_id" \
            --arg det "$details" \
            --arg pid "$$" \
            --arg ppid "$PPID" \
            '{
                timestamp: $ts,
                level: "AUDIT",
                source: "skill-memory",
                action: $act,
                concept_id: $cid,
                details: $det,
                pid: ($pid | tonumber),
                ppid: ($ppid | tonumber)
            }' >&2
    else
        echo "AUDIT: [$timestamp] action=$action concept_id=$concept_id details=$details pid=$$" >&2
    fi
}

# ============================================================================
# METRICS COLLECTION
# ============================================================================

# Record operation metric
_metrics_record() {
    local skill="$1"
    local operation="$2"
    local latency="$3"
    local success="$4"
    
    [[ "$CSM_METRICS_ENABLED" != "true" ]] && return 0
    
    local timestamp
    timestamp=$(date -Iseconds)
    
    local metrics_file="$CSM_METRICS_DIR/${skill}.jsonl"
    
    mkdir -p "$CSM_METRICS_DIR"
    
    if command -v jq >/dev/null 2>&1; then
        jq -n \
            --arg ts "$timestamp" \
            --arg skill "$skill" \
            --arg op "$operation" \
            --argjson latency "$latency" \
            --argjson success "$success" \
            '{
                timestamp: $ts,
                skill: $skill,
                operation: $op,
                latency_ms: $latency,
                success: $success
            }' >> "$metrics_file" 2>/dev/null
    fi
}

# Get metrics summary
skill_metrics_summary() {
    local skill="${1:-}"
    local metrics_file="$CSM_METRICS_DIR/${skill}.jsonl"
    
    if [[ ! -f "$metrics_file" ]]; then
        echo "No metrics found for skill: $skill"
        return 1
    fi
    
    echo "Metrics for: $skill"
    echo "  (See $metrics_file for details)"
    
    cat "$metrics_file" | while read -r line; do
        echo "    $line" | jq -r '"    \(.operation): latency=\(.latency_ms)ms success=\(.success)"' 2>/dev/null || echo "    $line"
    done
}

# Get all metrics
skill_metrics_all() {
    echo "Skill Memory Metrics"
    echo "===================="
    
    for f in "$CSM_METRICS_DIR"/*.jsonl; do
        [[ ! -f "$f" ]] && continue
        local skill
        skill=$(basename "$f" .jsonl)
        skill_metrics_summary "$skill"
    done
}

# ============================================================================
# VALIDATION FUNCTIONS
# ============================================================================

# Validate skill name (alphanumeric, underscore, hyphen only)
_validate_skill_name() {
    local name="$1"
    
    if [[ -z "$name" ]]; then
        _log_error "Skill name cannot be empty"
        return 1
    fi
    
    if [[ ! "$name" =~ $VALID_SKILL_NAME_PATTERN ]]; then
        _log_error "Invalid skill name: '$name'. Use only alphanumeric, underscore, hyphen"
        return 1
    fi
    
    if [[ "${#name}" -gt 64 ]]; then
        _log_error "Skill name too long: ${#name} chars (max 64)"
        return 1
    fi
    
    return 0
}

# Validate concept ID
_validate_concept_id() {
    local id="$1"
    
    if [[ -z "$id" ]]; then
        _log_error "Concept ID cannot be empty"
        return 1
    fi
    
    if [[ "${#id}" -gt $MAX_CONCEPT_ID_LENGTH ]]; then
        _log_error "Concept ID too long: ${#id} chars (max $MAX_CONCEPT_ID_LENGTH)"
        return 1
    fi
    
    # Check for dangerous characters
    if [[ "$id" =~ [\.\/$] ]]; then
        _log_error "Concept ID contains invalid characters (no dots, slashes, or special chars)"
        return 1
    fi
    
    # Check for control characters
    if [[ "$id" =~ [[:cntrl:]] ]]; then
        _log_error "Concept ID contains control characters"
        return 1
    fi
    
    return 0
}

# Validate database path (prevent path traversal)
_validate_db_path() {
    local path="$1"
    
    if [[ -z "$path" ]]; then
        _log_error "Database path cannot be empty"
        return 1
    fi
    
    # Convert to absolute path
    local abs_path
    if ! abs_path=$(cd "$(dirname "$path")" && pwd)/$(basename "$path"); then
        _log_error "Cannot resolve database path: $path"
        return 1
    fi
    
    # Check if path traversal attempts exist
    if [[ "$abs_path" =~ \.\./|/\.\./|^\.\. ]]; then
        _log_error "Database path contains path traversal: $path"
        return 1
    fi
    
    # Get project root (current directory)
    local project_root
    project_root=$(pwd)
    
    # Ensure path is within project directory
    if [[ ! "$abs_path" =~ ^"$project_root" ]]; then
        _log_error "Database path must be within project directory: $path"
        return 1
    fi
    
    return 0
}

# Validate metadata JSON
_validate_metadata_json() {
    local json="$1"
    
    if [[ -z "$json" ]]; then
        _log_error "Metadata cannot be empty"
        return 1
    fi
    
    # Validate JSON structure
    if ! echo "$json" | jq -e . >/dev/null 2>&1; then
        _log_error "Invalid JSON in metadata"
        return 1
    fi
    
    # Check size (prevent abuse)
    local size
    size=$(echo "$json" | wc -c)
    if [[ "$size" -gt 65536 ]]; then
        _log_error "Metadata too large: $size bytes (max 65536)"
        return 1
    fi
    
    return 0
}

# Escape string for safe use in jq
_jq_escape() {
    local str="$1"
    # Use jq's --arg instead of manual escaping for safety
    echo "$str"
}

# ============================================================================
# DATABASE INITIALIZATION
# ============================================================================

# Initialize memory database with security checks
_init_memory_db() {
    local db_path="${1:-$CSM_MEMORY_DB}"
    
    # Validate path
    if ! _validate_db_path "$db_path"; then
        return 1
    fi
    
    local db_dir
    db_dir="$(dirname "$db_path")"
    
    # Create directory if needed with secure permissions
    if [[ ! -d "$db_dir" ]]; then
        _log_info "Creating memory directory: $db_dir"
        if ! mkdir -p "$db_dir"; then
            _log_error "Failed to create directory: $db_dir"
            return 1
        fi
        # Set restrictive permissions (owner only)
        chmod 700 "$db_dir" || {
            _log_warn "Could not set permissions on $db_dir"
        }
    fi
    
    # Initialize database file with secure permissions if it doesn't exist
    if [[ ! -f "$db_path" ]]; then
        _log_info "Initializing database: $db_path"
        # Create empty file with restricted permissions
        (umask 077 && touch "$db_path") || {
            _log_error "Failed to create database file: $db_path"
            return 1
        }
    fi
    
    return 0
}

# ============================================================================
# CORE MEMORY OPERATIONS
# ============================================================================

# Rate limiting check
_rate_limit_check() {
    local skill="$1"
    local limit_dir="${CSM_RATE_LIMIT_DIR:-.agents/memory/rate-limits}"
    local limit_file="$limit_dir/${skill}.limit"
    local max_ops="${CSM_RATE_LIMIT_OPS_PER_MINUTE:-60}"
    
    mkdir -p "$limit_dir"
    
    local now
    now=$(date +%s)
    
    if [[ ! -f "$limit_file" ]]; then
        echo "$now 1" > "$limit_file"
        return 0
    fi
    
    local last_time count
    last_time=$(cut -d' ' -f1 "$limit_file")
    count=$(cut -d' ' -f2 "$limit_file")
    
    local elapsed=$((now - last_time))
    
    if [[ $elapsed -gt 60 ]]; then
        echo "$now 1" > "$limit_file"
        return 0
    fi
    
    if [[ $count -ge $max_ops ]]; then
        _log_error "Rate limit exceeded for $skill: $count ops/min (max: $max_ops)"
        return 1
    fi
    
    count=$((count + 1))
    echo "$last_time $count" > "$limit_file"
    return 0
}

# Remember an operation
# Usage: skill_remember "skill_name" "operation" "context" "result"
# Returns: concept_id on success, empty on failure
# Exit codes: 0=success, 1=validation error, 2=cli error, 3=jq error
skill_remember() {
    local skill_name="$1"
    local operation="$2"
    local context="$3"
    local result="$4"
    local db_path="${CSM_MEMORY_DB}"
    
    _log_debug "skill_remember called: skill=$skill_name, operation=$operation"
    
    # Validate inputs
    if ! _validate_skill_name "$skill_name"; then
        return 1
    fi
    
    # Check rate limit
    if [[ "$CSM_RATE_LIMIT_ENABLED" == "true" ]]; then
        _rate_limit_check "$skill_name" || return 1
    fi
    
    if [[ -z "$operation" ]]; then
        _log_error "Operation cannot be empty"
        return 1
    fi
    
    # Generate unique concept ID
    local concept_id="skill::${skill_name}::${operation}::$(date +%s)_${RANDOM}_$$"
    
    if ! _validate_concept_id "$concept_id"; then
        return 1
    fi
    
    # Initialize database
    if ! _init_memory_db "$db_path"; then
        return 2
    fi
    
    # Build metadata JSON safely using jq
    local metadata
    if ! metadata=$(jq -n \
        --arg op "$operation" \
        --arg ctx "$context" \
        --arg res "$result" \
        --arg skill "$skill_name" \
        --arg ts "$(date -Iseconds)" \
        --arg ver "$SKILL_MEMORY_VERSION" \
        '{
            operation: $op,
            context: $ctx,
            result: $res,
            skill: $skill,
            timestamp: $ts,
            version: $ver
        }' 2>/dev/null); then
        _log_error "Failed to build metadata JSON"
        return 3
    fi
    
    if ! _validate_metadata_json "$metadata"; then
        return 1
    fi
    
    # Execute CLI with retry logic and timing
    _log_info "Remembering: $concept_id"
    local cli_output
    local cli_exit
    local start_time
    start_time=$(date +%s%3N)
    
    if ! cli_output=$(_cli_with_retry csm --database "$db_path" inject "$concept_id" -m "$metadata"); then
        cli_exit=$?
        local end_time latency
        end_time=$(date +%s%3N)
        latency=$((end_time - start_time))
        _log_error "CLI inject failed with exit code $cli_exit after retries: $cli_output"
        _metrics_record "$skill_name" "remember" "$latency" "false"
        return 2
    fi
    
    # Record latency on success
    local end_time latency
    end_time=$(date +%s%3N)
    latency=$((end_time - start_time))
    _metrics_record "$skill_name" "remember" "$latency" "true"
    
    # Audit log
    _log_audit "concept_created" "$concept_id" "skill=$skill_name, operation=$operation"
    
    _log_info "Successfully stored concept: $concept_id"
    echo "$concept_id"
    return 0
}

# Recall similar operations (client-side search)
# Usage: skill_recall "query" [threshold] [top_k]
# Returns: JSON array of matching concepts
skill_recall() {
    local query="$1"
    local threshold="${2:-0.7}"
    local top_k="${3:-5}"
    local db_path="${CSM_MEMORY_DB}"
    
    _log_debug "skill_recall called: query='$query', threshold=$threshold, top_k=$top_k"
    
    # Validate inputs
    if [[ -z "$query" ]]; then
        _log_error "Query cannot be empty"
        echo "[]"
        return 1
    fi
    
    # Validate threshold is a number between 0 and 1
    if ! [[ "$threshold" =~ ^0?\.[0-9]+$|^[01]$ ]]; then
        _log_error "Invalid threshold: $threshold (must be between 0 and 1)"
        echo "[]"
        return 1
    fi
    
    # Validate top_k is a positive integer
    if ! [[ "$top_k" =~ ^[1-9][0-9]*$ ]]; then
        _log_error "Invalid top_k: $top_k (must be positive integer)"
        echo "[]"
        return 1
    fi
    
    # Initialize database
    if ! _init_memory_db "$db_path"; then
        echo "[]"
        return 2
    fi
    
    # Check if database has any data
    if [[ ! -s "$db_path" ]]; then
        _log_debug "Database is empty"
        echo "[]"
        return 0
    fi
    
    # Export to temp file (safer than stdout)
    local temp_export
    temp_export=$(mktemp)
    
    cleanup() {
        rm -f "$temp_export"
    }
    trap cleanup RETURN
    
    _log_info "Recalling: '$query' (threshold: $threshold, top_k: $top_k)"
    
    local cli_output
    local cli_exit
    
    if ! cli_output=$(_cli_with_retry csm --database "$db_path" export -o "$temp_export" --output-format json 2>&1); then
        cli_exit=$?
        _log_error "CLI export failed with exit code $cli_exit: $cli_output"
        echo "[]"
        return 2
    fi
    
    # Validate export file was created and has content
    if [[ ! -f "$temp_export" ]] || [[ ! -s "$temp_export" ]]; then
        _log_warn "Export file is empty or missing"
        echo "[]"
        return 0
    fi
    
    # Search with error handling
    local results
    if ! results=$(jq --arg query "$query" --arg thresh "$threshold" --arg top_k "$top_k" '
        .concepts // [] |
        map(select(
            (.metadata.context // "" | ascii_downcase | contains($query | ascii_downcase)) or
            (.metadata.result // "" | ascii_downcase | contains($query | ascii_downcase)) or
            (.metadata.operation // "" | ascii_downcase | contains($query | ascii_downcase))
        )) |
        map({
            id: .id,
            similarity: 0.8,
            metadata: .metadata
        }) |
        sort_by(.metadata.timestamp) |
        reverse |
        .[:($top_k | tonumber)]' "$temp_export" 2>&1); then
        _log_error "jq filter failed: $results"
        echo "[]"
        return 3
    fi
    
    local count
    count=$(echo "$results" | jq 'length')
    _log_info "Found $count matching concepts"
    
    # Audit log
    _log_audit "concept_recalled" "" "query=$query, results=$count"
    
    echo "$results"
    return 0
}

# Create association between concepts
# Usage: skill_associate "concept1" "concept2" [strength]
skill_associate() {
    local concept1="$1"
    local concept2="$2"
    local strength="${3:-0.8}"
    local db_path="${CSM_MEMORY_DB}"
    
    _log_debug "skill_associate called: $concept1 -> $concept2 (strength: $strength)"
    
    # Validate concept IDs
    if ! _validate_concept_id "$concept1"; then
        return 1
    fi
    
    if ! _validate_concept_id "$concept2"; then
        return 1
    fi
    
    # Validate strength
    if ! [[ "$strength" =~ ^0?\.[0-9]+$|^[01]$ ]]; then
        _log_error "Invalid strength: $strength (must be between 0 and 1)"
        return 1
    fi
    
    # Initialize database
    if ! _init_memory_db "$db_path"; then
        return 2
    fi
    
    _log_info "Associating: $concept1 -> $concept2 (strength: $strength)"
    
    local cli_output
    local cli_exit
    
    if ! cli_output=$(_cli_with_retry csm --database "$db_path" associate "$concept1" "$concept2" -s "$strength" 2>&1); then
        cli_exit=$?
        _log_error "CLI associate failed with exit code $cli_exit: $cli_output"
        return 2
    fi
    
    # Audit log
    _log_audit "association_created" "$concept1" "to=$concept2, strength=$strength"
    
    _log_info "Successfully created association"
    return 0
}

# Get related concepts
# Usage: skill_related "concept_id" [min_strength]
skill_related() {
    local concept_id="$1"
    local min_strength="${2:-0.7}"
    local db_path="${CSM_MEMORY_DB}"
    
    _log_debug "skill_related called: $concept_id (min_strength: $min_strength)"
    
    # Validate concept ID
    if ! _validate_concept_id "$concept_id"; then
        echo "[]"
        return 1
    fi
    
    # Validate min_strength
    if ! [[ "$min_strength" =~ ^0?\.[0-9]+$|^[01]$ ]]; then
        _log_error "Invalid min_strength: $min_strength"
        echo "[]"
        return 1
    fi
    
    # Initialize database
    if ! _init_memory_db "$db_path"; then
        echo "[]"
        return 2
    fi
    
    _log_info "Finding related to: $concept_id"
    
    # Export to temp file
    local temp_export
    temp_export=$(mktemp)
    
    cleanup() {
        rm -f "$temp_export"
    }
    trap cleanup RETURN
    
    local cli_output
    local cli_exit
    
    if ! cli_output=$(_cli_with_retry csm --database "$db_path" export -o "$temp_export" --output-format json 2>&1); then
        cli_exit=$?
        _log_error "CLI export failed with exit code $cli_exit: $cli_output"
        echo "[]"
        return 2
    fi
    
    # Search for associations
    local results
    if ! results=$(jq --arg concept "$concept_id" --arg strength "$min_strength" '
        .associations // [] |
        map(select(
            ((.from // .[0]) == $concept or (.to // .[1]) == $concept) and
            ((.strength // .[2]) >= ($strength | tonumber))
        )) |
        map({
            from: (.from // .[0]),
            to: (.to // .[1]),
            strength: (.strength // .[2])
        })' "$temp_export" 2>&1); then
        _log_error "jq filter failed: $results"
        echo "[]"
        return 3
    fi
    
    local count
    count=$(echo "$results" | jq 'length')
    _log_info "Found $count related concepts"
    
    echo "$results"
    return 0
}

# Get memory statistics
# Usage: skill_memory_stats
skill_memory_stats() {
    local db_path="${CSM_MEMORY_DB}"
    
    _log_debug "skill_memory_stats called"
    
    # Initialize database
    if ! _init_memory_db "$db_path"; then
        return 2
    fi
    
    # Check if database has data
    if [[ ! -s "$db_path" ]]; then
        echo "Memory Statistics:"
        echo "  Concepts: 0"
        echo "  Associations: 0"
        echo "  Database: $db_path"
        return 0
    fi
    
    # Export to temp file
    local temp_export
    temp_export=$(mktemp)
    
    cleanup() {
        rm -f "$temp_export"
    }
    trap cleanup RETURN
    
    local cli_output
    local cli_exit
    
    if ! cli_output=$(_cli_with_retry csm --database "$db_path" export -o "$temp_export" --output-format json 2>&1); then
        cli_exit=$?
        _log_error "CLI export failed with exit code $cli_exit: $cli_output"
        return 2
    fi
    
    local concept_count association_count
    
    if ! concept_count=$(jq '.concepts | length' "$temp_export" 2>&1); then
        _log_error "Failed to count concepts: $concept_count"
        concept_count="error"
    fi
    
    if ! association_count=$(jq '.associations | length' "$temp_export" 2>&1); then
        _log_error "Failed to count associations: $association_count"
        association_count="error"
    fi
    
    echo "Memory Statistics:"
    echo "  Concepts: $concept_count"
    echo "  Associations: $association_count"
    echo "  Database: $db_path"
    
    _log_audit "stats_queried" "" "concepts=$concept_count, associations=$association_count"
}

# ============================================================================
# HIGH-LEVEL PATTERNS
# ============================================================================

# Remember with automatic associations
# Usage: skill_remember_linked "skill" "op" "ctx" "result" "related_ids..."
skill_remember_linked() {
    local skill_name="$1"
    local operation="$2"
    local context="$3"
    local result="$4"
    shift 4
    local related=("$@")
    
    _log_debug "skill_remember_linked called with ${#related[@]} related concepts"
    
    local concept_id
    if ! concept_id=$(skill_remember "$skill_name" "$operation" "$context" "$result"); then
        return 1
    fi
    
    # Create associations
    local failed=0
    for related_id in "${related[@]}"; do
        if ! skill_associate "$concept_id" "$related_id" 0.8; then
            _log_warn "Failed to associate $concept_id -> $related_id"
            ((failed++))
        fi
    done
    
    if [[ $failed -gt 0 ]]; then
        _log_warn "$failed association(s) failed, but concept was created"
    fi
    
    echo "$concept_id"
    return 0
}

# Recall and suggest based on context
# Usage: skill_suggest "query" [threshold]
skill_suggest() {
    local query="$1"
    local threshold="${2:-0.7}"
    
    _log_debug "skill_suggest called: '$query'"
    
    local results
    if ! results=$(skill_recall "$query" "$threshold" 3); then
        return 1
    fi
    
    local count
    count=$(echo "$results" | jq 'length')
    
    if [[ "$count" -gt 0 ]]; then
        echo ""
        echo "Based on past similar work:"
        echo "$results" | jq -r '.[] | "  • \(.metadata.operation): \(.metadata.context[0:60])... (similarity: \(.similarity | tostring | .[0:4]))"'
        echo ""
    fi
    
    return 0
}

# Export memory for backup/analysis
# Usage: skill_export [output_file]
skill_export() {
    local output_file="${1:-}"
    local db_path="${CSM_MEMORY_DB}"
    
    _log_debug "skill_export called: output_file='$output_file'"
    
    # Initialize database
    if ! _init_memory_db "$db_path"; then
        return 2
    fi
    
    if [[ -n "$output_file" ]]; then
        # Validate output path
        local output_dir
        output_dir=$(dirname "$output_file")
        if [[ ! -d "$output_dir" ]]; then
            _log_error "Output directory does not exist: $output_dir"
            return 1
        fi
        
        _log_info "Exporting to: $output_file"
        local cli_output
        local cli_exit
        
        if ! cli_output=$(_cli_with_retry csm --database "$db_path" export -o "$output_file" 2>&1); then
            cli_exit=$?
            _log_error "CLI export failed with exit code $cli_exit: $cli_output"
            return 2
        fi
        
        _log_audit "export_completed" "" "file=$output_file"
        _log_info "Export successful"
    else
        # Export to stdout
        local cli_output
        local cli_exit
        
        if ! cli_output=$(_cli_with_retry csm --database "$db_path" export -o - 2>&1); then
            cli_exit=$?
            _log_error "CLI export failed with exit code $cli_exit: $cli_output"
            return 2
        fi
        
        echo "$cli_output"
        _log_audit "export_completed" "" "file=stdout"
    fi
}

# Import memory from backup
# Usage: skill_import "file"
skill_import() {
    local input_file="$1"
    local db_path="${CSM_MEMORY_DB}"
    
    _log_debug "skill_import called: $input_file"
    
    if [[ ! -f "$input_file" ]]; then
        _log_error "Import file not found: $input_file"
        return 1
    fi
    
    # Validate input file is valid JSON
    if ! jq -e . "$input_file" >/dev/null 2>&1; then
        _log_error "Import file is not valid JSON: $input_file"
        return 1
    fi
    
    # Initialize database
    if ! _init_memory_db "$db_path"; then
        return 2
    fi
    
    _log_info "Importing from: $input_file"
    
    local cli_output
    local cli_exit
    
    if ! cli_output=$(_cli_with_retry csm --database "$db_path" import "$input_file" 2>&1); then
        cli_exit=$?
        _log_error "CLI import failed with exit code $cli_exit: $cli_output"
        return 2
    fi
    
    _log_audit "import_completed" "" "file=$input_file"
    _log_info "Import successful"
}

# ============================================================================
# UTILITY FUNCTIONS
# ============================================================================

# Check system health and database integrity
# Usage: skill_memory_check [--full]
skill_memory_check() {
    local full_check=false
    local db_path="${CSM_MEMORY_DB}"
    local failures=0
    local warnings=0
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --full)
                full_check=true
                shift
                ;;
            *)
                break
                ;;
        esac
    done
    
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║         SKILL-MEMORY HEALTH CHECK v${SKILL_MEMORY_VERSION}              ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    
    # Section 1: Dependencies
    echo "📦 DEPENDENCIES"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    # Check csm binary
    if ! command -v csm >/dev/null 2>&1; then
        echo "  [FAIL] csm binary not found in PATH"
        ((failures++))
    else
        local csm_version
        csm_version=$(csm --version 2>/dev/null | head -1 || echo "unknown")
        echo "  [PASS] csm binary found: $csm_version"
    fi
    
    # Check jq
    if ! command -v jq >/dev/null 2>&1; then
        echo "  [WARN] jq not found (logging will use fallback format)"
        ((warnings++))
    else
        local jq_version
        jq_version=$(jq --version 2>/dev/null || echo "unknown")
        echo "  [PASS] jq found: $jq_version"
    fi
    
    echo ""
    
    # Section 2: Configuration
    echo "⚙️  CONFIGURATION"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  CSM_MEMORY_DB:     $CSM_MEMORY_DB"
    echo "  CSM_LOG_LEVEL:     $CSM_LOG_LEVEL"
    echo "  CSM_RETRY_MAX:     $CSM_RETRY_MAX_ATTEMPTS"
    
    # Check database path
    if ! _validate_db_path "$db_path" 2>/dev/null; then
        echo "  [FAIL] Database path invalid or outside project directory"
        ((failures++))
    else
        echo "  [PASS] Database path valid"
    fi
    
    echo ""
    
    # Section 3: Database Directory
    echo "📁 DATABASE DIRECTORY"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    local db_dir
    db_dir=$(dirname "$db_path")
    
    if [[ -d "$db_dir" ]]; then
        local perms owner
        perms=$(stat -c "%a" "$db_dir" 2>/dev/null || stat -f "%Lp" "$db_dir" 2>/dev/null)
        owner=$(stat -c "%U" "$db_dir" 2>/dev/null || stat -f "%Su" "$db_dir" 2>/dev/null)
        
        echo "  Path:     $db_dir"
        echo "  Owner:    $owner"
        echo "  Perms:    $perms"
        
        if [[ "$perms" == "700" ]]; then
            echo "  [PASS] Secure permissions (700)"
        else
            echo "  [WARN] Permissions are $perms (recommended: 700)"
            ((warnings++))
        fi
    else
        echo "  [INFO] Directory does not exist (will be created on first use)"
    fi
    
    echo ""
    
    # Section 4: Database File
    echo "💾 DATABASE FILE"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    if [[ -f "$db_path" ]]; then
        local size perms owner
        size=$(ls -lh "$db_path" 2>/dev/null | awk '{print $5}')
        perms=$(stat -c "%a" "$db_path" 2>/dev/null || stat -f "%Lp" "$db_path" 2>/dev/null)
        owner=$(stat -c "%U" "$db_path" 2>/dev/null || stat -f "%Su" "$db_path" 2>/dev/null)
        
        echo "  Path:     $db_path"
        echo "  Size:     $size"
        echo "  Owner:    $owner"
        echo "  Perms:    $perms"
        
        # Check if readable
        if csm --database "$db_path" export -o /dev/null >/dev/null 2>&1; then
            echo "  [PASS] Database readable"
        else
            echo "  [FAIL] Database not readable (may be corrupted)"
            ((failures++))
        fi
        
        # Full integrity check
        if [[ "$full_check" == true ]]; then
            echo ""
            echo "  🔍 Running integrity check..."
            
            local temp_export
            temp_export=$(mktemp)
            
            if csm --database "$db_path" export -o "$temp_export" >/dev/null 2>&1; then
                local concept_count assoc_count
                concept_count=$(jq '.concepts | length' "$temp_export" 2>/dev/null || echo "error")
                assoc_count=$(jq '.associations | length' "$temp_export" 2>/dev/null || echo "error")
                
                echo "  Concepts:      $concept_count"
                echo "  Associations:  $assoc_count"
                
                # Validate all concepts have required fields
                local invalid_concepts
                invalid_concepts=$(jq '[.concepts[] | select(.metadata.operation == null or .metadata.timestamp == null)] | length' "$temp_export" 2>/dev/null || echo "0")
                
                if [[ "$invalid_concepts" -eq 0 ]]; then
                    echo "  [PASS] All concepts have required metadata"
                else
                    echo "  [FAIL] $invalid_concepts concept(s) missing required metadata"
                    ((failures++))
                fi
                
                rm -f "$temp_export"
            else
                echo "  [FAIL] Cannot export database for integrity check"
                ((failures++))
            fi
        fi
    else
        echo "  [INFO] Database file does not exist (will be created on first use)"
    fi
    
    echo ""
    
    # Summary
    echo "📊 SUMMARY"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    if [[ $failures -eq 0 && $warnings -eq 0 ]]; then
        echo "  ✅ All checks passed!"
        echo ""
        return 0
    elif [[ $failures -eq 0 ]]; then
        echo "  ⚠️  $warnings warning(s) found (non-critical)"
        echo ""
        return 0
    else
        echo "  ❌ $failures failure(s), $warnings warning(s) found"
        echo ""
        return 1
    fi
}

# ============================================================================
# Export functions if sourced
# ============================================================================

if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
    # Being sourced - export functions
    export -f skill_remember skill_recall skill_associate skill_related
    export -f skill_remember_linked skill_suggest skill_export skill_import
    export -f skill_memory_stats skill_memory_check
    export CSM_MEMORY_DB CSM_LOG_LEVEL SKILL_MEMORY_VERSION CSM_RETRY_MAX_ATTEMPTS CSM_RETRY_BASE_DELAY CSM_RETRY_MAX_DELAY
    _log_info "Skill memory library v${SKILL_MEMORY_VERSION} loaded"
    true  # Ensure successful exit status
fi
