#!/usr/bin/env bash
# Fixture for scripts/release-{verdict,no-verdict,cli-matrix}.sh (issue #969,
# #974). No network: every input is an environment variable.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT
PASS=0
FAIL=0

run() {
  OUT="$(env "$@" bash "${SCRIPT_DIR}/$1_SCRIPT" 2>&1)" && RC=0 || RC=$?
}

# Simpler explicit wrappers.
run_verdict() { OUT="$(env "$@" bash "${SCRIPT_DIR}/release-verdict.sh" 2>&1)" && RC=0 || RC=$?; }
run_no_verdict() { OUT="$(bash "${SCRIPT_DIR}/release-no-verdict.sh" "$@" 2>&1)" && RC=0 || RC=$?; }

expect() {
  local label="$1" expected="$2" needle="$3"
  if [[ "${RC}" -ne "${expected}" ]]; then
    echo "❌ ${label}: expected rc ${expected}, got ${RC}" >&2
    printf '%s\n' "${OUT}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
  elif ! grep -qF -- "${needle}" <<< "${OUT}"; then
    echo "❌ ${label}: output lacks '${needle}'" >&2
    printf '%s\n' "${OUT}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
  else
    PASS=$((PASS + 1))
  fi
}

expect_not() {
  local label="$1" needle="$2"
  if grep -qF -- "${needle}" <<< "${OUT}"; then
    echo "❌ ${label}: unexpected '${needle}'" >&2
    printf '%s\n' "${OUT}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
  else
    PASS=$((PASS + 1))
  fi
}

echo "Test 1: the CLI matrix is valid JSON and the workflow consumes it"
bash "${SCRIPT_DIR}/release-cli-matrix.sh" > "${TMP}/matrix.json"
if jq -e 'length == 5 and ([.[] | "\(.platform)-\(.arch)"] | unique | length == 5)' "${TMP}/matrix.json" >/dev/null; then
  PASS=$((PASS + 1)); echo "✅ matrix has 5 unique platform-arch entries"
else
  FAIL=$((FAIL + 1)); echo "❌ matrix shape is wrong" >&2
fi
if grep -q 'fromJSON(needs.validate.outputs.cli-matrix)' "${SCRIPT_DIR}/../.github/workflows/release.yml" \
  && grep -q 'release-cli-matrix.sh' "${SCRIPT_DIR}/../.github/workflows/release.yml"; then
  PASS=$((PASS + 1)); echo "✅ release.yml consumes the shared matrix"
else
  FAIL=$((FAIL + 1)); echo "❌ release.yml still hardcodes the CLI matrix" >&2
fi

echo "Test 2: a failed CI trigger is a warning, not a green release"
run_no_verdict workflow_run push failure
expect "no-verdict warning" 0 "::warning title=Release::No verdict"
expect "summary says not a verdict" 0 "not a release verdict"
expect_not "no false success claim" "Release pipeline finished"

echo "Test 3: a successful push with no validation is an inconsistency"
run_no_verdict workflow_run push success
expect "contradiction exits 1" 1 "::error title=Release::"
expect "summary names the inconsistency" 1 "not a release"

echo "Test 4: a manual dispatch without a validated release is a no-verdict"
run_no_verdict workflow_dispatch n/a n/a
expect "dispatch no-verdict" 0 "manual dispatch"

echo "Test 5: no-verdict usage error"
run_no_verdict
expect "missing event is usage error" 2 "usage:"

echo "Test 6: validate skipped is not-a-release"
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=skipped RELEASE_NEEDED=
expect "not-a-release" 0 "Verdict: not-a-release"
expect "warning annotation" 0 "::warning title=Release::not-a-release"

echo "Test 7: validate failure is not-a-release"
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=failure RELEASE_NEEDED=
expect "validation failed" 0 "validation finished \`failure\`"

echo "Test 8: an existing tag is nothing-to-do"
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=success RELEASE_NEEDED=false
expect "nothing-to-do" 0 "Verdict: nothing-to-do"
expect "tag already exists" 0 "tag v0.3.9 already exists"
expect_not "nothing-to-do is not an error" "::error"

echo "Test 9: a complete release is published"
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=success RELEASE_NEEDED=true \
  BUILD_CLI_RESULT=success BUILD_WASM_RESULT=success \
  NPM_WASM_RESULT=success NPM_CLI_RESULT=success RELEASE_RESULT=success \
  CRATES_TRIGGER_RESULT=success VERIFY_RESULT=success
