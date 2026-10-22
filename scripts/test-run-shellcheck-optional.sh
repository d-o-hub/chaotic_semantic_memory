#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/run-shellcheck-optional.sh"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

mkdir -p "${TEST_DIR}/bin"
set +e
OUTPUT="$(CSM_ENABLE_SHELLCHECK=true PATH="${TEST_DIR}/bin" /usr/bin/bash "${SUBJECT}" ignored.sh 2>&1)"
STATUS=$?
set -e
if [[ ${STATUS} -eq 0 && "${OUTPUT}" == *"SKIPPED — NOT A PASS: ShellCheck was requested but is not installed"* ]]; then
    echo "PASS: missing optional ShellCheck is reported as an explicit non-pass"
else
    echo "FAIL: missing ShellCheck returned ${STATUS}: ${OUTPUT}" >&2
    exit 1
fi

OUTPUT="$(CSM_ENABLE_SHELLCHECK=false /usr/bin/bash "${SUBJECT}" ignored.sh)"
if [[ "${OUTPUT}" == *"SKIPPED — NOT A PASS: ShellCheck is opt-in"* ]]; then
    echo "PASS: disabled ShellCheck is clearly outside the required gate graph"
else
    echo "FAIL: disabled ShellCheck was not classified explicitly: ${OUTPUT}" >&2
    exit 1
fi
