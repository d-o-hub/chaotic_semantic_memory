---
name: memory-lifecycle-verification
description: Portable verification skill for memory lifecycle operations (save, load, archive, delete) with file-level and database-level checks.
---

# Memory Lifecycle Verification

Use this skill when validating memory behavior across repositories or before release.

## Goal

Prove that lifecycle operations are correct and auditable for both:

- files (export/import/archive artifacts)
- database entries (concept and association records)

## Required Lifecycle Coverage

1. **save**: data is persisted and discoverable
2. **load**: saved data roundtrips without corruption
3. **archive**: data is retained with explicit archived state
4. **delete**: data is removed (or tombstoned) consistently

## Portable Test Contract

Set these variables per target codebase:

```bash
DB_PATH="${DB_PATH:-.tmp/memory-lifecycle.db}"
ARTIFACT_DIR="${ARTIFACT_DIR:-.tmp/memory-artifacts}"
mkdir -p "$ARTIFACT_DIR"
```

### Step 1: Save

```bash
csm --database "$DB_PATH" inject "test::lifecycle::alpha" --metadata '{"phase":"save","v":1}'
csm --database "$DB_PATH" inject "test::lifecycle::beta" --metadata '{"phase":"save","v":1}'
csm --database "$DB_PATH" associate "test::lifecycle::alpha" "test::lifecycle::beta" -s 0.9
```

Expected:
- `probe` can find `test::lifecycle::alpha`
- DB row exists for both concepts and one association

### Step 2: Load / Roundtrip

```bash
csm --database "$DB_PATH" export -o "$ARTIFACT_DIR/export.json"
csm --database "$ARTIFACT_DIR/roundtrip.db" import "$ARTIFACT_DIR/export.json"
csm --database "$ARTIFACT_DIR/roundtrip.db" probe "test::lifecycle::alpha" -k 3 --output-format json
```

Expected:
- exported file exists and is non-empty
- imported DB returns matching IDs and metadata

### Step 3: Archive

If native archive command exists, use it. Otherwise write an archive marker concept:

```bash
csm --database "$DB_PATH" inject "archive::test::lifecycle::alpha" \
  --metadata '{"status":"archived","target":"test::lifecycle::alpha"}'
```

Expected:
- archive marker exists in DB
- audit artifact records source, timestamp, and target

### Step 4: Delete

Use native delete command when available. If not available, run retention policy that tombstones the concept.

Expected:
- deleted/tombstoned concept cannot be returned as active memory
- no orphaned associations to deleted ID

## Verification Evidence

Capture all of the following:

- command log for each phase
- exported files and checksums
- DB verification outputs (row counts or query results)
- final pass/fail matrix

## References

- `references/VALIDATION_CHECKLIST.md`
- `references/sql_checks.sql`
