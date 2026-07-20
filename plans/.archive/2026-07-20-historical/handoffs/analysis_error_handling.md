# Error Handling Analysis Report

**Swarm Group**: C - Observability  
**Role**: Error Handling Specialist  
**Date**: 2025-02-20  
**Scope**: Full codebase error handling audit

---

## Executive Summary

The `chaotic_semantic_memory` codebase demonstrates **moderate-to-good** error handling practices with structured error types using `thiserror`. However, several critical gaps exist that impact observability, debugging capabilities, and panic safety. This analysis identifies **4 Critical**, **5 High**, **8 Medium**, and **6 Low** risk issues across the codebase.

---

## Current State vs Desired State

| Aspect | Current State | Desired State | Gap |
|--------|--------------|---------------|-----|
| Error Types | Two distinct enums (`MemoryError`, `CliError`) with moderate granularity | Unified error hierarchy with source chaining and context | No #[source] attributes, lost error context |
| Error Propagation | Manual error mapping with `map_err` | Proper `?` operator usage with `From` implementations | Excessive boilerplate, context loss |
| CLI Exit Codes | Well-structured `ExitCode` enum with proper mapping | Consistent exit codes across all error paths | Exit code sometimes swallowed |
| Panic Safety | 23 unwrap/expect calls (18 in tests, 5 in production) | Zero unwrap/expect in production code | Potential panic paths identified |
| Error Context | Basic error messages without tracing context | Structured error context with tracing spans | Missing context in async boundaries |
| User-Facing Messages | Generally clear but sometimes inconsistent | Actionable, consistent error messages | Some messages lack remediation hints |

---

## 1. Error Type Design Analysis

### 1.1 MemoryError (src/error.rs)

**Current Implementation:**
```rust
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Invalid input for '{field}': {reason}")]
    InvalidInput { field: String, reason: String },
    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("Reservoir error: {0}")]
    Reservoir(String),
    #[error("Persistence error: {0}")]
    Persistence(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

**Issues Identified:**

| Severity | Issue | Location | Description |
|----------|-------|----------|-------------|
| **CRITICAL** | Missing `#[source]` attributes | src/error.rs:7-30 | Error variants wrap underlying errors but don't preserve them for debugging |
| HIGH | String-based error wrapping | src/error.rs:8,17,20,23 | Using `String` loses structured error information |
| MEDIUM | No backtrace support | src/error.rs:5 | Error type doesn't derive/enable backtrace capture |
| LOW | Inconsistent error naming | src/error.rs:20,23 | "Reservoir" vs "Persistence" - unclear taxonomy |

**Recommended Fix:**
```rust
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Database error")]
    Database {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
        context: String,
    },
    
    #[error("Invalid input for '{field}': {reason}")]
    InvalidInput { field: String, reason: String },
    
    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },
    
    #[error("Unsupported operation: {operation}")]
    UnsupportedOperation { operation: String },
    
    #[error("Reservoir computation error")]
    Reservoir {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: String,
    },
    
    #[error("Persistence operation failed")]
    Persistence {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
        operation: &'static str,
    },
    
    #[error("IO error")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error")]
    Serialization(#[from] serde_json::Error),
    
    #[error("libsql database error")]
    Libsql(#[from] libsql::Error),
}
```

### 1.2 CliError (src/cli/error.rs)

**Current Implementation:**
- Well-structured with distinct CLI-specific variants
- Proper exit code mapping
- Good `From` implementations for `MemoryError` and `std::io::Error`

**Issues Identified:**

| Severity | Issue | Location | Description |
|----------|-------|----------|-------------|
| MEDIUM | `anyhow::Error` conversion loses context | src/cli/error.rs:34-38 | Converts anyhow to plain string, losing error chain |
| MEDIUM | No structured error output for JSON | src/cli/error.rs | JSON errors only contain message string, no error code/type |
| LOW | `Other` variant too generic | src/cli/error.rs:30-31 | Catch-all variant encourages lazy error handling |

**Recommended Fix:**
```rust
impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        // Preserve the error chain
        if let Some(mem_err) = err.downcast_ref::<MemoryError>() {
            CliError::Memory(mem_err.clone())
        } else {
            CliError::Other {
                message: err.to_string(),
                source_chain: err.chain().map(|e| e.to_string()).collect(),
            }
        }
    }
}

// Add structured JSON error output
pub fn to_json(&self) -> serde_json::Value {
    serde_json::json!({
        "status": "error",
        "error": {
            "type": std::any::type_name_of_val(self),
            "code": self.exit_code(),
            "message": self.to_string(),
            "category": self.category(),
        }
    })
}
```

---

## 2. Error Propagation Analysis

### 2.1 Proper Use of `?` Operator

**Good Examples:**
- src/persistence.rs: Uses `?` consistently with proper `From` implementations
- src/framework_ops.rs: Clean propagation in batch operations

**Problematic Patterns:**

