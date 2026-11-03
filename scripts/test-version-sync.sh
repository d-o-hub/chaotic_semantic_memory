#!/bin/bash
# test-version-sync.sh - Automated tests for verify-version-sync.sh
#
# Resolved through SCRIPT_DIR, not the working directory: this fixture was wired
# into scripts/validate.sh on 2026-10-05 and had never been invoked by anything,
# so running it from outside the project root failed on `cp: cannot stat
# 'scripts/verify-version-sync.sh'` — a missing-file error that looks like a
# version-sync regression. Same idiom as scripts/test-llms-sync.sh.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Setup temporary test environment
TEST_DIR=$(mktemp -d)
trap 'rm -rf "$TEST_DIR"' EXIT

echo "Setting up mock project in $TEST_DIR..."
cp "${SCRIPT_DIR}/verify-version-sync.sh" "$TEST_DIR/"
cd "$TEST_DIR"

# Create mock files
mkdir -p wasm cli-npm tests examples scripts
# The verifier audits intra-workspace pins through scripts/workspace-version-edges.py
# (issue #964), so the mock project carries it the same way it carries the verifier.
cp "${SCRIPT_DIR}/workspace-version-edges.py" scripts/
# The docs/tests/examples literal gate (issue #982) is invoked by the verifier too;
# a verifier copied without it must fail closed (exit 2) rather than skip the check.
cp "${SCRIPT_DIR}/check-version-literals.py" scripts/
cat > Cargo.toml <<EOF
[package]
name = "test"
version = "0.3.6"
EOF

cat > wasm/package.json <<EOF
{ "version": "0.3.6" }
EOF

cat > cli-npm/package.json <<EOF
{ "version": "0.3.6" }
EOF

echo "0.3.6" > VERSION

echo "Test 1: All versions synchronized"
if bash verify-version-sync.sh > /dev/null 2>&1; then
    echo "✅ Success"
else
    echo "❌ Failure"
    exit 1
fi

echo "Test 2: VERSION file mismatch"
echo "0.3.5" > VERSION
if ! bash verify-version-sync.sh > /dev/null 2>&1; then
    echo "✅ Success (Correctly identified mismatch)"
else
    echo "❌ Failure (Failed to identify mismatch)"
    exit 1
fi
echo "0.3.6" > VERSION

echo "Test 3: cli-npm/package.json mismatch"
cat > cli-npm/package.json <<EOF
{ "version": "0.3.5" }
EOF
if ! bash verify-version-sync.sh > /dev/null 2>&1; then
    echo "✅ Success (Correctly identified mismatch)"
else
    echo "❌ Failure (Failed to identify mismatch)"
    exit 1
fi
cat > cli-npm/package.json <<EOF
{ "version": "0.3.6" }
EOF

echo "Test 4: wasm/package.json mismatch"
cat > wasm/package.json <<EOF
{ "version": "0.3.5" }
EOF
if ! bash verify-version-sync.sh > /dev/null 2>&1; then
    echo "✅ Success (Correctly identified mismatch)"
else
    echo "❌ Failure (Failed to identify mismatch)"
    exit 1
fi
cat > wasm/package.json <<EOF
{ "version": "0.3.6" }
EOF

# Tests 5-9: the intra-workspace pin audit (issue #964), which this fixture never had
# because the verifier it tests grepped root -> member pins only. A member -> root pin at
# a stale version passed while the script printed "✓ All local versions synchronized".
# The pin audit lives in scripts/workspace-version-edges.py, so the mock project has to
# carry it — a copied-alone verifier is the "sensor not shipped with the tree" case.
echo "Test 5: a stale intra-workspace pin is a finding, not a pass"
mkdir -p crates/thing/src
cat > crates/thing/Cargo.toml <<EOF
[package]
name = "thing"
version = "0.3.6"

[dependencies]
test = { path = "../..", version = "0.3.5" }
EOF
if bash verify-version-sync.sh > out5.txt 2>&1; then
    echo "❌ Failure (a stale pin was reported as synchronized)"
    cat out5.txt
    exit 1
