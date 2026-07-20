# Dependabot Alerts — Open (Cannot Auto-Resolve)

This file documents Dependabot alerts that remain open because no
upstream fix is available.

---

## Alert #4, #1: libsql-sqlite3-parser — Crash due to invalid UTF-8 input

- **Severity**: Low
- **Package**: `libsql-sqlite3-parser`
- **Vulnerable versions**: `<= 0.13.0`
- **Our version**: `0.13.0` (in Cargo.lock)
- **Patched version**: None available (2026-05-21)
- **Dependency chain**: `chaotic_semantic_memory` → `libsql 0.9.30` → `libsql-sqlite3-parser 0.13.0`
- **Impact**: Low — only affects parsing of malformed SQL from untrusted sources,
  which is not a typical attack vector for this project.
- **Mitigation**: We do not accept arbitrary SQL from external sources.
  All SQL in this project is either hardcoded in migrations or generated
  from parameterized queries in the persistence layer.
- **Resolution path**: Wait for libsql to upgrade to a patched
  libsql-sqlite3-parser (>= 0.14.0 when available), then update our
  libsql dependency.
- **Last reviewed**: 2026-05-21

---

## Alert #14: rand — Unsound with custom logger using rand::rng()

- **Status**: DISMISSED as false positive (2026-05-21)
- **Reason**: Our Cargo.lock has rand 0.8.6 (patched version).
  No transitive dependency resolves to a version < 0.8.6.
  See `git log` for dismissal record.

---

## Process

1. **New alert**: Add entry above with investigation results.
2. **Alert fixed upstream**: Update our dependency, verify in Cargo.lock,
   verify Dependabot auto-closes it, move to Resolved.
3. **Alert dismissible**: Dismiss via GitHub API with reason.

## Resolved

None yet — pending `libsql-sqlite3-parser` upstream fix.
