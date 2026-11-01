#!/usr/bin/env bash
# Fixture for scripts/check-fuzz-lock.sh (issue #948).
#
# The failure mode this pins is invisible to every other local sensor: `fuzz/` is
# a separate workspace with its own Cargo.lock, so a dependency change that the
# root lock absorbs cleanly leaves the fuzz lock stale, and only the CI
# `Fuzz Workspace Build` job says so. validate.sh used to pass (rc 0) and the push
# went red.
#
# Cases are hermetic. The stale/fresh pair is a synthetic path-only workspace --
# no registry, no network, no compile -- because the probe under test is a
# resolution check, not a build.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/check-fuzz-lock.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

RUNS=0
FAILED=0
ok() { RUNS=$((RUNS + 1)); echo "  ok: $1"; }
no() { RUNS=$((RUNS + 1)); FAILED=$((FAILED + 1)); echo "  FAIL: $1"; }

# A two-package workspace: fuzz/Cargo.toml depends on ../inner by path with a
# caret requirement, so bumping inner's version is resolvable but lock-breaking.
mk_workspace() { # mk_workspace <dir> <inner-version>
  local dir="$1" version="$2"
  mkdir -p "${dir}/inner/src" "${dir}/fuzz/src"
  : >"${dir}/inner/src/lib.rs"
  : >"${dir}/fuzz/src/lib.rs"
  printf '[package]\nname = "inner"\nversion = "%s"\nedition = "2021"\n' "${version}" \
    >"${dir}/inner/Cargo.toml"
  cat >"${dir}/fuzz/Cargo.toml" <<'TOML'
[package]
name = "fuzzprobe"
version = "0.1.0"
edition = "2021"

[dependencies]
inner = { path = "../inner", version = "0.1" }
TOML
}

# ------------------------------------------------------------------ case 1: real tree
echo "== the committed fuzz lock is fresh (positive control on the real tree) =="
status=0
bash "${SUBJECT}" >"${WORK}/real.log" 2>&1 || status=$?
if [[ ${status} -eq 0 ]]; then
    ok "current tree: exit 0"
else
    no "current tree: exited ${status}, expected 0"
    cat "${WORK}/real.log"
fi
if grep -qF "resolves against its committed lockfile" "${WORK}/real.log"; then
    ok "a fresh lock prints what it checked"
else
    no "a fresh lock prints what it checked"
    cat "${WORK}/real.log"
fi

# ------------------------------------------------------------------ case 2: synthetic fresh
echo "== a synthetic workspace with a matching lock passes =="
mk_workspace "${WORK}/fresh" "0.1.0"
if ! cargo metadata --format-version 1 --manifest-path "${WORK}/fresh/fuzz/Cargo.toml" \
    >/dev/null 2>&1; then
    no "fixture could not generate its own lockfile"
else
    ok "fixture generated a lockfile for the fresh case"
fi
status=0
bash "${SUBJECT}" "${WORK}/fresh/fuzz/Cargo.toml" >"${WORK}/fresh.log" 2>&1 || status=$?
if [[ ${status} -eq 0 ]]; then
    ok "fresh synthetic lock: exit 0"
else
    no "fresh synthetic lock: exited ${status}, expected 0"
    cat "${WORK}/fresh.log"
fi

# ------------------------------------------------------------------ case 3: synthetic stale
echo "== the same workspace with a bumped dependency version fails =="
# inner 0.1.0 -> 0.1.1 still satisfies "^0.1", so the requirement is resolvable
# and the ONLY problem is that the lock no longer describes the resolution. That
# is the drift the CI job catches and validate.sh used to miss.
mk_workspace "${WORK}/stale" "0.1.0"
cargo metadata --format-version 1 --manifest-path "${WORK}/stale/fuzz/Cargo.toml" >/dev/null 2>&1
printf '[package]\nname = "inner"\nversion = "0.1.1"\nedition = "2021"\n' \
    >"${WORK}/stale/inner/Cargo.toml"
status=0
bash "${SUBJECT}" "${WORK}/stale/fuzz/Cargo.toml" >"${WORK}/stale.log" 2>&1 || status=$?
if [[ ${status} -eq 1 ]]; then
    ok "stale synthetic lock: exit 1 (fails closed)"
