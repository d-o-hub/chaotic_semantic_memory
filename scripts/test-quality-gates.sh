#!/usr/bin/env bash
# test-quality-gates.sh — negative fixtures for scripts/quality-gates.sh
# (issue #849).
#
# Follows the precedent of scripts/test-llms-sync.sh /
# scripts/test-check-test-attributes.sh: build an isolated mock tree in
# `mktemp -d`, copy the subject in, drive it with stubs so nothing real is
# compiled.
#
# The bug this gate exists for is a *tool invoked with a flag that tool does not
# accept*, behind a `command -v` gate. That shape is invisible in CI (`grep -rn
# nextest .github/workflows/` is empty) and asymmetric across machines, so a
# single-layer test is not enough. These fixtures assert in two layers:
#
#   (a) argv shape — a fake `cargo` records the exact argv the subject passes to
#       `cargo nextest run`, and the test asserts no bare `--quiet` token is in
#       it (nextest rejects that during argument parsing, RC=2, before a single
#       test is compiled or run);
#   (b) argv acceptance — ONE smoke assertion against the REAL cargo-nextest
#       binary, replaying the exact recorded flag vector through the cheap
#       `list` subcommand and asserting it is not rejected. Guarded by
#       `command -v`; when nextest is absent it prints a loud skip line and is
#       counted as skipped, never as a pass — an offline skip that pretends to
#       be a pass is precisely the failure mode of issue #849.
#
# It also pins the subject's output contract: success emits exactly one [PASS]
# summary line, a failing test run still surfaces $OUTPUT and exits 1, and the
# no-nextest fallback still runs `cargo test`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/quality-gates.sh"

ORIG_PATH="${PATH}"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

# Stub bin dir WITH cargo-nextest (the nextest branch), and one WITHOUT it (the
# plain `cargo test` fallback branch). PATH is prepended with the stub dir so the
# subject's `command -v` probes see exactly what this fixture intends.
STUB_BIN="${TEST_DIR}/stubbin"
STUB_BIN_NO_NEX="${TEST_DIR}/stubbin-no-nex"
mkdir -p "${STUB_BIN}" "${STUB_BIN_NO_NEX}"
ARGV_LOG="${STUB_BIN}/argv.log"

# Fake cargo: records its full argv, one call per line, then succeeds. Stubs the
# whole toolchain (`fmt`, `clippy`, `build`, `nextest`, `test`, `audit`, `deny`)
# so the fixture needs no compilation and no network. Fails with a distinctive
# diagnostic when CSM_STUB_FAIL matches its argv, which is how the "failure still
# surfaces $OUTPUT" contract is proven.
cat > "${STUB_BIN}/cargo" <<'STUB'
#!/usr/bin/env bash
{
    printf 'cargo'
    printf ' %s' "$@"
    printf '\n'
} >> "${CSM_ARGV_LOG:?}"
if [[ -n "${CSM_STUB_FAIL:-}" && "$*" == *"${CSM_STUB_FAIL}"* ]]; then
    echo "STUB-CARGO-DIAGNOSTIC-849"
    exit 3
fi
exit 0
STUB
chmod 755 "${STUB_BIN}/cargo"
ln -s "${STUB_BIN}/cargo" "${STUB_BIN_NO_NEX}/cargo"

# Presence-only shim: the subject probes `command -v cargo-nextest` and then calls
# `cargo nextest …` (i.e. the stubbed cargo above), so this binary is never run.
printf '#!/usr/bin/env bash\nexit 0\n' > "${STUB_BIN}/cargo-nextest"
chmod 755 "${STUB_BIN}/cargo-nextest"

# A real, minimal crate: layer (b) runs the genuine `cargo nextest list` against
# it, so the manifest and sources must actually parse.
cat > "${TEST_DIR}/Cargo.toml" <<'EOF'
[package]
name = "quality_gates_fixture"
version = "0.0.0"
edition = "2021"
EOF
mkdir -p "${TEST_DIR}/src"
cat > "${TEST_DIR}/src/lib.rs" <<'EOF'
#[cfg(test)]
mod smoke {
    #[test]
    fn placeholder() {}
}
EOF

mkdir -p "${TEST_DIR}/scripts"
cp "${SUBJECT}" "${TEST_DIR}/scripts/quality-gates.sh"

RUNS=0
FAILURES=0
SKIPPED=0

