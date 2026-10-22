#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE_ROOT="$(mktemp -d)"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT
mkdir -p "$FIXTURE_ROOT/tests"
cat > "$FIXTURE_ROOT/tests/inventory.rs" <<'RUST'
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parameterized_tokio_test() {}

#[test]
fn plain_test() {}

#[tokio::test]
async fn tokio_test_without_options() {}
RUST

OUTPUT="$(COVERAGE_INVENTORY_ROOT="$FIXTURE_ROOT" "$SCRIPT_DIR/coverage-report.sh" inventory)"
TOTAL="$(awk '$1 == "TOTAL" { print $2 ":" $3 }' <<<"$OUTPUT")"
[[ "$TOTAL" == "3:3" ]] || {
  printf 'expected 3 unique tests, got %s\n%s\n' "$TOTAL" "$OUTPUT" >&2
  exit 1
}
printf 'coverage inventory parameterized-attribute fixture: pass\n'
