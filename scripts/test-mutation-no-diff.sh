#!/usr/bin/env bash
# Fixture for scripts/mutation_test.sh's "zero mutants" classification (PR #977).
#
# WHY THIS EXISTS
# The old preflight ran `cargo mutants --list … 2>/dev/null || true` and then
# mapped ANY empty result to one of two outcomes: exit 1 in CI ("selected this
# Rust diff but generated no mutants") or exit 0 locally. Both were wrong half
# the time, because "empty" had four different causes and the script could not
# tell them apart:
#   1. the probe itself failed (parse error, bad scope) — a broken gate;
#   2. the diff selects production Rust but no changed line carries a mutant —
#      no evidence exists, which is not the PR's fault;
#   3. the diff selects no production Rust at all;
#   4. mutants DO exist and the run should proceed.
# Cause 2 was reported as cause 1 for PR #977, and the real reason was a fifth
# thing nobody had named: `cargo mutants` with no `-p` mutates the *current
# package*, and the root manifest is both a package and a workspace, so every
# `crates/**` file was invisible to generation (measured: 1259 mutants listed,
# none under `crates/`) even though ci.yml:127 sets `mutation=true` for those
# paths. Case 5 below is the regression pin: generation must carry the same
# package scope as the tests.
#
# Every case stubs `cargo` and `git` on PATH, so no build runs; the assertions
# are about the gate's classification, its exit code, and what it writes.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/mutation_test.sh"
# Must match the script's own REPO_ROOT: package resolution is anchored there,
# so the metadata fixture has to describe the same tree the script walks.
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT
mkdir -p "${TEST_DIR}/bin" "${TEST_DIR}/output" "${TEST_DIR}/mutants.out"

# A consistent cargo-mutants 27.1.0 report, so scripts/mutation_inventory.py can
# run against the positive-control case (its aggregate cross-check is fatal).
cat >"${TEST_DIR}/mutants.out/outcomes.json" <<'JSON'
{
  "cargo_mutants_version": "27.1.0",
  "caught": 2, "missed": 0, "timeout": 0, "unviable": 0, "success": 0,
  "total_mutants": 2,
  "outcomes": [
    {"scenario": "Baseline", "summary": "Success"},
    {"scenario": {"Mutant": {"file": "src/example.rs", "name": "replace a with ()"}}, "summary": "CaughtMutant"},
    {"scenario": {"Mutant": {"file": "crates/nested/src/lib.rs", "name": "replace + with - in b"}}, "summary": "CaughtMutant"}
  ]
}
JSON

cat >"${TEST_DIR}/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
if [[ "${1:-}" == "mutants" && "${2:-}" == "--version" ]]; then
    echo "cargo-mutants 27.1.0"
    exit 0
fi
if [[ "${1:-}" == "mutants" && "${2:-}" == "--help" ]]; then
    echo "--in-diff"
    exit 0
fi
if [[ "${1:-}" == "metadata" ]]; then
    if [[ "${FAKE_METADATA_MODE:-ok}" == "fail" ]]; then
        echo "error: failed to load manifest for workspace" >&2
        exit 101
    fi
    printf '{"packages":[{"name":"example","manifest_path":"%s/Cargo.toml"},{"name":"nested","manifest_path":"%s/crates/nested/Cargo.toml"}]}\n' \
        "${FAKE_ROOT}" "${FAKE_ROOT}"
    exit 0
fi
if [[ "${1:-}" == "mutants" && "${2:-}" == "--list" ]]; then
    printf '%s\n' "$*" >>"${FIXTURE_ARGV}"
    case "${FAKE_LIST_MODE:-empty}" in
        empty)
            exit 0
            ;;
        rc)
            echo "error: could not read the diff file" >&2
            exit 3
            ;;
        mutants | mutants_split | mutants_missed)
            echo "src/example.rs:12:5: replace example::alpha -> u32 with 0"
            echo "crates/nested/src/lib.rs:4:9: replace + with - in nested::beta"
            exit 0
            ;;
        rootonly)
            # One arm has a mutant on a changed line and the other has none. `--list`
            # commands carry only the generation-side scope, so this substring is
            # unambiguous. An empty arm must be skipped, not veto the whole gate.
            if [[ "$*" == *"-p chaotic_semantic_memory"* ]]; then
                echo "src/example.rs:12:5: replace example::alpha -> u32 with 0"
            fi
            exit 0
            ;;
    esac
