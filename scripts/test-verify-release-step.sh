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
    if [[ "${FIXTURE_NPM_OK}" == "yes" ]]; then
        FIXTURE_NPM_PACKUMENT="${TEST_DIR}/npm_attested.json"
    else
        FIXTURE_NPM_PACKUMENT="${TEST_DIR}/npm_unattested.json"
    fi
    export FIXTURE_NPM_PACKUMENT
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
body = body.replace("${{ vars.CRATES_IO_EXPECTED_LOGIN || 'd-o-hub' }}", 'd-o-hub')
matrix = [
    {"runner": "ubuntu-24.04", "target": "x86_64-unknown-linux-gnu", "platform": "linux", "arch": "x64"},
    {"runner": "ubuntu-24.04", "target": "aarch64-unknown-linux-gnu", "platform": "linux", "arch": "arm64"},
    {"runner": "macos-latest", "target": "aarch64-apple-darwin", "platform": "darwin", "arch": "arm64"},
    {"runner": "macos-latest", "target": "x86_64-apple-darwin", "platform": "darwin", "arch": "x64"},
    {"runner": "windows-latest", "target": "x86_64-pc-windows-msvc", "platform": "win32", "arch": "x64"},
]
import json
body = body.replace('${{ needs.validate.outputs.cli-matrix }}', json.dumps(matrix))
lines = ['sleep() { :; }   # fixture stub: the 245 s backoff is not the subject']
lines += body.splitlines()
open(out, 'w', encoding='utf-8').write('\n'.join(lines) + '\n')
PY
}

# un_fix_bare_calls <in-file> <out-file> — restore the #954 shape mechanically.
un_fix_bare_calls() {
    # Keep the indentation (the probes sit inside a for loop now) and turn
    # `if ! check_with_retry …; then :; fi` into the bare call #954 shipped, which
    # `bash -e` ends on the first miss.
    sed -E 's|^([[:space:]]*)if ! (check_with_retry .*); then :; fi[[:space:]]*$|\1\2|' "$1" > "$2"
}

