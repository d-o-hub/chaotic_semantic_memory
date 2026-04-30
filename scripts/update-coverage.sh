#!/usr/bin/env bash
# Update coverage metrics in README.md automatically
# Run during pre-commit or before releases

set -euo pipefail

README="README.md"

echo "Updating coverage metrics in ${README}..."

# Count total tests (sum of all test runs)
TOTAL_TESTS=$(cargo test --quiet 2>&1 | grep "passed" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' | awk '{sum+=$1} END {print sum}')

# Count test file LOC
TEST_LOC=$(find tests -name '*.rs' -exec cat {} \; 2>/dev/null | wc -l)

# Count source LOC (all src/*.rs files)
SRC_LOC=$(find src -name '*.rs' -exec cat {} \; 2>/dev/null | wc -l)

# Estimate inline test LOC by extracting test module content
INLINE_TEST_LOC=0
for f in src/*.rs; do
  if grep -q '#\[cfg(test)\]' "$f" 2>/dev/null; then
    lines=$(awk '/#\[cfg\(test\)\]/,/^}$/' "$f" | wc -l)
    INLINE_TEST_LOC=$((INLINE_TEST_LOC + lines))
  fi
done

# Calculate coverage ratio
# (Test LOC + Inline test LOC) / (Source LOC - Inline test LOC) * 100
NON_TEST_SRC=$((SRC_LOC - INLINE_TEST_LOC))
if [ "$NON_TEST_SRC" -gt 0 ]; then
  COVERAGE_RATIO=$(( (TEST_LOC + INLINE_TEST_LOC) * 100 / NON_TEST_SRC ))
else
  COVERAGE_RATIO=100
fi

# Cap at 100%
if [ "$COVERAGE_RATIO" -gt 100 ]; then
  COVERAGE_RATIO=100
fi

# Check if target achieved
TARGET_ACHIEVED=""
if [ "$COVERAGE_RATIO" -ge 90 ]; then
  TARGET_ACHIEVED=" ✅"
fi

echo "  Tests: $TOTAL_TESTS"
echo "  Test LOC: $TEST_LOC"
echo "  Inline test LOC: $INLINE_TEST_LOC"
echo "  Source LOC: $SRC_LOC"
echo "  Coverage: ${COVERAGE_RATIO}%"

# Update README.md using perl (cross-platform compatible)
# Export variables for perl's $ENV{} to access
export TOTAL_TESTS INLINE_TEST_LOC COVERAGE_RATIO TARGET_ACHIEVED

# Update Total tests
perl -i -pe 's/\| Total tests \| \d+ \|/"| Total tests | $ENV{TOTAL_TESTS} |"/e' "$README"

# Update Inline test LOC (handle comma-formatted numbers)
perl -i -pe 's/\| Inline test LOC \| [\d,]+ \(in src modules\)/"| Inline test LOC | $ENV{INLINE_TEST_LOC} (in src modules)"/e' "$README"

# Update Test:Source ratio (handle asterisks and optional checkmark)
perl -i -pe 's/\| Test:Source ratio \| \*\*[\d]+%?\*\* \(target: 90%\)[: ]*[^|]*\|/"| Test:Source ratio | **$ENV{COVERAGE_RATIO}%** (target: 90%)$ENV{TARGET_ACHIEVED} |"/e' "$README"

echo "✅ Coverage metrics updated in ${README}"