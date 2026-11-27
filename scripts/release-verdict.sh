#!/usr/bin/env bash
# scripts/release-verdict.sh — issue #969.
#
# The `notify` job used to print "Release pipeline finished for v${VERSION}" no
# matter what its siblings did, so a partial release and a complete one were
# byte-identical in the summary. This script reads the sibling `needs` results
# explicitly and renders one of five verdicts:
#
#   published      every artifact landed and verify-release passed
#   dry-run        an untagged dispatch: built and gated only, published nothing
#   nothing-to-do  no publication was attempted in this run
#   not-a-release  validation did not succeed, so nothing was published
#   partial        at least one publish/build artifact failed or was skipped
#
# It is diagnostic and never decides whether the workflow should fail —
# `verify-release` owns that — but a partial/unverified verdict emits an
# `::error` annotation so the run list and summary cannot read as routine.
#
# Inputs are environment variables (empty is rendered as "skipped"):
#   VERSION, EVENT_NAME, RELEASE_NEEDED, DRY_RUN, VALIDATE_RESULT,
#   BUILD_CLI_RESULT, BUILD_WASM_RESULT, NPM_WASM_RESULT, NPM_CLI_RESULT,
#   RELEASE_RESULT, CRATES_TRIGGER_RESULT, VERIFY_RESULT, WINDOWS_NPM_RESULT
# CRATES_TRIGGER_RESULT is `trigger-crates-io-publish`, the job that dispatches
# publish-crates.yml and waits for it. It is reported separately from VERIFY_RESULT on
# purpose: crates.io publishing lives in another workflow (ADR-0104), so its failure is
# a named job, not a mysterious index miss. VERIFY_RESULT still probes all nine rows, so
# a crates gap is caught even when the trigger job never ran.
# Since the tag-push port, `release.yml` runs on `v*` tag pushes and dispatches; the
# trigger that used to produce "not-a-release" (a skipped `validate` under the old
# workflow_run trigger) can no longer occur, and the untagged-dispatch dry run gets its
# own verdict instead of reading as "nothing to do".
# Exit: 0 always except usage errors (2).
# Fixture: scripts/test-release-verdict.sh

set -euo pipefail

VERSION="${VERSION:-unknown}"
EVENT_NAME="${EVENT_NAME:-unknown}"
RELEASE_NEEDED="${RELEASE_NEEDED:-}"
DRY_RUN="${DRY_RUN:-}"
VALIDATE_RESULT="${VALIDATE_RESULT:-skipped}"
VERIFY_RESULT="${VERIFY_RESULT:-skipped}"

result() {
    local value="${1:-skipped}"
    printf '%s' "${value}"
}

verdict=""
detail=""
annotation="info"

if [[ "${VALIDATE_RESULT}" != "success" ]]; then
    annotation="warning"
    if [[ "${VALIDATE_RESULT}" == "skipped" ]]; then
        verdict="not-a-release"
        detail="validation was skipped for \`${EVENT_NAME}\`; nothing was published or verified"
    else
        verdict="not-a-release"
        detail="validation finished \`${VALIDATE_RESULT}\` before any publication; nothing was published"
    fi
elif [[ "${DRY_RUN}" == "true" ]]; then
    verdict="dry-run"
    detail="untagged dispatch: built and gated v${VERSION} without publishing (supply a tag to publish)"
elif [[ "${RELEASE_NEEDED}" != "true" ]]; then
    verdict="nothing-to-do"
    detail="no publish was attempted in this run"
else
    failed=()
    skipped=()
    for pair in \
        "build-cli-matrix:$(result "${BUILD_CLI_RESULT:-}")" \
        "build-wasm:$(result "${BUILD_WASM_RESULT:-}")" \
        "publish-npm-wasm:$(result "${NPM_WASM_RESULT:-}")" \
        "publish-npm-cli:$(result "${NPM_CLI_RESULT:-}")" \
        "trigger-crates-io-publish:$(result "${CRATES_TRIGGER_RESULT:-}")" \
        "create-github-release:$(result "${RELEASE_RESULT:-}")" \
        "verify-windows-npm:$(result "${WINDOWS_NPM_RESULT:-}")"; do
        job="${pair%%:*}"
        state="${pair#*:}"
        case "${state}" in
            failure|cancelled|timed_out) failed+=("${job} (${state})") ;;
            skipped) skipped+=("${job}") ;;
        esac
    done
    if [[ "${#failed[@]}" -gt 0 ]]; then
        verdict="partial"
        detail="failed: ${failed[*]}"
        annotation="error"
    elif [[ "${VERIFY_RESULT}" != "success" ]]; then
        verdict="partial"
        detail="verify-release finished \`${VERIFY_RESULT}\`; the release is unverified"
        annotation="error"
    elif [[ "${#skipped[@]}" -gt 0 ]]; then
        verdict="partial"
        detail="skipped: ${skipped[*]}"
        annotation="error"
    else
        verdict="published"
        detail="v${VERSION} verified: 9 crates, 2 npm packages, and 7 release assets"
    fi
fi

echo "## Release Summary"
echo ""
echo "**Verdict: ${verdict}** — ${detail}"
echo ""
case "${annotation}" in
    error) printf '::error title=Release::%s — %s\n' "${verdict}" "${detail}" ;;
    warning) printf '::warning title=Release::%s — %s\n' "${verdict}" "${detail}" ;;
    *) ;;
esac