fi
if grep -q 'DRIFT: crates/thing/Cargo.toml \[dependencies\] test' out5.txt; then
    echo "✅ Success (the finding names the edge and both versions)"
else
    echo "❌ Failure (the finding did not name the edge)"
    cat out5.txt
    exit 1
fi

echo "Test 6: the same tree passes once the pin matches"
cat > crates/thing/Cargo.toml <<EOF
[package]
name = "thing"
version = "0.3.6"

[dependencies]
test = { path = "../..", version = "0.3.6" }
EOF
if bash verify-version-sync.sh > out6.txt 2>&1; then
    echo "✅ Success"
else
    echo "❌ Failure (a synchronized tree was reported dirty)"
    cat out6.txt
    exit 1
fi

echo "Test 7: a pin form the audit cannot read stops the verifier with exit 2"
# Single quotes are valid TOML and invisible to a `version = "` regex, so the only honest
# answer is "I could not audit this" — an exit 1 here would read as drift, and an exit 0
# would read as synchronized. Both are wrong; #964's whole class is a green verifier.
cat > crates/thing/Cargo.toml <<EOF
[package]
name = "thing"
version = "0.3.6"

[dependencies]
test = { path = "../..", version = '0.3.5' }
EOF
rc=0
bash verify-version-sync.sh > out7.txt 2>&1 || rc=$?
if [[ "${rc}" -eq 2 ]] \
    && grep -q "could not run (exit 2) — not a verdict" out7.txt \
    && ! grep -q "All local versions synchronized" out7.txt; then
    echo "✅ Success (exit 2, and no synchronization claim)"
else
    echo "❌ Failure (exit ${rc}, expected 2 with no synchronization claim)"
    cat out7.txt
    exit 1
fi
cat > crates/thing/Cargo.toml <<EOF
[package]
name = "thing"
version = "0.3.6"

[dependencies]
test = { path = "../..", version = "0.3.6" }
EOF

echo "Test 8: a member outside crates/ is still version-checked"
# Two sensors cover this file now, and the mock tree proves the older one was blind: the
# pin audit reaches it by walking the filesystem (no git index, and no `crates/*` pattern
# matches `benchmarks/`), while `find crates benchmarks` names it for the *member version*
# check — which is the gate that fires here, because its pin matches and its own version
# does not (#964: benchmarks/Cargo.toml shipped at 0.3.6 for three releases).
mkdir -p benchmarks/src
cat > benchmarks/Cargo.toml <<EOF
[package]
name = "bench"
version = "0.3.5"

[dependencies]
test = { path = "..", version = "0.3.6" }
EOF
if bash verify-version-sync.sh > out8.txt 2>&1; then
    echo "❌ Failure (a stale member outside crates/ was not reported)"
    cat out8.txt
    exit 1
fi
if grep -q "benchmarks/Cargo.toml" out8.txt; then
    echo "✅ Success (the mismatch names benchmarks/Cargo.toml)"
else
    echo "❌ Failure (the mismatch did not name the member)"
    cat out8.txt
    exit 1
fi
sed -i 's/^version = "0.3.5"/version = "0.3.6"/' benchmarks/Cargo.toml
if bash verify-version-sync.sh > out8b.txt 2>&1; then
    echo "✅ Success (the tree passes once every member version matches)"
else
    echo "❌ Failure (a synchronized tree was reported dirty)"
    cat out8b.txt
    exit 1
fi

echo "Test 9: a verifier missing its pin-audit tool refuses instead of claiming sync"
rm -rf scripts
rc=0
bash verify-version-sync.sh > out9.txt 2>&1 || rc=$?
if [[ "${rc}" -eq 2 ]] \
    && grep -q "the pin audit cannot run" out9.txt \
    && ! grep -q "All local versions synchronized" out9.txt; then
    echo "✅ Success (exit 2, and no synchronization claim)"
else
    echo "❌ Failure (exit ${rc}, expected 2 with no synchronization claim)"
    cat out9.txt
    exit 1
fi

echo "All tests passed!"
