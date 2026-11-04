#!/usr/bin/env bash
# scripts/release-no-verdict.sh — issue #969.
#
# `release.yml` triggers on CI completion, but `validate` only runs for a
# successful `push` to this repository. When the triggering CI run failed (an
# infra fault, a flaky job), every dependent job skipped and only `notify`
# (`if: always()`) executed — GitHub rendered that as a green Release run that
# published nothing. This script makes the no-verdict case explicit: a warning
# and a summary line that say what the trigger was.
#
# Usage: release-no-verdict.sh <event-name> <ci-event> <ci-conclusion>
# Exit: 0 for a genuine no-verdict trigger, 1 for the contradictory case
#       (CI push/success but validate did not succeed), 2 usage error.
# Fixture: scripts/test-release-verdict.sh

set -euo pipefail

EVENT_NAME="${1:-}"
CI_EVENT="${2:-unknown}"
CI_CONCLUSION="${3:-unknown}"
if [[ -z "${EVENT_NAME}" ]]; then
    echo "usage: release-no-verdict.sh <event-name> <ci-event> <ci-conclusion>" >&2
    exit 2
fi

if [[ "${EVENT_NAME}" == "workflow_dispatch" ]]; then
    DETAIL="a manual dispatch whose validation did not succeed"
elif [[ "${CI_EVENT}" == "push" && "${CI_CONCLUSION}" == "success" ]]; then
    printf '::error title=Release::CI push/success did not produce a validated release; this run is incomplete\n'
    echo "## Release: no verdict"
    echo ""
    echo "This is **not a release**: the triggering CI run was a successful push, so validation should have run and did not. Treat this as an internal inconsistency, not as 'nothing to do'."
    exit 1
elif [[ "${CI_EVENT}" == "push" ]]; then
    DETAIL="the triggering CI run concluded \`${CI_CONCLUSION}\`, not \`success\`"
else
    DETAIL="the trigger was \`${CI_EVENT}/${CI_CONCLUSION}\`, not a successful CI push"
fi

printf '::warning title=Release::No verdict — %s; nothing was published or verified\n' "${DETAIL}"
echo "## Release: no verdict"
echo ""
echo "This run is **not a release verdict**: ${DETAIL}. No tag was created, nothing was published, and nothing was verified."
