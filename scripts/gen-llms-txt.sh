#!/usr/bin/env bash
# Generate llms-full.txt for AI tool integration
# Uses cargo-llms-txt: https://github.com/masinc/cargo-llms-txt

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

# Check if cargo-llms-txt is installed
if ! command -v cargo-llms-txt &> /dev/null; then
    echo "Installing cargo-llms-txt..."
    cargo install cargo-llms-txt
fi

echo "Generating llms-full.txt..."
cargo llms-txt

echo "✅ llms-full.txt generated"
