#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

# One compiler/linker job and stripped test/dev debug info reduce peak memory.
# `set -e` ensures a killed compiler/linker cannot be mistaken for a pass.
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0

printf '%s\n' '==> cargo fmt --all -- --check'
cargo fmt --all -- --check

printf '%s\n' '==> cargo clippy --workspace --all-targets --all-features -- -D warnings'
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

mapfile -t PACKAGES < <(
    cargo metadata --locked --no-deps --format-version 1 \
        | python3 -c 'import json, sys; print("\n".join(sorted({p["name"] for p in json.load(sys.stdin)["packages"]})))'
)
if ((${#PACKAGES[@]} == 0)); then
    printf '%s\n' 'error: cargo metadata returned no workspace packages' >&2
    exit 1
fi

for package in "${PACKAGES[@]}"; do
    printf '==> cargo test --locked --package %s --all-features --all-targets\n' "${package}"
    cargo test --locked --package "${package}" --all-features --all-targets
done
