# Release Guardrails

Three-layer defense against CHANGELOG formatting issues that cause release failures.

## Problem Statement

**v0.2.9 npm publish failed** due to duplicate CHANGELOG header:
```
## [0.2.9]           ← Duplicate (broken)
## [0.2.9] - 2026-04-06  ← Correct format
```

This broke awk extraction in release workflow, resulting in empty release notes.

## Solution: Three-Layer Guardrails

### Layer 1: CI Validation (release.yml)

**Location**: `.github/workflows/release.yml` lines 39-65

**Checks**:
1. Version header exists: `## [VERSION]`
2. **NO duplicate headers** (guards against the exact v0.2.9 issue)
3. Header has date: `## [VERSION] - YYYY-MM-DD`

**Failure Mode**: Blocks release before tag creation

**Example**:
```yaml
- name: Verify changelog entry exists
  run: |
    VERSION="${{ steps.version.outputs.version }}"
    
    # Guardrail: Check for duplicate headers
    HEADER_COUNT=$(grep -c "^## \\[${VERSION}\\]" CHANGELOG.md || true)
    if [ "$HEADER_COUNT" -gt 1 ]; then
      echo "❌ Duplicate CHANGELOG header for ${VERSION}"
      exit 1
    fi
```

### Layer 2: Developer Validation (sync-version.sh)

**Location**: `scripts/sync-version.sh` lines 52-78

**Checks**:
1. `[Unreleased]` section exists (required by Keep a Changelog)
2. **No existing headers for target version** (prevents duplicates)
3. Version link entry at bottom (optional warning)

**Failure Mode**: Blocks version sync before commit

**Example**:
```bash
validate_changelog() {
  # Check for duplicate version headers
  existing_count=$(grep -c "^## \\[${ver}\\]" "$changelog" || echo "0")
  if [ "$existing_count" -gt 0 ]; then
    echo "Error: Version ${ver} already has header(s)"
    exit 1
  fi
}
```

### Layer 3: Pre-commit Hook (pre-commit.sh)

**Location**: `scripts/pre-commit.sh` lines 30-49

**Checks**:
1. **No duplicate headers across ALL versions** (catches any duplicates)
2. All version headers have dates (except [Unreleased])

**Failure Mode**: Blocks commit before push

**Example**:
```bash
# Check for duplicate version headers
DUPLICATES=$(grep "^## \\[" CHANGELOG.md | cut -d'[' -f2 | cut -d']' -f1 | sort | uniq -d)
if [ -n "$DUPLICATES" ]; then
  echo "❌ Duplicate CHANGELOG headers found: $DUPLICATES"
  exit 1
fi
```

## Validation Flow

```
Developer writes CHANGELOG
    ↓
[1] Pre-commit hook validates format
    ↓
Developer runs sync-version.sh
    ↓
[2] Sync script validates no existing headers
    ↓
Developer commits and pushes
    ↓
[3] Release workflow validates before tag
    ↓
Release proceeds successfully
```

## Testing Guardrails

### Test Duplicate Detection

```bash
# Add duplicate header (should fail all three layers)
echo "## [0.2.9] - 2026-04-06" >> CHANGELOG.md

# Layer 3: Pre-commit
bash scripts/pre-commit.sh
# Output: ❌ Duplicate CHANGELOG headers found: 0.2.9

# Layer 2: Sync-version
bash scripts/sync-version.sh 0.2.9
# Output: Error: Version 0.2.9 already has 2 header(s)
```

### Test Missing Date

```bash
# Remove date from header
sed -i 's/## \[0.2.9\] - 2026-04-06/## [0.2.9]/' CHANGELOG.md

# Layer 3: Pre-commit
bash scripts/pre-commit.sh
# Output: ❌ CHANGELOG headers missing dates
```

### Test Missing Unreleased

```bash
# Remove Unreleased section
sed -i '/## \[Unreleased\]/d' CHANGELOG.md

# Layer 2: Sync-version
bash scripts/sync-version.sh 0.3.0
# Output: Error: Missing [Unreleased] section in CHANGELOG.md
```

## CHANGELOG Requirements

**Required Format**:
```markdown
## [Unreleased]

## [0.2.9] - 2026-04-06

### Added
- Description

### Fixed
- Description

[unreleased]: https://github.com/.../compare/v0.2.9...HEAD
[0.2.9]: https://github.com/.../releases/tag/v0.2.9
```

**Common Mistakes**:
- ❌ Duplicate headers: `## [0.2.9]` appears twice
- ❌ Missing date: `## [0.2.9]` (should be `## [0.2.9] - YYYY-MM-DD`)
- ❌ Missing Unreleased: No `## [Unreleased]` section
- ❌ Missing version link: No `[0.2.9]:` at bottom

## Enforcement Strategy

1. **Pre-commit**: Fast feedback during development
2. **Sync-version**: Validation before version bump
3. **CI**: Final gate before release

This ensures issues are caught at the earliest possible stage, reducing CI failures and manual cleanup.

## Related

- ADR-0049: Release Checklist
- LEARNINGS.md: 2026-04-06 Release Workflow Production Solution
- `.github/workflows/release.yml`: Lines 39-65
