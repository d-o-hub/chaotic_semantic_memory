#!/usr/bin/env bash
# Fixture test for scripts/mutation_inventory.py (PR #861 review, issue #830).
#
# WHY: the first version of the inventory parser lowercased `summary` and
# compared it to `caught`/`missed`/..., but cargo-mutants 27.1.0 serializes the
# SummaryOutcome *variant names* (CaughtMutant/MissedMutant). Every caught or
# missed mutant was counted in `total` and in no counter, so the module table
# showed totals above zero next to 0.0% scores. This fixture pins the real
# schema, the aggregate cross-check, and the fail-closed directions. Reproduce
# the old behavior by restoring the lowercased compare — case 1 must go red.
#
# The module takes no filesystem arguments (stdout only), so every case runs it
# with the fixture directory as CWD.
#
# shellcheck disable=SC2016  # expected-row needles are literal backticks, not expansions
#
# Usage: bash scripts/test-mutation-inventory.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INVENTORY="${SCRIPT_DIR}/mutation_inventory.py"

WORK_DIR="$(mktemp -d)"
chmod 700 "${WORK_DIR}"
trap 'rm -rf "${WORK_DIR}"' EXIT

TESTS_RUN=0
FAILED=0
pass() { TESTS_RUN=$((TESTS_RUN + 1)); echo "  ok: $1"; }
fail() { TESTS_RUN=$((TESTS_RUN + 1)); FAILED=$((FAILED + 1)); echo "  FAIL: $1"; }
check_contains() { # check_contains <description> <file> <needle>
    local desc="$1" file="$2" needle="$3"
    if grep -qF -- "$needle" "$file"; then pass "${desc}"; else fail "${desc}"; fi
}
check_missing() { # check_missing <description> <file> <needle>
    local desc="$1" file="$2" needle="$3"
    if grep -qF -- "$needle" "$file"; then fail "${desc}"; else pass "${desc}"; fi
}

run_inventory() { # run_inventory <workdir> <stdout-file> <stderr-file>
    local wdir="$1" out="$2" err="$3"
    (cd "${wdir}" && python3 "${INVENTORY}") >"${out}" 2>"${err}"
}

# --- case 1: well-formed 27.1.0 schema, one of each summary + baseline -------
mkdir -p "${WORK_DIR}/mutants.out"
cat > "${WORK_DIR}/mutants.out/outcomes.json" <<'JSON'
{
  "cargo_mutants_version": "27.1.0",
  "caught": 1, "missed": 1, "timeout": 1, "unviable": 1, "success": 0,
  "total_mutants": 4,
  "outcomes": [
    {"scenario": "Baseline", "summary": "Success"},
    {"scenario": {"Mutant": {"file": "src/a.rs", "name": "replace a with ()"}}, "summary": "CaughtMutant"},
    {"scenario": {"Mutant": {"file": "src/b.rs", "name": "replace b with ()"}}, "summary": "MissedMutant"},
    {"scenario": {"Mutant": {"file": "src/c.rs", "name": "replace c with ()"}}, "summary": "Timeout"},
    {"scenario": {"Mutant": {"file": "src/d.rs", "name": "replace d with ()"}}, "summary": "Unviable"}
  ]
}
JSON
if run_inventory "${WORK_DIR}" "${WORK_DIR}/case1.out" "${WORK_DIR}/case1.err"; then
    pass "case 1: inventory run succeeds on the real schema"
else
    fail "case 1: inventory run succeeds on the real schema"
fi
check_contains "case 1: caught file gets caught=1 and 100.0%" "${WORK_DIR}/case1.out" '| `src/a.rs` | 1 | 0 | 0 | 0 | 1 | 100.0% |'
check_contains "case 1: missed file gets missed=1 and 0.0%" "${WORK_DIR}/case1.out" '| `src/b.rs` | 0 | 1 | 0 | 0 | 1 | 0.0% |'
check_contains "case 1: timeout file counted" "${WORK_DIR}/case1.out" '| `src/c.rs` | 0 | 0 | 1 | 0 | 1 | 0.0% |'
check_contains "case 1: unviable file scores N/A" "${WORK_DIR}/case1.out" '| `src/d.rs` | 0 | 0 | 0 | 1 | 1 | N/A |'
check_contains "case 1: source is the fixed outcomes path" "${WORK_DIR}/case1.out" 'Source: `mutants.out/outcomes.json` (cargo-mutants schema)'

# --- case 2: aggregate mismatch is fatal ------------------------------------
mkdir -p "${WORK_DIR}/mismatch/mutants.out"
sed -E 's/"caught": 1/"caught": 3/' "${WORK_DIR}/mutants.out/outcomes.json" > "${WORK_DIR}/mismatch/mutants.out/outcomes.json"
if run_inventory "${WORK_DIR}/mismatch" "${WORK_DIR}/case2.out" "${WORK_DIR}/case2.err"; then
    fail "case 2: aggregate mismatch exits nonzero"
else
    pass "case 2: aggregate mismatch exits nonzero"
fi
check_contains "case 2: mismatch names the key" "${WORK_DIR}/case2.err" "aggregate mismatch for 'caught'"
check_missing "case 2: no inventory table on stdout" "${WORK_DIR}/case2.out" "Module-Level Mutation Inventory"

# --- case 3: unknown summary is fatal ---------------------------------------
mkdir -p "${WORK_DIR}/unknown/mutants.out"
sed -E 's/"CaughtMutant"/"MangledMutant"/' "${WORK_DIR}/mutants.out/outcomes.json" > "${WORK_DIR}/unknown/mutants.out/outcomes.json"
if run_inventory "${WORK_DIR}/unknown" "${WORK_DIR}/case3.out" "${WORK_DIR}/case3.err"; then
    fail "case 3: unknown summary exits nonzero"
else
    pass "case 3: unknown summary exits nonzero"
fi
check_contains "case 3: message points at SUMMARY_MAP" "${WORK_DIR}/case3.err" "update SUMMARY_MAP"

# --- case 4: a mutant scenario must be a dict with a Mutant key -------------
mkdir -p "${WORK_DIR}/scenario/mutants.out"
cat > "${WORK_DIR}/scenario/mutants.out/outcomes.json" <<'JSON'
{
  "cargo_mutants_version": "27.1.0",
  "outcomes": [
    {"scenario": {"NotAMutant": {"file": "src/e.rs"}}, "summary": "CaughtMutant"}
  ]
}
JSON
if run_inventory "${WORK_DIR}/scenario" "${WORK_DIR}/case4.out" "${WORK_DIR}/case4.err"; then
    fail "case 4: unrecognized scenario exits nonzero"
else
    pass "case 4: unrecognized scenario exits nonzero"
fi
check_contains "case 4: message names the scenario shape" "${WORK_DIR}/case4.err" "unrecognized scenario"

# --- case 5: missing outcomes.json is fatal (no silent fallback) ------------
mkdir -p "${WORK_DIR}/missing"
if run_inventory "${WORK_DIR}/missing" "${WORK_DIR}/case5.out" "${WORK_DIR}/case5.err"; then
    fail "case 5: missing outcomes.json exits nonzero"
else
    pass "case 5: missing outcomes.json exits nonzero"
fi
check_contains "case 5: message says nothing to inventory" "${WORK_DIR}/case5.err" "nothing to inventory"
check_missing "case 5: no inventory table on stdout" "${WORK_DIR}/case5.out" "Module-Level Mutation Inventory"

echo
echo "mutation inventory fixture: ${TESTS_RUN} assertions, ${FAILED} failed"
[[ "${FAILED}" -eq 0 ]]
