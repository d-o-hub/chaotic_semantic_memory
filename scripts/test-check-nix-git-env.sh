#!/usr/bin/env bash
# Fixture for scripts/check-nix-git-env.sh (issue #949).
#
# Every case builds a throwaway tree — two workflow files, a flake.nix, and the two
# scripts a wrapped step execs — copies the checker in, and asserts the checker's
# failure direction. The tree is synthetic on purpose: the gate's subject is the shape
# of a step body, and a fixture that read the real repository could only ever assert
# the one state the repository happens to be in.
#
# The directions that matter, in the order they are exercised:
#   the measured bug (cargo deny under the wrapper, scrub lost) -> red;
#   the measured fix (same line, env -u LD_LIBRARY_PATH)        -> green;
#   the measured control (unwrapped cargo deny)                 -> green;
#   a host helper written straight into a nix step              -> red;
#   prose that only quotes a helper                             -> green;
#   a helper after `echo … &&`                                  -> red (quoting is not
#     an excuse for the whole line, only for the quoted span);
#   a wrapped step execing a script that is not in the tree     -> red (the leak sites are
#     discovered through that exec, so an unreadable one is the gate passing on silence);
#   a new nix job, a stale inventory entry, a devShell that sets LD_LIBRARY_PATH -> red;
#   an unreadable precondition -> exit 2, never a verdict.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/check-nix-git-env.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

RUNS=0
FAILED=0
ok() { RUNS=$((RUNS + 1)); echo "  ok: $1"; }
no() { RUNS=$((RUNS + 1)); FAILED=$((FAILED + 1)); echo "  FAIL: $1"; }

# ---------------------------------------------------------------- tree factory
# make_tree <name>: the repository as measured on 2026-11-04 — one wrapped lint job
# with the scrubbed cargo-deny line, one unwrapped cargo-deny job, one wrapped mutation
# shard job whose script uses local git plumbing only, and the `mutation-test` aggregator
# that scores the union without a devShell (so an inventory entry naming it is stale).
make_tree() {
    local root="${WORK}/$1"
    mkdir -p "${root}/.github/workflows" "${root}/scripts"
    cat >"${root}/.github/workflows/ci.yml" <<'YML'
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - name: LOC gate
        run: |
          nix develop --command scripts/run-with-libs.sh bash scripts/validate.sh
  cargo-deny:
    runs-on: ubuntu-latest
    steps:
      - name: cargo deny check
        run: nix develop --command cargo deny --locked check
  mutation-shard:
    runs-on: ubuntu-latest
    steps:
      - name: mut
        run: |
          nix develop --command scripts/run-with-libs.sh bash scripts/mutation_test.sh --ci fast
  mutation-test:
    runs-on: ubuntu-latest
    steps:
      - name: score the union
        run: |
          python3 scripts/mutation_union.py --artifacts shards --expected-shards 8
  test:
    runs-on: ubuntu-latest
    steps:
      - run: nix develop --command scripts/run-with-libs.sh cargo test --locked
  wasm:
    runs-on: ubuntu-latest
    steps:
      - run: nix develop --command cargo check -p csm-wasm
YML
    cat >"${root}/.github/workflows/benchmark-ci.yml" <<'YML'
jobs:
  benchmark-small:
    runs-on: ubuntu-latest
    steps:
      - name: bench
        run: nix develop --command scripts/run-with-libs.sh cargo test --manifest-path benchmarks/Cargo.toml
YML
    cat >"${root}/flake.nix" <<'NIX'
            env = {
              RUST_SRC_PATH = "x";
              CSM_LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];
            };
NIX
    # The entrypoint a wrapped step execs, in the shipped shape: it is a wrapped-script
    # name in every one of those step bodies, so the audit has to be able to read it.
    cat >"${root}/scripts/run-with-libs.sh" <<'SH'
#!/usr/bin/env bash
if [[ -n "${CSM_LD_LIBRARY_PATH:-}" ]]; then
  export LD_LIBRARY_PATH="${CSM_LD_LIBRARY_PATH}"
fi
exec "$@"
SH
    # The two scripts a wrapped step execs, in the shipped shape.
    cat >"${root}/scripts/validate.sh" <<'SH'
#!/usr/bin/env bash
echo "==> cargo deny --locked check"
env -u LD_LIBRARY_PATH cargo deny --locked check
SH
    cat >"${root}/scripts/mutation_test.sh" <<'SH'
#!/usr/bin/env bash
git rev-parse --verify HEAD
git diff "${BASE}...HEAD" >"${DIFF_FILE}"
SH
    printf '%s' "${root}"
}

