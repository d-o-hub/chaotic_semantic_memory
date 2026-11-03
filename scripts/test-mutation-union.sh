#!/usr/bin/env bash
# Fixture for scripts/mutation_union.py (issue #951).
#
# The union arithmetic is the point: an all-missed shard must not be rescued by
# a sibling, a missing shard must block the verdict, and timeout budgets must be
# enforced once over the union rather than once per shard.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/mutation_union.py"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT
PASS=0
FAIL=0

run_union() {
  OUT="$(python3 "${SUBJECT}" "$@" 2>&1)" && RC=0 || RC=$?
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

# make_shard <root> <i> <N> <caught> <missed> <timeout> <unviable> [complete|empty]
make_shard() {
  local root="$1" i="$2" n="$3" caught="$4" missed="$5" timeout="$6" unviable="$7" kind="${8:-complete}"
  local base="${root}/mutants-shard-${i}/progress/mutation"
  mkdir -p "${base}"
  if [[ "${kind}" == "empty" ]]; then
    printf '%s/%s\n' "${i}" "${n}" > "${base}/no-mutants"
    return
  fi
  python3 - "${base}" "${caught}" "${missed}" "${timeout}" "${unviable}" <<'PY'
import json, pathlib, sys
base = pathlib.Path(sys.argv[1]) / "run-1" / "mutants.out"
base.mkdir(parents=True, exist_ok=True)
caught, missed, timeout, unviable = map(int, sys.argv[2:6])
outcomes = [{"scenario": "Baseline", "summary": "Success"}]
for index in range(caught):
    outcomes.append({"scenario": {"Mutant": {"file": f"src/a{index}.rs"}}, "summary": "CaughtMutant"})
for index in range(missed):
    outcomes.append({"scenario": {"Mutant": {"file": f"src/m{index}.rs"}}, "summary": "MissedMutant"})
for index in range(timeout):
    outcomes.append({"scenario": {"Mutant": {"file": f"src/t{index}.rs"}}, "summary": "Timeout"})
for index in range(unviable):
    outcomes.append({"scenario": {"Mutant": {"file": f"src/u{index}.rs"}}, "summary": "Unviable"})
data = {
    "cargo_mutants_version": "27.1.0",
    "outcomes": outcomes,
    "caught": caught,
    "missed": missed,
    "timeout": timeout,
    "unviable": unviable,
    "total_mutants": caught + missed + timeout + unviable,
}
(base / "outcomes.json").write_text(json.dumps(data))
PY
  printf '%s/%s\n' "${i}" "${n}" > "${base}/shard-complete"
}

echo "Test 1: union, not average — one all-missed shard cannot ride a sibling"
D="${TMP}/union"; make_shard "${D}" 1 2 0 1 0 0; make_shard "${D}" 2 2 100 0 0 0
run_union --artifacts "${D}" --expected-shards 2
expect_case "union score is 99.0099% (average would be 50%)" 0 "score=99.0099%"
expect_case "union verdict passes" 0 "CI check passed"

echo "Test 2: below-threshold union fails"
D="${TMP}/fail"; make_shard "${D}" 1 2 1 1 0 0; make_shard "${D}" 2 2 1 1 0 0
run_union --artifacts "${D}" --expected-shards 2
expect_case "50% union fails 85%" 1 "CI check failed"

echo "Test 3: a missing shard is not a verdict"
D="${TMP}/missing"; make_shard "${D}" 1 2 2 0 0 0
run_union --artifacts "${D}" --expected-shards 2
expect_case "missing shard exits 2" 2 "missing shard evidence"

echo "Test 4: duplicate shard ids are rejected"
D="${TMP}/dupe"; make_shard "${D}" 1 2 2 0 0 0
mkdir -p "${D}/mutants-shard-1-copy/progress/mutation"
printf '1/2\n' > "${D}/mutants-shard-1-copy/progress/mutation/shard-complete"
run_union --artifacts "${D}" --expected-shards 2
expect_case "duplicate shard id exits 2" 2 "two shard-complete markers"

echo "Test 5: every shard reports an empty slice"
D="${TMP}/empty"; make_shard "${D}" 1 2 0 0 0 0 empty; make_shard "${D}" 2 2 0 0 0 0 empty
run_union --artifacts "${D}" --expected-shards 2
expect_case "0-mutant union passes" 0 "0 mutants in changed Rust sources"

echo "Test 6: mixed empty and scored shards use the scored evidence"
D="${TMP}/mixed"; make_shard "${D}" 1 2 0 0 0 0 empty; make_shard "${D}" 2 2 5 0 0 0
run_union --artifacts "${D}" --expected-shards 2
expect_case "mixed union passes" 0 "score=100.0000%"

echo "Test 7: timeout budget is enforced over the union"
D="${TMP}/timeouts"; make_shard "${D}" 1 2 10 0 3 0; make_shard "${D}" 2 2 10 0 3 0
run_union --artifacts "${D}" --expected-shards 2 --timeout-budget 5
expect_case "6 union timeouts exceed budget 5" 1 "exceed union budget 5"

echo "Test 8: timeout fraction uses union viable"
D="${TMP}/fraction"; make_shard "${D}" 1 2 45 0 6 0; make_shard "${D}" 2 2 49 0 0 0
run_union --artifacts "${D}" --expected-shards 2 --timeout-budget 100
expect_case "6 of 100 viable is within the 10% fraction" 0 "CI check passed"

D="${TMP}/fraction2"; make_shard "${D}" 1 2 45 0 11 0; make_shard "${D}" 2 2 44 0 0 0
run_union --artifacts "${D}" --expected-shards 2 --timeout-budget 100
expect_case "11 of 100 viable exceeds the 10% fraction" 1 "of union viable=100"

echo "Test 9: unviable mutants leave the denominator"
D="${TMP}/unviable"; make_shard "${D}" 1 1 5 0 0 5
run_union --artifacts "${D}" --expected-shards 1
expect_case "5/5 viable = 100%" 0 "score=100.0000%"

echo "Test 10: malformed evidence is exit 2"
D="${TMP}/malformed"; make_shard "${D}" 1 1 1 0 0 0
printf 'not json\n' > "${D}/mutants-shard-1/progress/mutation/run-1/mutants.out/outcomes.json"
run_union --artifacts "${D}" --expected-shards 1
expect_case "malformed outcomes exits 2" 2 "unreadable outcomes"

echo "Test 11: the wrong shard total is exit 2"
D="${TMP}/wrong-total"; make_shard "${D}" 1 2 1 0 0 0
run_union --artifacts "${D}" --expected-shards 4
expect_case "wrong total exits 2" 2 "expected i/4"

echo "Test 12: a missing artifact directory is exit 2"
run_union --artifacts "${TMP}/nope" --expected-shards 1
expect_case "missing artifact dir exits 2" 2 "does not exist"

echo "mutation-union fixture: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
