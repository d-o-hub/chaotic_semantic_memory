#!/usr/bin/env bash
# Negative fixtures for skill-local validation commands.
# Verifies that validation commands fail appropriately on bad input.

echo "=== Negative Fixtures for Skill Validation ==="
echo ""

PASS=0
FAIL=0

# Test 1: cargo check should fail on invalid syntax
echo -n "Testing cargo check (invalid syntax)... "
echo 'fn main() { invalid_syntax!' > /tmp/test_invalid.rs
if ! rustc /tmp/test_invalid.rs >/dev/null 2>&1; then
    echo "PASS"
    PASS=$((PASS + 1))
else
    echo "FAIL"
    FAIL=$((FAIL + 1))
fi

# Test 2: cargo fmt should detect unformatted code
echo -n "Testing cargo fmt (unformatted)... "
printf 'fn main(){let x=1;}\n' > /tmp/test_fmt.rs
if ! rustfmt --check /tmp/test_fmt.rs >/dev/null 2>&1; then
    echo "PASS"
    PASS=$((PASS + 1))
else
    echo "FAIL"
    FAIL=$((FAIL + 1))
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
