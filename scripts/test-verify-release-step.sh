#!/usr/bin/env bash
# test-verify-release-step.sh — issue #954 (+ #971, #972): the release-verification
# step must collect every registry verdict, tell the truth about each one, and
# actually re-probe on retry.
#
# WHY THIS EXISTS AT ALL: `verify-release` is inline workflow shell, so no Rust
# test and no `--lib` mutant can ever see it — the mutation surface does not
# contain `.github/workflows/*.yml`, and CI only runs the step after a tag. The
# three defects were each found by *replaying the step body*, so the replay is
# what gets committed here. The body is read out of the live workflow file with
# PyYAML, never transcribed: a fixture that tests a copy of the step proves the
# copy.
#
# The three properties, and the issue each one locks:
#   #954  `bash -e` (GitHub's default run shell) must not end the step at the
#         first miss — all four verdicts and the aggregate ::error:: are reachable.
#   #971  the crates.io index probe must match a row that exists. The shipped
#         pattern was quote-spliced to `vers:0.3.9`, which the sparse index never
#         contains, so the check said "Missing" for published versions.
#   #972  the retry must re-query. A probe inlined in the double-quoted argument
#         was substituted by the caller once, so eval re-ran a frozen `[[ … ]]`.
#
# Case group two is the sharpness control: it un-fixes the extracted body with sed
# and requires the *bug* to reappear. Without it the fixture could pass on a step
# that had silently regressed. The `publish-crates` group replays the same #971
# pattern where the dead branch meant seven needless `cargo publish` calls rather
# than one wrong word in a summary.
#
# No network: curl/npm/gh/cargo are stubbed on PATH. `sleep` is stubbed *inside*
# the extracted body (245 s of backoff per miss is not the subject) — that is the
# only mutation this fixture makes to the code under test, and it replaces a call
# the step already makes rather than removing one.
#
# The stub `curl` answers every index URL with the same file. That is deliberate:
# the property under test is whether the pattern matches the row shape crates.io
# stores, not per-crate URL routing.
#
# Usage: scripts/test-verify-release-step.sh          (no arguments)
# Exit:  0 all cases pass, or PyYAML is absent and CSM_VALIDATION_REQUIRED is not
#        "true" (that path prints "SKIPPED — NOT A PASS" rather than pretending);
#        1 any case fails; 2 a precondition the fixture cannot work around is
#        missing (workflow file unreadable, PyYAML required but absent).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKFLOW="${REPO_ROOT}/.github/workflows/release.yml"

if ! python3 -c 'import yaml' 2>/dev/null; then
    # Same convention as the LLM-context gate in scripts/validate.sh: a required run
    # fails loudly, an optional one says it did not check anything. Silent skip is the
    # failure mode that kept #954 invisible — the step is inline workflow shell, so this
    # fixture is its only test surface and it must not look like a pass when it ran none.
    if [[ "${CSM_VALIDATION_REQUIRED:-false}" == "true" ]]; then
        printf 'ERROR: PyYAML is required by this fixture; install python3-yaml.\n' >&2
        exit 2
    fi
    printf 'SKIPPED — NOT A PASS: python3 has no PyYAML (apt: python3-yaml); release.yml went un-replayed\n'
    exit 0
fi
if [[ ! -r "${WORKFLOW}" ]]; then
    printf 'ERROR: cannot read %s\n' "${WORKFLOW}" >&2
    exit 2
fi

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT
STUBS="${TEST_DIR}/stubs"
mkdir -p "${STUBS}"

RUNS=0
FAILURES=0
BODY_RC=0
SUMMARY=""
STDOUT=""
CALLS=""

# Registry state for the stubs. set_registry_state <index-file> <npm> <gh>
FIXTURE_CALLS="${TEST_DIR}/calls"
FIXTURE_INDEX=""
FIXTURE_NPM_OK="no"
FIXTURE_NPM_VERSION="9.9.9"
FIXTURE_GH_OK="no"
FIXTURE_CARGO_RC="0"
export FIXTURE_CALLS FIXTURE_INDEX FIXTURE_NPM_OK FIXTURE_NPM_VERSION FIXTURE_GH_OK FIXTURE_CARGO_RC

pass() {
    RUNS=$((RUNS + 1))
    printf 'OK   %s\n' "$1"
}

