#!/usr/bin/env bash
# Fixture for scripts/mutation_test.sh's arm split (#1006).
#
# WHY THIS EXISTS
# cargo-mutants builds one `cargo test --no-run --package=X` per selected package and
# passes the same `--features` string to all of them. That has bitten this gate twice
# from opposite directions:
#   * PR #977 went red because one flat call passed `--features chaotic_semantic_memory/mcp`
#     into `cargo test --package=csm-core-lib`, where no such feature exists — "cargo build
#     failed in an unmutated tree, so no mutants were tested";
#   * #1006 is the mirror image: the `crates` arm carries *no* features, so a package whose
#     code sits behind its own feature is never compiled there. `voronoi_lsh`
#     (#[cfg(feature = "experimental-voronoi-lsh")], crates/csm-chaos/src/hashing/mod.rs:2)
#     and the whole `chaotic_lsh` wrapper (#![cfg(feature = "chaotic-hashing")]) were listed,
#     built without, passed every test, and reported MISSED forever. Measured on the #998
#     diff with cargo-mutants 27.1.0 `-- --lib`: `-p csm-chaos` 3 MISSED vs
#     `-p csm-chaos --all-features` 0 MISSED; `-p csm-core-lib` 3 MISSED vs
#     `--all-features` 2 MISSED (those two being a real test gap, fixed in #998's 71f04548).
# Both hazards are the same rule — a feature flag must never meet a package it does not
# apply to — so the assertions here are about the *shape of every command the gate emits*,
# not about whether a mutant was killed.
#
# Every case stubs `cargo` and `git` on PATH, so nothing builds; each recorded
# `cargo mutants` invocation is one line in ${TEST_DIR}/argv.txt and is the thing under test.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/mutation_test.sh"
# Must match the script's own REPO_ROOT: package resolution is anchored there, so the
# metadata fixture has to describe the same tree the script walks.
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT
mkdir -p "${TEST_DIR}/bin" "${TEST_DIR}/output"

cat >"${TEST_DIR}/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
record() { printf '%s\n' "$*" >>"${FIXTURE_ARGV}"; }

if [[ "${1:-}" == "mutants" && "${2:-}" == "--version" ]]; then
    echo "cargo-mutants 27.1.0"
    exit 0
fi
if [[ "${1:-}" == "mutants" && "${2:-}" == "--help" ]]; then
    echo "--in-diff"
    exit 0
fi
if [[ "${1:-}" == "metadata" ]]; then
    cat <<EOF
{"packages":[{"name":"chaotic_semantic_memory","manifest_path":"${FAKE_ROOT}/Cargo.toml"},{"name":"csm-chaos","manifest_path":"${FAKE_ROOT}/crates/csm-chaos/Cargo.toml"},{"name":"csm-core-lib","manifest_path":"${FAKE_ROOT}/crates/csm-core-lib/Cargo.toml"},{"name":"csm-retrieval","manifest_path":"${FAKE_ROOT}/crates/csm-retrieval/Cargo.toml"}]}
EOF
    exit 0
fi
if [[ "${1:-}" == "mutants" && "${2:-}" == "--list" ]]; then
    record "$*"
    # One mutant per package the command selects. `srconly` answers only for the
    # packages a src-only diff really touches, so the feature arms come back empty
    # and must be skipped rather than paid for with a baseline build.
    case "${FAKE_LIST_MODE:-full}:${*}" in
        full:*"-p csm-chaos"*)
            echo "crates/csm-chaos/src/hashing/voronoi_lsh.rs:51:9: replace VoronoiLsh::try_new -> Result<Self, InputDimensionError> with Ok(Default::default())"
            ;;
    esac
    case "${FAKE_LIST_MODE:-full}:${*}" in
        full:*"-p csm-core-lib"*)
            echo "crates/csm-core-lib/src/hashing/chaotic_lsh.rs:44:9: replace ChaoticLsh::input_dim -> usize with 0"
            ;;
    esac
    case "${*}" in
        *"-p csm-retrieval"*)
            echo "crates/csm-retrieval/src/lib.rs:9:5: replace + with - in beta"
            ;;
    esac
    case "${*}" in
        *"-p chaotic_semantic_memory"*)
            echo "src/example.rs:12:5: replace example::alpha -> u32 with 0"
            ;;
    esac
    exit 0
fi
if [[ "${1:-}" == "mutants" ]]; then
    record "$*"
    echo "1 mutants tested in 1.0s: 1 caught, 0 missed, 0 unviable, 0 timeouts"
    exit 0
fi
echo "unexpected cargo command: $*" >&2
exit 2
MOCK

cat >"${TEST_DIR}/bin/git" <<'MOCK'
#!/usr/bin/env bash
# The gate runs every git call as `git -C <workspace-root> …`; drop the prefix so the
# mock sees the subcommand in $1 the way a developer's shell would.
if [[ "${1:-}" == "-C" ]]; then
    shift 2
fi
if [[ "${1:-}" == "rev-parse" ]]; then
    echo "deadbeef"
    exit 0
