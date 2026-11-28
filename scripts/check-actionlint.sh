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
# `concurrency.queue` is a documented GitHub Actions key (`single` | `max`;
# workflow syntax "Example: queueing multiple pending runs") that actionlint
# has not implemented yet (rhysd/actionlint#654, #657 -- still open at
# v1.7.12), so it rejects `queue: max` in release.yml as an unknown key. The
# suppression is the one exact message: a typo'd key in that same section, or
# any other `unexpected key` finding, still fails this gate.
QUEUE_KEY_IGNORE='^unexpected key "queue" for "concurrency" section'
# The repository's workflow scripts are linted separately when ShellCheck is
# explicitly enabled; keep actionlint responsible for workflow semantics here.
actionlint -shellcheck '' -ignore "${QUEUE_KEY_IGNORE}" "${workflows[@]}"
