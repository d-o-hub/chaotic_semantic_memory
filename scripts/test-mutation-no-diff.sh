#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/mutation_test.sh"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT
mkdir -p "${TEST_DIR}/bin" "${TEST_DIR}/output"

cat >"${TEST_DIR}/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
if [[ "${1:-}" == "mutants" && "${2:-}" == "--version" ]]; then
    echo "cargo-mutants mock"
    exit 0
fi
if [[ "${1:-}" == "mutants" && "${2:-}" == "--help" ]]; then
    echo "--in-diff"
    exit 0
fi
if [[ "${1:-}" == "mutants" && "${2:-}" == "--list" ]]; then
    exit 0
fi
echo "unexpected cargo command: $*" >&2
exit 2
MOCK
cat >"${TEST_DIR}/bin/git" <<'MOCK'
#!/usr/bin/env bash
if [[ "${1:-}" == "rev-parse" ]]; then
    if [[ "${FAKE_GIT_MODE:-empty}" == "diff" ]]; then
        echo "deadbeef"
        exit 0
    fi
    exit 1
fi
if [[ "${1:-}" == "diff" ]]; then
    if [[ "${FAKE_GIT_MODE:-empty}" == "diff" ]]; then
        printf '%s\n' 'diff --git a/src/example.rs b/src/example.rs' '+fn changed() {}'
    fi
    exit 0
fi
exec /usr/bin/git "$@"
MOCK
chmod +x "${TEST_DIR}/bin/cargo" "${TEST_DIR}/bin/git"

run_mutation() {
    local mode="$1"
    local expected_status="$2"
    local output_file="${TEST_DIR}/${mode}.out"
    local status=0
    (
        cd "${TEST_DIR}"
        FAKE_GIT_MODE="${mode}" \
            MUTATION_OUTPUT_DIR="${TEST_DIR}/output/${mode}" \
            DIFF_TARGET=origin/main \
            PATH="${TEST_DIR}/bin:${PATH}" \
            bash "${SUBJECT}" --ci fast
    ) >"${output_file}" 2>&1 || status=$?
    if [[ ${status} -ne ${expected_status} ]]; then
        echo "FAIL: ${mode} returned ${status}, expected ${expected_status}" >&2
        cat "${output_file}" >&2
        exit 1
    fi
}

run_mutation empty 2
if ! grep -q "no bounded diff.*mutation tests did not run" "${TEST_DIR}/empty.out" \
    || ! grep -q "not run (no bounded diff" "${TEST_DIR}/output/empty/fast-latest.md"; then
    echo "FAIL: an unbounded CI run was not recorded as not run" >&2
    exit 1
fi
echo "PASS: no bounded CI diff fails without claiming mutation success"

run_mutation diff 1
if ! grep -q "generated no mutants" "${TEST_DIR}/diff.out" \
    || ! grep -q "not run (no mutants in diff)" "${TEST_DIR}/output/diff/fast-latest.md"; then
    echo "FAIL: zero mutants were not recorded as a failed evidence gap" >&2
    exit 1
fi
echo "PASS: zero mutants in a selected CI diff is not a passing mutation result"
