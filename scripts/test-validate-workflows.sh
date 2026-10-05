#!/usr/bin/env bash
# test-validate-workflows.sh — negative fixtures for
# scripts/validate-workflows.sh (issue #841: a YAML *lint* finding was reported
# to the user as "YAML syntax error", and per-file counts were categories, not
# findings).
#
# Follows the precedent of scripts/test-llms-sync.sh,
# scripts/test-validate-changelog.sh and scripts/test-quality-gates.sh: build an
# isolated mock tree in `mktemp -d`, copy the subject (plus its sourced helper
# and the repo `.yamllint` profile) in, and assert BOTH directions.
#
# Why these specific cases:
#   - trailing space must fail with the LINT label and must NOT say "syntax
#     error": that conflation was the reported defect (`yamllint -d relaxed`
#     exits 1 for any error-level rule, and the old branch printed one label for
#     all of it);
#   - a real parse failure must still say "syntax error" — otherwise the fix
#     would only have swapped which case lies;
#   - a 120-column line must NOT fail: that is the assertion that makes the
#     repo `.yamllint` profile load-bearing instead of decorative. Delete
#     `.yamllint` and this case goes red, because without it yamllint falls back
#     to its default profile where line-length is an 80-column ERROR;
#   - a file with three error findings must report "Errors: 3", because the old
#     caller collapsed any non-zero return into one category;
#   - with no YAML parser at all the gate must print a loud SKIPPED line and
#     exit 0 — never a pass, and never the fabricated syntax error the old
#     `python3 -c "import yaml"` fallback produced on boxes without PyYAML.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SUBJECT="${SCRIPT_DIR}/validate-workflows.sh"
HELPER="${SCRIPT_DIR}/lib/yaml-findings.sh"
PROFILE="${REPO_ROOT}/.yamllint"

ESC="$(printf '\033')"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

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
    echo "❌ Failure ($*)"
}
skip() {
    SKIPPED=$((SKIPPED + 1))
    echo "⚠️  SKIPPED (LOUD — NOT A PASS): $1"
}

# The subject derives its repo root from its own location, so copying it into
# the mock tree is what isolates this fixture from the nine real workflows on
# disk. The profile and the sourced helper must be copied with it: if the
# profile is missing the gate has no line-length budget at all and the
# profile-assertion case below cannot be made, so fail loudly rather than
# silently linting against yamllint's built-in default.
if [[ ! -f "${PROFILE}" ]]; then
    echo "❌ Failure (${PROFILE} is missing — the gate's line-length budget is" \
        "undefined, so nothing here can prove the profile is load-bearing)"
    exit 1
fi
if [[ ! -f "${HELPER}" ]]; then
    echo "❌ Failure (${HELPER} is missing — the subject sources it fail-closed)"
    exit 1
fi

mkdir -p "${TEST_DIR}/scripts/lib" "${TEST_DIR}/.github/workflows"
cp "${SUBJECT}" "${TEST_DIR}/scripts/validate-workflows.sh"
cp "${HELPER}" "${TEST_DIR}/scripts/lib/yaml-findings.sh"
cp "${PROFILE}" "${TEST_DIR}/.yamllint"

GATE_STATUS=0
GATE_OUTPUT=""

# run_gate [path-to-run-with] — ANSI is stripped on capture because the subject
# wraps every label in colour codes: a substring test written against raw
# output would match nothing and pass vacuously. The `|| GATE_STATUS=$?` guard
# is required — `set -e` makes a bare failing command substitution fatal, and a
# non-zero subject exit is the data being asserted on.
run_gate() {
    local run_path="${1:-${PATH}}"
    GATE_STATUS=0
    GATE_OUTPUT="$(
        PATH="${run_path}" bash "${TEST_DIR}/scripts/validate-workflows.sh" --check 2>&1
    )" || GATE_STATUS=$?
    GATE_OUTPUT="$(printf '%s\n' "${GATE_OUTPUT}" | sed -e "s/${ESC}\[[0-9;]*m//g")"
}

