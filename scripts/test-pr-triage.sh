#!/usr/bin/env bash
# Fixture for scripts/pr-triage.sh (issue #993).
#
# Every gh response is a local JSON file: the point is to prove that failed,
# empty, skipped, unknown, cancelled and head-changing responses can never be
# rendered as green, and that exactly one check-inventory request runs per PR.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/pr-triage.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT
BIN="${TMP}/bin"
SCN="${TMP}/scenario"
mkdir -p "${BIN}" "${SCN}"
PASS=0
FAIL=0

cat > "${BIN}/gh" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
echo "$*" >> "${FAKE_GH_DIR}/calls.log"
case "${1:-} ${2:-}" in
  "pr list")
    cat "${FAKE_GH_DIR}/list.json"
    ;;
  "pr checks")
    rc=0
    [[ -f "${FAKE_GH_DIR}/checks.rc" ]] && rc="$(cat "${FAKE_GH_DIR}/checks.rc")"
    cat "${FAKE_GH_DIR}/checks.json"
    if [[ "${rc}" -ne 0 ]]; then
      echo "${FAKE_GH_STDERR:-fake gh failure}" >&2
    fi
    exit "${rc}"
    ;;
  "pr view")
    cat "${FAKE_GH_DIR}/view.json"
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 99
    ;;
esac
FAKE
chmod +x "${BIN}/gh"

reset_scenario() {
  rm -f "${SCN}/calls.log" "${SCN}/checks.rc"
  printf '%s\n' '[{"number":1,"title":"test PR","mergeable":"MERGEABLE","headRefName":"b","headRefOid":"aaa"}]' > "${SCN}/list.json"
  printf '%s\n' '{"headRefOid":"aaa"}' > "${SCN}/view.json"
}

set_checks() {
  printf '%s\n' "$1" > "${SCN}/checks.json"
  if [[ -n "${2:-}" ]]; then printf '%s\n' "$2" > "${SCN}/checks.rc"; fi
}

run_triage() {
  OUT="$(PATH="${BIN}:${PATH}" FAKE_GH_DIR="${SCN}" bash "${SUBJECT}" "$@" 2>&1)" && RC=0 || RC=$?
}

expect_case() {
  local label="$1" expected_rc="$2" needle="$3"
  if [[ "${RC}" -ne "${expected_rc}" ]]; then
    echo "❌ ${label}: expected rc ${expected_rc}, got ${RC}" >&2
    printf '%s\n' "${OUT}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
    return
  fi
  if ! grep -qF -- "${needle}" <<< "${OUT}"; then
    echo "❌ ${label}: output lacks '${needle}'" >&2
    printf '%s\n' "${OUT}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
    return
  fi
  PASS=$((PASS + 1))
}

echo "Test 1: all checks pass"
reset_scenario
set_checks '[{"name":"lint","state":"SUCCESS","link":"u1"},{"name":"test","state":"SUCCESS","link":"u2"}]'
run_triage
expect_case "PASS verdict" 0 "PASS"
expect_case "inventory caveat" 0 "check inventory only"
single_call_count="$(grep -c '^pr checks' "${SCN}/calls.log")"
run_triage --strict
expect_case "strict exit 0 on PASS" 0 "PASS"
if [[ "${single_call_count}" -eq 1 ]]; then
  PASS=$((PASS + 1))
else
  echo "❌ PASS case did not issue exactly one gh pr checks call" >&2
  FAIL=$((FAIL + 1))
fi

echo "Test 2: a failing check is FAIL, and non-strict still exits 0"
reset_scenario
set_checks '[{"name":"lint","state":"SUCCESS","link":"u1"},{"name":"test","state":"FAILURE","link":"u2"}]'
run_triage
expect_case "FAIL verdict" 0 "FAIL"
run_triage --strict
expect_case "strict exit 1 on FAIL" 1 "FAIL"

echo "Test 3: queued/pending checks are PENDING, never PASS"
reset_scenario
set_checks '[{"name":"lint","state":"QUEUED","link":"u1"},{"name":"test","state":"IN_PROGRESS","link":"u2"}]'
run_triage --strict
expect_case "PENDING verdict" 1 "PENDING"

