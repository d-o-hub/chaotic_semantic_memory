# CLI Edge Cases Documentation

Reference for CLI implementation based on `chaotic_semantic_memory` crate API analysis.

## Library Validation Summary

| Validation | Source | Error Type |
|------------|--------|------------|
| Concept ID empty/length | `framework_validation.rs:10-27` | `InvalidInput` |
| Concept ID > 256 bytes | `framework_validation.rs:7,17-26` | `InvalidInput` |
| Association strength non-finite | `framework_validation.rs:31-35` | `InvalidInput` |
| Association strength < 0 | `framework_validation.rs:37-42` | `InvalidInput` |
| top_k = 0 | `framework_validation.rs:69-74` | `InvalidInput` |
| top_k > max_probe_top_k | `framework_validation.rs:75-83` | `InvalidInput` |
| Vector dimension != 80 | `singularity.rs:156-160` | `InvalidDimension` |
| Metadata size exceeds limit | `framework_validation.rs:46-61` | `InvalidInput` |

---

## 1. INJECT Command Edge Cases

### 1.1 Empty Concept ID
- **Source**: `framework_validation.rs:11-16`
- **Error**: `MemoryError::InvalidInput { field: "id", reason: "concept ID must not be empty" }`
- **CLI Handling**: Reject with exit code 1, print "error: concept ID cannot be empty"

### 1.2 Concept ID Exceeds 256 Bytes
- **Source**: `framework_validation.rs:7,17-26`
- **Error**: `MemoryError::InvalidInput { field: "id", reason: "concept ID exceeds 256 bytes (got N)" }`
- **CLI Handling**: Reject with exit code 1, print "error: concept ID too long (max 256 bytes, got N)"

### 1.3 Duplicate Concept ID
- **Source**: `persistence.rs:148-160` (INSERT OR REPLACE semantics)
- **Behavior**: Uperts the concept (replaces existing)
- **CLI Handling**: Warn if `--warn-duplicate` flag set, otherwise silent upsert

### 1.4 Invalid Metadata JSON
- **Source**: `singularity.rs:428-440` (ConceptBuilder), `error.rs:28-29`
- **Error**: `MemoryError::Serialization(serde_json::Error)`
- **CLI Handling**: Parse JSON argument before API call; reject malformed JSON with "error: invalid JSON for metadata"

### 1.5 Vector Dimension Mismatch
- **Source**: `singularity.rs:156-160`
- **Error**: `MemoryError::InvalidDimension { expected: 80, actual: N }`
- **CLI Handling**: Validate vector input format; HVec10240 requires 80 x 128-bit words (1280 hex chars or binary format)

### 1.6 Metadata Size Exceeds Limit
- **Source**: `framework_validation.rs:46-61`
- **Condition**: Only when `max_metadata_bytes` configured in `FrameworkConfig`
- **Error**: `MemoryError::InvalidInput { field: "metadata", reason: "metadata exceeds N bytes (got M)" }`
- **CLI Handling**: Check metadata size if `--max-metadata-bytes` configured

---

## 2. PROBE Command Edge Cases

### 2.1 Empty Query
- **Source**: `framework.rs:162-170`
- **Behavior**: Empty query vector (all zeros) returns lowest similarity results
- **CLI Handling**: Accept empty/null query; treat as zero vector

### 2.2 top_k = 0
- **Source**: `framework_validation.rs:69-74`
- **Error**: `MemoryError::InvalidInput { field: "top_k", reason: "top_k must be greater than 0" }`
- **CLI Handling**: Reject with exit code 1, print "error: top_k must be at least 1"

### 2.3 top_k > max_probe_top_k
- **Source**: `framework_validation.rs:75-83`, default `DEFAULT_MAX_PROBE_TOP_K = 10_000`
- **Error**: `MemoryError::InvalidInput { field: "top_k", reason: "top_k exceeds configured limit N (got M)" }`
- **CLI Handling**: Reject with exit code 1, suggest `--max-probe-top-k` config option

### 2.4 No Concepts in Memory
- **Source**: `singularity.rs:223-225`
- **Behavior**: Returns empty `Vec<(String, f32)>`
- **CLI Handling**: Return empty result with exit code 0; optionally warn "no concepts in memory"

### 2.5 Query Vector Dimension Mismatch
- **Source**: Same as 1.5 (HVec10240 requires 80 words)
- **CLI Handling**: Validate query vector input format before API call

---

## 3. ASSOCIATE Command Edge Cases

### 3.1 Non-existent Concept IDs
- **Source**: `singularity.rs:287-291`
- **Error**: `MemoryError::Persistence("Both concepts must exist to create association")`
- **CLI Handling**: Check both IDs exist before calling; exit 1 with "error: concept 'X' not found"

### 3.2 Association Strength Out of Range
- **Source**: `framework_validation.rs:30-44`
- **Errors**:
  - Non-finite (NaN/Inf): `InvalidInput { field: "strength", reason: "association strength must be finite" }`
  - Negative: `InvalidInput { field: "strength", reason: "association strength must be non-negative" }`
