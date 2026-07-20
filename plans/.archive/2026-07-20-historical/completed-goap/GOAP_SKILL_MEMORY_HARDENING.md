# GOAP Plan: Skill-Memory Production Hardening

**Current State:** Quick wins implemented (security validation, structured logging, error handling)  
**Target State:** Production-ready skill-memory with comprehensive error recovery, metrics, and monitoring  
**Estimated Duration:** 2-3 weeks

---

## Current World State

```yaml
skill_memory:
  version: "2.0.0"
  status: "security_hardened"
  
  completed:
    - input_validation: true        # ✓ Concept ID validation
    - path_traversal_protection: true  # ✓ Database path validation
    - structured_logging: true      # ✓ JSON logs with severity levels
    - audit_trail: true            # ✓ All operations logged
    - secure_permissions: true     # ✓ 700 permissions on database
    - error_capture: true          # ✓ CLI stderr captured
    - exit_codes: true            # ✓ Differentiated (0,1,2,3)
  
  pending:
    - retry_logic: false           # ✗ No retry on database locks
    - metrics_collection: false   # ✗ No performance metrics
    - log_rotation: false         # ✗ No automatic rotation
    - health_checks: false        # ✗ No integrity validation
    - encryption: false          # ✗ No metadata encryption
    - rate_limiting: false      # ✗ No abuse prevention
```

---

## Target State

```yaml
skill_memory:
  version: "3.0.0"
  status: "production_ready"
  
  capabilities:
    - resilient_operations: true    # Retry with exponential backoff
    - metrics_dashboard: true      # Prometheus-compatible metrics
    - automated_maintenance: true  # Log rotation, cleanup
    - integrity_validation: true  # Database health checks
    - optional_encryption: true   # Sensitive metadata encryption
    - abuse_prevention: true      # Rate limiting and quotas
```

---

## Action Plan

### Phase 1: Resilience (Week 1) - Priority: HIGH

#### Action 1.1: Implement Retry Logic
**Preconditions:** [error_capture_implemented]  
**Effects:** [retry_logic_implemented]  
**Cost:** 4  
**File:** `.opencode/lib/skill-memory.sh`  

```bash
# Add retry wrapper for CLI operations
_cli_with_retry() {
    local max_attempts=3
    local delay=1
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if output=$("$@" 2>&1); then
            echo "$output"
            return 0
        fi
        
        exit_code=$?
        if [[ $attempt -lt $max_attempts ]]; then
            _log_warn "Attempt $attempt failed, retrying in ${delay}s..."
            sleep $delay
            delay=$((delay * 2))  # Exponential backoff
        fi
        
        ((attempt++))
    done
    
    return $exit_code
}
```

**Success Criteria:**
- Database locks trigger retry (not immediate failure)
- Exponential backoff works (1s, 2s, 4s delays)
- All operations have retry capability

---

#### Action 1.2: Database Health Checks
**Preconditions:** [init_memory_db_exists]  
**Effects:** [health_checking_implemented]  
**Cost:** 3  
**File:** `.opencode/lib/skill-memory.sh`  

```bash
skill_memory_check() {
    # Check database integrity
    # Verify all concepts have valid metadata
    # Detect corruption
    # Report statistics
}
```

**Success Criteria:**
- `skill_memory_check()` function works
- Detects corrupted databases
- Reports concept/association counts
- Validates permissions

---

### Phase 2: Observability (Week 2) - Priority: MEDIUM

#### Action 2.1: Metrics Collection
**Preconditions:** [structured_logging_implemented]  
**Effects:** [metrics_collection_implemented]  
**Cost:** 5  
**File:** `.opencode/lib/skill-memory.sh`, new `.agents/memory/metrics/`  

Implement metrics for:
- Operation counts (remember/recall/associate per skill)
- Latency percentiles (p50, p95, p99)
- Error rates by type
- Database size growth
- Cache hit rates (if implemented)

**Output Format:**
```json
{
  "timestamp": "2026-02-20T10:30:00Z",
  "skill": "adr-creation",
  "operation": "remember",
  "latency_ms": 45,
  "success": true
}
```

**Success Criteria:**
- Metrics written to `.agents/memory/metrics/`
- Prometheus-compatible format available
- Can generate performance reports

---

#### Action 2.2: Log Rotation
**Preconditions:** [structured_logging_implemented]  
**Effects:** [log_rotation_implemented]  
**Cost:** 3  
**File:** `.opencode/lib/skill-memory.sh`  

```bash
# Rotate logs when they exceed size limit
_rotate_logs() {
    local max_size=10485760  # 10MB
    local max_files=5
    
    # Check current log size
    # Rotate if needed (move current to .1, .1 to .2, etc.)
    # Delete oldest if exceeding max_files
}
```

**Success Criteria:**
- Logs rotate at 10MB default
- Maximum 5 log files retained
- Rotation is atomic (no data loss)

