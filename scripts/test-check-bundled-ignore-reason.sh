#!/usr/bin/env bash
# Fixture for scripts/check-bundled-ignore-reason.sh (issue #966).
#
# The real gate only ever runs against a release tree, so its failure directions
# must be proven on synthetic roots: an ignore that outlived the zip package, a
# bundled flag that disappeared while the ignores stayed, and the prose record
# that keeps the GHSA from being re-derived from scratch.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/check-bundled-ignore-reason.sh"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT
RUNS=0
FAIL=0

pass() { RUNS=$((RUNS + 1)); printf '✅ %s\n' "$1"; }
fail_case() { RUNS=$((RUNS + 1)); FAIL=$((FAIL + 1)); printf '❌ %s\n' "$1" >&2; }

GHSA="GHSA-m665-v7pm-7grj"
write_record() {
    local file="$1"
    cat > "${file}" <<EOF
# ${GHSA} / CVE-2026-115905: recorded for the next reader.
# Re-check when libduckdb-sys reintroduces zip.
[advisories]
ignore = [
EOF
}

write_ids() {
    local file="$1"
    shift
    for id in "$@"; do printf '    "%s",\n' "${id}" >> "${file}"; done
    printf '%s\n' ']' >> "${file}"
}

new_root() {
    local dir="$1"
    mkdir -p "${dir}/.cargo" "${dir}/crates/csm-duckdb"
}

# args: root zip(0/1) bundled(0/1) ids...
base_tree() {
    local dir="$1" zip="$2" bundled="$3"
    shift 3
    new_root "${dir}"
    write_record "${dir}/deny.toml"
    write_ids "${dir}/deny.toml" "$@"
    write_record "${dir}/.cargo/audit.toml"
    write_ids "${dir}/.cargo/audit.toml" "$@"
    if [[ "${zip}" -eq 1 ]]; then
        printf '[[package]]\nname = "zip"\nversion = "6.0.0"\n' > "${dir}/Cargo.lock"
    else
        printf '[[package]]\nname = "other"\nversion = "1.0.0"\n' > "${dir}/Cargo.lock"
    fi
    if [[ "${bundled}" -eq 1 ]]; then
        printf '[dependencies]\nduckdb = { version = "1.1", features = ["bundled"] }\n' > "${dir}/crates/csm-duckdb/Cargo.toml"
    else
        printf '[dependencies]\nduckdb = { version = "1.1" }\n' > "${dir}/crates/csm-duckdb/Cargo.toml"
    fi
}

run_case() {
    local label="$1" expected="$2" needle="$3" dir="$4"
    local out rc
    if out="$(bash "${SUBJECT}" "${dir}" 2>&1)"; then rc=0; else rc=$?; fi
    if [[ "${rc}" -ne "${expected}" ]]; then
        fail_case "${label}: expected rc ${expected}, got ${rc}"
        printf '%s\n' "${out}" | sed 's/^/  /' >&2
        return
    fi
    if [[ -n "${needle}" ]] && ! grep -qF -- "${needle}" <<< "${out}"; then
        fail_case "${label}: output lacks '${needle}'"
        printf '%s\n' "${out}" | sed 's/^/  /' >&2
        return
    fi
    pass "${label}"
}

ZIP_IDS=(RUSTSEC-2026-0367 RUSTSEC-2026-0368 RUSTSEC-2026-0369)

D="${TEST_DIR}/current"; base_tree "${D}" 0 1
run_case "no zip, no ignores, recorded" 0 "no zip package" "${D}"

D="${TEST_DIR}/live"; base_tree "${D}" 1 1 "${ZIP_IDS[@]}"
run_case "zip locked + bundled + ignores" 0 "zip advisories ignored only while" "${D}"

D="${TEST_DIR}/outlived"; base_tree "${D}" 0 1 "${ZIP_IDS[@]}"
run_case "ignore outlived zip" 1 "outlived its reason" "${D}"

D="${TEST_DIR}/unbundled"; base_tree "${D}" 1 0 "${ZIP_IDS[@]}"
run_case "zip locked but bundled removed" 1 "does not enable duckdb/bundled" "${D}"

D="${TEST_DIR}/no-record"; base_tree "${D}" 0 1
sed -i '/GHSA-m665/d' "${D}/deny.toml"
run_case "missing GHSA record" 1 "does not record GHSA-m665-v7pm-7grj" "${D}"

D="${TEST_DIR}/no-recheck"; base_tree "${D}" 0 1
sed -i '/Re-check/d' "${D}/deny.toml"
run_case "record without re-check" 1 "without a re-check trigger" "${D}"

D="${TEST_DIR}/zip-no-ignore"; base_tree "${D}" 1 1
run_case "zip locked, cargo deny owns it" 0 "cargo deny owns the decision" "${D}"

D="${TEST_DIR}/audit-only"; base_tree "${D}" 0 1 "${ZIP_IDS[@]}"
# Remove the ids from deny.toml but keep them in audit.toml.
python3 - "${D}/deny.toml" <<'PY'
import sys
from pathlib import Path
p = Path(sys.argv[1])
p.write_text(p.read_text().replace('    "RUSTSEC-2026-0367",\n    "RUSTSEC-2026-0368",\n    "RUSTSEC-2026-0369",\n', ''))
PY
run_case "ignore in audit only is still seen" 1 "outlived its reason" "${D}"

D="${TEST_DIR}/missing-lock"; base_tree "${D}" 0 1; rm "${D}/Cargo.lock"
run_case "missing Cargo.lock is exit 2" 2 "cannot run" "${D}"

D="${TEST_DIR}/missing-duckdb"; base_tree "${D}" 0 1; rm "${D}/crates/csm-duckdb/Cargo.toml"
run_case "missing duckdb manifest is exit 2" 2 "cannot run" "${D}"

D="${TEST_DIR}/multi-line"; base_tree "${D}" 1 1 "${ZIP_IDS[@]}"
cat > "${D}/crates/csm-duckdb/Cargo.toml" <<'EOF'
[dependencies.duckdb]
version = "1.1"
features = ["bundled"]
EOF
run_case "bundled in a dependencies.duckdb table" 0 "zip advisories ignored only while" "${D}"

D="${TEST_DIR}/wrong-dep"; base_tree "${D}" 1 1 "${ZIP_IDS[@]}"
cat > "${D}/crates/csm-duckdb/Cargo.toml" <<'EOF'
[dependencies]
other = { version = "1.0", features = ["bundled"] }
duckdb = { version = "1.1" }
EOF
run_case "bundled on another dependency does not count" 1 "does not enable duckdb/bundled" "${D}"

echo "bundled-ignore-reason fixture: ${RUNS} cases, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