fi
if [[ "${1:-}" == "mutants" ]]; then
    printf '%s\n' "$*" >>"${FIXTURE_ARGV}"
    if [[ "${FAKE_LIST_MODE:-empty}" == "mutants_missed" ]]; then
        echo "2 mutants tested in 20.0s: 1 caught, 1 missed, 0 unviable, 0 timeouts"
        exit 2
    fi
    if [[ "${FAKE_LIST_MODE:-empty}" != "mutants_split" ]]; then
        echo "2 mutants tested in 20.0s: 2 caught, 0 missed, 0 unviable, 0 timeouts"
        exit 0
    fi
    # `mutants_split`: per-arm summaries whose scores *disagree*, which makes the
    # aggregate a discriminator rather than a formality. The run command carries its
    # package scope before the `--` separator, so the mock can tell the root arm
    # (2 of 3 caught) from the crates arm (5 of 5). Union = 7/8 = 87.5000% (clears
    # 85); an average of the two arm scores = 83.3333% (fails). So a regression back
    # to per-arm or averaged scoring turns the positive control red.
    # Join before cutting: `${*%% -- *}` applies the operator to each positional
    # parameter separately, which never matches and silently leaves the test-side
    # args in place -- both arms then read as the root arm (measured: 6/4 instead
    # of 8/7).
    joined="$*"
    gen="${joined%% -- *}"
    if [[ "${gen}" == *"-p chaotic_semantic_memory"* ]]; then
        echo "3 mutants tested in 30.0s: 2 caught, 1 missed, 0 unviable, 0 timeouts"
    else
        echo "5 mutants tested in 50.0s: 5 caught, 0 missed, 0 unviable, 0 timeouts"
    fi
    exit 0
fi
echo "unexpected cargo command: $*" >&2
exit 2
MOCK
cat >"${TEST_DIR}/bin/git" <<'MOCK'
#!/usr/bin/env bash
# The gate runs every git call as `git -C <workspace-root> …`; drop the prefix so
# the mock sees the subcommand in $1 the way a developer's shell would.
if [[ "${1:-}" == "-C" ]]; then
    shift 2
fi
if [[ "${1:-}" == "rev-parse" ]]; then
    if [[ "${FAKE_GIT_MODE:-empty}" == "empty" ]]; then
        exit 1
    fi
    echo "deadbeef"
    exit 0
fi
if [[ "${1:-}" == "diff" ]]; then
    case "${FAKE_GIT_MODE:-empty}" in
        empty) ;;
        rust|nomutants|metafail)
            printf '%s\n' \
                'diff --git a/src/example.rs b/src/example.rs' \
                '+++ b/src/example.rs' \
                '@@ -10,2 +10,2 @@' \
                '+fn changed() {}'
            ;;
        crates)
            printf '%s\n' \
                'diff --git a/crates/nested/src/lib.rs b/crates/nested/src/lib.rs' \
                '+++ b/crates/nested/src/lib.rs' \
                '@@ -2,2 +2,2 @@' \
                '+    let x = a + b;'
            ;;
        docs)
            printf '%s\n' \
                'diff --git a/plans/NOTES.md b/plans/NOTES.md' \
                '+++ b/plans/NOTES.md' \
                '@@ -1,1 +1,1 @@' \
                '+a prose edit'
            ;;
    esac
    exit 0
fi
exec /usr/bin/git "$@"
MOCK
chmod +x "${TEST_DIR}/bin/cargo" "${TEST_DIR}/bin/git"

RUNS=0
FAILED=0
ok() { RUNS=$((RUNS + 1)); echo "  ok: $1"; }
no() { RUNS=$((RUNS + 1)); FAILED=$((FAILED + 1)); echo "  FAIL: $1"; }