run_check() { # run_check <root> <case> [<script>] -> status; log at ${WORK}/<case>.log
    local status=0
    bash "${3:-${SUBJECT}}" "$1" >"${WORK}/$2.log" 2>&1 || status=$?
    echo "${status}"
}

expect() { # expect <expected-status> <description> <root> <case>
    local got
    got="$(run_check "$3" "$4")"
    if [[ "${got}" == "$1" ]]; then
        ok "$2 (exit ${got})"
        return
    fi
    no "$2 — expected exit $1, got ${got}"
    sed 's/^/    /' "${WORK}/$4.log"
}

expect_has() { # expect_has <description> <needle> <case>
    if grep -qF -- "$2" "${WORK}/$3.log"; then
        ok "$1"
    else
        no "$1 — missing [$2]"
        sed 's/^/    /' "${WORK}/$3.log"
    fi
}

expect_lacks() { # expect_lacks <description> <needle> <case>
    if grep -qF -- "$2" "${WORK}/$3.log"; then
        no "$1 — unexpected [$2]"
        sed 's/^/    /' "${WORK}/$3.log"
    else
        ok "$1"
    fi
}

# ------------------------------------------------------------- what this needs to run
if [[ ! -r "${SUBJECT}" ]]; then
    printf 'ERROR: cannot read %s; the fixture has no subject\n' "${SUBJECT}" >&2
    exit 2
fi
if ! python3 -c 'import yaml' 2>/dev/null; then
    # Same convention as scripts/test-verify-release-step.sh: an optional run says out loud
    # that it checked nothing, a required one fails. A fixture that skips quietly is how a
    # gate rots into a pass-for-the-wrong-reason (#846's orphaned-fixture class).
    if [[ "${CSM_VALIDATION_REQUIRED:-false}" == "true" ]]; then
        printf 'ERROR: PyYAML is required by this fixture; install python3-yaml.\n' >&2
        exit 2
    fi
    printf 'SKIPPED — NOT A PASS: python3 has no PyYAML (apt: python3-yaml); nix step bodies went un-replayed\n'
    exit 0
fi

# ------------------------------------------------------------------- case 0: baseline
echo "== the shipped tree audits clean =="
BASE="$(make_tree baseline)"
# scripts/validate.sh calls both by path, so the exec bit is part of the contract: a
# 644 gate trips validate.sh's "missing or not executable" branch, which reads like the
# file was deleted.
for probe_script in "${SUBJECT}" "${BASH_SOURCE[0]}"; do
    if [[ -x "${probe_script}" ]]; then
        ok "$(basename "${probe_script}") is executable (validate.sh calls it by path)"
    else
        no "$(basename "${probe_script}") is not executable: validate.sh treats that as missing"
    fi
done
expect 0 "the measured tree (scrubbed deny line, unwrapped deny control) passes" "${BASE}" base
expect_has "the pass prints what it counted" "no unscrubbed host helper" base

# Sharpness of rule 2: the wrapped script really was read. If wrapped_scripts were
# ever empty the gate would report clean while scanning nothing.
expect_has "the pass names the wrapped entrypoints it scanned" "3 wrapped entrypoint script(s) scanned" base

# --print-inventory is the durable form of the per-job table issue #949 asked for, and the
# distinction the whole argument rests on is wrapped vs plain: the wrapped jobs are the
# leak class, the plain `cargo deny` job is the control that falsified the runner theory.
# Columns are matched by awk so the assertion does not encode printf's padding.
inv_rc=0
bash "${SUBJECT}" --print-inventory "${BASE}" >"${WORK}/inv.log" 2>&1 || inv_rc=$?
if [[ ${inv_rc} -eq 0 ]]; then
    ok "--print-inventory is a view, not a verdict: exit 0 on a clean tree"
else
    no "--print-inventory exited ${inv_rc} on a clean tree"
    sed 's/^/    /' "${WORK}/inv.log"
fi
expect_has "the inventory lists the wrapped lint job" "ci.yml::lint" inv
for row in "ci.yml::lint wrapped" "ci.yml::cargo-deny plain" "ci.yml::wasm plain"; do
    # shellcheck disable=SC2086 # the split IS the parse: `row` holds two fields, used as $1/$2
    set -- ${row}
    if awk -v job="$1" -v ctx="$2" '$1 == job && $3 == ctx { found = 1 } END { exit !found }' \
        "${WORK}/inv.log"; then
        ok "the inventory marks $1 as $2"
    else
        no "the inventory does not mark $1 as $2"
        sed 's/^/    /' "${WORK}/inv.log"
    fi
done