ok() {
    RUNS=$((RUNS + 1))
    echo "✅ Success ($1)"
}
ko() {
    RUNS=$((RUNS + 1))
    FAILURES=$((FAILURES + 1))
    echo "❌ Failure ($1)"
}

GATE_STATUS=0
GATE_OUTPUT=""
ARGV_LOG_CONTENT=""

# The fallback scenario must be genuinely nextest-free: PREPENDING a stub bin dir
# is not enough, because this box has a real cargo-nextest under ~/.cargo/bin and
# the subject's `command -v cargo-nextest` probe would still find it. So the real
# nextest's own directory is filtered out of PATH for that case only.
NEX_REAL="$(command -v cargo-nextest 2>/dev/null || true)"
NEX_DIR=""
if [[ -n "${NEX_REAL}" ]]; then
    NEX_DIR="$(dirname "${NEX_REAL}")"
fi

# path_without_dir <dir-to-drop> — PATH entries compared exactly, entry by entry,
# so a substring replace cannot damage a sibling directory name.
path_without_dir() {
    local drop="$1"
    local -a parts=()
    local part=""
    local out=""
    IFS=':' read -r -a parts <<< "${ORIG_PATH}"
    for part in "${parts[@]}"; do
        if [[ "${part}" == "${drop}" ]]; then
            continue
        fi
        if [[ -z "${out}" ]]; then
            out="${part}"
        else
            out="${out}:${part}"
        fi
    done
    printf '%s' "${out}"
}

PATH_WITHOUT_NEX="${ORIG_PATH}"
if [[ -n "${NEX_DIR}" ]]; then
    PATH_WITHOUT_NEX="$(path_without_dir "${NEX_DIR}")"
fi

# run_gate <stub-bin-dir> <path-base> [fail-substring]
# The `|| GATE_STATUS=$?` guard is required: `set -e` makes a bare failing
# command substitution fatal, and a non-zero subject exit is data to assert on,
# not a reason to abort the fixture (same guard as scripts/test-llms-sync.sh).
run_gate() {
    local stub_bin="$1"
    local path_base="$2"
    local stub_fail="${3:-}"
    GATE_STATUS=0
    : > "${ARGV_LOG}"
    GATE_OUTPUT="$(
        CSM_ARGV_LOG="${ARGV_LOG}" \
        CSM_STUB_FAIL="${stub_fail}" \
        PATH="${stub_bin}:${path_base}" \
            bash "${TEST_DIR}/scripts/quality-gates.sh" 2>&1
    )" || GATE_STATUS=$?
    ARGV_LOG_CONTENT="$(cat "${ARGV_LOG}" 2>/dev/null || true)"
}

echo "Setting up mock repository in ${TEST_DIR}..."

# ---------------------------------------------------------------------------
echo "Test 1: the nextest branch is taken when cargo-nextest is installed"
run_gate "${STUB_BIN}" "${ORIG_PATH}"
# `|| true` so a missing match yields an empty string the caller can name as a
# failure, instead of grep's exit 1 aborting the fixture under `set -e`.
NEXTEST_LINE="$(printf '%s\n' "${ARGV_LOG_CONTENT}" | grep -m1 '^cargo nextest run' || true)"
if [[ -n "${NEXTEST_LINE}" ]]; then
    ok "nextest branch reached: '${NEXTEST_LINE}'"
else
    ko "nextest branch never reached although cargo-nextest was on PATH"
fi