run_mutation() { # run_mutation <git-mode> <list-mode> <metadata-mode> <expected-status> <case-name>
    local mode="$1" list="$2" meta="$3" expected="$4" name="$5"
    : >"${TEST_DIR}/argv.txt"
    local output_file="${TEST_DIR}/case-${name}.log"
    local status=0
    (
        cd "${TEST_DIR}"
        FAKE_GIT_MODE="${mode}" \
            FAKE_LIST_MODE="${list}" \
            FAKE_METADATA_MODE="${meta}" \
            FAKE_ROOT="${REPO_ROOT}" \
            FIXTURE_ARGV="${TEST_DIR}/argv.txt" \
            MUTATION_OUTPUT_DIR="${TEST_DIR}/output/${name}" \
            DIFF_TARGET=origin/main \
            PATH="${TEST_DIR}/bin:${PATH}" \
            bash "${SUBJECT}" --ci fast
    ) >"${output_file}" 2>&1 || status=$?
    if [[ ${status} -ne ${expected} ]]; then
        no "${name}: exited ${status}, expected ${expected}"
        cat "${output_file}"
    else
        ok "${name}: exit ${status}"
    fi
    MUT_LOG="${output_file}"
    MUT_REPORT="${TEST_DIR}/output/${name}/fast-latest.md"
}

has() {
    if grep -qF -- "$2" "${MUT_LOG}"; then ok "$1"; else no "$1"; cat "${MUT_LOG}"; fi
}
lacks() {
    if grep -qF -- "$2" "${MUT_LOG}"; then no "$1"; else ok "$1"; fi
}
report_has() {
    if grep -qF -- "$2" "${MUT_REPORT}"; then ok "$1"; else no "$1"; cat "${MUT_REPORT}" 2>/dev/null; fi
}

echo
echo "== an unbounded CI run is still not a result =="
run_mutation empty empty ok 2 unbounded
if ! grep -q "no bounded diff.*mutation tests did not run" "${MUT_LOG}" \
    || ! grep -q "not run (no bounded diff" "${MUT_REPORT}"; then
    no "an unbounded CI run was not recorded as not run"
else
    ok "an unbounded CI run fails closed as not run"
fi

echo
echo "== a broken scope resolution fails the gate (cause 1) =="
run_mutation rust empty fail 1 metafail
has "cargo metadata failure names the resolution gap" "cannot resolve which packages the diff touches"
has "cargo metadata failure says what broke" "cargo metadata failed"
lacks "a metadata failure must not be dressed up as a warning pass" "::warning title=Mutation gate::"
lacks "a metadata failure must not be reported as zero mutants" "generated no mutants"

echo
echo "== zero mutants on selected production Rust is no evidence, not a red PR (cause 2) =="
run_mutation nomutants empty ok 0 zeromut
has "the zero-mutant CI case warns instead of failing" "::warning title=Mutation gate::0 mutation-visible lines"
has "the warning refuses to read as a pass" "this is NOT a passing mutation result"
report_has "the report records the inspected package scope" "Packages inspected: csm-core-lib"
report_has "the report says NOT A PASS" "no changed line is mutation-visible; NOT A PASS"

echo
echo "== a diff with no production Rust is a different answer (cause 3) =="
run_mutation docs empty ok 0 norust
lacks "a docs-only diff must not claim a mutation-visible-line warning" "0 mutation-visible lines"
report_has "a docs-only diff is reported as selecting no production Rust" "no production Rust source selected by the diff; NOT A PASS"

echo
echo "== positive control: mutants exist, the gate scores them (cause 4) =="
run_mutation rust mutants_split ok 0 scored
has "a diff with mutants reaches the score" "unit-target mutation score 87.5000% >= 85%"
has "the score is the union of the arms, not an average" "caught=7 missed=1 timeout=0 unviable=0"
lacks "neither arm is scored alone" "unit-target mutation score 100.0000%"
lacks "a diff with mutants must not warn about zero evidence" "::warning title=Mutation gate::0 mutation-visible lines"
report_has "the report states the derived target set" "- Target Set: lib target only (--lib -p csm-core-lib"

