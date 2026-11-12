#!/usr/bin/env bash
# test-validate-github-actions-shas.sh — behavior fixtures for the workflow SHA gate.
#
# The subject derives its repository root from its own path. Copy it into a
# throwaway tree so recursive discovery, API classification, and required-mode
# behavior are exercised without touching the real workflow files or network.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/validate-github-actions-shas.sh"
ORIG_PATH="${PATH}"
GOOD_SHA="3d3c42e5aac5ba805825da76410c181273ba90b1"
BAD_SHA="cf430e030ddbb5b0abf93d22962f4752f3646cd9"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

mkdir -p "${TEST_DIR}/scripts" "${TEST_DIR}/bin" "${TEST_DIR}/no-gh"
cp "${SUBJECT}" "${TEST_DIR}/scripts/validate-github-actions-shas.sh"
GH_CALL_LOG="${TEST_DIR}/gh-calls.log"
: > "${GH_CALL_LOG}"

# The subject needs only these tools when gh is deliberately absent.
for tool in awk bash basename cat dirname find grep sed sort tr wc; do
    ln -s "$(command -v "${tool}")" "${TEST_DIR}/no-gh/${tool}"
done

cat > "${TEST_DIR}/bin/gh" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail

endpoint=""
for argument in "$@"; do
    if [[ "${argument}" == repos/*/commits/* ]]; then
        endpoint="${argument}"
    fi
done
printf '%s\n' "${endpoint:-<missing-endpoint>}" >> "${CSM_GH_LOG:?}"
if [[ -z "${endpoint}" ]]; then
    echo "fixture gh received no commit endpoint" >&2
    exit 9
fi

case "${CSM_GH_MODE:-success}" in
    network)
        echo "gh: API request failed: network unreachable" >&2
        exit 1
        ;;
    not-found)
        printf 'HTTP/2.0 404 Not Found\n\n{"message":"Not Found"}\n'
        echo "gh: Not Found (HTTP 404)" >&2
        exit 1
        ;;
    rate-limit)
        printf 'HTTP/2.0 429 Too Many Requests\n\n{"message":"rate limited"}\n'
        echo "gh: rate limited (HTTP 429)" >&2
        exit 1
        ;;
    ambiguous-422)
        printf 'HTTP/2.0 422 Unprocessable Entity\n\nValidation Failed\n'
        echo "gh: Validation Failed (HTTP 422)" >&2
        exit 1
        ;;
esac

case "${endpoint}" in
    "repos/actions/checkout/commits/${CSM_TEST_GOOD_SHA:?}"|\
    "repos/actions/tooling/commits/${CSM_TEST_GOOD_SHA:?}")
        printf 'HTTP/2.0 200 OK\n\n%s\n' "${CSM_TEST_GOOD_SHA}"
        ;;
    "repos/actions/download-artifact/commits/${CSM_TEST_BAD_SHA:?}")
        printf 'HTTP/2.0 422 Unprocessable Entity\n\nNo commit found for SHA: %s\n' \
            "${CSM_TEST_BAD_SHA}"
        printf 'gh: No commit found for SHA: %s (HTTP 422)\n' \
            "${CSM_TEST_BAD_SHA}" >&2
        exit 1
        ;;
    *)
        printf 'HTTP/2.0 404 Not Found\n\nNot Found\n'
        echo "gh: Not Found (HTTP 404)" >&2
        exit 1
        ;;
esac
STUB
chmod 755 "${TEST_DIR}/bin/gh"

GH_PATH="${TEST_DIR}/bin:${ORIG_PATH}"
NO_GH_PATH="${TEST_DIR}/no-gh"
RUNS=0
FAILURES=0
RUN_STATUS=0
RUN_OUTPUT=""

ok() {
    RUNS=$((RUNS + 1))
    printf 'PASS: %s\n' "$1"
}

ko() {
    RUNS=$((RUNS + 1))
    FAILURES=$((FAILURES + 1))
    printf 'FAIL: %s (exit %s)\n' "$1" "${RUN_STATUS}"
    printf '%s\n' "${RUN_OUTPUT}"
}

reset_workflows() {
    rm -rf "${TEST_DIR}/.github/workflows"
    mkdir -p "${TEST_DIR}/.github/workflows"
    : > "${GH_CALL_LOG}"
}

run_validator() {
    local path="$1"
    local mode="$2"
    local required="$3"
    shift 3
    : > "${GH_CALL_LOG}"
    RUN_STATUS=0
    RUN_OUTPUT="$(
        CSM_GH_LOG="${GH_CALL_LOG}" \
        CSM_GH_MODE="${mode}" \
        CSM_TEST_GOOD_SHA="${GOOD_SHA}" \
        CSM_TEST_BAD_SHA="${BAD_SHA}" \
        CSM_VALIDATE_GITHUB_ACTIONS_SHAS_REQUIRED="${required}" \
        PATH="${path}" \
            bash "${TEST_DIR}/scripts/validate-github-actions-shas.sh" "$@" 2>&1
    )" || RUN_STATUS=$?
}

# Recursive .yml/.yaml discovery; a nested action subpath must query its root
# repository, while local and Docker actions remain outside upstream SHA checks.
reset_workflows
cat > "${TEST_DIR}/.github/workflows/ci.yml" <<YAML
name: ci
jobs:
  build:
    steps:
      - uses: actions/checkout@${GOOD_SHA} # v4.1.7
      - uses: ./actions/local
      - uses: docker://alpine:3.20
YAML
mkdir -p "${TEST_DIR}/.github/workflows/release/nested" \
    "${TEST_DIR}/.github/workflows/fuzz"
cat > "${TEST_DIR}/.github/workflows/release/nested/release.yaml" <<YAML
name: release
jobs:
  release:
    steps:
      - uses: actions/tooling/path/to/action.yml@${GOOD_SHA} # v1
YAML
cat > "${TEST_DIR}/.github/workflows/fuzz/fuzz.yml" <<YAML
name: fuzz
jobs:
  fuzz:
    steps:
      - uses: actions/checkout@${GOOD_SHA} # v4.1.7
YAML
run_validator "${GH_PATH}" success false
if [[ "${RUN_STATUS}" -eq 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"Workflow files: 3"* ]] \
    && [[ "${RUN_OUTPUT}" == *"Actions checked: 3"* ]] \
    && [[ "${RUN_OUTPUT}" == *"PASSED:"* ]] \
    && grep -Fxq "repos/actions/tooling/commits/${GOOD_SHA}" "${GH_CALL_LOG}" \
    && [[ "$(grep -Fxc "repos/actions/checkout/commits/${GOOD_SHA}" "${GH_CALL_LOG}")" -eq 1 ]]; then
    ok "recursive .yml/.yaml scan verifies nested action paths against owner/repo"
else
    ko "recursive workflow scan or nested action repository resolution failed"
fi

# A definitive 422 response naming the missing SHA is a real pin error.
reset_workflows
cat > "${TEST_DIR}/.github/workflows/bogus.yml" <<YAML
name: bogus
jobs:
  test:
    steps:
      - uses: actions/download-artifact@${BAD_SHA} # v7.0.2
YAML
run_validator "${GH_PATH}" success false
if [[ "${RUN_STATUS}" -ne 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"does not exist in actions/download-artifact"* ]] \
    && [[ "${RUN_OUTPUT}" == *"Errors: 1"* ]] \
    && [[ "${RUN_OUTPUT}" != *"SKIP — NOT A PASS"* ]]; then
    ok "GitHub's explicit no-commit 422 fails the bogus pin"
else
    ko "bogus pin was not classified as definitively invalid"
fi

# 404s, rate limits, other 422s, and transport errors are unverified—not proof
# that a SHA is invalid. They are nonfatal locally and fatal in required mode.
reset_workflows
cat > "${TEST_DIR}/.github/workflows/api.yml" <<YAML
name: api
jobs:
  test:
    steps:
      - uses: actions/checkout@${GOOD_SHA} # v4.1.7
YAML
for mode in not-found rate-limit ambiguous-422 network; do
    run_validator "${GH_PATH}" "${mode}" false
    if [[ "${RUN_STATUS}" -eq 0 ]] \
        && [[ "${RUN_OUTPUT}" == *"SKIP — NOT A PASS"* ]] \
        && [[ "${RUN_OUTPUT}" == *"Unverified: 1"* ]] \
        && [[ "${RUN_OUTPUT}" != *"does not exist"* ]]; then
        ok "${mode} API failure is reported as an optional unverified skip"
    else
        ko "${mode} API failure was not kept distinct from an invalid SHA"
    fi

    run_validator "${GH_PATH}" "${mode}" true
    if [[ "${RUN_STATUS}" -ne 0 ]] \
        && [[ "${RUN_OUTPUT}" == *"SKIP — NOT A PASS"* ]] \
        && [[ "${RUN_OUTPUT}" == *"CSM_VALIDATE_GITHUB_ACTIONS_SHAS_REQUIRED=true"* ]]; then
        ok "${mode} API failure fails required verification"
    else
        ko "${mode} API failure did not fail required verification"
    fi
done

# Repeated immutable pins are queried once. After a rate limit, no further API
# requests are sent in the same run; remaining refs stay explicitly unverified.
reset_workflows
cat > "${TEST_DIR}/.github/workflows/rate-limit.yml" <<YAML
name: rate-limit
jobs:
  test:
    steps:
      - uses: actions/checkout@${GOOD_SHA} # v4.1.7
      - uses: actions/tooling/path/to/action.yml@${GOOD_SHA} # v1
YAML
run_validator "${GH_PATH}" rate-limit false
if [[ "${RUN_STATUS}" -eq 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"SKIP — NOT A PASS"* ]] \
    && [[ "${RUN_OUTPUT}" == *"Unverified: 2"* ]] \
    && [[ "$(wc -l < "${GH_CALL_LOG}")" -eq 1 ]]; then
    ok "duplicate refs are cached and rate limiting stops further API calls"
else
    ko "duplicate API calls or continued requests after a rate limit"
fi

# Missing gh and explicit offline mode have the same skip-vs-required contract.
run_validator "${NO_GH_PATH}" success false
if [[ "${RUN_STATUS}" -eq 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"SKIP — NOT A PASS"* ]] \
    && [[ "${RUN_OUTPUT}" == *"gh CLI is unavailable"* ]]; then
    ok "missing gh is a nonfatal, explicit local skip"
else
    ko "missing gh was not reported as an unverified local skip"
fi

run_validator "${NO_GH_PATH}" success true
if [[ "${RUN_STATUS}" -ne 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"SKIP — NOT A PASS"* ]]; then
    ok "missing gh fails required verification"
else
    ko "missing gh did not fail required verification"
fi

run_validator "${GH_PATH}" success false --offline
if [[ "${RUN_STATUS}" -eq 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"SKIP — NOT A PASS"* ]] \
    && [[ ! -s "${GH_CALL_LOG}" ]]; then
    ok "offline mode reports a skip without calling the API"
else
    ko "offline mode did not remain an explicit API-free skip"
fi

run_validator "${GH_PATH}" success true --offline
if [[ "${RUN_STATUS}" -ne 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"SKIP — NOT A PASS"* ]]; then
    ok "offline mode fails when online verification is required"
else
    ko "offline mode passed despite required online verification"
fi

# Remote refs without a literal SHA must not disappear through the parser.
reset_workflows
cat > "${TEST_DIR}/.github/workflows/unpinned.yml" <<'YAML'
name: unpinned
jobs:
  test:
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact
      - uses: ${{ inputs.action }}
YAML
run_validator "${GH_PATH}" success false --offline
if [[ "${RUN_STATUS}" -ne 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"uses version tag"* ]] \
    && [[ "${RUN_OUTPUT}" == *"has no @ref"* ]] \
    && [[ "${RUN_OUTPUT}" == *"dynamic action reference cannot be verified"* ]] \
    && [[ "${RUN_OUTPUT}" == *"Errors: 3"* ]]; then
    ok "tag, missing-ref, and dynamic remote uses are rejected"
else
    ko "an unpinned remote action was ignored or misclassified"
fi

# A required scan of an empty workflow directory cannot be treated as a pass.
reset_workflows
run_validator "${GH_PATH}" success true
if [[ "${RUN_STATUS}" -ne 0 ]] \
    && [[ "${RUN_OUTPUT}" == *"SKIP — NOT A PASS"* ]]; then
    ok "required mode fails when no workflow files were scanned"
else
    ko "required mode accepted an empty workflow inventory"
fi

printf '\nResults: %s passed, %s failed\n' "$((RUNS - FAILURES))" "${FAILURES}"
if [[ "${FAILURES}" -gt 0 ]]; then
    exit 1
fi