fail() {
    RUNS=$((RUNS + 1))
    FAILURES=$((FAILURES + 1))
    printf 'FAIL %s\n' "$1" >&2
}

dump_actual() {
    printf '     stdout:\n%s\n' "$(printf '%s\n' "${STDOUT}" | sed 's/^/       /')" >&2
    printf '     summary:\n%s\n' "$(printf '%s\n' "${SUMMARY}" | sed 's/^/       /')" >&2
    printf '     probes: %s\n' "$(printf '%s\n' "${CALLS}" | tr '\n' ' ')" >&2
}

expect_eq() {
    if [[ "$2" == "$3" ]]; then
        pass "$1"
    else
        fail "$1 — expected [$2] got [$3]"
        dump_actual
    fi
}

expect_has() {
    if printf '%s\n' "${SUMMARY}${STDOUT}" | grep -qF -- "$2"; then
        pass "$1"
    else
        fail "$1 — missing [$2]"
        dump_actual
    fi
}

expect_not_has() {
    if printf '%s\n' "${SUMMARY}${STDOUT}" | grep -qF -- "$2"; then
        fail "$1 — unexpected [$2]"
        dump_actual
    else
        pass "$1"
    fi
}

count_in_summary() {
    printf '%s\n' "${SUMMARY}" | grep -cF -- "$1" || true
}

count_in_stdout() {
    printf '%s\n' "${STDOUT}" | grep -cF -- "$1" || true
}

set_registry_state() {
    FIXTURE_INDEX="$1"
    FIXTURE_NPM_OK="$2"
    FIXTURE_GH_OK="$3"
}

# ---------------------------------------------------------------- extraction

# extract_step <out-file> <job> <step-name-prefix> <version>
extract_step() {
    local out="$1" job="$2" prefix="$3" version="$4"
    python3 - "${WORKFLOW}" "${job}" "${prefix}" "${version}" "${out}" <<'PY'
import sys, yaml
workflow, job, prefix, version, out = sys.argv[1:6]
doc = yaml.safe_load(open(workflow, encoding='utf-8'))
steps = doc['jobs'][job]['steps']
matches = [s for s in steps if str(s.get('name', '')).startswith(prefix)]
if len(matches) != 1:
    sys.exit('fixture: %r matched %d steps in job %r (need exactly 1)'
             % (prefix, len(matches), job))
body = matches[0]['run']
# The runner interpolates these; the fixture supplies literals so a replay can
# never quietly read a live output value off the machine it runs on.
body = body.replace('${{ needs.validate.outputs.version }}', version)
body = body.replace('${{ github.repository }}', 'd-o-hub/chaotic_semantic_memory')
body = body.replace('${{ secrets.GITHUB_TOKEN }}', 'fixture-token')
body = body.replace('${{ secrets.CARGO_REGISTRY_TOKEN }}', 'fixture-token')
lines = ['sleep() { :; }   # fixture stub: the 245 s backoff is not the subject']
lines += body.splitlines()
open(out, 'w', encoding='utf-8').write('\n'.join(lines) + '\n')
PY
}

# un_fix_bare_calls <in-file> <out-file> — restore the #954 shape mechanically.
un_fix_bare_calls() {
    sed -E 's|^if ! (check_with_retry .*)$|\1|; s|; then :; fi$||' "$1" > "$2"
}

# ------------------------------------------------------------------- stubs
write_stubs() {
    cat > "${STUBS}/curl" <<'STUB'
#!/usr/bin/env bash
printf 'curl\n' >> "${FIXTURE_CALLS}"
if [[ -n "${FIXTURE_INDEX}" && -f "${FIXTURE_INDEX}" ]]; then
    cat "${FIXTURE_INDEX}"
    exit 0
fi
exit 1
STUB
    cat > "${STUBS}/npm" <<'STUB'
#!/usr/bin/env bash
printf 'npm\n' >> "${FIXTURE_CALLS}"
if [[ "${FIXTURE_NPM_OK}" == "yes" ]]; then
    printf '%s\n' "${FIXTURE_NPM_VERSION}"
    exit 0
fi
printf 'npm ERR! 404 Not Found\n' >&2
exit 1
STUB
    cat > "${STUBS}/gh" <<'STUB'
#!/usr/bin/env bash
printf 'gh\n' >> "${FIXTURE_CALLS}"
if [[ "${FIXTURE_GH_OK}" == "yes" ]]; then
    printf '{"tagName":"v9.9.9"}\n'
    exit 0
fi
exit 1
STUB
    cat > "${STUBS}/cargo" <<'STUB'
#!/usr/bin/env bash
printf 'cargo\n' >> "${FIXTURE_CALLS}"
exit "${FIXTURE_CARGO_RC}"
STUB
    chmod +x "${STUBS}/curl" "${STUBS}/npm" "${STUBS}/gh" "${STUBS}/cargo"
}