# ------------------------------------------------------- case 1: the measured bug
echo "== the measured bug: cargo deny under the wrapper, scrub lost =="
BUGGY="$(make_tree buggy)"
sed -i 's/^env -u LD_LIBRARY_PATH cargo deny/cargo deny/' "${BUGGY}/scripts/validate.sh"
expect 1 "losing the scrub on the advisory-db clone is a finding" "${BUGGY}" buggy
expect_has "the finding names the file and line" "scripts/validate.sh:3" buggy
expect_has "the finding refuses the runner-label explanation" "Runner labels are not the fix" buggy

echo "== the fix, re-applied: same line, same file =="
FIXED="$(make_tree fixed)"
expect 0 "the shipped scrubbed line passes" "${FIXED}" fixed
expect_lacks "a pass must not print a finding" "LEAK:" fixed

# ------------------------------------------------------ case 2: the measured control
echo "== the control: cargo deny with no wrapper at all =="
# The Cargo Deny job runs `nix develop --command cargo deny` and passes every time,
# because nix develop exports no LD_LIBRARY_PATH. A gate that flagged it would re-assert
# the runner-image theory this issue retired.
CONTROL="$(make_tree control)"
sed -i 's|^env -u LD_LIBRARY_PATH cargo deny|cargo deny|' "${CONTROL}/scripts/validate.sh"
# Also point the lint step at something unwrapped, so no wrapped script contains deny.
sed -i 's|scripts/run-with-libs.sh bash scripts/validate.sh|bash scripts/validate.sh|' \
    "${CONTROL}/.github/workflows/ci.yml"
expect 0 "unwrapped cargo deny is the control, not a finding" "${CONTROL}" control

# ------------------------------------------- case 3: a helper in the step body itself
echo "== a host helper written into a nix step body =="
DIRECT="$(make_tree direct)"
cat >>"${DIRECT}/.github/workflows/benchmark-ci.yml" <<'YML'
  fetcher:
    runs-on: ubuntu-latest
    steps:
      - name: grab
        run: nix develop --command git clone https://example.invalid/db
YML
expect 1 "git clone inside nix develop is a finding even unwrapped" "${DIRECT}" direct
expect_has "the finding names the job and the offending line" "benchmark-ci.yml::fetcher" direct
expect_has "the finding prints the line it objecting to" "git clone https://example.invalid/db" direct

# ------------------------------------------------------------ case 4: prose handling
echo "== quoting is not an excuse, and prose is not a command =="
PROSE="$(make_tree prose)"
cat >>"${PROSE}/scripts/validate.sh" <<'SH'
echo "==> cargo deny --locked check"
SH
expect 0 "a banner that only quotes the helper passes" "${PROSE}" prose

COMPOUND="$(make_tree compound)"
cat >>"${COMPOUND}/scripts/validate.sh" <<'SH'
echo "starting" && cargo deny --locked check
SH
expect 1 "a helper after && on an echo line is still a helper" "${COMPOUND}" compound
expect_has "the compound finding names the line" "scripts/validate.sh:4" compound

NIXPROSE="$(make_tree nixprose)"
cat >>"${NIXPROSE}/flake.nix" <<'NIX'
            # LD_LIBRARY_PATH = "documented as unset, so a comment is not a setting";
NIX
expect 0 "a commented-out devShell setting does not fire rule 4" "${NIXPROSE}" nixprose

# ------------------------------------------------ case 5: the wrapped script is missing
echo "== a wrapped step execing a script the tree cannot read =="
# The leak sites live inside the execed scripts, so a wrapped step pointing at something
# unreadable means rule 2 scanned nothing. Reporting clean there is the vacuous-gate
# failure mode in a second costume: the first was a pattern that cannot match, this is a
# file list that is empty.
GHOST="$(make_tree ghost)"
sed -i 's|bash scripts/validate.sh|bash scripts/absent.sh|' "${GHOST}/.github/workflows/ci.yml"
expect 1 "a wrapped step whose script is missing is a finding, not a skip" "${GHOST}" ghost
expect_has "the finding names the unreadable script" "scripts/absent.sh is execed under the wrapper" ghost

# ------------------------------------------------------ case 6: the flake moves class
echo "== a devShell that exports LD_LIBRARY_PATH itself =="
FLAKED="$(make_tree flaked)"
printf '            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];\n' >>"${FLAKED}/flake.nix"
expect 1 "the devShell setting the variable is a finding" "${FLAKED}" flaked
expect_has "the finding says the control stops being one" "Cargo Deny control" flaked

