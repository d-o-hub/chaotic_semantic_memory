#!/usr/bin/env bash
# Generate llms-full.txt for AI tool integration
# Uses cargo-llms-txt: https://github.com/masinc/cargo-llms-txt

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

# Ensure cargo-llms-txt v0.1.1 is installed for deterministic output
if ! command -v cargo-llms-txt &> /dev/null; then
    echo "Installing cargo-llms-txt v0.1.1..."
    cargo install cargo-llms-txt --version 0.1.1
fi

echo "Generating llms.txt and llms-full.txt..."
if ! cargo llms-txt; then
    echo "❌ Failed to generate llms.txt and llms-full.txt" >&2
    exit 1
fi

# Normalize output to ensure deterministic results (sort dependencies/features)
if command -v python3 &> /dev/null; then
    python3 "${SCRIPT_DIR}/normalize_llms.py" llms.txt
    python3 "${SCRIPT_DIR}/normalize_llms.py" llms-full.txt
fi

echo "✅ llms.txt and llms-full.txt generated and normalized"
