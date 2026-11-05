#!/usr/bin/env bash
# test-verify-release-step.sh — issue #954 (+ #971, #972, ADR-0104): the
# release-verification step must collect every registry verdict, tell the truth about
# each one, actually re-probe on retry — and the crates.io publish path must stay on
# the OIDC exchange rather than drifting back to a stored token.
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
# that had silently regressed. The `publish-crates.yml` groups replay the same #971
# pattern where the dead branch meant seven needless `cargo publish` calls rather
# than one wrong word in a summary.
#
# Since the crates.io migration, this fixture is also the only gate standing between
# the OIDC design and a silent regression, so it carries structural assertions on
# both workflow files (ADR-0104). Those lock properties no replay can show: that the
# token comes from the OIDC exchange and never from a stored secret, that the
# exchange step runs before anything consumes it, that the job's bound sits under
# crates.io's 30-minute token lifetime, and that release.yml no longer owns a crate
# job at all. Every one of them was mutation-checked against a broken copy of the
# workflow, because a `grep -q` over YAML that cannot fail is decoration.
#
# No network: curl/npm/gh/cargo are stubbed on PATH. `sleep` is stubbed *inside*
# the extracted body (245 s of backoff per miss is not the subject) — that is the
# only mutation this fixture makes to the code under test, and it replaces a call
# the step already makes rather than removing one.
#
# By default the stub `curl` answers every index URL with the same file. That is
# deliberate: the property under test is whether the pattern matches the row shape
# crates.io stores, not per-crate URL routing. The reconciliation group is the one
# exception and opts into per-crate routing with FIXTURE_INDEX_LIVE, because "seven
# of nine landed" is a state a single shared answer cannot express.
#
# Usage: scripts/test-verify-release-step.sh          (no arguments)
# Exit:  0 all cases pass, or PyYAML is absent and CSM_VALIDATION_REQUIRED is not
#        "true" (that path prints "SKIPPED — NOT A PASS" rather than pretending);
#        1 any case fails; 2 a precondition the fixture cannot work around is
#        missing (workflow file unreadable, PyYAML required but absent).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# The extracted bodies call `bash scripts/crates-io-index-path.sh`, so the replay's
# cwd is part of the fixture, not an accident of how it was invoked.
cd "${REPO_ROOT}" || exit 2
# Both paths are overridable so a deliberately broken copy can be the negative
# control for the structural group: an assertion nobody has seen fail is a guess.
WORKFLOW="${CSM_RELEASE_WORKFLOW:-${REPO_ROOT}/.github/workflows/release.yml}"
PUBLISH_WORKFLOW="${CSM_PUBLISH_WORKFLOW:-${REPO_ROOT}/.github/workflows/publish-crates.yml}"

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
for required_workflow in "${WORKFLOW}" "${PUBLISH_WORKFLOW}"; do
    if [[ ! -r "${required_workflow}" ]]; then
        printf 'ERROR: cannot read %s\n' "${required_workflow}" >&2
        exit 2
    fi
done

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
# TEST_DIR is exported because the stubs resolve their fixture files by name, and the
# stubs run in the replay's environment, not this shell's. A stub that quietly reads an
# empty ${TEST_DIR} answers "no such row" for every crate — which looks exactly like a
# publish that never happened and will be believed.
export TEST_DIR
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

