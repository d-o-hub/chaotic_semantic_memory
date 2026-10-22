#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKFLOWS_DIR="${SCRIPT_DIR}/../.github/workflows"

if ! command -v actionlint >/dev/null 2>&1; then
    printf 'ERROR: actionlint is required by the validation gate but is not installed.\n' >&2
    exit 127
fi

shopt -s nullglob
workflows=("${WORKFLOWS_DIR}"/*.yml "${WORKFLOWS_DIR}"/*.yaml)
if ((${#workflows[@]} == 0)); then
    printf 'ERROR: no GitHub Actions workflow files found under %s.\n' "${WORKFLOWS_DIR}" >&2
    exit 1
fi

echo "==> actionlint (${#workflows[@]} workflows)"
# The repository's workflow scripts are linted separately when ShellCheck is
# explicitly enabled; keep actionlint responsible for workflow semantics here.
actionlint -shellcheck '' "${workflows[@]}"