# The regression that caused all of this: `-p` must appear on the generation side
# (before `--`), not only on the test side. Root-manifest discovery is what made
# every crates/** diff read as "no mutants".
#
# Generation is now split into arms, so the old "one line carries all three -p"
# assertion no longer describes the command. The surviving invariant is the union:
# every package in scope reaches some arm's generation.
if grep -qE '^mutants --list [^&]*-p chaotic_semantic_memory' "${TEST_DIR}/argv.txt" &&
  grep -qE '^mutants --list [^&]*-p csm-core-lib -p csm-retrieval' "${TEST_DIR}/argv.txt"; then
    ok "mutant generation carries the package scope (union of arms)"
else
    no "mutant generation carries the package scope (union of arms)"
    cat "${TEST_DIR}/argv.txt"
fi
# ...and the split exists for a reason: cargo-mutants emits one build command per
# selected package, so a package-qualified feature is invalid anywhere else. A
# future edit that folds the arms back together reproduces "cargo build failed in
# an unmutated tree" -- measured, not theorised. Only the generation side is
# checked: every arm's *test* command carries the whole union by design.
if grep -E '^mutants --list ' "${TEST_DIR}/argv.txt" | grep -F -- '--features' | grep -qF -- '-p csm-core-lib'; then
    no "the mcp feature rides only with the root package arm"
else
    ok "the mcp feature rides only with the root package arm"
fi
if grep -qE -- '-- --lib -p csm-core-lib -p csm-retrieval -p chaotic_semantic_memory' "${TEST_DIR}/argv.txt"; then
    ok "the test invocation keeps its --lib package scope"
else
    no "the test invocation keeps its --lib package scope"
    cat "${TEST_DIR}/argv.txt"
fi

echo
echo "== an arm with no mutants is skipped, and does not veto the arm that has some =="
run_mutation rust rootonly ok 0 asymmetric
has "one arm's evidence still scores the gate" "unit-target mutation score 100.0000% >= 85%"
report_has "the preflight records the empty arm as 0, not as a failure" "- Preflight: root crates = 1 0"
RUNS_CMD="$(grep -cE '^mutants --in-diff ' "${TEST_DIR}/argv.txt" || true)"
if [[ "${RUNS_CMD}" -eq 1 ]]; then
    ok "the arm that generated nothing is not built or tested"
else
    no "the arm that generated nothing is not built or tested: ${RUNS_CMD} run commands"
    cat "${TEST_DIR}/argv.txt"
fi

echo
echo "== a crates/**-only diff pulls its own package into scope =="
run_mutation crates mutants ok 0 cratesonly
if grep -q "diff selects package nested — added to the mutation scope" "${MUT_LOG}"; then
    ok "a crates/** diff adds its owning package to the scope"
else
    no "a crates/** diff adds its owning package to the scope"
    cat "${MUT_LOG}"
fi
if grep -qE '^mutants --list [^&]*-p nested' "${TEST_DIR}/argv.txt" \
    && grep -qE -- '-- --lib [^&]*-p nested' "${TEST_DIR}/argv.txt"; then
    ok "the derived package reaches mutant generation, not only the tests"
else
    no "the derived package reaches mutant generation, not only the tests"
    cat "${TEST_DIR}/argv.txt"
fi

echo
echo "== a --list probe that errors must not be read as 'no mutants' =="
: >"${TEST_DIR}/argv.txt"
status=0
(
    cd "${TEST_DIR}"
    FAKE_GIT_MODE=rust \
        FAKE_LIST_MODE=rc \
        FAKE_METADATA_MODE=ok \
        FAKE_ROOT="${REPO_ROOT}" \
        FIXTURE_ARGV="${TEST_DIR}/argv.txt" \
        MUTATION_OUTPUT_DIR="${TEST_DIR}/output/probefail" \
        DIFF_TARGET=origin/main \
        PATH="${TEST_DIR}/bin:${PATH}" \
        bash "${SUBJECT}" --ci fast
) >"${TEST_DIR}/probefail.out" 2>&1 || status=$?
if [[ ${status} -eq 1 ]]; then
    ok "a failing probe exits 1 (fail closed)"
