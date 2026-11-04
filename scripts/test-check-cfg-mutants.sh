#!/usr/bin/env bash
# Fixture for scripts/check_cfg_mutants.py (issue #952).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/check_cfg_mutants.py"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT
PASS=0
FAIL=0

run_check() {
  OUT="$(python3 "${SUBJECT}" "$@" 2>&1)" && RC=0 || RC=$?
}

expect() {
  local label="$1" expected="$2" needle="$3"
  if [[ "${RC}" -ne "${expected}" ]]; then
    echo "❌ ${label}: expected rc ${expected}, got ${RC}" >&2
    printf '%s\n' "${OUT}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
  elif ! grep -qF -- "${needle}" <<< "${OUT}"; then
    echo "❌ ${label}: output lacks '${needle}'" >&2
    printf '%s\n' "${OUT}" | sed 's/^/  /' >&2
    FAIL=$((FAIL + 1))
  else
    PASS=$((PASS + 1))
  fi
}

new_root() { local dir="$1"; mkdir -p "${dir}/src" "${dir}/crates/csm-retrieval/src"; }

echo "Test 1: a cfg-split let pair is rejected by --all"
D="${TMP}/let"; new_root "${D}"
cat > "${D}/src/framework.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
let start = std::time::Instant::now();
#[cfg(target_arch = "wasm32")]
let start = js_sys::Date::now();
EOF
run_check "${D}" --all
expect "let pair fails" 1 "declared on both cfg arms"
expect "the file and line are named" 1 "src/framework.rs:1"

echo "Test 2: a cfg-split function pair is rejected"
D="${TMP}/fn"; new_root "${D}"
cat > "${D}/crates/csm-retrieval/src/lib.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> u64 { 0 }
#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> u64 { 1 }
EOF
run_check "${D}" --all
expect "function pair fails" 1 "crates/csm-retrieval/src/lib.rs:1"

echo "Test 3: one shared helper has no pair"
D="${TMP}/shared"; new_root "${D}"
cat > "${D}/src/clock.rs" <<'EOF'
pub(crate) fn elapsed_ms(start: u64) -> u64 {
    now_ms().saturating_sub(start)
}
EOF
run_check "${D}" --all
expect "shared helper passes" 0 "ok: no cfg-split declarations"

echo "Test 4: a wasm-only item with no native sibling is a different class"
D="${TMP}/wasmonly"; new_root "${D}"
cat > "${D}/src/wasm_ext.rs" <<'EOF'
#[cfg(target_arch = "wasm32")]
fn hybrid_result_to_array() -> u32 { 0 }
EOF
run_check "${D}" --all
expect "wasm-only item passes" 0 "ok: no cfg-split declarations"

echo "Test 5: test files are outside the mutation surface"
D="${TMP}/tests"; new_root "${D}"
cat > "${D}/src/framework_tests.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
let start = 0;
#[cfg(target_arch = "wasm32")]
let start = 1;
EOF
mkdir -p "${D}/src/tests"
cat > "${D}/src/tests/helper.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
let start = 0;
#[cfg(target_arch = "wasm32")]
let start = 1;
EOF
run_check "${D}" --all
expect "test modules are ignored" 0 "ok: no cfg-split declarations"

echo "Test 6: different names on the two arms are not a pair"
D="${TMP}/different"; new_root "${D}"
cat > "${D}/src/lib.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
let native_start = 0;
#[cfg(target_arch = "wasm32")]
let wasm_start = 1;
EOF
run_check "${D}" --all
expect "different declarations pass" 0 "ok: no cfg-split declarations"

echo "Test 7: an unrelated statement between the arms ends the search"
D="${TMP}/unrelated"; new_root "${D}"
cat > "${D}/src/lib.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
fn native_only() {}
some_other_item();
#[cfg(target_arch = "wasm32")]
fn native_only() {}
EOF
run_check "${D}" --all
expect "interrupted pair passes" 0 "ok: no cfg-split declarations"

echo "Test 8: a new pair added by the diff is rejected"
D="${TMP}/diff-new"; new_root "${D}"
cat > "${D}/src/new.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
let start = 0;
#[cfg(target_arch = "wasm32")]
let start = 1;
EOF
cat > "${D}/new.patch" <<'EOF'
diff --git a/src/new.rs b/src/new.rs
--- a/src/new.rs
+++ b/src/new.rs
@@ -1,0 +1,4 @@
+#[cfg(not(target_arch = "wasm32"))]
+let start = 0;
+#[cfg(target_arch = "wasm32")]
+let start = 1;
EOF
run_check "${D}" --diff - < "${D}/new.patch"
expect "new pair in the diff fails" 1 "src/new.rs:1"

echo "Test 9: a pre-existing pair in an untouched file is not a PR finding"
D="${TMP}/diff-old"; new_root "${D}"
cat > "${D}/src/old.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
let start = 0;
#[cfg(target_arch = "wasm32")]
let start = 1;
EOF
cat > "${D}/unrelated.patch" <<'EOF'
diff --git a/src/other.rs b/src/other.rs
--- a/src/other.rs
+++ b/src/other.rs
@@ -1,0 +1,2 @@
+// unrelated
+fn other() {}
EOF
run_check "${D}" --diff - < "${D}/unrelated.patch"
expect "untouched pre-existing pair passes" 0 "no cfg-split declarations"

echo "Test 10: modifying the wasm arm is a new PR finding"
D="${TMP}/diff-arm"; new_root "${D}"
cat > "${D}/src/old.rs" <<'EOF'
#[cfg(not(target_arch = "wasm32"))]
let start = 0;
#[cfg(target_arch = "wasm32")]
let start = 1;
EOF
cat > "${D}/arm.patch" <<'EOF'
diff --git a/src/old.rs b/src/old.rs
--- a/src/old.rs
+++ b/src/old.rs
@@ -3,0 +3,1 @@
+#[cfg(target_arch = "wasm32")]
EOF
run_check "${D}" --diff - < "${D}/arm.patch"
expect "touched wasm arm fails" 1 "declared on both cfg arms"

echo "Test 11: a filesystem path is refused as a diff argument"
D="${TMP}/missing-diff"; new_root "${D}"
run_check "${D}" --diff "${D}/nope.patch"
expect "a path diff argument exits 2" 2 "only accepts '-'"

echo "Test 12: no mode is a usage error"
D="${TMP}/no-mode"; new_root "${D}"
run_check "${D}"
expect "missing mode exits 2" 2 "pass --diff"

echo "Test 13: a missing root is exit 2"
run_check "${TMP}/does-not-exist" --all
expect "missing root exits 2" 2 "is not a directory"

echo "cfg-mutants fixture: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
