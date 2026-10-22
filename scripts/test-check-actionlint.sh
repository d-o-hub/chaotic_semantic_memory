#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/check-actionlint.sh"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

# A required workflow linter must fail closed when absent.
set +e
OUTPUT="$(PATH=/usr/bin:/bin bash "${SUBJECT}" 2>&1)"
STATUS=$?
set -e
if [[ ${STATUS} -eq 127 && "${OUTPUT}" == *"actionlint is required"* ]]; then
    echo "PASS: missing actionlint fails the required gate"
else
    echo "FAIL: missing actionlint returned ${STATUS}: ${OUTPUT}" >&2
    exit 1
fi

# A supplied binary is invoked on the real workflow files with shell parsing
# disabled explicitly; ShellCheck remains a separately classified diagnostic.
mkdir -p "${TEST_DIR}/bin"
cat >"${TEST_DIR}/bin/actionlint" <<'MOCK'
#!/usr/bin/env bash
printf '%s\n' "$*" >"${ACTIONLINT_ARGS}"
MOCK
chmod +x "${TEST_DIR}/bin/actionlint"
ACTIONLINT_ARGS="${TEST_DIR}/args" PATH="${TEST_DIR}/bin:${PATH}" \
    bash "${SUBJECT}"
if [[ ! -s "${TEST_DIR}/args" ]] \
    || ! grep -q -- "-shellcheck" "${TEST_DIR}/args" \
    || ! grep -q -- "\.github/workflows/" "${TEST_DIR}/args"; then
    echo "FAIL: actionlint was not called with the expected workflow arguments" >&2
    exit 1
fi
echo "PASS: actionlint receives the workflow set and explicit ShellCheck policy"
