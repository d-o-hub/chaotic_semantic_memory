#!/usr/bin/env bash
# test-llms-sync.sh - Regression tests for scripts/check-llms-sync.sh
#
# Builds an isolated mock repository (no Cargo, no network) where a stub
# scripts/gen-llms-txt.sh writes deterministic fixture output, then exercises
# the checker's pass/fail cases.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

echo "Setting up mock repository in ${TEST_DIR}..."
mkdir -p "${TEST_DIR}/scripts"
cp "${SCRIPT_DIR}/check-llms-sync.sh" "${TEST_DIR}/scripts/check-llms-sync.sh"

# Stub generator: deterministic fixture contents. STUB_FAIL=1 simulates a
# generator failure with a distinctive exit code.
cat > "${TEST_DIR}/scripts/gen-llms-txt.sh" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${STUB_FAIL:-0}" == "1" ]]; then
    echo "stub generator failure" >&2
    exit 3
fi
printf '%s\n' "- opentelemetry (0.32)" > llms.txt
printf '%s\n' "- opentelemetry (0.32)" > llms-full.txt
STUB
chmod +x "${TEST_DIR}/scripts/gen-llms-txt.sh"

CHECK="${TEST_DIR}/scripts/check-llms-sync.sh"

# Run the checker (optionally with a failing stub); sets CHECK_STATUS and CHECK_OUTPUT.
CHECK_STATUS=0
CHECK_OUTPUT=""
run_check() {
    local stub_fail="${1:-0}"
    CHECK_STATUS=0
    CHECK_OUTPUT="$(STUB_FAIL="${stub_fail}" bash "${CHECK}" 2>&1)" || CHECK_STATUS=$?
}

write_synced() {
    printf '%s\n' "- opentelemetry (0.32)" > "${TEST_DIR}/llms.txt"
    printf '%s\n' "- opentelemetry (0.32)" > "${TEST_DIR}/llms-full.txt"
}

echo "Test 1: synchronized inputs pass"
write_synced
run_check
if [[ "${CHECK_STATUS}" -eq 0 ]]; then
    echo "✅ Success"
else
    echo "❌ Failure (synchronized files reported as stale)"
    exit 1
fi

echo "Test 2: stale llms.txt fails and is named"
write_synced
printf '%s\n' "- opentelemetry (0.27)" > "${TEST_DIR}/llms.txt"
run_check
if [[ "${CHECK_STATUS}" -eq 0 ]]; then
    echo "❌ Failure (stale llms.txt passed)"
    exit 1
fi
if [[ "${CHECK_OUTPUT}" == *"stale generated file: llms.txt"* ]] \
    && [[ "${CHECK_OUTPUT}" != *"stale generated file: llms-full.txt"* ]]; then
    echo "✅ Success (Correctly identified stale llms.txt)"
else
    echo "❌ Failure (stale llms.txt not named correctly)"
    exit 1
fi

echo "Test 3: stale llms-full.txt fails and is named"
write_synced
printf '%s\n' "- opentelemetry (0.27)" > "${TEST_DIR}/llms-full.txt"
run_check
if [[ "${CHECK_STATUS}" -eq 0 ]]; then
    echo "❌ Failure (stale llms-full.txt passed)"
    exit 1
fi
if [[ "${CHECK_OUTPUT}" == *"stale generated file: llms-full.txt"* ]] \
    && [[ "${CHECK_OUTPUT}" != *"stale generated file: llms.txt"* ]]; then
    echo "✅ Success (Correctly identified stale llms-full.txt)"
else
    echo "❌ Failure (stale llms-full.txt not named correctly)"
    exit 1
fi

echo "Test 4: missing input fails explicitly"
write_synced
rm "${TEST_DIR}/llms.txt"
run_check
if [[ "${CHECK_STATUS}" -eq 0 ]]; then
    echo "❌ Failure (missing llms.txt passed)"
    exit 1
fi
if [[ "${CHECK_OUTPUT}" == *"missing generated file: llms.txt"* ]]; then
    echo "✅ Success (Correctly identified missing llms.txt)"
else
    echo "❌ Failure (missing llms.txt not named correctly)"
    exit 1
fi

echo "Test 5: generator failure fails and propagates its exit code"
write_synced
run_check 1
if [[ "${CHECK_STATUS}" -eq 3 ]]; then
    echo "✅ Success (Correctly propagated generator failure)"
else
    echo "❌ Failure (generator failure not propagated, status=${CHECK_STATUS})"
    exit 1
fi
if [[ "${CHECK_OUTPUT}" == *"generator failed"* ]]; then
    echo "✅ Success (generator failure reported)"
else
    echo "❌ Failure (generator failure not reported)"
    exit 1
fi

echo "All tests passed!"