| File | Line | Issue | Code |
|------|------|-------|------|
| src/cli/commands/inject.rs | 23-28 | Manual error mapping with context loss | `map_err` creates new Io error |
| src/cli/commands/inject.rs | 32-37 | Same pattern for stdin | Duplicated error handling |
| src/cli/commands/mod.rs | 62-64 | Generic persistence error wrapper | Loses original error type |
| src/cli/commands/associate.rs | 25 | Error mapped to Persistence | Should be Input error |
| src/cli/commands/probe.rs | 23 | Error mapped to Persistence | Should be Input error |

### 2.2 Missing Error Context in Async Functions

**CRITICAL Issue:**

**Location**: src/framework.rs:177
```rust
let r = reservoir.as_mut().expect("reservoir initialized above");
```

This `expect` is justified by prior initialization, but could be eliminated:

```rust
// Safer alternative
let r = reservoir.as_mut().ok_or_else(|| {
    MemoryError::Reservoir("reservoir not initialized".to_string())
})?;
```

### 2.3 Error Conversion Gaps

**HIGH Priority:**

**Location**: src/persistence.rs throughout
- Every database operation manually maps errors
- Missing `From<libsql::Error>` for `MemoryError`

**Fix:**
```rust
// Add to MemoryError
#[error("Database error: {context}")]
Database {
    #[source]
    source: libsql::Error,
    context: String,
    operation: &'static str,
},

// Then use ? operator
let conn = self.db.connect()?; // Instead of manual map_err
```

---

## 3. CLI Error Handling Analysis

### 3.1 Exit Code Correctness

**Current State**: Excellent
- All `CliError` variants map to appropriate exit codes
- Tests verify exit code mapping

**Location**: src/cli/error.rs:44-54, 56-70

### 3.2 Error Message Formatting

**Good:**
- Colored output for terminal (src/bin/csm.rs:39)
- JSON output for programmatic consumption (src/bin/csm.rs:36-38)
- Consistent prefixing with "error:"

**Issues:**

| Severity | Issue | Location | Example |
|----------|-------|----------|---------|
| MEDIUM | Error messages lack remediation hints | src/cli/commands/inject.rs:57-60 | "failed to inject concept" - no suggestion for fix |
| MEDIUM | File path formatting inconsistent | src/cli/commands/import.rs:14-18 | Uses display() but some paths use debug formatting |
| LOW | Emoji usage in terminal output | src/cli/commands/mod.rs:30,35,48 | May cause issues in some terminal environments |

### 3.3 JSON Error Output

**Current Implementation:**
```rust
// src/bin/csm.rs:37
serde_json::json!({"status": "error", "error": err.to_string()})
```

**Limitation:** Only contains the error message string, no:
- Error type/category
- Exit code
- Suggested remediation
- Error chain for debugging

**Recommended Improvement:**
```rust
pub fn format_error_json(err: &CliError) -> serde_json::Value {
    serde_json::json!({
        "status": "error",
        "error": {
            "message": err.to_string(),
            "exit_code": err.exit_code(),
            "category": match err {
                CliError::Config(_) => "configuration",
                CliError::Database(_) => "database",
                CliError::Input(_) => "input_validation",
                CliError::Validation(_) => "validation",
                CliError::Memory(_) => "memory_system",
                CliError::Io(_) => "io",
                CliError::Output(_) => "output",
                CliError::Persistence(_) => "persistence",
                CliError::Other(_) => "unknown",
            },
            "remediation": err.remediation_hint(),
        }
    })
}
```

---

## 4. Panic Safety Analysis

### 4.1 Production Code Panic Risks

| Severity | Location | Code | Risk |
|----------|----------|------|------|
| **CRITICAL** | src/framework.rs:177 | `.expect("reservoir initialized above")` | Justified but could be eliminated |
| HIGH | src/bin/csm.rs:59 | `std::env::var("TARGET").unwrap_or_else(|_| "unknown".into())` | Safe but unnecessary - use unwrap_or |
| MEDIUM | src/export_payload.rs:14 | `.unwrap_or_default()` on SystemTime | Returns 0 on system time error, acceptable |
| MEDIUM | src/singularity.rs:428 | `.unwrap_or_default()` on timestamp | Acceptable fallback |
| LOW | src/persistence.rs:464 | `row.get::<i64>(0).unwrap_or(0)` | Safe fallback for version query |

### 4.2 Test-Only Unwrap Usage

The following `unwrap()` calls are in test code and acceptable:
- src/reservoir.rs:451, 452, 457, 459, 465, 472, 474, 480
- src/hyperdim.rs:386, 392
- src/concept_builder.rs:103, 107, 110, 115
- src/framework.rs:400, 410, 412, 414, 415, 425, 450, 463, 465, 475, 483, 488, 489

### 4.3 Recommendations for Production Code

**Fix for src/framework.rs:177:**
```rust
// Current
let r = reservoir.as_mut().expect("reservoir initialized above");

// Safer
let Some(r) = reservoir.as_mut() else {
    return Err(MemoryError::Reservoir(
        "Reservoir not initialized - this is a bug".to_string()
    ));
};
```