# write_workflow <body> — replaces the whole workflows directory so exactly one
# file is ever under test.
write_workflow() {
    rm -f "${TEST_DIR}"/.github/workflows/*.yml
    cat > "${TEST_DIR}/.github/workflows/mock.yml"
}

# The baseline mock is deliberately free of error-class findings: it declares
# name/on/jobs/permissions, pins nothing, uses no checkout step and keeps every
# line under 80 columns. Anything the gate reports for it is therefore visible
# as a false positive.
baseline() {
    write_workflow <<'YML'
---
name: mock
on:
  push:
    branches:
      - main
permissions: read-all
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: echo hi
        shell: bash
YML
}

# make_trailing_space — append two spaces to the `name:` line with sed rather
# than writing a literal into this fixture: trailing whitespace in a heredoc is
# invisible to readers and gets eaten by any whitespace-trimming tool.
make_trailing_space() {
    sed -i 's/^name: mock$/name: mock  /' "${TEST_DIR}/.github/workflows/mock.yml"
}

# A minimal PATH, built as a symlink farm of exactly the tools the subject
# calls. Filtering PATH by directory is not portable here: on one box yamllint
# lives in ~/.local/bin and python3 in /usr/bin, on another apt installed
# yamllint into /usr/bin alongside every coreutils binary — dropping either
# directory would remove the shell tools the gate needs, so the "no parser"
# case could not be constructed at all.
MIN_BIN="${TEST_DIR}/minbin"
mkdir -p "${MIN_BIN}"
for tool in bash find grep sed cut head tail basename dirname cat sort tr wc env \
    printf rm; do
    ln -s "$(command -v "${tool}")" "${MIN_BIN}/${tool}"
done
ln -s "$(command -v python3)" "${MIN_BIN}/python3"

# Same farm, but with a python3 that behaves like a box without PyYAML: the
# import probe fails and any other call is refused loudly, so the fallback
# cannot silently reach the real parser.
MIN_BIN_NOYAML="${TEST_DIR}/minbin-noyaml"
cp -a "${MIN_BIN}" "${MIN_BIN_NOYAML}"
# Unlink first: ${MIN_BIN_NOYAML}/python3 is still the farm's symlink to the real
# interpreter, and `cat >` follows symlinks — writing through it would try to
# overwrite /usr/bin/python3 (permission denied) instead of replacing the link.
rm -f "${MIN_BIN_NOYAML}/python3"
cat > "${MIN_BIN_NOYAML}/python3" <<'STUB'
#!/usr/bin/env bash
if [[ "$*" == *"import yaml"* ]]; then
    echo "Traceback (most recent call last):" >&2
    echo "ModuleNotFoundError: No module named 'yaml'" >&2
    exit 1
fi
echo "STUB-PYTHON3-WAS-CALLED-841" >&2
exit 99
STUB
chmod 755 "${MIN_BIN_NOYAML}/python3"

HAS_YAMLLOINT=0
if command -v yamllint >/dev/null 2>&1; then
    HAS_YAMLLOINT=1
fi

HAS_PYYAML=0
if python3 -c 'import yaml' >/dev/null 2>&1; then
    HAS_PYYAML=1
fi

echo "Setting up mock repository in ${TEST_DIR}..."
echo "yamllint: $(command -v yamllint 2>/dev/null || echo absent)"
echo "python3 + PyYAML: $(if [[ ${HAS_PYYAML} -eq 1 ]]; then echo present; else echo absent; fi)"

# ---------------------------------------------------------------------------
echo "Test 1: the baseline mock passes --check (exit 0)"
baseline
run_gate
if [[ "${GATE_STATUS}" -eq 0 ]]; then
    ok "clean workflow exits 0"
else
    ko "clean workflow failed (status=${GATE_STATUS}); output was:"
    printf '%s\n' "${GATE_OUTPUT}"
fi

# ---------------------------------------------------------------------------
echo "Test 2: a trailing space fails with the LINT label, never 'syntax error'"
if [[ ${HAS_YAMLLOINT} -eq 1 ]]; then
    make_trailing_space
    run_gate
    if [[ "${GATE_STATUS}" -ne 0 ]] \
        && [[ "${GATE_OUTPUT}" == *"lint finding, error level"* ]] \
        && [[ "${GATE_OUTPUT}" == *"trailing spaces (trailing-spaces)"* ]] \
        && [[ "${GATE_OUTPUT}" != *"syntax error"* ]]; then
        ok "trailing space reported as a lint finding and not as a parse failure"
    else
        ko "trailing space mislabelled or not detected (status=${GATE_STATUS}); output was:"
        printf '%s\n' "${GATE_OUTPUT}"
    fi
else
    skip "yamllint is not installed, so the lint-vs-syntax classification did not run"
fi

# ---------------------------------------------------------------------------
echo "Test 3: a real parse failure IS reported as a syntax error"
if [[ ${HAS_YAMLLOINT} -eq 1 ]]; then
    write_workflow <<'YML'
---
name: mock
on:
  push:
    branches: [main
permissions: read-all
jobs:
  build:
    runs-on: ubuntu-latest
YML
    run_gate
    if [[ "${GATE_STATUS}" -ne 0 ]] \
        && [[ "${GATE_OUTPUT}" == *"syntax error"* ]] \
        && [[ "${GATE_OUTPUT}" == *"Errors:"*"syntax 1"* ]]; then
        ok "unclosed flow sequence reported as a syntax error"
    else
        ko "parse failure not classified as a syntax error (status=${GATE_STATUS}); output was:"
        printf '%s\n' "${GATE_OUTPUT}"
    fi
else
    skip "yamllint is not installed, so the parse-failure case did not run"
fi

# ---------------------------------------------------------------------------
echo "Test 4: a 120-column line does NOT fail (the .yamllint profile is load-bearing)"
if [[ ${HAS_YAMLLOINT} -eq 1 ]]; then
    write_workflow <<'YML'
---
name: mock
on:
  push:
    branches:
      - main
permissions: read-all
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.7.3
      - run: echo hi
        shell: bash
YML
    run_gate
    LINE_LEN="$(awk '{ if (length($0) > 80 && length($0) < 200) print FILENAME":"FNR }' \
        "${TEST_DIR}/.github/workflows/mock.yml" || true)"
    if [[ -z "${LINE_LEN}" ]]; then
        ko "fixture broken: the mock has no >80-column line, so the profile was not exercised"
    elif [[ "${GATE_STATUS}" -ne 0 ]] || [[ "${GATE_OUTPUT}" == *"lint finding, error level"* ]]; then
        ko "a 120-column line failed the gate — .yamllint raised line-length to 200 at" \
            "warning level and that profile is not being picked up (status=${GATE_STATUS})"
        printf '%s\n' "${GATE_OUTPUT}"
    else
        ok "line at ${LINE_LEN} (>80, <200) accepted; gate stayed at exit 0"
    fi
else
    skip "yamllint is not installed, so the profile-assertion case did not run"
fi

# ---------------------------------------------------------------------------
echo "Test 5: three error findings are counted as 3, not collapsed to 1 category"
if [[ ${HAS_YAMLLOINT} -eq 1 ]]; then
    baseline
    # Three distinct trailing-space lines: the old caller ran
    # `validate_yaml_syntax "$file" || ((file_errors++))`, so all three printed
    # as "Errors: 1".
    sed -i 's/^name: mock$/name: mock  /; s/^permissions: read-all$/permissions: read-all  /' \
        "${TEST_DIR}/.github/workflows/mock.yml"
    sed -i '/runs-on: ubuntu-latest$/s/$/  /' "${TEST_DIR}/.github/workflows/mock.yml"
    run_gate
    if [[ "${GATE_STATUS}" -ne 0 ]] \
        && [[ "${GATE_OUTPUT}" == *"Errors: 3 (syntax 0, lint 3, structural 0)"* ]]; then
        ok "finding count reported honestly (3, not 1)"
    else
        ko "count collapsed or wrong (status=${GATE_STATUS}); output was:"
        printf '%s\n' "${GATE_OUTPUT}"
    fi
else
    skip "yamllint is not installed, so the finding-count case did not run"
fi

# ---------------------------------------------------------------------------
echo "Test 6: advisory-only findings exit 0 and are labelled advisory"
if [[ ${HAS_YAMLLOINT} -eq 1 ]]; then
    write_workflow <<'YML'
---
name: mock
on:
  push:
    branches:
      - main
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo hi
        shell: bash
YML
    run_gate
    if [[ "${GATE_STATUS}" -eq 0 ]] \
        && [[ "${GATE_OUTPUT}" == *"advisory: no 'permissions' declaration"* ]] \
        && [[ "${GATE_OUTPUT}" == *"advisory: unpinned action: actions/checkout@v4"* ]] \
        && [[ "${GATE_OUTPUT}" == *"advisory: checkout without persist-credentials"* ]] \
        && [[ "${GATE_OUTPUT}" != *"syntax error"* ]]; then
        ok "missing permissions + unpinned action + persist-credentials stayed advisory and exit 0"
    else
        ko "advisory case failed the gate or was mislabelled (status=${GATE_STATUS}); output was:"
        printf '%s\n' "${GATE_OUTPUT}"
    fi
else
    skip "yamllint is not installed, so the advisory-only case did not run"
fi

# ---------------------------------------------------------------------------
echo "Test 7: 'add-paths:' is not mistaken for the removed ::add-path command"
baseline
sed -i 's/^name: mock$/name: mock\nenv:\n  add-paths: llms.txt,llms-full.txt/' \
    "${TEST_DIR}/.github/workflows/mock.yml"
run_gate
if [[ "${GATE_STATUS}" -eq 0 ]] \
    && [[ "${GATE_OUTPUT}" != *"deprecated workflow commands"* ]]; then
    ok "action input key 'add-paths:' did not trip the deprecated-command check"
else
    ko "'add-paths:' was reported as a deprecated command (status=${GATE_STATUS}); output was:"
    printf '%s\n' "${GATE_OUTPUT}"
fi

# ---------------------------------------------------------------------------
echo "Test 8: with no yamllint and no PyYAML the gate SKIPS loudly and exits 0"
baseline
make_trailing_space
run_gate "${MIN_BIN_NOYAML}"
if [[ "${GATE_STATUS}" -eq 0 ]] \
    && [[ "${GATE_OUTPUT}" == *"SKIPPED — NOT A PASS"* ]] \
    && [[ "${GATE_OUTPUT}" == *"NO YAML verdict (SKIPPED — NOT A PASS): 1"* ]] \
    && [[ "${GATE_OUTPUT}" != *"YAML syntax error"* ]]; then
    ok "no-parser case skipped loudly, exited 0, and invented no syntax error"
else
    ko "no-parser case wrong (status=${GATE_STATUS}); output was:"
    printf '%s\n' "${GATE_OUTPUT}"
fi

# ---------------------------------------------------------------------------
echo "Test 9: the PyYAML fallback parses correctly instead of failing on import"
if [[ ${HAS_PYYAML} -eq 1 ]]; then
    baseline
    run_gate "${MIN_BIN}"
    if [[ "${GATE_STATUS}" -eq 0 ]] && [[ "${GATE_OUTPUT}" != *"syntax error"* ]]; then
        ok "clean file passes under the python3 fallback"
    else
        ko "python3 fallback reported an error the repository does not have" \
            "(status=${GATE_STATUS}); output was:"
        printf '%s\n' "${GATE_OUTPUT}"
    fi

    make_trailing_space
    run_gate "${MIN_BIN}"
    if [[ "${GATE_STATUS}" -eq 0 ]] && [[ "${GATE_OUTPUT}" != *"syntax error"* ]]; then
        ok "trailing space does not become a parse failure under the fallback"
    else
        ko "fallback fabricated a syntax error (status=${GATE_STATUS}); output was:"
        printf '%s\n' "${GATE_OUTPUT}"
    fi

    write_workflow <<'YML'
---
name: mock
on:
  push:
    branches: [main
permissions: read-all
jobs:
  build:
    runs-on: ubuntu-latest
YML
    run_gate "${MIN_BIN}"
    if [[ "${GATE_STATUS}" -ne 0 ]] && [[ "${GATE_OUTPUT}" == *"syntax error"* ]]; then
        ok "real parse failure still reported as a syntax error by the fallback"
    else
        ko "fallback missed the parse failure (status=${GATE_STATUS}); output was:"
        printf '%s\n' "${GATE_OUTPUT}"
    fi
else
    skip "python3 has no PyYAML module here, so the fallback path did not run"
fi

# ---------------------------------------------------------------------------
echo "────────────────────────────────────────────────────────"
echo "Tests run: ${RUNS}, failed: ${FAILURES}, skipped: ${SKIPPED}"
if [[ "${FAILURES}" -ne 0 ]]; then
    exit 1
fi
if [[ "${SKIPPED}" -ne 0 ]]; then
    echo "⚠️  ${SKIPPED} assertion(s) were SKIPPED — the classification is NOT fully proven" \
        "on this machine. Install yamllint (apt install yamllint / pipx install yamllint)" \
        "and re-run."
fi
echo "All tests passed!"