# run_body <body-file> — GitHub's own default run shell, no network.
run_body() {
    local body="$1"
    : > "${FIXTURE_CALLS}"
    : > "${TEST_DIR}/summary.md"
    PATH="${STUBS}:${PATH}" GITHUB_STEP_SUMMARY="${TEST_DIR}/summary.md" \
        bash -e -o pipefail "${body}" > "${TEST_DIR}/stdout.log" 2> "${TEST_DIR}/stderr.log"
    BODY_RC=$?
    SUMMARY="$(cat "${TEST_DIR}/summary.md")"
    STDOUT="$(cat "${TEST_DIR}/stdout.log")"
    CALLS="$(cat "${FIXTURE_CALLS}")"
}

stub_tool_calls() {
    printf '%s\n' "${CALLS}" | grep -cxF -- "$1" || true
}

write_stubs

# Index rows in the shape crates.io actually stores: compact JSON, a quote on
# both sides of the colon. `spliced_only` holds the exact text the BROKEN pattern
# was looking for, so a re-splice cannot pass as a match.
cat > "${TEST_DIR}/index_present.json" <<'EOF'
{"name":"csm-memory","vers":"9.9.9","deps":[],"cm_id":"deadbeef"}
EOF
cat > "${TEST_DIR}/index_absent.json" <<'EOF'
{"name":"csm-memory","vers":"9.9.8","deps":[],"cm_id":"deadbeef"}
EOF
cat > "${TEST_DIR}/index_spliced_only.json" <<'EOF'
{"name":"csm-memory","vers":"9.9.8","note":"release prose says vers:9.9.9 with no quotes"}
EOF

printf '== #954: every verdict is collected (verify-release) ==\n'
if ! extract_step "${TEST_DIR}/vr.sh" verify-release "Verify complete" "9.9.9"; then
    printf 'ERROR: could not extract the verify-release step from %s\n' "${WORKFLOW}" >&2
    exit 2
fi

set_registry_state "${TEST_DIR}/index_present.json" yes yes
run_body "${TEST_DIR}/vr.sh"
expect_eq "all published: step exits 0" "0" "${BODY_RC}"
expect_eq "all published: four verdict rows" "4" "$(count_in_summary ':')"
expect_has "all published: crates.io credited" "✅ crates.io: Published"
expect_has "all published: GitHub release credited" "✅ GitHub Release: Created"
expect_not_has "all published: no aggregate error" "::error::"
expect_eq "all published: each registry probed once" "1" "$(stub_tool_calls gh)"
expect_eq "all published: crates.io probed once" "1" "$(stub_tool_calls curl)"

set_registry_state "${TEST_DIR}/index_absent.json" no no
run_body "${TEST_DIR}/vr.sh"
expect_eq "all missing: step exits 1" "1" "${BODY_RC}"
expect_eq "all missing: FOUR verdict rows, not one" "4" "$(count_in_summary ': Missing')"
expect_has "all missing: aggregate names crates.io" "Missing: crates.io;"
expect_has "all missing: aggregate names the wasm package" "npm (@d-o-hub/chaotic_semantic_memory);"
expect_has "all missing: aggregate names the cli package" "npm (@d-o-hub/csm);"
expect_has "all missing: aggregate names the github release" "github;"
expect_eq "#972: npm probed 8x per package, not once" "16" "$(stub_tool_calls npm)"
expect_eq "#972: crates.io re-probed 8x" "8" "$(stub_tool_calls curl)"
expect_eq "all missing: the release check is not retried (single gh call)" "1" "$(stub_tool_calls gh)"