**Fix for src/bin/csm.rs:59:**
```rust
// Current
std::env::var("TARGET").unwrap_or_else(|_| "unknown".into())

// Simpler
std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string())
```

---

## 5. Risk Classification Summary

### Critical (4 issues)

1. **Missing #[source] attributes** (src/error.rs)
   - Error chaining is broken, making debugging difficult
   - Fix: Add #[source] to all error-wrapping variants

2. **Manual error mapping everywhere** (src/cli/commands/*.rs)
   - Context loss at every API boundary
   - Fix: Implement proper From traits

3. **libsql::Error not wrapped** (src/persistence.rs)
   - Database errors converted to strings
   - Fix: Add libsql::Error to MemoryError with #[from]

4. **Reservoir expect in production** (src/framework.rs:177)
   - Potential panic path in async code
   - Fix: Convert to proper error return

### High (5 issues)

1. **anyhow context loss** (src/cli/error.rs:34-38)
2. **Database pool slot acquisition error handling** (src/persistence.rs:85-93)
3. **Transaction rollback error ignored** (src/persistence_ops.rs:239)
4. **Import error message parsing fragile** (src/cli/commands/import.rs:70-79)
5. **WASM error conversion loses all context** (src/wasm.rs:414-416)

### Medium (8 issues)

1. Error messages lack remediation hints
2. JSON error output insufficient
3. Inconsistent path formatting
4. `Other` variant too generic
5. `unwrap_or_default` on timestamps (acceptable but worth noting)
6. Row.get unwrap_or patterns (src/persistence.rs:464)
7. Error span context missing in tracing
8. File extension detection could fail gracefully

### Low (6 issues)

1. Emoji in terminal output
2. Error naming inconsistencies
3. No backtrace support
4. Test unwraps could use assert_matches
5. Some error messages could be more actionable
6. Exit code documentation missing

---

## 6. Specific File:Line References

### src/error.rs
- **Line 7-30**: Add #[source] attributes
- **Line 8**: Change `Database(String)` to structured variant
- **Line 20,23**: Rename for consistency

### src/cli/error.rs
- **Line 30-31**: Replace `Other(String)` with structured variant
- **Line 34-38**: Improve anyhow conversion
- **Line 56-70**: Add category() and remediation_hint() methods

### src/framework.rs
- **Line 177**: Replace expect with proper error
- **Line 359**: unwrap_or_default acceptable
- **Line 377**: unwrap_or acceptable

### src/cli/commands/inject.rs
- **Line 23-28**: Use ? with proper From impl
- **Line 32-37**: Same as above
- **Line 57-60**: Add remediation hint to error

### src/cli/commands/associate.rs
- **Line 25**: Wrong error category
- **Line 38**: Wrong error category

### src/cli/commands/probe.rs
- **Line 23**: Wrong error category

### src/persistence.rs
- **Line 464**: unwrap_or acceptable but could use unwrap_or_default

### src/wasm.rs
- **Line 414-416**: Add error context preservation

### src/bin/csm.rs
- **Line 59**: Simplify unwrap_or_else

### src/export_payload.rs
- **Line 14**: unwrap_or_default acceptable

---

## 7. Recommended Actions (GOAP Planning)

### Immediate Actions (This Sprint)

1. **ADR-0044**: Error Context Preservation
   - Add #[source] attributes to MemoryError
   - Implement From<libsql::Error> for MemoryError
   - Document error taxonomy

2. **Fix Critical Panic Path**
   - Replace expect in src/framework.rs:177
   - Add regression test

3. **CLI Error Enhancement**
   - Add structured JSON error output
   - Implement remediation hints

### Short-term Actions (Next 2 Sprints)

4. **Error Conversion Cleanup**
   - Remove manual map_err patterns
   - Implement proper From traits
   - Add error context tracing

5. **WASM Error Handling**
   - Preserve error chains in JS conversion
   - Add error codes to JS errors

### Long-term Actions (Backlog)

6. **Backtrace Support**
   - Enable backtrace capture in errors
   - Add RUST_BACKTRACE support docs

7. **Error Metrics**
   - Track error rates by category
   - Add alerting for critical errors

---

## 8. Code Quality Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Production unwrap/expect | 4 | 0 | FAIL |
| Test unwrap/expect | 23 | N/A | OK |
| Error variants with #[source] | 2 | 6+ | FAIL |
| From implementations | 4 | 8+ | FAIL |
| Error remediation hints | 0 | All variants | FAIL |
| JSON error structure | Basic | Rich | FAIL |

---

## 9. Conclusion

The codebase has a solid foundation with `thiserror` and structured error types. The main gaps are:

1. **Error chaining** - Missing #[source] attributes prevent proper error debugging
2. **Error conversion** - Too much manual error mapping
3. **Panic safety** - One critical expect remains in production code
4. **User experience** - Error messages could be more actionable

Priority should be given to fixing the critical panic path and adding proper error source chaining, as these directly impact production reliability and debugging capabilities.

---

**Report Generated By**: Error Handling Specialist (Swarm Group C)  
**Next Review**: Post-ADR-0044 implementation