---

### Phase 3: Advanced Features (Week 3) - Priority: LOW

#### Action 3.1: Optional Metadata Encryption
**Preconditions:** [security_hardening_complete]  
**Effects:** [encryption_available]  
**Cost:** 6  
**File:** `.opencode/lib/skill-memory.sh`  

Support encrypted metadata for sensitive data:

```bash
# If CSM_ENCRYPT_KEY is set, encrypt metadata
if [[ -n "${CSM_ENCRYPT_KEY:-}" ]]; then
    metadata=$(echo "$metadata" | openssl enc -aes-256-cbc -base64 -k "$CSM_ENCRYPT_KEY")
fi
```

**Success Criteria:**
- Encryption is opt-in (environment variable)
- Uses industry-standard encryption
- Can decrypt and read encrypted data
- No performance impact when disabled

---

#### Action 3.2: Rate Limiting
**Preconditions:** [metrics_collection_implemented]  
**Effects:** [rate_limiting_implemented]  
**Cost:** 4  
**File:** `.opencode/lib/skill-memory.sh`  

Prevent abuse:

```bash
# Limit operations per minute per skill
_check_rate_limit() {
    local skill="$1"
    local max_ops=60  # per minute
    
    # Track operations in last 60 seconds
    # Reject if limit exceeded
}
```

**Success Criteria:**
- Configurable limits (default: 60 ops/min/skill)
- Returns clear error when limited
- Metrics track rate limit hits

---

## Handoff Contracts

### Phase 1 → Phase 2
- Retry logic functions available
- Health check function available
- Error rates reduced to < 1%

### Phase 2 → Phase 3
- Metrics dashboard working
- Performance baselines established
- Log rotation automated

### Phase 3 → Production
- Encryption tested
- Rate limiting validated
- Security audit passed

---

## Success Criteria

### Phase 1 (Resilience)
- [ ] Retry logic works for all CLI operations
- [ ] Database locks handled gracefully (3 retries)
- [ ] Health check validates database integrity
- [ ] Can detect and report corrupted databases

### Phase 2 (Observability)
- [ ] Metrics collected for all operations
- [ ] Can generate latency percentiles
- [ ] Error rates tracked by type
- [ ] Logs rotate automatically at 10MB

### Phase 3 (Advanced)
- [ ] Encryption works with CSM_ENCRYPT_KEY
- [ ] Rate limiting prevents abuse
- [ ] All features documented
- [ ] Security review passed

---

## Resource Requirements

| Phase | Developer Hours | Testing Hours | Dependencies |
|-------|----------------|---------------|--------------|
| 1 (Resilience) | 16 | 8 | None |
| 2 (Observability) | 20 | 10 | Phase 1 |
| 3 (Advanced) | 16 | 12 | Phase 2 |
| **Total** | **52** | **30** | - |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Retry logic causes infinite loops | Low | High | Max attempts limit, exponential backoff |
| Metrics impact performance | Medium | Medium | Async metrics, sampling |
| Encryption key management | Medium | High | Document key rotation procedures |
| Rate limiting breaks legitimate use | Low | High | Configurable limits, bypass for admins |

---

## GOAP Actions Summary

```yaml
actions:
  - name: implement_retry_logic
    preconditions: [error_capture_implemented]
    effects: [retry_logic_implemented]
    cost: 4
    phase: 1
    
  - name: implement_health_checks
    preconditions: [init_memory_db_exists]
    effects: [health_checking_implemented]
    cost: 3
    phase: 1
    
  - name: implement_metrics
    preconditions: [structured_logging_implemented]
    effects: [metrics_collection_implemented]
    cost: 5
    phase: 2
    
  - name: implement_log_rotation
    preconditions: [structured_logging_implemented]
    effects: [log_rotation_implemented]
    cost: 3
    phase: 2
    
  - name: implement_encryption
    preconditions: [security_hardening_complete]
    effects: [encryption_available]
    cost: 6
    phase: 3
    
  - name: implement_rate_limiting
    preconditions: [metrics_collection_implemented]
    effects: [rate_limiting_implemented]
    cost: 4
    phase: 3
```

---

## Next Steps

1. **Immediate:** Begin Phase 1.1 (Retry Logic)
2. **This Week:** Complete Phase 1 (Resilience)
3. **Next Week:** Begin Phase 2 (Observability)
4. **Week 3:** Complete Phase 3 (Advanced Features)
5. **Week 4:** Production deployment and monitoring

---

## Progress Tracking

Update this section as actions are completed:

```yaml
progress:
  phase_1:
    retry_logic: "2026-02-27"  # Date completed
    health_checks: "pending"
  phase_2:
    metrics: "not_started"
    log_rotation: "not_started"
  phase_3:
    encryption: "not_started"
    rate_limiting: "not_started"
```
