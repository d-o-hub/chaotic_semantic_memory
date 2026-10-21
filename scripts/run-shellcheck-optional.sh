#!/usr/bin/env bash
set -euo pipefail

if [[ "${CSM_ENABLE_SHELLCHECK:-false}" != "true" ]]; then
    echo "SKIPPED — NOT A PASS: ShellCheck is opt-in and outside the required gate graph"
    exit 0
fi

if ! command -v shellcheck >/dev/null 2>&1; then
    echo "SKIPPED — NOT A PASS: ShellCheck was requested but is not installed"
    exit 0
fi

if (($# == 0)); then
    echo "ERROR: ShellCheck was enabled but no scripts were supplied" >&2
    exit 2
fi

shellcheck --severity=error "$@"