# ------------------------------------------------------------------- stubs
write_stubs() {
    cat > "${STUBS}/curl" <<'STUB'
#!/usr/bin/env bash
printf 'curl\n' >> "${FIXTURE_CALLS}"
out=""
prev=""
for arg in "$@"; do
    if [[ "${prev}" == "-o" ]]; then out="${arg}"; fi
    prev="${arg}"
done
if [[ "$*" == *"/api/v1/me"* ]]; then
    body="$(cat "${FIXTURE_ME_BODY_FILE}")"
    [[ -n "${out}" ]] && printf '%s' "${body}" > "${out}"
    printf '%s' "${FIXTURE_ME_CODE:-200}"
    exit 0
fi
if [[ "$*" == *"registry.npmjs.org"* ]]; then
    cat "${FIXTURE_NPM_PACKUMENT:-${TEST_DIR}/npm_attested.json}"
    exit 0
fi
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
if [[ "${FIXTURE_GH_OK}" != "yes" ]]; then
    exit 1
fi
if [[ "$*" == *"--json assets"* ]]; then
    cat "${FIXTURE_GH_ASSETS:-${TEST_DIR}/assets_all.txt}"
    exit 0
fi
printf '{"tagName":"v9.9.9"}\n'
STUB
    cat > "${STUBS}/cargo" <<'STUB'
#!/usr/bin/env bash
printf 'cargo\n' >> "${FIXTURE_CALLS}"
if [[ "${FIXTURE_CARGO_AUTH:-0}" == "1" ]]; then
    printf 'error: failed to publish\n' >&2
    printf 'Caused by: status 403 Forbidden: authentication failed\n' >&2
    exit 1
fi
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
        CARGO_REGISTRY_TOKEN=fixture-token GH_TOKEN=fixture-token \
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

# Credential/npm/asset defaults for the replay. Every case overrides the field it
# cares about, so the positive control is explicit rather than incidental.
FIXTURE_ME_CODE=200
FIXTURE_ME_BODY_FILE="${TEST_DIR}/me_ok.json"
FIXTURE_CARGO_AUTH=0
FIXTURE_NPM_PACKUMENT="${TEST_DIR}/npm_attested.json"
FIXTURE_GH_ASSETS="${TEST_DIR}/assets_all.txt"
export FIXTURE_ME_CODE FIXTURE_ME_BODY_FILE FIXTURE_CARGO_AUTH FIXTURE_NPM_PACKUMENT FIXTURE_GH_ASSETS

cat > "${TEST_DIR}/me_ok.json" <<'JSON'
{"user":{"login":"d-o-hub"}}
JSON
cat > "${TEST_DIR}/me_401.json" <<'JSON'
{"errors":[{"detail":"The given API token does not match the format used by crates.io."}]}
JSON
cat > "${TEST_DIR}/me_403.json" <<'JSON'
{"errors":[{"detail":"this action requires authentication"}]}
JSON
cat > "${TEST_DIR}/me_wrong_login.json" <<'JSON'
{"user":{"login":"someone-else"}}
JSON

cat > "${TEST_DIR}/npm_attested.json" <<'JSON'
{"versions":{
  "9.9.9":{"version":"9.9.9","dist":{"attestations":{"url":"https://registry.npmjs.org/-/npm/v1/attestations/csm@9.9.9","provenance":{"predicateType":"https://slsa.dev/provenance/v1"}}}},
  "0.3.8":{"version":"0.3.8","dist":{"attestations":{"url":"https://registry.npmjs.org/-/npm/v1/attestations/csm@0.3.8","provenance":{"predicateType":"https://slsa.dev/provenance/v1"}}}},
  "0.3.4":{"version":"0.3.4","dist":{}}
}}
JSON
cat > "${TEST_DIR}/npm_unattested.json" <<'JSON'
{"versions":{"9.9.9":{"version":"9.9.9","dist":{}}}}
JSON
cat > "${TEST_DIR}/assets_all.txt" <<'ASSETS'
SHA256SUMS
csm-linux-x64.tar.gz
csm-linux-arm64.tar.gz
csm-darwin-arm64.tar.gz
csm-darwin-x64.tar.gz
csm-win32-x64.tar.gz
chaotic_semantic_memory-9.9.9-wasm.tar.gz
ASSETS

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
expect_eq "all published: fourteen verdict rows (9 crates + 4 npm + assets)" "14" "$(count_in_summary ':')"
expect_has "all published: the root crate is credited" "✅ crates.io/chaotic_semantic_memory: Published"
expect_has "all published: csm-duckdb is credited" "✅ crates.io/csm-duckdb: Published"
expect_has "all published: npm version is credited" "✅ npm (@d-o-hub/csm) version: Published"
expect_has "all published: npm provenance is credited" "✅ npm (@d-o-hub/csm) provenance: Published"
expect_has "all published: every expected asset is present" "✅ GitHub Release assets: 7/7"
expect_not_has "all published: no aggregate error" "::error::"
expect_eq "all published: one gh call" "1" "$(stub_tool_calls gh)"
expect_eq "all published: 11 curl probes (9 crates + 2 provenance)" "11" "$(stub_tool_calls curl)"

set_registry_state "${TEST_DIR}/index_absent.json" no no
run_body "${TEST_DIR}/vr.sh"
expect_eq "all missing: step exits 1" "1" "${BODY_RC}"
expect_eq "all missing: thirteen missing rows" "13" "$(count_in_summary ': Missing')"
expect_has "all missing: aggregate names a crate" "Missing: crates.io/csm-chaos;"
expect_has "all missing: aggregate names the wasm npm version" "npm (@d-o-hub/chaotic_semantic_memory) version;"
expect_has "all missing: aggregate names the cli npm provenance" "npm (@d-o-hub/csm) provenance;"
expect_has "all missing: aggregate names the github assets" "github-assets;"
expect_eq "#972: npm probed 8x per package, not once" "16" "$(stub_tool_calls npm)"
expect_eq "#972: 9 crates and 2 provenance probes re-query 8x" "88" "$(stub_tool_calls curl)"
expect_eq "all missing: the release asset check is not retried (single gh call)" "1" "$(stub_tool_calls gh)"

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
expect_has "a published version is credited (said Missing for 0.3.8 on main)" "✅ crates.io/csm-memory: Published"

FIXTURE_INDEX="${TEST_DIR}/index_spliced_only.json"
export FIXTURE_INDEX
run_body "${TEST_DIR}/vr.sh"
expect_eq "the bare substring vers:9.9.9 must NOT satisfy the check" "1" "${BODY_RC}"
expect_has "splice-only index is reported Missing" "❌ crates.io/csm-memory: Missing"

set_registry_state "${TEST_DIR}/index_absent.json" yes yes
run_body "${TEST_DIR}/vr.sh"
expect_has "a lower published version is still Missing" "❌ crates.io/csm-memory: Missing"
expect_has "while the npm verdicts are unaffected" "✅ npm (@d-o-hub/csm) version: Published"

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
# The needle stops at the `[ ` that opens an inlined test expression, and the
# positive control names the function that replaced it. Together they pin the
# same property the longer pattern did — the second argument to check_with_retry
# is either `eval`ed text or a function name, and `[ ` vs `published_on_npm`
# tells you which — without carrying a `$(` inside single quotes, which
# ShellCheck reads as an accidental non-expansion (SC2016).
if grep -qF 'check_with_retry "npm (@d-o-hub/csm)" "[ ' "${WORKFLOW}"; then
    fail "verify-release inlines an npm probe (frozen substitution) again"
elif ! grep -qF "check_with_retry \"npm (\${pkg}) version\" \"published_on_npm \${pkg}\"" "${WORKFLOW}"; then
    fail "verify-release no longer calls the npm version probe through a named function"
elif ! grep -qF "check_with_retry \"npm (\${pkg}) provenance\" \"npm_attestation_present \${pkg}\"" "${WORKFLOW}"; then
    fail "verify-release no longer calls the npm provenance probe through a named function"
else
    pass "verify-release passes both npm probes by function name"
fi
# Issue #955: the abbreviated packument is the only endpoint that serves
# .dist.attestations; npm view reads the full packument and cannot see it.
if grep -qF "Accept: application/vnd.npm.install-v1+json" "${WORKFLOW}"; then
    pass "the provenance probe requests the abbreviated packument"
else
    fail "the provenance probe does not request the abbreviated packument"
fi


printf '\n== #967: the crates.io credential is probed before any cargo ==\n'
if ! extract_step "${TEST_DIR}/pc_auth.sh" publish-crates "Probe the crates.io credential" "9.9.9"; then
    printf 'ERROR: could not extract the credential preflight step\n' >&2
    exit 2
fi
FIXTURE_ME_CODE=200
FIXTURE_ME_BODY_FILE="${TEST_DIR}/me_ok.json"
export FIXTURE_ME_CODE FIXTURE_ME_BODY_FILE
run_body "${TEST_DIR}/pc_auth.sh"
expect_eq "a valid token exits 0" "0" "${BODY_RC}"
expect_has "the probe names the authenticated login" "authenticated as d-o-hub"
expect_not_has "the token value is never printed" "fixture-token"

FIXTURE_ME_CODE=401
FIXTURE_ME_BODY_FILE="${TEST_DIR}/me_401.json"
run_body "${TEST_DIR}/pc_auth.sh"
expect_eq "a 401 exits 1" "1" "${BODY_RC}"
expect_has "the HTTP status is named" "HTTP 401"
expect_has "crates.io's detail is surfaced" "token does not match the format"

FIXTURE_ME_CODE=403
FIXTURE_ME_BODY_FILE="${TEST_DIR}/me_403.json"
run_body "${TEST_DIR}/pc_auth.sh"
expect_eq "missing auth exits 1" "1" "${BODY_RC}"
expect_has "missing auth keeps the detail" "requires authentication"

FIXTURE_ME_CODE=200
FIXTURE_ME_BODY_FILE="${TEST_DIR}/me_wrong_login.json"
run_body "${TEST_DIR}/pc_auth.sh"
expect_eq "a valid token for the wrong account exits 1" "1" "${BODY_RC}"
expect_has "the wrong login is named" "authenticated as 'someone-else'"

printf '\n== #967: an authentication failure aborts the publish loop ==\n'
set_registry_state "${TEST_DIR}/index_absent.json" yes yes
FIXTURE_CARGO_AUTH=1
export FIXTURE_CARGO_AUTH
: > "${TEST_DIR}/argv.txt"
run_body "${TEST_DIR}/pc.sh"
expect_eq "the auth failure reds the step" "1" "${BODY_RC}"
expect_eq "it aborts after one crate, not seven" "1" "$(stub_tool_calls cargo)"
expect_has "the annotation names the credential class" "credential rejected while publishing"
expect_not_has "it does not reach the aggregate publish-failed message" "Publish failed for:"
FIXTURE_CARGO_AUTH=0
export FIXTURE_CARGO_AUTH

printf '\n== #974: a missing release asset is named ==\n'
set_registry_state "${TEST_DIR}/index_present.json" yes yes
grep -v '^csm-darwin-x64.tar.gz$' "${TEST_DIR}/assets_all.txt" > "${TEST_DIR}/assets_missing.txt"
FIXTURE_GH_ASSETS="${TEST_DIR}/assets_missing.txt"
export FIXTURE_GH_ASSETS
run_body "${TEST_DIR}/vr.sh"
expect_eq "a missing asset reds the step" "1" "${BODY_RC}"
expect_has "the summary names the missing asset" "GitHub Release assets: missing"
expect_has "the aggregate names the asset" "asset:csm-darwin-x64.tar.gz;"
FIXTURE_GH_ASSETS="${TEST_DIR}/assets_all.txt"
export FIXTURE_GH_ASSETS

set_registry_state "${TEST_DIR}/index_present.json" yes no
run_body "${TEST_DIR}/vr.sh"
expect_eq "an unreadable release reds the step" "1" "${BODY_RC}"
expect_has "the aggregate names the missing asset set" "github-assets;"
set_registry_state "${TEST_DIR}/index_present.json" yes yes

printf '\n== #955: provenance probe positive controls (0.3.8 attested, 0.3.4 not) ==\n'
awk '/^npm_attestation_present\(\) \{/{f=1} f{print} f&&/^\}$/{exit}' "${TEST_DIR}/vr.sh" > "${TEST_DIR}/npm_probe.sh"
cat >> "${TEST_DIR}/npm_probe.sh" <<'PROBE'
VERSION=0.3.8
if npm_attestation_present @d-o-hub/csm; then echo "0.3.8 attested"; else echo "0.3.8 unattested"; fi
VERSION=0.3.4
if npm_attestation_present @d-o-hub/csm; then echo "0.3.4 attested"; else echo "0.3.4 unattested"; fi
PROBE
PROBE_OUT="$(PATH="${STUBS}:${PATH}" FIXTURE_CALLS="${TEST_DIR}/calls" \
    FIXTURE_NPM_PACKUMENT="${TEST_DIR}/npm_attested.json" \
    bash "${TEST_DIR}/npm_probe.sh" 2>&1)"
if grep -qF '0.3.8 attested' <<< "${PROBE_OUT}" && grep -qF '0.3.4 unattested' <<< "${PROBE_OUT}"; then
    pass "the provenance probe reproduces the two known answers"
else
    fail "the provenance probe does not reproduce the positive controls"
    printf '%s\n' "${PROBE_OUT}" >&2
fi

printf '\n----\n'
printf 'verify-release fixture: %s case(s), %s failure(s)\n' "${RUNS}" "${FAILURES}"
if [[ "${FAILURES}" -gt 0 ]]; then
    exit 1
fi
exit 0