# ---------------------------------------------------------------------------
echo "Test 2 (layer a): argv passed to 'cargo nextest run' contains no bare --quiet"
# Split the recorded argv; tokens 0..2 are 'cargo' 'nextest' 'run'.
read -r -a NEXTEST_ARGV <<< "${NEXTEST_LINE}"
BARE_QUIET=""
for ((i = 3; i < ${#NEXTEST_ARGV[@]}; i++)); do
    if [[ "${NEXTEST_ARGV[i]}" == "--quiet" ]]; then
        BARE_QUIET="${NEXTEST_ARGV[i]}"
    fi
done
if [[ -n "${BARE_QUIET}" ]]; then
    ko "bare --quiet passed to cargo nextest (issue #849 regression): ${NEXTEST_LINE}"
elif [[ ${#NEXTEST_ARGV[@]} -le 3 ]]; then
    ko "no flags recorded for cargo nextest run — nothing was asserted"
else
    ok "no bare --quiet in: ${NEXTEST_LINE}"
fi

# ---------------------------------------------------------------------------
echo "Test 3: a passing nextest run reports exactly one [PASS] summary and exits 0"
if [[ "${GATE_STATUS}" -eq 0 ]] \
    && [[ "${GATE_OUTPUT}" == *"[PASS] Tests (nextest): OK"* ]]; then
    ok "success path prints the nextest [PASS] line"
else
    ko "success path wrong (status=${GATE_STATUS}); output was:"
    printf '%s\n' "${GATE_OUTPUT}"
fi

# ---------------------------------------------------------------------------
echo "Test 4 (layer b): the REAL cargo-nextest accepts the recorded flag vector"
if command -v cargo-nextest >/dev/null 2>&1; then
    # Replay the subject's own flags against the real binary through the cheap
    # `list` subcommand (as suggested by issue #849). `--cargo-quiet` and the
    # feature/workspace flags are accepted identically by `run` and `list`; only
    # the `run` subcommand name is swapped, so this cannot drift from the script.
    REAL_FLAGS=("${NEXTEST_ARGV[@]:3}")
    LIST_STATUS=0
    LIST_OUTPUT="$(
        cd "${TEST_DIR}" &&
            CARGO_TARGET_DIR="${TEST_DIR}/target-real" \
                cargo nextest list "${REAL_FLAGS[@]}" 2>&1
    )" || LIST_STATUS=$?
    if [[ "${LIST_STATUS}" -eq 2 ]] || [[ "${LIST_OUTPUT}" == *"unexpected argument"* ]]; then
        ko "real cargo nextest REJECTED the flags (RC=${LIST_STATUS}): ${LIST_OUTPUT%%$'\n'*}"
    elif [[ "${LIST_OUTPUT}" == *"arguments are unsupported"* ]]; then
        ko "real cargo nextest rejected post-'--' test-binary args (RC=${LIST_STATUS})"
    elif [[ "${LIST_STATUS}" -ne 0 ]]; then
        ko "real cargo nextest list failed for an unexpected reason (RC=${LIST_STATUS})"
        printf '%s\n' "${LIST_OUTPUT}"
    else
        ok "real cargo nextest accepted: cargo nextest list ${REAL_FLAGS[*]}"
    fi
else
    # Loud, and counted separately: a skip must never be mistaken for a pass.
    SKIPPED=$((SKIPPED + 1))
    echo "⚠️  SKIPPED (LOUD — NOT A PASS): real cargo-nextest is not on PATH,"
    echo "    so the argv-acceptance layer did NOT run. Only the argv-shape layer"
    echo "    was verified on this machine."
fi

# ---------------------------------------------------------------------------
echo "Test 5: a failing test run still surfaces \$OUTPUT and exits 1"
run_gate "${STUB_BIN}" "${ORIG_PATH}" "nextest run"
if [[ "${GATE_STATUS}" -eq 1 ]] \
    && [[ "${GATE_OUTPUT}" == *"[FAIL] Tests: failed"* ]] \
    && [[ "${GATE_OUTPUT}" == *"STUB-CARGO-DIAGNOSTIC-849"* ]]; then
    ok "failure path prints [FAIL] plus the captured output and exits 1"
else
    ko "failure path wrong (status=${GATE_STATUS}); output was:"
    printf '%s\n' "${GATE_OUTPUT}"
fi

# ---------------------------------------------------------------------------
echo "Test 6: without cargo-nextest the plain 'cargo test' fallback still runs"
run_gate "${STUB_BIN_NO_NEX}" "${PATH_WITHOUT_NEX}"
TEST_LINE="$(printf '%s\n' "${ARGV_LOG_CONTENT}" | grep -m1 '^cargo test' || true)"
if [[ "${GATE_STATUS}" -eq 0 ]] \
    && [[ -n "${TEST_LINE}" ]] \
    && [[ -z "$(printf '%s\n' "${ARGV_LOG_CONTENT}" | grep -m1 '^cargo nextest' || true)" ]] \
    && [[ "${GATE_OUTPUT}" == *"[PASS] Tests: OK"* ]]; then
    ok "fallback branch used '${TEST_LINE}'"
else
    ko "fallback branch broken (status=${GATE_STATUS}, line='${TEST_LINE}')"
fi

# ---------------------------------------------------------------------------
echo "────────────────────────────────────────────────────────"
echo "Tests run: ${RUNS}, failed: ${FAILURES}, skipped: ${SKIPPED}"
if [[ "${FAILURES}" -ne 0 ]]; then
    exit 1
fi
echo "All tests passed!"
