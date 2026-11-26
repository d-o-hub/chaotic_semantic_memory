#!/usr/bin/env bash
# Fixture for `scripts/coverage-report.sh inventory`: the test-attribute parser,
# and the invariant that the shared-name header count equals the lead rows
# printed (#1075: a leading `[:10]` slice printed ten rows under a "12" header).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE_ROOT="$(mktemp -d)"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

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
[[ "$TOTAL" == "3:3" ]] ||
  fail "expected 3 unique tests, got ${TOTAL}"$'\n'"${OUTPUT}"
printf 'coverage inventory parameterized-attribute fixture: pass\n'

# Count/listing agreement, with 12 shared names - past the old ten-row cap, so a
# truncated listing fails here instead of under-reporting leads silently.
SHARED_ROOT="${FIXTURE_ROOT}/shared"
mkdir -p "${SHARED_ROOT}/tests"
for file in a b; do
  : > "${SHARED_ROOT}/tests/${file}.rs"
  for index in $(seq 1 12); do
    printf '#[test]\nfn duplicated_lead_%d() {}\n\n' "$index" >>"${SHARED_ROOT}/tests/${file}.rs"
  done
done

SHARED_OUTPUT="$(COVERAGE_INVENTORY_ROOT="$SHARED_ROOT" "$SCRIPT_DIR/coverage-report.sh" inventory)"
CLAIMED="$(sed -n 's/.*names present in more than one file: \([0-9][0-9]*\).*/\1/p' <<<"${SHARED_OUTPUT}")"
LISTED="$(grep -c '^  ' <<<"${SHARED_OUTPUT}" || true)"
[[ "$CLAIMED" == "12" ]] ||
  fail "expected 12 shared names, got '${CLAIMED}'"$'\n'"${SHARED_OUTPUT}"
[[ "$LISTED" == "$CLAIMED" ]] ||
  fail "header claims ${CLAIMED} shared names but the listing printed ${LISTED}"$'\n'"${SHARED_OUTPUT}"
printf 'coverage inventory full-lead-listing fixture: pass\n'