expect "published" 0 "Verdict: published"
expect "artifact counts" 0 "9 crates, 2 npm packages, and 7 release assets"
expect_not "published is not an error" "::error"

echo "Test 10: an unproven crates.io is caught by the verifier even when the trigger passed"
# trigger-crates-io-publish dispatches publish-crates.yml and waits, so a crates gap can
# arrive two ways: the sibling job went red (Test 14) or the spawned run said success and
# the index disagrees. Both must refuse the published verdict, and the second is why
# verify-release still probes nine rows instead of trusting the sibling's exit code.
# Asserting the old job name is absent is what keeps a resurrected `publish-crates` row
# from passing as evidence.
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=success RELEASE_NEEDED=true \
  BUILD_CLI_RESULT=success BUILD_WASM_RESULT=success \
  NPM_WASM_RESULT=success NPM_CLI_RESULT=success RELEASE_RESULT=success \
  CRATES_TRIGGER_RESULT=success VERIFY_RESULT=failure
expect "partial" 0 "Verdict: partial"
expect "the verifier owns the gap" 0 "the release is unverified"
expect "error annotation" 0 "::error title=Release::partial"
expect_not "no publish-crates job row remains" "publish-crates:"

echo "Test 11: a verifier that never ran is unverified, not published"
# Distinct from Test 10 by branch, not by wording: `failure` means someone checked and
# the release is short, `skipped` means nothing was ever checked. Both must refuse the
# published verdict, and the second is what a release run looks like when a publish job
# reds and verify-release never gets a chance to run.
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=success RELEASE_NEEDED=true \
  BUILD_CLI_RESULT=success BUILD_WASM_RESULT=success \
  NPM_WASM_RESULT=success NPM_CLI_RESULT=success RELEASE_RESULT=success \
  CRATES_TRIGGER_RESULT=success VERIFY_RESULT=skipped
expect "unverified" 0 "the release is unverified"
expect_not "a skipped verifier never says published" "Verdict: published"

echo "Test 12: a skipped publish job is incomplete"
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=success RELEASE_NEEDED=true \
  BUILD_CLI_RESULT=success BUILD_WASM_RESULT=success \
  NPM_WASM_RESULT=success NPM_CLI_RESULT=skipped RELEASE_RESULT=success \
  CRATES_TRIGGER_RESULT=success VERIFY_RESULT=success
expect "skipped job named" 0 "skipped: publish-npm-cli"

echo "Test 13: a cancelled build is partial"
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=success RELEASE_NEEDED=true \
  BUILD_CLI_RESULT=cancelled BUILD_WASM_RESULT=success \
  NPM_WASM_RESULT=skipped NPM_CLI_RESULT=skipped RELEASE_RESULT=skipped \
  CRATES_TRIGGER_RESULT=success VERIFY_RESULT=skipped
expect "cancelled build named" 0 "build-cli-matrix (cancelled)"

echo "Test 14: the crates.io dispatch job failing is named, not folded into a verify miss"
# The trigger job is the only place a release learns the spawned publish run went wrong,
# so the verdict must say which job to open. If it only showed up as "unverified", the
# reader would chase the npm/asset probes that all passed.
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=success RELEASE_NEEDED=true \
  BUILD_CLI_RESULT=success BUILD_WASM_RESULT=success \
  NPM_WASM_RESULT=success NPM_CLI_RESULT=success RELEASE_RESULT=success \
  CRATES_TRIGGER_RESULT=failure VERIFY_RESULT=failure
expect "trigger failure named" 0 "trigger-crates-io-publish (failure)"
expect "still partial" 0 "Verdict: partial"
expect_not "a red trigger never reads as published" "Verdict: published"

echo "Test 15: a crates.io dispatch that never ran is incomplete"
run_verdict VERSION=0.3.9 EVENT_NAME=workflow_run VALIDATE_RESULT=success RELEASE_NEEDED=true \
  BUILD_CLI_RESULT=success BUILD_WASM_RESULT=success \
  NPM_WASM_RESULT=success NPM_CLI_RESULT=success RELEASE_RESULT=success \
  CRATES_TRIGGER_RESULT=skipped VERIFY_RESULT=success
expect "skipped trigger named" 0 "skipped: trigger-crates-io-publish"
expect_not "an undispatched publish never reads as published" "Verdict: published"

echo "release-verdict fixture: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