printf '\n== #954 sharpness: the un-fixed body must reproduce the truncation ==\n'
if ! un_fix_bare_calls "${TEST_DIR}/vr.sh" "${TEST_DIR}/vr_broken.sh"; then
    printf 'ERROR: could not derive the negative-control body\n' >&2
    exit 2
fi
if grep -q '^if ! check_with_retry' "${TEST_DIR}/vr_broken.sh"; then
    fail "negative control: bare-call form restored"
else
    pass "negative control: bare-call form restored"
fi
run_body "${TEST_DIR}/vr_broken.sh"
expect_eq "un-fixed: step still exits 1 — fail-closed, verdict right, evidence short" "1" "${BODY_RC}"
expect_eq "un-fixed: only the FIRST verdict survives" "1" "$(count_in_summary ': Missing')"
expect_not_has "un-fixed: the aggregate ::error:: is unreachable" "::error::"

printf '\n== #971: the index probe matches a row that exists ==\n'
set_registry_state "${TEST_DIR}/index_present.json" yes yes
run_body "${TEST_DIR}/vr.sh"
expect_has "a published version is credited (said Missing for 0.3.8 on main)" "✅ crates.io: Published"

FIXTURE_INDEX="${TEST_DIR}/index_spliced_only.json"
export FIXTURE_INDEX
run_body "${TEST_DIR}/vr.sh"
expect_eq "the bare substring vers:9.9.9 must NOT satisfy the check" "1" "${BODY_RC}"
expect_has "splice-only index is reported Missing" "❌ crates.io: Missing"

set_registry_state "${TEST_DIR}/index_absent.json" yes yes
run_body "${TEST_DIR}/vr.sh"
expect_has "a lower published version is still Missing" "❌ crates.io: Missing"
expect_has "while the npm verdicts are unaffected" "✅ npm (@d-o-hub/csm): Published"

printf '\n== #971: publish-crates no longer re-publishes what is already live ==\n'
if ! extract_step "${TEST_DIR}/pc.sh" publish-crates "Publish workspace crates" "9.9.9"; then
    printf 'ERROR: could not extract the publish-crates step from %s\n' "${WORKFLOW}" >&2
    exit 2
fi
set_registry_state "${TEST_DIR}/index_present.json" yes yes
run_body "${TEST_DIR}/pc.sh"
expect_eq "publish-crates: step exits 0" "0" "${BODY_RC}"
expect_eq "publish-crates: zero cargo publishes for a live version" "0" "$(stub_tool_calls cargo)"
expect_eq "publish-crates: all seven members skipped" "7" "$(count_in_stdout 'already on crates.io, skipping')"

set_registry_state "${TEST_DIR}/index_absent.json" yes yes
run_body "${TEST_DIR}/pc.sh"
expect_eq "publish-crates: seven publishes when none is live" "7" "$(stub_tool_calls cargo)"
expect_eq "publish-crates: one index probe per member" "7" "$(stub_tool_calls curl)"

printf '\n== workflow file: the splice and the bare call must not come back ==\n'
# Comments are excluded, and deliberately so: this file's own rationale comment
# quotes the broken pattern to explain the fix, and a gate that reads prose as
# code fails on the documentation of the repair (the same trap as the generated
# script catalog's caller columns). An executable line never starts with `#`.
if sed '/^[[:space:]]*#/d' "${WORKFLOW}" | grep -qn 'grep -q ""'; then
    fail "release.yml contains a quote-spliced grep pattern again"
    sed '/^[[:space:]]*#/d' "${WORKFLOW}" | grep -n 'grep -q ""' >&2
else
    pass "release.yml has no quote-spliced grep pattern"
fi
if grep -qF 'check_with_retry "crates.io" "curl' "${WORKFLOW}"; then
    fail "verify-release inlines the index probe into check_with_retry again"
else
    pass "verify-release passes the crates.io probe by function name"
fi
if grep -qF 'check_with_retry "npm (@d-o-hub/csm)" "[ "$(npm' "${WORKFLOW}"; then
    fail "verify-release inlines an npm probe (frozen substitution) again"
else
    pass "verify-release passes the npm probes by function name"
fi

printf '\n----\n'
printf 'verify-release fixture: %s case(s), %s failure(s)\n' "${RUNS}" "${FAILURES}"
if [[ "${FAILURES}" -gt 0 ]]; then
    exit 1
fi
exit 0
