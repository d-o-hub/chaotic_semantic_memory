# [ADR-0020] Fix Silent Data Loss on Association Load/Import

## Status
Accepted

## Context and Problem Statement
`load_replace`, `load_merge`, and `import_json` use `let _ = sing.associate(...)` which silently drops association errors. When a referenced concept is missing or an association is invalid, the error is discarded. This is silent data loss that operators cannot detect or recover from.

## Decision Drivers
- Data integrity: silent drops are unacceptable in a production system
- Observability: operators need visibility into skipped data
- Robustness: orphaned association references are expected during load and should not abort the entire operation

## Considered Options
- Ignore errors silently (current)
- Log warnings and continue
- Fail on first error
- Collect all errors and return summary

## Decision Outcome
Chosen option: "Log warnings and continue", because association failures during load are expected (orphaned references) and should not abort the entire load operation, but operators must have visibility.

### Implementation
- Replace `let _ = sing.associate(...)` with `if let Err(e) = sing.associate(...) { tracing::warn!(...) }`
- Include from_id, to_id, and error message in the warning
- Optionally return count of skipped associations in load result

### Positive Consequences
- No silent data loss
- Operators get structured log visibility via tracing
- Non-breaking for existing control flow

### Negative Consequences
- Adds log noise when many orphaned references exist (mitigated by warn level)