# extract_step <out-file> <workflow> <job> <step-name-prefix> <version>
extract_step() {
    local out="$1" workflow="$2" job="$3" prefix="$4" version="$5"
    python3 - "${workflow}" "${job}" "${prefix}" "${version}" "${out}" <<'PY'
import sys, yaml
workflow, job, prefix, version, out = sys.argv[1:6]
doc = yaml.safe_load(open(workflow, encoding='utf-8'))
steps = doc['jobs'][job]['steps']
matches = [s for s in steps if str(s.get('name', '')).startswith(prefix)]
if len(matches) != 1:
    sys.exit('fixture: %r matched %d steps in job %r of %s (need exactly 1)'
             % (prefix, len(matches), job, workflow))
body = matches[0]['run']
# The runner interpolates these; the fixture supplies literals so a replay can
# never quietly read a live output value off the machine it runs on.
body = body.replace('${{ needs.validate.outputs.version }}', version)
body = body.replace('${{ github.repository }}', 'd-o-hub/chaotic_semantic_memory')
body = body.replace('${{ secrets.GITHUB_TOKEN }}', 'fixture-token')
# Deliberately no substitution for a registry secret: crates.io credentials come from
# the OIDC exchange step now, so a body that reads `secrets.CARGO_REGISTRY_TOKEN`
# should replay with that literal text in it and fail, not be quietly papered over.
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

# structural_checks — the OIDC properties only the workflow files can prove.
# Each line the checker prints goes through pass/fail, so a structural failure counts
# against the fixture the same way a replay failure does; printing a red word without
# a non-zero exit is how a gate becomes a comment.
structural_checks() {
    local status message
    if ! python3 - "${PUBLISH_WORKFLOW}" "${WORKFLOW}" \
        > "${TEST_DIR}/structural.tsv" 2> "${TEST_DIR}/structural.err" <<'PY'
import re
import sys
import yaml

publish_path, release_path = sys.argv[1:3]
checks = []


def check(name, ok, got=''):
    checks.append((name, bool(ok), '' if ok else ' -- got %s' % (got,)))


pub_raw = open(publish_path, encoding='utf-8').read()
rel_raw = open(release_path, encoding='utf-8').read()
pub = yaml.safe_load(pub_raw)
rel = yaml.safe_load(rel_raw)

job = pub['jobs']['publish']
steps = job['steps']
jobenv = job.get('env') or {}
step_env = [s.get('env') or {} for s in steps]
token_steps = [i for i, e in enumerate(step_env) if 'CARGO_REGISTRY_TOKEN' in e]
auth = [i for i, s in enumerate(steps) if 'crates-io-auth-action' in str(s.get('uses', ''))]
bodies = [str(s.get('run', '')) for s in steps]

check('publish-crates.yml asks for exactly contents:read + id-token:write',
      job.get('permissions') == {'contents': 'read', 'id-token': 'write'}, job.get('permissions'))
check('the workflow declares no default permissions',
      pub.get('permissions') == {}, pub.get('permissions'))
check('the job runs in the crates.io environment (a publisher without it is refused)',
      job.get('environment') == 'crates.io', job.get('environment'))
bound = job.get('timeout-minutes')
check('the job is bounded under the 30-minute lifetime of an exchange token',
      isinstance(bound, int) and 0 < bound <= 25, bound)

# PyYAML reads the `on:` key as the boolean True -- YAML 1.1 treats `on` as a reserved
# word. A checker that only asked for the string 'on' would find no triggers at all and
# report a forbidden workflow_run as merely absent, which reads as a pass.
triggers = pub.get(True, pub.get('on'))
check('workflow_dispatch is the only trigger (crates.io refuses workflow_run and pull_request_target)',
      isinstance(triggers, dict) and list(triggers) == ['workflow_dispatch'], triggers)
check('the job cannot be cancelled mid-upload',
      (pub.get('concurrency') or {}).get('cancel-in-progress') is False, pub.get('concurrency'))

check('exactly one step uses the crates.io OIDC action', len(auth) == 1, auth)
if len(auth) == 1:
    uses = str(steps[auth[0]].get('uses', ''))
    check('it is the step the token consumers read, named crates-auth',
          steps[auth[0]].get('id') == 'crates-auth', steps[auth[0]].get('id'))
    check('the action is pinned to a commit SHA, not a moving tag',
          bool(re.search(r'@[0-9a-f]{40}$', uses)), uses)
    check('the pin says v1.0.5 in the file itself (PyYAML drops comments, so grep the raw text)',
          bool(re.search(r'crates-io-auth-action@[0-9a-f]{40} # v1\.0\.5', pub_raw)), uses)
    check('the exchange runs before anything consumes a token',
          bool(token_steps) and min(token_steps) > auth[0], (auth[0], token_steps))

check('at least the guard and the three publish steps consume the exchange token',
      len(token_steps) >= 4, token_steps)
check('every crates.io token comes from the exchange, none from a stored secret',
      all(step_env[i]['CARGO_REGISTRY_TOKEN'] == '${{ steps.crates-auth.outputs.token }}'
          for i in token_steps),
      [step_env[i]['CARGO_REGISTRY_TOKEN'] for i in token_steps])
check('publish-crates.yml never names CARGO_REGISTRY_TOKEN as a secret',
      'secrets.CARGO_REGISTRY_TOKEN' not in pub_raw)

# Dispatch text inside a run: body is a shell-injection surface, and ref/version are
# exactly what an operator types at dispatch time.
interpolating = [s.get('name', '<unnamed>') for s in steps if '${{' in str(s.get('run', ''))]
check('no run: body interpolates an expression (event text reaches the shell only as env)',
      not interpolating, interpolating)
check('the checkout keeps no credential for a ref named at dispatch time',
      (steps[0].get('with') or {}).get('persist-credentials') is False, steps[0].get('with'))

check('the published crate set is written exactly once in this file',
      pub_raw.count('csm-core-lib') == 1, pub_raw.count('csm-core-lib'))
crates = str(jobenv.get('CSM_PUBLISH_CRATES', '')).split()
check('the two crates with their own steps are still members of that set',
      {jobenv.get('CSM_ROOT_CRATE'), jobenv.get('CSM_DUCKDB_CRATE')} <= set(crates),
      [jobenv.get('CSM_ROOT_CRATE'), jobenv.get('CSM_DUCKDB_CRATE'), crates])
hard_paths = [ln.strip() for b in bodies for ln in b.splitlines()
              if 'index.crates.io/' in ln and '${' not in ln.split('index.crates.io/', 1)[1]]
check('every index URL is built by crates-io-index-path.sh, none hand-spliced',
      not hard_paths, hard_paths)

check('release.yml no longer owns a crate-publish job',
      'publish-crates' not in (rel.get('jobs') or {}), sorted(rel.get('jobs') or {}))
needing = [name for name, j in (rel.get('jobs') or {}).items()
           if 'publish-crates' in ([j['needs']] if isinstance(j.get('needs'), str)
                                   else (j.get('needs') or []))]
check('no release.yml job waits on the removed crate job', not needing, needing)
check('release.yml no longer reads a crates.io secret',
      'secrets.CARGO_REGISTRY_TOKEN' not in rel_raw)
check('release.yml no longer reads the removed job context',
      'needs.publish-crates' not in rel_raw)
check('release.yml no longer reads the identity variable the dead probe used',
      'vars.CRATES_IO_EXPECTED_LOGIN' not in rel_raw)

for name, ok, detail in checks:
    sys.stdout.write('%s\t%s%s\n' % ('PASS' if ok else 'FAIL', name, detail))
PY
    then
        printf 'ERROR: structural checker exited nonzero:\n' >&2
        cat "${TEST_DIR}/structural.err" >&2
        return 1
    fi
    while IFS=$'\t' read -r status message; do
        [[ -n "${status}" ]] || continue
        if [[ "${status}" == "PASS" ]]; then
            pass "${message}"
        else
            fail "${message}"
        fi
    done < "${TEST_DIR}/structural.tsv"
}

# ------------------------------------------------------------------- stubs
write_stubs() {
    cat > "${STUBS}/curl" <<'STUB'
#!/usr/bin/env bash
printf 'curl\n' >> "${FIXTURE_CALLS}"
if [[ "$*" == *"registry.npmjs.org"* ]]; then
    cat "${FIXTURE_NPM_PACKUMENT:-${TEST_DIR}/npm_attested.json}"
    exit 0
fi
# Per-crate routing, opt-in: the sparse index URL ends in the crate name, so a live-set
# file can express "seven of nine landed" — the partial-publish state the reconciliation
# step exists to catch, and one the shared FIXTURE_INDEX answer below cannot show.
if [[ -n "${FIXTURE_INDEX_LIVE:-}" && "$*" == *"index.crates.io"* ]]; then
    # `${*##*/}` is NOT the URL tail: a `##` on `$*` applies per word, so it yields
    # "-sL csm-chaos" here and a stub like this answers Missing for everything. Fold to
    # one scalar first, then strip.
    allargs="$*"
    crate="${allargs##*/}"
    if grep -qxF -- "${crate}" "${FIXTURE_INDEX_LIVE}"; then
        cat "${TEST_DIR}/index_present.json"
    else
        cat "${TEST_DIR}/index_absent.json"
    fi
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
#
# The job-level `env:` block of publish-crates.yml is read out of that file (see
# load_publish_env) and replayed, because the step bodies now read VERSION/DRY_RUN/
# CSM_PUBLISH_CRATES/CSM_ROOT_CRATE/CSM_DUCKDB_CRATES from job env instead of inline
# `${{ }}` interpolation. Replaying the workflow's own values means a change to the
# crate set moves the replays with it; a fixture that hardcoded nine names of its own
# would keep passing after the thing it tests drifted.
run_body() {
    local body="$1"
    : > "${FIXTURE_CALLS}"
    : > "${TEST_DIR}/summary.md"
    local -a assigns=(
        "PATH=${STUBS}:${PATH}"
        "GITHUB_STEP_SUMMARY=${TEST_DIR}/summary.md"
        "GH_TOKEN=fixture-token"
        "VERSION=${FIXTURE_VERSION}"
        "DRY_RUN=${FIXTURE_DRY_RUN}"
    )
    # FIXTURE_NO_TOKEN models the exchange producing nothing: the guard step must red
    # before a single cargo call, and only an absent variable can prove that.
    if [[ "${FIXTURE_NO_TOKEN}" != "1" ]]; then
        assigns+=("CARGO_REGISTRY_TOKEN=fixture-token")
    fi
    if [[ -f "${TEST_DIR}/publish.env" ]]; then
        local -a jobenv=()
        mapfile -t jobenv < "${TEST_DIR}/publish.env"
        assigns+=("${jobenv[@]}")
    fi
    env "${assigns[@]}" bash -e -o pipefail "${body}" \
        > "${TEST_DIR}/stdout.log" 2> "${TEST_DIR}/stderr.log"
    BODY_RC=$?
    SUMMARY="$(cat "${TEST_DIR}/summary.md")"
    STDOUT="$(cat "${TEST_DIR}/stdout.log")"
    CALLS="$(cat "${FIXTURE_CALLS}")"
}

# load_publish_env — copy publish-crates.yml's job env into KEY=VALUE lines for the
# replay. `${{ ... }}` values are skipped on purpose: those are the dispatch inputs,
# which the fixture pins explicitly rather than reading from the machine it runs on.
load_publish_env() {
    python3 - "${PUBLISH_WORKFLOW}" "${TEST_DIR}/publish.env" <<'PY'
import sys, yaml
workflow, out = sys.argv[1:3]
env = yaml.safe_load(open(workflow, encoding='utf-8'))['jobs']['publish'].get('env') or {}
lines = []
for key, value in env.items():
    text = str(value).strip()
    if '${{' in text:
        continue
    lines.append('%s=%s' % (key, text))
open(out, 'w', encoding='utf-8').write('\n'.join(lines) + '\n')
PY
}

stub_tool_calls() {
    printf '%s\n' "${CALLS}" | grep -cxF -- "$1" || true
}

write_stubs
load_publish_env

# Credential/npm/asset defaults for the replay. Every case overrides the field it
# cares about, so the positive control is explicit rather than incidental.
FIXTURE_CARGO_AUTH=0
FIXTURE_NPM_PACKUMENT="${TEST_DIR}/npm_attested.json"
FIXTURE_GH_ASSETS="${TEST_DIR}/assets_all.txt"
# 9.9.9 is not a real release: no row in any fixture index can be mistaken for live
# history, and a case that forgets to set the version fails loudly instead of
# accidentally matching a version that exists on crates.io.
FIXTURE_VERSION="9.9.9"
FIXTURE_DRY_RUN="false"
FIXTURE_NO_TOKEN="0"
FIXTURE_INDEX_LIVE=""
export FIXTURE_CARGO_AUTH FIXTURE_NPM_PACKUMENT FIXTURE_GH_ASSETS
export FIXTURE_VERSION FIXTURE_DRY_RUN FIXTURE_NO_TOKEN FIXTURE_INDEX_LIVE

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
if ! extract_step "${TEST_DIR}/vr.sh" "${WORKFLOW}" verify-release "Verify complete" "${FIXTURE_VERSION}"; then
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

printf '\n== the published set has one source across both workflow files ==\n'
# release.yml's verifier and publish-crates.yml's publisher each carry the crate set,
# and a set that drifts between them is the bug class this repository keeps re-finding:
# `check-goap-queue-issues.sh` and ci-workspace-crate-matrix.sh both exist for exactly
# this reason ("a comment is not a gate"). So the two lists are read out of the files
# and compared, and every count below is derived from them rather than written here —
# a hardcoded 7 would only prove the fixture remembers last year's crate set.
if ! python3 - "${PUBLISH_WORKFLOW}" "${WORKFLOW}" "${TEST_DIR}" <<'PY'
import re, sys, yaml
publish_wf, release_wf, test_dir = sys.argv[1:4]
pub_env = yaml.safe_load(open(publish_wf, encoding='utf-8'))['jobs']['publish']['env']
publish_set = pub_env['CSM_PUBLISH_CRATES'].split()
own_steps = {pub_env['CSM_ROOT_CRATE'], pub_env['CSM_DUCKDB_CRATE']}
member_set = [c for c in publish_set if c not in own_steps]
verifier = [s for s in yaml.safe_load(open(release_wf, encoding='utf-8'))['jobs']
            ['verify-release']['steps'] if str(s.get('name', '')).startswith('Verify complete')]
if len(verifier) != 1:
    sys.exit('fixture: expected 1 verify-release step, found %d' % len(verifier))
match = re.search(r'^[ \t]*CRATES_PUBLISHED="([^"]*)"', verifier[0]['run'], re.M)
if not match:
    sys.exit('fixture: verify-release no longer builds CRATES_PUBLISHED as one list')
verify_set = match.group(1).split()
for name, rows in (('publish_set', publish_set), ('member_set', member_set),
                   ('verify_set', verify_set)):
    open('%s/%s.txt' % (test_dir, name), 'w', encoding='utf-8').write('\n'.join(rows) + '\n')
PY
then
    printf 'ERROR: could not compare the published crate sets\n' >&2
    exit 2
fi
PUBLISH_TOTAL="$(wc -l < "${TEST_DIR}/publish_set.txt" | tr -d ' ')"
PUBLISH_MEMBERS="$(wc -l < "${TEST_DIR}/member_set.txt" | tr -d ' ')"
PUBLISH_UNIQUE="$(sort -u "${TEST_DIR}/publish_set.txt" | wc -l | tr -d ' ')"
expect_eq "publish-crates.yml and release.yml publish the same set" \
    "$(cat "${TEST_DIR}/publish_set.txt")" "$(cat "${TEST_DIR}/verify_set.txt")"
expect_eq "the crate set has no duplicates" "${PUBLISH_TOTAL}" "${PUBLISH_UNIQUE}"
expect_eq "exactly the root crate and csm-duckdb leave the member loop" \
    "$(( PUBLISH_TOTAL - 2 ))" "${PUBLISH_MEMBERS}"

printf '\n== #971: publish-crates no longer re-publishes what is already live ==\n'
if ! extract_step "${TEST_DIR}/pc.sh" "${PUBLISH_WORKFLOW}" publish "Publish workspace crates" "${FIXTURE_VERSION}"; then
    printf 'ERROR: could not extract the member publish step from %s\n' "${PUBLISH_WORKFLOW}" >&2
    exit 2
fi
set_registry_state "${TEST_DIR}/index_present.json" yes yes
run_body "${TEST_DIR}/pc.sh"
expect_eq "publish step exits 0" "0" "${BODY_RC}"
expect_eq "publish step: zero cargo publishes for a live version" "0" "$(stub_tool_calls cargo)"
expect_eq "publish step: every member skipped when all are live" \
    "${PUBLISH_MEMBERS}" "$(count_in_stdout 'already on crates.io, skipping')"

set_registry_state "${TEST_DIR}/index_absent.json" yes yes
run_body "${TEST_DIR}/pc.sh"
expect_eq "publish step: one publish per member when none is live" \
    "${PUBLISH_MEMBERS}" "$(stub_tool_calls cargo)"
expect_eq "publish step: one index probe per member" \
    "${PUBLISH_MEMBERS}" "$(stub_tool_calls curl)"

printf '\n== workflow file: the splice and the bare call must not come back ==\n'
# Comments are excluded, and deliberately so: this file's own rationale comment
# quotes the broken pattern to explain the fix, and a gate that reads prose as
# code fails on the documentation of the repair (the same trap as the generated
# script catalog's caller columns). An executable line never starts with `#`.
# Both workflow files are scanned: #971 was proven in release.yml and the pattern is
# now re-expressed in publish-crates.yml, where a re-splice would re-publish nine
# crates that are already live.
for scanned in "${WORKFLOW}" "${PUBLISH_WORKFLOW}"; do
    if sed '/^[[:space:]]*#/d' "${scanned}" | grep -qn 'grep -q ""'; then
        fail "$(basename "${scanned}") contains a quote-spliced grep pattern again"
        sed '/^[[:space:]]*#/d' "${scanned}" | grep -n 'grep -q ""' >&2
    else
        pass "$(basename "${scanned}") has no quote-spliced grep pattern"
    fi
done
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


printf '\n== ADR-0104: the crates.io credential comes from the OIDC exchange ==\n'
# Structural, not behavioural: no replay can show that a secret is *not referenced*,
# and the whole point of the migration is that the file stops mentioning one. These
# are the properties that keep the next editor from "just adding back" the
# CARGO_REGISTRY_TOKEN that cost nine 403s on v0.3.9 (#967) — a fix that worked, but
# only until the token expired again, which is exactly what the registry's 90-day
# default did.
#
# The #967 credential *probe* it replaces is gone, and deliberately so: crates.io
# serves /api/v1/me behind AuthCheck::only_cookie(), so that step could never see a
# token as a token. What remains is the one question the runner can actually answer —
# did the exchange return anything — replayed below.
if ! structural_checks; then
    printf 'ERROR: the structural checker could not run\n' >&2
    exit 2
fi

printf '\n== ADR-0104: an exchange that returns nothing publishes nothing ==\n'
if ! extract_step "${TEST_DIR}/guard.sh" "${PUBLISH_WORKFLOW}" publish "Refuse to publish on an empty token" "${FIXTURE_VERSION}"; then
    printf 'ERROR: could not extract the empty-token guard from %s\n' "${PUBLISH_WORKFLOW}" >&2
    exit 2
fi
FIXTURE_NO_TOKEN=0
export FIXTURE_NO_TOKEN
run_body "${TEST_DIR}/guard.sh"
expect_eq "a returned token lets the job continue" "0" "${BODY_RC}"
expect_eq "a returned token calls no tool" "0" "$(stub_tool_calls cargo)"
expect_not_has "the guard never prints the token" "fixture-token"

FIXTURE_NO_TOKEN=1
export FIXTURE_NO_TOKEN
run_body "${TEST_DIR}/guard.sh"
expect_eq "an empty exchange reds the guard" "1" "${BODY_RC}"
expect_has "it names the credential class" "::error title=Crates.io OIDC::"
expect_not_has "it does not leak an empty value into a publish" "cargo publish"
FIXTURE_NO_TOKEN=0
export FIXTURE_NO_TOKEN

printf '\n== ADR-0104: the reconciler turns a partial publish into a red run ==\n'
# crates.io scopes the exchange token to the subset of crates whose trusted publishers
# match, so one missing config publishes seven and 403s two — and the publish loop
# above reports success for the seven it did. Without this step the migration would
# re-create the 0.3.9 shape: a green release with a registry that never got the crate.
if ! extract_step "${TEST_DIR}/recon.sh" "${PUBLISH_WORKFLOW}" publish "Reconcile the sparse index" "${FIXTURE_VERSION}"; then
    printf 'ERROR: could not extract the reconciliation step from %s\n' "${PUBLISH_WORKFLOW}" >&2
    exit 2
fi
printf 'csm-chaos\nchaotic_semantic_memory\n' > "${TEST_DIR}/live_two.txt"
FIXTURE_INDEX_LIVE="${TEST_DIR}/live_two.txt"
export FIXTURE_INDEX_LIVE
run_body "${TEST_DIR}/recon.sh"
expect_eq "seven of nine live is a failure, not a warning" "1" "${BODY_RC}"
expect_eq "two verdicts are credited" "2" "$(count_in_stdout ': Published')"
expect_eq "the rest are named Missing" "$(( PUBLISH_TOTAL - 2 ))" "$(count_in_stdout ': Missing')"
expect_has "the aggregate lists the gap" "::error title=Partial publish::"
expect_has "it says what closes the gap" "re-dispatch this workflow"
FIXTURE_INDEX_LIVE=""
export FIXTURE_INDEX_LIVE

set_registry_state "${TEST_DIR}/index_present.json" yes yes
run_body "${TEST_DIR}/recon.sh"
expect_eq "all nine live exits 0" "0" "${BODY_RC}"
expect_eq "one verdict row per crate" "${PUBLISH_TOTAL}" "$(count_in_stdout ': Published')"
expect_has "and the run says so" "crates are live at ${FIXTURE_VERSION}"

set_registry_state "${TEST_DIR}/index_absent.json" yes yes
FIXTURE_DRY_RUN=true
export FIXTURE_DRY_RUN
run_body "${TEST_DIR}/recon.sh"
expect_eq "a dry run reconciles nothing and passes" "0" "${BODY_RC}"
expect_eq "a dry run probes the index zero times" "0" "$(stub_tool_calls curl)"
FIXTURE_DRY_RUN=false
export FIXTURE_DRY_RUN

printf '\n== #967: an authentication failure aborts the publish loop ==\n'
set_registry_state "${TEST_DIR}/index_absent.json" yes yes
FIXTURE_CARGO_AUTH=1
export FIXTURE_CARGO_AUTH
: > "${TEST_DIR}/argv.txt"
run_body "${TEST_DIR}/pc.sh"
expect_eq "the auth failure reds the step" "1" "${BODY_RC}"
expect_eq "it aborts after one crate, not the whole set" "1" "$(stub_tool_calls cargo)"
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
