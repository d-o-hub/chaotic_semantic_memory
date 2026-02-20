#!/bin/bash
#
# skill-memory-metrics.sh - Metrics collection for skill-memory
#
# Usage: source scripts/skill-memory/skill-memory.sh
#        source scripts/skill-memory/skill-memory-metrics.sh
#
# Collects and exports metrics for monitoring skill-memory usage
#

# Metrics directory
: "${CSM_METRICS_DIR:=.agents/memory/metrics}"

# Metrics enabled?
: "${CSM_METRICS_ENABLED:=true}"

# ============================================================================
# METRICS COLLECTION
# ============================================================================

# Record an operation metric
# Usage: _metrics_record "skill" "operation" latency_ms success
_metrics_record() {
    local skill="$1"
    local operation="$2"
    local latency="$3"
    local success="$4"
    
    [[ "$CSM_METRICS_ENABLED" != "true" ]] && return 0
    
    local timestamp
    timestamp=$(date -Iseconds)
    
    local metrics_file="$CSM_METRICS_DIR/${skill}.jsonl"
    
    # Ensure directory exists
    mkdir -p "$CSM_METRICS_DIR"
    
    # Append metric
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
            }' >> "$metrics_file"
    else
        echo "{\"timestamp\":\"$timestamp\",\"skill\":\"$skill\",\"operation\":\"$operation\",\"latency_ms\":$latency,\"success\":$success}" >> "$metrics_file"
    fi
}

# Get metrics summary for a skill
# Usage: skill_metrics_summary "skill_name"
skill_metrics_summary() {
    local skill="${1:-}"
    local metrics_file="$CSM_METRICS_DIR/${skill}.jsonl"
    
    if [[ ! -f "$metrics_file" ]]; then
        echo "No metrics found for skill: $skill"
        return 1
    fi
    
    local total_ops
    total_ops=$(wc -l < "$metrics_file")
    
    local successful
    successful=$(grep -c '"success":true' "$metrics_file" || echo "0")
    
    local failed=$((total_ops - successful))
    
    # Calculate latency stats
    local avg_latency
    avg_latency=$(awk -F'latency_ms":' '{sum+=$2; n++} END {if(n>0) printf "%.2f", sum/n; else print "0"}' "$metrics_file")
    
    echo "Metrics for: $skill"
    echo "  Total operations: $total_ops"
    echo "  Successful: $successful"
    echo "  Failed: $failed"
    echo "  Average latency: ${avg_latency}ms"
}

# Get all metrics
# Usage: skill_metrics_all
skill_metrics_all() {
    mkdir -p "$CSM_METRICS_DIR"
    
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║         SKILL-MEMORY METRICS SUMMARY                    ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    
    local total_ops=0
    local total_successful=0
    
    for metrics_file in "$CSM_METRICS_DIR"/*.jsonl; do
        [[ ! -f "$metrics_file" ]] && continue
        
        local skill
        skill=$(basename "$metrics_file" .jsonl)
        
        local ops
        ops=$(wc -l < "$metrics_file")
        total_ops=$((total_ops + ops))
        
        local success
        success=$(grep -c '"success":true' "$metrics_file" 2>/dev/null || echo "0")
        total_successful=$((total_successful + success))
        
        local avg_latency
        avg_latency=$(awk -F'latency_ms":' '{sum+=$2; n++} END {if(n>0) printf "%.0f", sum/n; else print "0"}' "$metrics_file")
        
        local failed=$((ops - success))
        local success_rate
        if [[ $ops -gt 0 ]]; then
            success_rate=$((success * 100 / ops))
        else
            success_rate=0
        fi
        
        echo "  $skill:"
        echo "    Operations:   $ops"
        echo "    Success:     $success ($success_rate%)"
        echo "    Failed:      $failed"
        echo "    Avg latency: ${avg_latency}ms"
        echo ""
    done
    
    echo "  ─────────────────────────────────"
    echo "  TOTAL:"
    echo "    Operations:   $total_ops"
    echo "    Successful:   $total_successful"
    if [[ $total_ops -gt 0 ]]; then
        echo "    Success rate: $((total_successful * 100 / total_ops))%"
    fi
}

# Export metrics in Prometheus format
# Usage: skill_metrics_prometheus
skill_metrics_prometheus() {
    mkdir -p "$CSM_METRICS_DIR"
    
    for metrics_file in "$CSM_METRICS_DIR"/*.jsonl; do
        [[ ! -f "$metrics_file" ]] && continue
        
        local skill
        skill=$(basename "$metrics_file" .jsonl)
        
        local ops
        ops=$(wc -l < "$metrics_file")
        
        local successful
        successful=$(grep -c '"success":true' "$metrics_file" 2>/dev/null || echo "0")
        
        echo "# TYPE skill_memory_${skill}_operations_total counter"
        echo "skill_memory_${skill}_operations_total $ops"
        
        echo "# TYPE skill_memory_${skill}_operations_success_total counter"
        echo "skill_memory_${skill}_operations_success_total $successful"
    done
}

# Clear metrics
# Usage: skill_metrics_clear ["skill_name"]
skill_metrics_clear() {
    local skill="$1"
    
    if [[ -n "$skill" ]]; then
        rm -f "$CSM_METRICS_DIR/${skill}.jsonl"
        echo "Cleared metrics for: $skill"
    else
        rm -f "$CSM_METRICS_DIR"/*.jsonl
        echo "Cleared all metrics"
    fi
}

# ============================================================================
# Export functions
# ============================================================================

if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
    export -f _metrics_record
    export -f skill_metrics_summary
    export -f skill_metrics_all
    export -f skill_metrics_prometheus
    export -f skill_metrics_clear
    export CSM_METRICS_DIR CSM_METRICS_ENABLED
fi
