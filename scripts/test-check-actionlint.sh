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
printf '%s\n' "$@" >"${ACTIONLINT_ARGS}"
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

# Exactly one suppression travels with the call, it is the documented
# `concurrency.queue` key actionlint cannot parse yet, and it stays narrow
# enough not to hide an unrelated unknown key.
if [[ "$(grep -cx -- "-ignore" "${TEST_DIR}/args" || true)" -ne 1 ]]; then
    echo "FAIL: expected exactly one -ignore suppression, got: $(grep -c -- "-ignore" "${TEST_DIR}/args" || true)" >&2
    exit 1
fi
PATTERN="$(awk '/^-ignore$/ { getline; print; exit }' "${TEST_DIR}/args")"
QUEUE_MESSAGE='unexpected key "queue" for "concurrency" section. expected one of "cancel-in-progress", "group" [syntax-check]'
if ! printf '%s\n' "${QUEUE_MESSAGE}" | grep -Eq -- "${PATTERN}"; then
    echo "FAIL: the suppression does not cover the documented queue key: ${PATTERN}" >&2
    exit 1
fi
for UNSCOPED in \
    'unexpected key "quueue" for "concurrency" section. expected one of "cancel-in-progress", "group" [syntax-check]' \
    'unexpected key "queue" for "jobs" section. expected one of "name", "runs-on" [syntax-check]'; do
    if printf '%s\n' "${UNSCOPED}" | grep -Eq -- "${PATTERN}"; then
        echo "FAIL: the suppression hides an unrelated finding: ${UNSCOPED}" >&2
        exit 1
    fi
done
echo "PASS: the one -ignore suppression covers only the documented queue key"