# ------------------------------------------------------ case 7: the inventory ratchet
echo "== a new nix job is not audited by default =="
NEWJOB="$(make_tree newjob)"
cat >>"${NEWJOB}/.github/workflows/ci.yml" <<'YML'
  extra:
    runs-on: ubuntu-latest
    steps:
      - run: nix develop --command cargo test -p csm-core-lib
YML
expect 1 "an un-inventoried nix job fails the audit" "${NEWJOB}" newjob
expect_has "the finding tells you what to do" "not in the audited inventory" newjob

DROPPED="$(make_tree dropped)"
python3 - "${DROPPED}" <<'PY'
import sys
p = f"{sys.argv[1]}/.github/workflows/ci.yml"
s = open(p, encoding="utf-8").read()
open(p, "w", encoding="utf-8").write(s.replace("nix develop --command cargo check -p csm-wasm",
                                              "cargo check -p csm-wasm"))
PY
expect 1 "an inventory entry that stopped using nix is stale" "${DROPPED}" dropped
expect_has "the stale-entry finding names the entry" "ci.yml::wasm is in the audited inventory" dropped

# ---------------------------------------------------------- case 8: exit 2 vs verdict
echo "== an unreadable precondition is exit 2, not a verdict =="
NOARGS="${WORK}/noargs"
mkdir -p "${NOARGS}"
status="$(run_check "${NOARGS}" nodir)"
if [[ "${status}" == "2" ]]; then
    ok "missing .github/workflows: exit 2"
else
    no "missing .github/workflows: exited ${status}, expected 2"
fi
sed 's/^/    /' "${WORK}/nodir.log"

NOFLAKE="$(make_tree noflake)"
rm -f "${NOFLAKE}/flake.nix"
status="$(run_check "${NOFLAKE}" noflake)"
if [[ "${status}" == "2" ]]; then
    ok "missing flake.nix: exit 2, the devShell env is a precondition"
else
    no "missing flake.nix: exited ${status}, expected 2"
fi

# A tree with no nix-develop step at all must not report clean: it means the audit had
# nothing to read, which is the difference between a verdict and a silence.
EMPTY="${WORK}/emptytree"
mkdir -p "${EMPTY}/.github/workflows" "${EMPTY}/scripts"
printf 'name: x\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo test\n' \
    >"${EMPTY}/.github/workflows/a.yml"
cp "${BASE}/flake.nix" "${EMPTY}/flake.nix"
status="$(run_check "${EMPTY}" emptywf)"
if [[ "${status}" == "2" ]]; then
    ok "a workflow set with no nix step: exit 2, not a pass"
else
    no "a workflow set with no nix step: exited ${status}, expected 2"
fi

# ----------------------------------------------------- case 9: a broken pattern cannot
# quietly pass. Corrupt the shipped WRAP_RE the way the first draft was corrupted (drop
# the closing paren, which glibc reads as a literal) and require exit 2 from the probe.
# The corruption is asserted to have happened: a replace() that silently matches nothing
# would test the shipped script and report a pass for the fixture.
echo "== the self-probe fires on a pattern that cannot match =="
BROKEN="$(make_tree broken)"
cp "${SUBJECT}" "${BROKEN}/check-nix-git-env.sh"
python3 - "${BROKEN}/check-nix-git-env.sh" <<'PY'
import sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
old = r"cargo[[:space:]]+deny)([[:space:]]|$)'"
if s.count(old) != 1:
    raise SystemExit(f"anchor found {s.count(old)} time(s), expected 1: {old}")
open(p, "w", encoding="utf-8").write(s.replace(old, old[:-1] + ")'"))
PY
py_rc=$?
if [[ ${py_rc} -ne 0 ]]; then
    no "the self-probe case could not corrupt WRAP_RE (the fixture is anchored wrong)"
fi
status="$(run_check "${BROKEN}" broken "${BROKEN}/check-nix-git-env.sh")"
if [[ "${status}" == "2" ]]; then
    ok "a WRAP_RE that matches nothing exits 2 instead of reporting clean"
else
    no "a WRAP_RE that matches nothing exited ${status}, expected 2 (the vacuous-gate bug)"
    sed 's/^/    /' "${WORK}/broken.log"
fi
expect_has "the probe names the pattern it rejected" "does not match its reference sample" broken
# Control on the same tree: the shipped checker passes it, so the exit 2 above is the
# probe, not the tree.
expect 0 "the uncorrupted checker still passes the same tree" "${BROKEN}" broken_control

printf '\nnix environment fixture: %s case(s), %s failure(s)\n' "${RUNS}" "${FAILED}"
[[ ${FAILED} -eq 0 ]] || exit 1
