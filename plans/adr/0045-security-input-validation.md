# ADR-0045: Security Policy for Input Validation

## Status

Proposed

## Context

The specialist security analysis identified critical vulnerabilities in input validation:

1. **Bincode Deserialization Without Size Limits** (CRITICAL, CVSS 9.1)
   - `import_from_bytes` can consume arbitrary memory
   - No protection against nested structure attacks
   
2. **Path Traversal in File Operations** (HIGH, CVSS 7.5)
   - `export_json`, `import_json` accept arbitrary paths
   - Can write to sensitive system locations
   
3. **Metadata Size Unvalidated** (HIGH)
   - No limits on metadata JSON size during concept building
   - Can cause memory exhaustion

4. **Token Exposure in Error Messages** (MEDIUM)
   - Database connection errors may expose Turso tokens

5. **Unbounded Association Strength** (MEDIUM)
   - No validation on association strength parameter

## Decision

Implement comprehensive input validation policy with size limits, path validation, and sanitization.

### 1. Bincode Size Limits

```rust
// src/wasm.rs and src/framework_ops.rs
pub const MAX_IMPORT_SIZE_BYTES: usize = 100 * 1024 * 1024; // 100MB
pub const MAX_IMPORT_DEPTH: usize = 100; // For nested structures

pub async fn import_from_bytes(&self, data: Uint8Array, merge: bool) -> Result<usize, JsValue> {
    let bytes = data.to_vec();
    
    // Size validation
    if bytes.len() > MAX_IMPORT_SIZE_BYTES {
        return Err(JsValue::from_str(&format!(
            "Import data exceeds maximum size of {} bytes (got {})",
            MAX_IMPORT_SIZE_BYTES, bytes.len()
        )));
    }
    
    // Optional: depth validation via custom deserializer
    let payload: ExportPayload = bincode::deserialize(&bytes)
        .map_err(|e| JsValue::from_str(&format!("Deserialization failed: {}", e)))?;
    
    // ...
}
```

### 2. Path Traversal Protection

```rust
// src/framework_ops.rs
use std::path::{Path, PathBuf};

pub const MAX_PATH_LENGTH: usize = 4096;

fn validate_export_path(path: &str) -> Result<PathBuf> {
    // Length check
    if path.len() > MAX_PATH_LENGTH {
        return Err(MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: format!("path exceeds maximum length of {}", MAX_PATH_LENGTH),
        });
    }
    
    let path = Path::new(path);
    
    // Reject absolute paths
    if path.is_absolute() {
        return Err(MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: "absolute paths not allowed".to_string(),
        });
    }
    
    // Normalize and check for traversal
    let normalized = path.absolutize().map_err(|e| MemoryError::Io(e))?;
    let current_dir = std::env::current_dir()?;
    
    if !normalized.starts_with(&current_dir) {
        return Err(MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: "path traversal detected".to_string(),
        });
    }
    
    // Check for suspicious components
    let path_str = path.to_string_lossy();
    if path_str.contains("..") || path_str.contains("//") || path_str.starts_with('/') {
        return Err(MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: "invalid path components".to_string(),
        });
    }
    
    Ok(normalized)
}

pub async fn export_json(&self, path: &str) -> Result<()> {
    let validated_path = validate_export_path(path)?;
    // ... proceed with validated path
}
```

### 3. Metadata Size Validation

See ADR-0044 for metadata size limits (64KB default).

### 4. Error Message Sanitization

```rust
// src/error.rs - helper to sanitize error messages
fn sanitize_error_message(msg: &str) -> String {
    // Remove potential tokens/secrets
    let patterns = [
        (r"token=[a-zA-Z0-9_-]+", "token=***"),
        (r"password=[^\s]+", "password=***"),
        (r"key=[a-zA-Z0-9_-]{20,}", "key=***"),
    ];
    
    let mut result = msg.to_string();
    for (pattern, replacement) in &patterns {
        result = regex::Regex::new(pattern)
            .unwrap()
            .replace_all(&result, *replacement)
            .to_string();
    }
    result
}

// Usage in error conversion
#[error("Database error: {message}")]
Database { message: String },

impl From<libsql::Error> for MemoryError {
    fn from(err: libsql::Error) -> Self {
        let sanitized = sanitize_error_message(&err.to_string());
        MemoryError::Database { message: sanitized }
    }
}
```

### 5. Association Strength Validation

```rust
// src/singularity.rs or src/framework.rs
pub const MAX_ASSOCIATION_STRENGTH: f32 = 1.0;
pub const MIN_ASSOCIATION_STRENGTH: f32 = -1.0;

pub async fn associate(&self, from: &str, to: &str, strength: f32) -> Result<()> {
    if !strength.is_finite() {
        return Err(MemoryError::InvalidInput {
            field: "strength".to_string(),
            reason: "strength must be a finite number".to_string(),
        });
    }
    
    if strength < MIN_ASSOCIATION_STRENGTH || strength > MAX_ASSOCIATION_STRENGTH {
        return Err(MemoryError::InvalidInput {
            field: "strength".to_string(),
            reason: format!(
                "strength must be between {} and {}, got {}",
                MIN_ASSOCIATION_STRENGTH, MAX_ASSOCIATION_STRENGTH, strength
            ),
        });
    }
    
    // ... proceed
}
```

### 6. Security Constants

```rust
// src/security.rs or src/lib.rs
pub mod security {
    pub const MAX_IMPORT_SIZE_BYTES: usize = 100 * 1024 * 1024;
    pub const MAX_METADATA_SIZE_BYTES: usize = 64 * 1024;
    pub const MAX_PATH_LENGTH: usize = 4096;
    pub const MAX_CONCEPT_ID_LENGTH: usize = 1024;
    pub const MAX_ASSOCIATION_STRENGTH: f32 = 1.0;
    pub const MIN_ASSOCIATION_STRENGTH: f32 = -1.0;
}
```

## Consequences

### Positive

- Protection against DoS attacks via oversized inputs
- Prevention of path traversal attacks
- No sensitive data leakage in errors
- Clear validation boundaries
- Security-focused property tests possible

### Negative

- Breaking change for users with legitimate large imports (>100MB)
- Additional validation overhead on all inputs
- Path validation may reject valid relative paths in edge cases
- Regex dependency for sanitization (or implement manually)

### Security Testing

```rust
#[test]
fn test_import_size_limit_enforced() {
    let oversized = vec![0u8; 101 * 1024 * 1024];
    let result = framework.import_binary_from_bytes(&oversized);
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[test]
fn test_path_traversal_blocked() {
    let malicious_paths = [
        "../../../etc/passwd",
        "/etc/passwd",
        "./../secret",
        "file/../../etc/hosts",
    ];
    
    for path in &malicious_paths {
        assert!(validate_export_path(path).is_err());
    }
}
```

## Compliance Checklist

- [x] Size limits on all deserialization
- [x] Path traversal protection
- [x] Metadata size validation
- [x] Error message sanitization
- [x] Input bounds checking
- [ ] Rate limiting (deferred to post-1.0)
- [ ] Audit logging (deferred to post-1.0)

## References

- Security Analysis: `plans/handoffs/analysis_security.md`
- Master Coordination: `plans/handoffs/MASTER_ANALYSIS_COORDINATION.md`
- Memory Limits: `plans/adr/0044-memory-limits-governance.md`
- GOAP State: `plans/GOAP_STATE.md` Phase 27