- **CLI Handling**: Reject with exit 1; print "error: strength must be finite and >= 0 (got X)"
- **Note**: Strength is not bounded above; any finite non-negative value accepted

### 3.3 Self-Association
- **Source**: `singularity.rs:286-312` (no explicit check)
- **Behavior**: Allowed; creates association from concept to itself
- **CLI Handling**: Allow; optionally warn if `--warn-self-associate` flag set

### 3.4 Max Associations Limit Reached
- **Source**: `singularity.rs:296-308`
- **Condition**: Only when `max_associations_per_concept` configured
- **Behavior**: Evicts weakest association to make room
- **CLI Handling**: Silent eviction; optionally log if verbose mode enabled

---

## 4. EXPORT/IMPORT Edge Cases

### 4.1 Non-existent Database Path (Local)
- **Source**: `persistence.rs:35-49` (Builder::new_local)
- **Behavior**: Creates new database if path doesn't exist
- **CLI Handling**: For export, verify source DB exists; for import, create if missing

### 4.2 Permission Denied
- **Source**: `error.rs:25-26` (Io error from std::io::Error)
- **Error**: `MemoryError::Io(std::io::Error)` with kind `PermissionDenied`
- **CLI Handling**: Exit 1 with "error: permission denied: PATH"

### 4.3 Disk Full During Export
- **Source**: `error.rs:25-26`
- **Error**: `MemoryError::Io(std::io::Error)` with kind `StorageFull`
- **CLI Handling**: Exit 1 with "error: disk full"; attempt cleanup of partial file

### 4.4 Corrupted Import File
- **Source**: `error.rs:28-29` (Serialization error), `persistence.rs:303-304` (HVec10240::from_bytes)
- **Errors**:
  - `MemoryError::Serialization` for malformed JSON
  - `MemoryError::InvalidDimension` for wrong vector size
- **CLI Handling**: Exit 1 with "error: corrupt import file: N records failed"; support `--skip-invalid` to continue

### 4.5 Version Mismatch During Import
- **Source**: `persistence.rs:13` (`LATEST_SCHEMA_VERSION = 2`)
- **Behavior**: Schema migrations applied automatically in `init_schema`
- **CLI Handling**: Warn if import file has older schema; migrations applied silently

### 4.6 Concurrent Access During Export
- **Source**: `persistence.rs:72-83` (connection handling)
- **Behavior**: SQLite WAL mode allows concurrent reads; writes blocked during checkpoint
- **CLI Handling**: Use read transaction for export; timeout after configurable wait

---

## 5. Global Edge Cases

### 5.1 Database Locked
- **Source**: libsql connection semantics
- **Error**: `MemoryError::Database("Failed to ...: database is locked")`
- **CLI Handling**: Retry with exponential backoff (max 3 retries); exit 1 with "error: database locked after N retries"

### 5.2 Invalid Config File
- **Source**: Application-level (not in crate)
- **CLI Handling**:
  - Missing config: Use defaults, warn if `--config` specified
  - Malformed config: Exit 1 with "error: invalid config at PATH: reason"
  - Unknown keys: Warn but continue

### 5.3 SIGINT Handling (Graceful Shutdown)
- **Source**: Application-level signal handling
- **CLI Handling**:
  - Install SIGINT handler with `tokio::signal::ctrl_c()`
  - On interrupt: Cancel in-flight operations, flush pending writes, close DB
  - Exit 130 (128 + SIGINT signal number)

### 5.4 TTY Detection (Colors, Pager)
- **Source**: Application-level terminal detection
- **CLI Handling**:
  - Use `atty::is(atty::Stream::Stdout)` or equivalent
  - No TTY: Disable colors, disable pager, use machine-parseable output
  - TTY: Colors by default, pipe to pager for long output
  - Override: `--color=always|never|auto`, `--no-pager`

---

## Error Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation error / user error |
| 2 | Configuration error |
| 3 | Database error |
| 4 | I/O error |
| 5 | Serialization error |
| 130 | SIGINT received (128 + 2) |

---

## Recommended CLI Flags

```
Global:
  --config PATH           Configuration file path
  --db PATH               Database path (overrides config)
  --color WHEN            Color output: always, never, auto
  --no-pager              Disable pager for output
  --verbose               Enable debug logging
  --quiet                 Suppress non-error output

INJECT:
  --metadata JSON         JSON metadata string
  --metadata-file PATH    Read metadata from file
  --vector HEX            Hex-encoded vector (80 x 128-bit)
  --vector-random         Generate random vector
  --warn-duplicate        Warn on duplicate ID

PROBE:
  --top-k N               Number of results (default: 10)
  --query-vector HEX      Query vector
  --format FORMAT         Output format: json, table, csv

ASSOCIATE:
  --warn-self-associate   Warn on self-association

IMPORT:
  --skip-invalid          Skip invalid records, continue import
  --format FORMAT         Input format: json, csv
```
