#!/usr/bin/env bash
# Fixture for scripts/check-version-literals.py (issue #982).
#
# The checker exists because a version sweep that substitutes the *previous*
# manifest value cannot see a literal that is already older, and the old
# staleness probe matched only a hardcoded `0.2.[0-4]` pattern. These cases
# prove the new gate fails on that exact tree shape and passes both documented
# fixtures and current values. Every tree is synthetic: no repository file is
# read or written.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHECK="${ROOT}/scripts/check-version-literals.py"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT
PASS=0
FAIL=0

new_tree() {
  local dir="$1"
  mkdir -p "${dir}/tests" "${dir}/examples" "${dir}/book/src"
  printf '[package]\nname = "synthetic"\nversion = "0.3.9"\n' > "${dir}/Cargo.toml"
}

run_case() {
  local name="$1" expected="$2" needle="$3" dir="$4"
  local out rc
  if out="$(python3 "${CHECK}" "${dir}" 2>&1)"; then rc=0; else rc=$?; fi
  if [[ "${rc}" -ne "${expected}" ]]; then
    echo "FAIL ${name}: expected rc ${expected}, got ${rc}" >&2
    printf '%s\n' "${out}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
    return
  fi
  if [[ -n "${needle}" ]] && ! grep -qF -- "${needle}" <<< "${out}"; then
    echo "FAIL ${name}: output does not contain: ${needle}" >&2
    printf '%s\n' "${out}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
    return
  fi
  PASS=$((PASS + 1))
}

# 1. A literal equal to the current version passes.
D="${TMP}/current"; new_tree "${D}"
printf 'let payload = json!({"version": "0.3.9"});\n' > "${D}/tests/current.rs"
run_case "current-version" 0 "ok:" "${D}"

# 2. The exact drifted shape from #982 fails and names the file and line.
D="${TMP}/drift"; new_tree "${D}"
printf 'let payload = json!({"version": "0.3.2"});\n' > "${D}/tests/framework_lifecycle.rs"
run_case "older-literal-fails" 1 "0.3.2" "${D}"
run_case "older-literal-names-line" 1 "tests/framework_lifecycle.rs:1" "${D}"

# 3. A same-line marker documents a deliberately older fixture.
D="${TMP}/same-line"; new_tree "${D}"
printf 'let payload = json!({"version": "0.3.2"}); // version-fixture: import payload\n' > "${D}/tests/old.rs"
run_case "same-line-marker" 0 "ok:" "${D}"

# 4. A marker up to three lines above documents the literal.
D="${TMP}/near-marker"; new_tree "${D}"
printf '# version-fixture: illustrative payload\nlet payload = json!({"version": "0.3.2"});\n' > "${D}/tests/old.rs"
run_case "near-marker" 0 "ok:" "${D}"

# 5. A marker farther away does not bless the literal.
D="${TMP}/far-marker"; new_tree "${D}"
printf '# version-fixture: too far away\n\n\n\nlet payload = json!({"version": "0.3.2"});\n' > "${D}/tests/old.rs"
run_case "far-marker-fails" 1 "0.3.2" "${D}"

# 6. README is scanned too.
D="${TMP}/readme"; new_tree "${D}"
printf 'Example: {"version": "0.2.0"}\n' > "${D}/README.md"
run_case "readme-drift" 1 "README.md:1" "${D}"

# 7. book/src is scanned and current values pass there.
D="${TMP}/book"; new_tree "${D}"
printf 'Export: {"version": "0.3.9"}\n' > "${D}/book/src/export.md"
run_case "book-current" 0 "ok:" "${D}"

# 8. examples/ (shell fixtures) are scanned and can be annotated.
D="${TMP}/examples"; new_tree "${D}"; mkdir -p "${D}/examples/cli"
printf '# version-fixture: example export payload\necho \x27  {"version": "0.3.2"}\x27\n' > "${D}/examples/cli/example.sh"
run_case "example-annotated" 0 "ok:" "${D}"

# 9. A marker that documents nothing is itself an error.
D="${TMP}/dangling"; new_tree "${D}"
printf '# version-fixture: nothing follows\n' > "${D}/tests/dangling.rs"
run_case "dangling-marker" 1 "documents no version literal" "${D}"

# 10. A missing manifest is exit 2 (not a verdict).
D="${TMP}/no-manifest"; new_tree "${D}"; rm "${D}/Cargo.toml"
run_case "missing-manifest" 2 "cannot determine the current version" "${D}"

# 11. Non-semver values are not crate-version literals.
D="${TMP}/non-semver"; new_tree "${D}"
printf 'let a = json!({"version": "test"});\nlet b = json!({"version": 1});\nlet c = "X.Y.Z";\n' > "${D}/tests/other.rs"
run_case "non-semver-ignored" 0 "ok:" "${D}"

# 12. --list reports the inventory without changing the verdict.
D="${TMP}/list"; new_tree "${D}"
printf 'let p = json!({"version": "0.3.9"});\n' > "${D}/tests/list.rs"
out="$(python3 "${CHECK}" "${D}" --list 2>&1)" || true
if grep -qF 'tests/list.rs:1: 0.3.9 [current]' <<< "${out}"; then
  PASS=$((PASS + 1))
else
  echo "FAIL list-output: missing inventory row" >&2
  printf '%s\n' "${out}" | sed 's/^/  /' >&2
  FAIL=$((FAIL + 1))
fi

# 13. A marker placed after the literal does not document it.
D="${TMP}/after-marker"; new_tree "${D}"
printf 'let payload = json!({"version": "0.3.2"});\n// version-fixture: after the fact\n' > "${D}/tests/after.rs"
run_case "marker-after-literal-fails" 1 "0.3.2" "${D}"

# 14. Unscanned suffixes are ignored.
D="${TMP}/suffix"; new_tree "${D}"
printf '{"version": "0.3.2"}\n' > "${D}/tests/notes.txt"
run_case "unscanned-suffix" 0 "ok:" "${D}"

echo "check-version-literals fixture: ${PASS} passed, ${FAIL} failed"
if [[ "${FAIL}" -ne 0 ]]; then
  exit 1
fi
