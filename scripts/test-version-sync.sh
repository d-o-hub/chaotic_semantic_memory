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
mkdir -p wasm cli-npm tests examples
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

echo "All tests passed!"
