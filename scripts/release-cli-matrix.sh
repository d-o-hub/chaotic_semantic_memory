#!/usr/bin/env bash
# scripts/release-cli-matrix.sh — one source for the release CLI matrix.
#
# Issue #974: the platform list lived inside `build-cli-matrix.strategy`, and the
# verifier would have needed a second hardcoded copy to know which tarballs to
# expect. `validate` now emits this JSON as a job output; the matrix and the
# asset expectation both consume it, so adding a target cannot drift one from
# the other.
#
# Usage: release-cli-matrix.sh [--json]
set -euo pipefail

cat <<'JSON'
[
  {"runner":"ubuntu-24.04","target":"x86_64-unknown-linux-gnu","platform":"linux","arch":"x64"},
  {"runner":"ubuntu-24.04","target":"aarch64-unknown-linux-gnu","platform":"linux","arch":"arm64"},
  {"runner":"macos-latest","target":"aarch64-apple-darwin","platform":"darwin","arch":"arm64"},
  {"runner":"macos-latest","target":"x86_64-apple-darwin","platform":"darwin","arch":"x64"},
  {"runner":"windows-latest","target":"x86_64-pc-windows-msvc","platform":"win32","arch":"x64"}
]
JSON