else
    no "stale synthetic lock: exited ${status}, expected 1"
    cat "${WORK}/stale.log"
fi
if grep -qF "does not resolve against that manifest" "${WORK}/stale.log"; then
    ok "the failure names the lockfile/manifest pair, not a guessed cause"
else
    no "the failure names the lockfile/manifest pair, not a guessed cause"
    cat "${WORK}/stale.log"
fi
if grep -qF "cargo generate-lockfile --manifest-path" "${WORK}/stale.log"; then
    ok "the failure prints the remedy"
else
    no "the failure prints the remedy"
    cat "${WORK}/stale.log"
fi

# ------------------------------------------------------------------ case 4/5: bad invocation
echo "== a missing manifest or missing cargo is a broken check, not a stale lock =="
status=0
bash "${SUBJECT}" "${WORK}/nope/Cargo.toml" >"${WORK}/missing.log" 2>&1 || status=$?
if [[ ${status} -eq 2 ]]; then
    ok "missing manifest: exit 2 (usage), not exit 1"
else
    no "missing manifest: exited ${status}, expected 2"
    cat "${WORK}/missing.log"
fi
mkdir -p "${WORK}/bin"
# A non-executable `cargo` stub in a PATH that holds nothing else. `env PATH=…`
# rather than `PATH=… bash`: bash resolves the command name of a prefixed
# assignment through the *modified* PATH, so the plain form fails with 127
# "bash: command not found" and would assert on the harness, not the sensor. The
# symlinks keep the utilities the subject legitimately uses reachable, so the only
# thing missing is cargo itself.
ln -sf "$(command -v bash)" "${WORK}/bin/bash"
ln -sf "$(command -v dirname)" "${WORK}/bin/dirname"
printf '#!/usr/bin/env bash\nexit 127\n' >"${WORK}/bin/cargo"
chmod -x "${WORK}/bin/cargo"
status=0
env PATH="${WORK}/bin" bash "${SUBJECT}" "${WORK}/fresh/fuzz/Cargo.toml" \
    >"${WORK}/nocargo.log" 2>&1 || status=$?
if [[ ${status} -eq 2 ]]; then
    ok "cargo present but not executable: exit 2, not a lock verdict"
else
    no "cargo present but not executable: exited ${status}, expected 2"
    cat "${WORK}/nocargo.log"
fi
if grep -qF "resolves against its committed lockfile" "${WORK}/nocargo.log"; then
    no "an unusable cargo must not print a fresh-lock line"
else
    ok "an unusable cargo does not print a fresh-lock line"
fi
if grep -qF "does not resolve against that manifest" "${WORK}/nocargo.log"; then
    no "an unusable cargo must not report a stale lockfile"
else
    ok "an unusable cargo does not report a stale lockfile"
fi

# Same probe with no cargo file at all: `command -v` catches it before the
# execution attempt, so the diagnosis names the missing tool.
rm -f "${WORK}/bin/cargo"
status=0
env PATH="${WORK}/bin" bash "${SUBJECT}" "${WORK}/fresh/fuzz/Cargo.toml" \
    >"${WORK}/nopath.log" 2>&1 || status=$?
if [[ ${status} -eq 2 ]] && grep -qF "cargo not found" "${WORK}/nopath.log"; then
    ok "cargo absent from PATH: exit 2 naming the missing tool"
else
    no "cargo absent from PATH: exit 2 naming the missing tool (got ${status})"
    cat "${WORK}/nopath.log"
fi

# ------------------------------------------------------------------ case 6: wiring
echo "== validate.sh runs the check =="
if grep -qF 'check-fuzz-lock.sh' "${SCRIPT_DIR}/validate.sh"; then
    ok "validate.sh calls scripts/check-fuzz-lock.sh"
else
    no "validate.sh calls scripts/check-fuzz-lock.sh"
fi
if grep -qF 'test-check-fuzz-lock.sh' "${SCRIPT_DIR}/validate.sh"; then
    ok "validate.sh runs this fixture"
else
    no "validate.sh runs this fixture"
fi

printf '\nfuzz lock fixture: %s case(s), %s failure(s)\n' "${RUNS}" "${FAILED}"
[[ ${FAILED} -eq 0 ]] || exit 1
