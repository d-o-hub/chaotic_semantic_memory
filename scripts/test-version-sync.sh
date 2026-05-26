#!/bin/bash
# test-version-sync.sh - Automated tests for verify-version-sync.sh

set -e

# Setup temporary test environment
TEST_DIR=$(mktemp -d)
trap 'rm -rf "$TEST_DIR"' EXIT

echo "Setting up mock project in $TEST_DIR..."
cp scripts/verify-version-sync.sh "$TEST_DIR/"
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
