#!/usr/bin/env bash
# scripts/release-verdict.sh — issue #969.
#
# The `notify` job used to print "Release pipeline finished for v${VERSION}" no
# matter what its siblings did, so a partial release and a complete one were
# byte-identical in the summary. This script reads the sibling `needs` results
# explicitly and renders one of four verdicts:
#
#   published      every artifact landed and verify-release passed
#   nothing-to-do  the tag already exists; no publish was attempted
#   not-a-release  the trigger was not a successful CI push (no verdict)
#   partial        at least one publish/build artifact failed or was skipped
#
# It is diagnostic and never decides whether the workflow should fail —
# `verify-release` owns that — but a partial/unverified verdict emits an
# `::error` annotation so the run list and summary cannot read as routine.
#
# Inputs are environment variables (empty is rendered as "skipped"):
#   VERSION, EVENT_NAME, RELEASE_NEEDED, VALIDATE_RESULT, BUILD_CLI_RESULT,
#   BUILD_WASM_RESULT, NPM_WASM_RESULT, NPM_CLI_RESULT, RELEASE_RESULT, VERIFY_RESULT
# There is no crates-publish row here: crates.io is published by the dispatch-only
# publish-crates.yml (crates.io refuses OIDC tokens minted from release.yml's
# workflow_run trigger), so a crates gap reaches this verdict through VERIFY_RESULT,
# which still probes all nine index rows.
# Exit: 0 always except usage errors (2).
# Fixture: scripts/test-release-verdict.sh

set -euo pipefail

VERSION="${VERSION:-unknown}"
EVENT_NAME="${EVENT_NAME:-unknown}"
RELEASE_NEEDED="${RELEASE_NEEDED:-}"
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
        detail="trigger was \`${EVENT_NAME}\`, not a successful CI push; nothing was published or verified"
    else
        verdict="not-a-release"
        detail="validation finished \`${VALIDATE_RESULT}\` before any publication; nothing was published"
    fi
elif [[ "${RELEASE_NEEDED}" != "true" ]]; then
    verdict="nothing-to-do"
    detail="tag v${VERSION} already exists; no publish was attempted in this run"
else
    failed=()
    skipped=()
    for pair in \
        "build-cli-matrix:$(result "${BUILD_CLI_RESULT:-}")" \
        "build-wasm:$(result "${BUILD_WASM_RESULT:-}")" \
        "publish-npm-wasm:$(result "${NPM_WASM_RESULT:-}")" \
        "publish-npm-cli:$(result "${NPM_CLI_RESULT:-}")" \
        "create-github-release:$(result "${RELEASE_RESULT:-}")"; do
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