fi
if [[ "${1:-}" == "diff" ]]; then
    case "${FAKE_GIT_MODE:-crates}" in
        crates)
            printf '%s\n' \
                'diff --git a/crates/csm-chaos/src/hashing/voronoi_lsh.rs b/crates/csm-chaos/src/hashing/voronoi_lsh.rs' \
                '+++ b/crates/csm-chaos/src/hashing/voronoi_lsh.rs' \
                '@@ -49,2 +49,2 @@' \
                '+    pub fn try_new(' \
                'diff --git a/crates/csm-core-lib/src/hashing/chaotic_lsh.rs b/crates/csm-core-lib/src/hashing/chaotic_lsh.rs' \
                '+++ b/crates/csm-core-lib/src/hashing/chaotic_lsh.rs' \
                '@@ -42,2 +42,2 @@' \
                '+    pub const fn input_dim(&self) -> usize {' \
                'diff --git a/crates/csm-retrieval/src/lib.rs b/crates/csm-retrieval/src/lib.rs' \
                '+++ b/crates/csm-retrieval/src/lib.rs' \
                '@@ -8,2 +8,2 @@' \
                '+    let x = a + b;'
            ;;
        rootonly)
            printf '%s\n' \
                'diff --git a/src/example.rs b/src/example.rs' \
                '+++ b/src/example.rs' \
                '@@ -10,2 +10,2 @@' \
                '+fn changed() {}' \
                'diff --git a/crates/csm-retrieval/src/lib.rs b/crates/csm-retrieval/src/lib.rs' \
                '+++ b/crates/csm-retrieval/src/lib.rs' \
                '@@ -8,2 +8,2 @@' \
                '+    let x = a + b;'
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

ARMS=""
run_mutation() { # run_mutation <git-mode> <list-mode> <case-name> <expected-status>
    local mode="$1" list="$2" name="$3" expected="$4"
    : >"${TEST_DIR}/argv.txt"
    local output_file="${TEST_DIR}/case-${name}.log"
    local status=0
    (
        cd "${TEST_DIR}"
        FAKE_GIT_MODE="${mode}" \
            FAKE_LIST_MODE="${list}" \
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
    # Only the *generation-side* scope decides what a mutation arm builds, so the
    # `-- <cargo test args>` tail is cut: that tail always carries every selected
    # package (`--lib -p A -p B -p C`) and would make any `-p` count here meaningless.
    ARMS="$(grep -- '-- --lib' "${TEST_DIR}/argv.txt" | sed 's/ -- .*//' || true)"
}

# argv_has <description> <fixed-string> — some run-phase command must contain it.
argv_has() {
    if grep -qF -- "$2" <<<"${ARMS}"; then ok "$1"; else no "$1"; cat <<<"${ARMS}"; fi
}
# argv_lacks <description> <fixed-string> — no run-phase command may contain it.
argv_lacks() {
    if grep -qF -- "$2" <<<"${ARMS}"; then no "$1"; cat <<<"${ARMS}"; else ok "$1"; fi
}
# argv_pure <description> <package> <forbidden-substring>: the command that selects
# <package> must not mention <forbidden-substring>, which is how a feature flag can
# only ever reach the package that owns it.
argv_pure() {
    if grep -- "${2}" <<<"${ARMS}" | grep -qF -- "$3"; then
        no "$1"
        grep -- "${2}" <<<"${ARMS}"
    else
        ok "$1"
    fi
}

echo
echo "== a crate whose code is feature-gated is scored with that feature on =="
run_mutation crates full crates 0
argv_has "csm-chaos gets its own arm with --all-features" "-p csm-chaos --all-features"
argv_has "csm-core-lib gets its own arm with --all-features" "-p csm-core-lib --all-features"
argv_has "the un-gated crate keeps the plain crates arm" "-p csm-retrieval"
argv_pure "the feature arm for csm-chaos selects no other package" "-p csm-chaos" "-p csm-retrieval"
argv_pure "the feature arm for csm-core-lib selects no other package" "-p csm-core-lib" "-p csm-retrieval"
argv_pure "csm-chaos is no longer hidden inside the multi-package crates arm" "-p csm-retrieval" "-p csm-chaos"
if [[ -n "${ARMS}" ]]; then
    ok "the gate emitted $(printf '%s\n' "${ARMS}" | grep -c .) arm commands"
else
    no "no arm commands were recorded"
fi

echo
echo "== no feature flag ever meets a package that does not own it (#977) =="
# Every command either has one -p, or has no --features/--all-features at all.
bad=0
while IFS= read -r line; do
    [[ -z "${line}" ]] && continue
    pkgs="$(grep -o -- '-p [^ ]*' <<<"${line}" | wc -l)"
    if [[ "${pkgs}" -gt 1 ]] && grep -qE -- '--features|--all-features' <<<"${line}"; then
        no "multi-package command carries a feature flag: ${line}"
        bad=$((bad + 1))
    fi
done <<<"${ARMS}"
[[ ${bad} -eq 0 ]] && ok "every feature-bearing arm selects exactly one package"
# The root arm is the one place a package-qualified feature is legal, and it must
# still be there — dropping it would silently re-blind src/mcp/**.
argv_has "the root arm still enables the mcp feature" "-p chaotic_semantic_memory --features chaotic_semantic_memory/mcp"

echo
echo "== an arm with nothing to say is skipped, not built =="
run_mutation rootonly srconly rootonly 0
argv_lacks "a diff that touches no csm-chaos code must not build csm-chaos" "-p csm-chaos --all-features"
argv_lacks "a diff that touches no csm-core-lib code must not build csm-core-lib with features" "-p csm-core-lib --all-features"
if grep -qF "mutation arm 'feat-csm-core-lib': 0 selected mutants, skipped" "${MUT_LOG}"; then
    ok "the empty feature arm is reported as skipped"
else
    no "the empty feature arm was not reported as skipped"
    cat "${MUT_LOG}"
fi
argv_has "the root arm still runs on a src-only diff" "-p chaotic_semantic_memory"

echo
echo "== the union still scores across every arm =="
if grep -qE 'unit-target mutation summary: total=[0-9]+ caught=[0-9]+' "${MUT_LOG}"; then
    ok "the gate published a union score"
else
    no "no union score was published"
    cat "${MUT_LOG}"
fi

echo
echo "Tests run: ${RUNS}, failed: ${FAILED}"
[[ ${FAILED} -eq 0 ]]