else
    no "a failing probe exits 1 (fail closed): got ${status}"
fi
if grep -qE "mutation preflight failed in arm '[a-z-]+' \(exit 3\)" "${TEST_DIR}/probefail.out"; then
    ok "a failing probe names the failing arm and its exit code"
else
    no "a failing probe names the failing arm and its exit code"
    cat "${TEST_DIR}/probefail.out"
fi
if grep -qF "could not read the diff file" "${TEST_DIR}/probefail.out"; then
    ok "the probe's stderr is surfaced instead of swallowed"
else
    no "the probe's stderr is surfaced instead of swallowed"
fi
if grep -qF "generated no mutants" "${TEST_DIR}/probefail.out"; then
    no "a broken probe must not reuse the old zero-mutant message"
else
    ok "a broken probe does not claim 'generated no mutants'"
fi

echo
echo "== shard mode produces evidence and hands the verdict to the aggregator =="
: >"${TEST_DIR}/argv.txt"
status=0
(
    cd "${TEST_DIR}"
    FAKE_GIT_MODE=rust \
        FAKE_LIST_MODE=mutants_missed \
        FAKE_METADATA_MODE=ok \
        FAKE_ROOT="${REPO_ROOT}" \
        FIXTURE_ARGV="${TEST_DIR}/argv.txt" \
        MUTANTS_SHARD=3/8 \
        MUTATION_OUTPUT_DIR="${TEST_DIR}/output/shardrun" \
        DIFF_TARGET=origin/main \
        PATH="${TEST_DIR}/bin:${PATH}" \
        bash "${SUBJECT}" --ci fast
) >"${TEST_DIR}/shardrun.out" 2>&1 || status=$?
if [[ "${status}" -eq 0 ]]; then
    ok "a shard with missed mutants exits 0 (the aggregator scores the union)"
else
    no "a shard exited ${status}; it must not enforce the threshold on its slice"
    cat "${TEST_DIR}/shardrun.out"
fi
if grep -q -- '--shard 2/8' "${TEST_DIR}/argv.txt"; then
    ok "the 1-based shard id reaches cargo mutants as its 0-based slice"
else
    no "the shard translation is missing from the cargo mutants argv (want --shard 2/8)"
    cat "${TEST_DIR}/argv.txt"
fi
if grep -q -- '--shard 3/8' "${TEST_DIR}/argv.txt"; then
    no "the 1-based id must not reach cargo mutants verbatim: 3/8 is the fourth slice"
fi
if [[ -f "${TEST_DIR}/output/shardrun/shard-complete" ]] \
    && [[ "$(cat "${TEST_DIR}/output/shardrun/shard-complete")" == "3/8" ]]; then
    ok "a completed shard writes its completeness marker"
else
    no "a completed shard did not write shard-complete"
fi
if grep -q "aggregator owns the union verdict" "${TEST_DIR}/shardrun.out"; then
    ok "the shard log says the aggregator owns the verdict"
else
    no "the shard log does not hand off the verdict"
    cat "${TEST_DIR}/shardrun.out"
fi
if grep -q "CI check failed" "${TEST_DIR}/shardrun.out"; then
    no "a shard must not print the per-slice threshold failure"
else
    ok "no per-slice threshold failure is printed"
fi

echo
echo "== malformed shard ids fail before any cargo run =="
status=0
(
    cd "${TEST_DIR}"
    FAKE_GIT_MODE=rust \
        FAKE_LIST_MODE=mutants \
        FAKE_METADATA_MODE=ok \
        FAKE_ROOT="${REPO_ROOT}" \
        FIXTURE_ARGV="${TEST_DIR}/argv.txt" \
        MUTANTS_SHARD=3of8 \
        MUTATION_OUTPUT_DIR="${TEST_DIR}/output/badshard" \
        DIFF_TARGET=origin/main \
        PATH="${TEST_DIR}/bin:${PATH}" \
        bash "${SUBJECT}" --ci fast
) >"${TEST_DIR}/badshard.out" 2>&1 || status=$?
if [[ "${status}" -eq 2 ]] && grep -q "MUTANTS_SHARD must look like i/N" "${TEST_DIR}/badshard.out"; then
    ok "an invalid shard id is exit 2 with a useful message"
