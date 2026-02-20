#!/bin/bash
#
# skill-memory-log-rotation.sh - Log rotation for skill-memory
#
# Usage: source scripts/skill-memory/skill-memory.sh
#        source scripts/skill-memory/skill-memory-log-rotation.sh
#
# Provides automatic log rotation for skill-memory
#

# Log rotation settings
: "${CSM_LOG_ROTATION_ENABLED:=true}"
: "${CSM_LOG_MAX_SIZE:=10485760}"  # 10MB default
: "${CSM_LOG_MAX_FILES:=5}"

# ============================================================================
# LOG ROTATION
# ============================================================================

# Rotate log if it exceeds size limit
# Usage: _rotate_log_if_needed "log_file_path"
_rotate_log_if_needed() {
    local log_file="$1"
    
    [[ "$CSM_LOG_ROTATION_ENABLED" != "true" ]] && return 0
    [[ ! -f "$log_file" ]] && return 0
    
    local size
    size=$(stat -c%s "$log_file" 2>/dev/null || echo "0")
    
    if [[ $size -gt $CSM_LOG_MAX_SIZE ]]; then
        _rotate_log "$log_file"
    fi
}

# Rotate a log file
# Usage: _rotate_log "log_file_path"
_rotate_log() {
    local log_file="$1"
    local base_name="${log_file%.log}"
    local dir
    dir=$(dirname "$log_file")
    
    # Remove oldest if at max
    local oldest="${base_name}.$((CSM_LOG_MAX_FILES - 1)).log"
    [[ -f "$oldest" ]] && rm -f "$oldest"
    
    # Shift each log file
    for i in $(seq $((CSM_LOG_MAX_FILES - 2)) -1 0); do
        local current="${base_name}.${i}.log"
        local next="${base_name}.$((i + 1)).log"
        [[ -f "$current" ]] && mv "$current" "$next"
    done
    
    # Rotate current to .1
    mv "$log_file" "${base_name}.1.log"
    
    # Create new empty log file
    touch "$log_file"
}

# Manual log rotation trigger
# Usage: skill_log_rotate ["log_file"]
skill_log_rotate() {
    local log_file="${1:-/dev/stderr}"
    
    if [[ "$log_file" == "/dev/stderr" ]]; then
        echo "Log rotation: Cannot rotate stderr"
        return 1
    fi
    
    _rotate_log "$log_file"
    echo "Rotated: $log_file"
}

# Configure log rotation
# Usage: skill_log_rotation_config [--enable|--disable] [--max-size MB] [--max-files N]
skill_log_rotation_config() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --enable)
                CSM_LOG_ROTATION_ENABLED=true
                echo "Log rotation: enabled"
                shift
                ;;
            --disable)
                CSM_LOG_ROTATION_ENABLED=false
                echo "Log rotation: disabled"
                shift
                ;;
            --max-size)
                CSM_LOG_MAX_SIZE=$(( $2 * 1024 * 1024 ))
                echo "Log rotation: max size set to ${2}MB"
                shift 2
                ;;
            --max-files)
                CSM_LOG_MAX_FILES=$2
                echo "Log rotation: max files set to $2"
                shift 2
                ;;
            *)
                echo "Unknown option: $1"
                return 1
                ;;
        esac
    done
}

# Get log rotation status
skill_log_rotation_status() {
    echo "Log Rotation Status:"
    echo "  Enabled:    $CSM_LOG_ROTATION_ENABLED"
    echo "  Max Size:   $((CSM_LOG_MAX_SIZE / 1024 / 1024))MB"
    echo "  Max Files:  $CSM_LOG_MAX_FILES"
}

# ============================================================================
# Cleanup old metrics
# ============================================================================

# Clean up old metrics files
# Usage: skill_cleanup [--older-than-days N]
skill_cleanup() {
    local days=30
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --older-than-days)
                days=$2
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done
    
    local count=0
    
    # Clean old metrics
    if [[ -d "$CSM_METRICS_DIR" ]]; then
        count=$(find "$CSM_METRICS_DIR" -name "*.jsonl" -mtime +$days -type f 2>/dev/null | wc -l)
        find "$CSM_METRICS_DIR" -name "*.jsonl" -mtime +$days -type f -delete 2>/dev/null
    fi
    
    # Clean old rotated logs
    if [[ -d ".agents/memory" ]]; then
        local log_count
        log_count=$(find ".agents/memory" -name "*.log.*" -mtime +$days -type f 2>/dev/null | wc -l)
        find ".agents/memory" -name "*.log.*" -mtime +$days -type f -delete 2>/dev/null
        count=$((count + log_count))
    fi
    
    echo "Cleaned up $count old file(s) (older than $days days)"
}

# ============================================================================
# Export functions
# ============================================================================

if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
    export -f skill_log_rotate
    export -f skill_log_rotation_config
    export -f skill_log_rotation_status
    export -f skill_cleanup
    export CSM_LOG_ROTATION_ENABLED CSM_LOG_MAX_SIZE CSM_LOG_MAX_FILES CSM_METRICS_DIR
fi