echo "Test 4: only skipped/neutral checks are NO_VERDICT, never PASS"
reset_scenario
set_checks '[{"name":"lint","state":"SKIPPED","link":"u1"},{"name":"test","state":"NEUTRAL","link":"u2"}]'
run_triage --strict
expect_case "NO_VERDICT verdict" 1 "NO_VERDICT"
if grep -qF "PASS" <<< "${OUT}"; then
  echo "❌ skipped-only input was rendered as PASS" >&2
  FAIL=$((FAIL + 1))
else
  PASS=$((PASS + 1))
fi

echo "Test 5: an unrecognised state is UNKNOWN and is named"
reset_scenario
set_checks '[{"name":"weird-job","state":"SOMETHING_NEW","link":"u1"}]'
run_triage --strict
expect_case "UNKNOWN verdict" 1 "UNKNOWN"
expect_case "unknown state names the check" 1 "weird-job"

echo "Test 6: cancelled and action-required are not green"
reset_scenario
set_checks '[{"name":"lint","state":"CANCELLED","link":"u1"}]'
run_triage --strict
expect_case "CANCELLED verdict" 1 "CANCELLED"
set_checks '[{"name":"deploy","state":"ACTION_REQUIRED","link":"u1"}]'
run_triage --strict
expect_case "ACTION_REQUIRED counted as FAIL" 1 "FAIL"

echo "Test 7: an empty check inventory is UNKNOWN"
reset_scenario
set_checks '[]' 1
run_triage --strict
expect_case "empty list (exit 1) is UNKNOWN" 1 "UNKNOWN"
set_checks '[]'
run_triage --strict
expect_case "empty list (exit 0) is UNKNOWN" 1 "UNKNOWN"

echo "Test 8: an authentication/API failure is BLOCKED with its message"
reset_scenario
set_checks '' 1
printf '%s\n' '' > "${SCN}/checks.json"
OUT="$(PATH="${BIN}:${PATH}" FAKE_GH_DIR="${SCN}" FAKE_GH_STDERR='authentication failed: token invalid' bash "${SUBJECT}" --strict 2>&1)" && RC=0 || RC=$?
expect_case "BLOCKED verdict" 1 "BLOCKED"
expect_case "query failure message is shown" 1 "authentication failed"

echo "Test 9: a head change during collection is STALE"
reset_scenario
set_checks '[{"name":"lint","state":"SUCCESS","link":"u1"}]'
printf '%s\n' '{"headRefOid":"bbb"}' > "${SCN}/view.json"
run_triage --strict
expect_case "STALE verdict" 1 "STALE"
expect_case "both heads are reported" 1 "aaa -> bbb"

echo "Test 10: no open PRs is an empty result, not an error"
reset_scenario
printf '%s\n' '[]' > "${SCN}/list.json"
run_triage
expect_case "no open PRs" 0 "No open PRs."
run_triage --json
expect_case "JSON mode returns []" 0 "[]"

echo "Test 11: JSON mode is machine-readable and carries the verdict"
reset_scenario
set_checks '[{"name":"lint","state":"FAILURE","link":"u1"}]'
run_triage --json --strict
expect_case "JSON mode exits 1 in strict" 1 "FAIL"
if jq -e '.[0].verdict == "FAIL" and .[0].number == 1 and .[0].counts.fail == 1' >/dev/null 2>&1 <<< "${OUT}"; then
  PASS=$((PASS + 1))
else
  echo "❌ JSON mode output is not the documented shape" >&2
  printf '%s\n' "${OUT}" | sed 's/^/  /' >&2
  FAIL=$((FAIL + 1))
fi

echo "Test 12: one check-inventory request per PR, with several PRs"
reset_scenario
printf '%s\n' '[{"number":1,"title":"one","mergeable":"MERGEABLE","headRefName":"a","headRefOid":"aaa"},{"number":2,"title":"two","mergeable":"MERGEABLE","headRefName":"b","headRefOid":"aaa"}]' > "${SCN}/list.json"
set_checks '[{"name":"lint","state":"SUCCESS","link":"u1"}]'
run_triage
expect_case "two PRs both PASS" 0 "PASS"
if [[ "$(grep -c '^pr checks' "${SCN}/calls.log")" -eq 2 ]]; then
  PASS=$((PASS + 1))
else
  echo "❌ expected 2 gh pr checks calls, got $(grep -c '^pr checks' "${SCN}/calls.log")" >&2
  FAIL=$((FAIL + 1))
fi

echo "Test 13: unknown arguments are a usage error"
run_triage --nonsense
expect_case "unknown argument" 2 "unknown argument"

echo "pr-triage fixture: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