else
    no "an invalid shard id was not rejected (exit ${status})"
    cat "${TEST_DIR}/badshard.out"
fi

echo
echo "== shard boundaries: 1 and N translate, 0 and N+1 fail before any cargo run =="
for pair in 1/8:0/8 8/8:7/8 0/8:refuse 9/8:refuse; do
    id="${pair%%:*}"
    want="${pair#*:}"
    : >"${TEST_DIR}/argv.txt"
    status=0
    (
        cd "${TEST_DIR}"
        FAKE_GIT_MODE=rust \
            FAKE_LIST_MODE=mutants \
            FAKE_METADATA_MODE=ok \
            FAKE_ROOT="${REPO_ROOT}" \
            FIXTURE_ARGV="${TEST_DIR}/argv.txt" \
            MUTANTS_SHARD="${id}" \
            MUTATION_OUTPUT_DIR="${TEST_DIR}/output/boundary" \
            DIFF_TARGET=origin/main \
            PATH="${TEST_DIR}/bin:${PATH}" \
            bash "${SUBJECT}" --ci fast
    ) >"${TEST_DIR}/boundary.out" 2>&1 || status=$?
    if [[ "${want}" == "refuse" ]]; then
        if [[ "${status}" -eq 2 ]] && grep -q "MUTANTS_SHARD" "${TEST_DIR}/boundary.out"; then
            ok "MUTANTS_SHARD=${id} is refused with exit 2"
        else
            no "MUTANTS_SHARD=${id} was not refused (exit ${status})"
            cat "${TEST_DIR}/boundary.out"
        fi
        if [[ -s "${TEST_DIR}/argv.txt" ]]; then
            no "MUTANTS_SHARD=${id} reached cargo before the boundary check"
            cat "${TEST_DIR}/argv.txt"
        fi
    elif grep -q -- "--shard ${want}\b" "${TEST_DIR}/argv.txt"; then
        ok "MUTANTS_SHARD=${id} reaches cargo mutants as --shard ${want}"
    else
        no "MUTANTS_SHARD=${id} did not reach cargo mutants as --shard ${want}"
        cat "${TEST_DIR}/argv.txt"
    fi
done

echo
echo "== an empty shard slice writes no-mutants instead of failing =="
status=0
(
    cd "${TEST_DIR}"
    FAKE_GIT_MODE=nomutants \
        FAKE_LIST_MODE=empty \
        FAKE_METADATA_MODE=ok \
        FAKE_ROOT="${REPO_ROOT}" \
        FIXTURE_ARGV="${TEST_DIR}/argv.txt" \
        MUTANTS_SHARD=4/8 \
        MUTATION_OUTPUT_DIR="${TEST_DIR}/output/emptyshard" \
        DIFF_TARGET=origin/main \
        PATH="${TEST_DIR}/bin:${PATH}" \
        bash "${SUBJECT}" --ci fast
) >"${TEST_DIR}/emptyshard.out" 2>&1 || status=$?
if [[ "${status}" -eq 0 ]] \
    && [[ -f "${TEST_DIR}/output/emptyshard/no-mutants" ]] \
    && [[ "$(cat "${TEST_DIR}/output/emptyshard/no-mutants")" == "4/8" ]]; then
    ok "an empty slice writes its no-mutants marker"
else
    no "an empty slice did not record no-mutants (exit ${status})"
    cat "${TEST_DIR}/emptyshard.out"
fi

printf '\nmutation zero-mutant fixture: %s case(s), %s failure(s)\n' "${RUNS}" "${FAILED}"
[[ "${FAILED}" -eq 0 ]] || exit 1
