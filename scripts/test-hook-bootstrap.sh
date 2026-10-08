#!/usr/bin/env bash
# =============================================================================
# test-hook-bootstrap.sh - Test git hook installation and validation
# =============================================================================
# Tests hook bootstrap operations in isolated temporary git repositories:
#   1. Clean installation via install-hooks.sh
#   2. setup-hooks.sh shim delegation
#   3. Actual Git hook execution in parent and linked worktrees
#   4. Validation pass under valid setup
#   5. Validation failure when local core.hooksPath is missing
#   6. Validation failure when a required hook is missing or non-executable
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

TEST_TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEST_TMP_DIR}"' EXIT

# Keep the fixture independent of the contributor's global identity/hooks.
export GIT_CONFIG_NOSYSTEM=1
export GIT_CONFIG_GLOBAL="${TEST_TMP_DIR}/global.config"

echo "🧪 Running hook bootstrap tests..."

# Helper to initialize mock repo
init_mock_repo() {
  local target_dir="$1"
  mkdir -p "${target_dir}"
  git init "${target_dir}" >/dev/null 2>&1
  git -C "${target_dir}" config user.name "Hook Fixture"
  git -C "${target_dir}" config user.email "hook-fixture@example.invalid"
  mkdir -p "${target_dir}/.githooks"

  # Copy real pre-commit and pre-push
  cp "${REPO_ROOT}/.githooks/pre-commit" "${target_dir}/.githooks/pre-commit"
  cp "${REPO_ROOT}/.githooks/pre-push" "${target_dir}/.githooks/pre-push"
  chmod 755 "${target_dir}/.githooks/pre-commit" "${target_dir}/.githooks/pre-push"

  mkdir -p "${target_dir}/scripts"
  cp "${REPO_ROOT}/scripts/install-hooks.sh" "${target_dir}/scripts/install-hooks.sh"
  cp "${REPO_ROOT}/scripts/setup-hooks.sh" "${target_dir}/scripts/setup-hooks.sh"
  cp "${REPO_ROOT}/scripts/validate-git-hooks.sh" "${target_dir}/scripts/validate-git-hooks.sh"
  chmod 755 "${target_dir}/scripts"/*.sh
}

# 1. Clean installation
MOCK_1="${TEST_TMP_DIR}/mock_repo_1"
init_mock_repo "${MOCK_1}"
(
  cd "${MOCK_1}"
  bash scripts/install-hooks.sh >/dev/null 2>&1

  HOOKS_PATH="$(git config --local core.hooksPath)"
  if [[ "${HOOKS_PATH}" != ".githooks" ]]; then
    echo "❌ Test 1 failed: core.hooksPath expected .githooks, got ${HOOKS_PATH}"
    exit 1
  fi

  if [[ ! -x .git/hooks/pre-commit || ! -x .git/hooks/pre-push ]]; then
    echo "❌ Test 1 failed: hooks not copied to .git/hooks or not executable"
    exit 1
  fi
)
echo "  ✓ Test 1: Clean install sets core.hooksPath=.githooks and copies hooks"

# 2. setup-hooks.sh shim delegation
MOCK_2="${TEST_TMP_DIR}/mock_repo_2"
init_mock_repo "${MOCK_2}"
(
  cd "${MOCK_2}"
  bash scripts/setup-hooks.sh >/dev/null 2>&1

  HOOKS_PATH="$(git config --local core.hooksPath)"
  if [[ "${HOOKS_PATH}" != ".githooks" ]]; then
    echo "❌ Test 2 failed: setup-hooks.sh shim failed to set core.hooksPath"
    exit 1
  fi
)
echo "  ✓ Test 2: setup-hooks.sh shim delegates correctly"

# 3. Validation success
(
  cd "${MOCK_1}"
  if ! bash scripts/validate-git-hooks.sh --check >/dev/null 2>&1; then
    echo "❌ Test 3 failed: validate-git-hooks.sh --check failed on valid repo"
    exit 1
  fi
)
echo "  ✓ Test 3: validate-git-hooks.sh passes on valid setup"

# 4. Validation failure on missing core.hooksPath
MOCK_4="${TEST_TMP_DIR}/mock_repo_4"
init_mock_repo "${MOCK_4}"
(
  cd "${MOCK_4}"
  if bash scripts/validate-git-hooks.sh --check >/dev/null 2>&1; then
    echo "❌ Test 4 failed: validate-git-hooks.sh should fail when core.hooksPath is unset"
    exit 1
  fi
)
echo "  ✓ Test 4: validate-git-hooks.sh fails when core.hooksPath is unset"

# 5. Validation failure on missing hook in .githooks
MOCK_5="${TEST_TMP_DIR}/mock_repo_5"
init_mock_repo "${MOCK_5}"
(
  cd "${MOCK_5}"
  bash scripts/install-hooks.sh >/dev/null 2>&1
  rm -f .githooks/pre-push
  if bash scripts/validate-git-hooks.sh --check >/dev/null 2>&1; then
    echo "❌ Test 5 failed: validate-git-hooks.sh should fail when pre-push is missing"
    exit 1
  fi
)
echo "  ✓ Test 5: validate-git-hooks.sh fails when required hook is missing"

# 6. Validation failure on non-executable hook
MOCK_6="${TEST_TMP_DIR}/mock_repo_6"
init_mock_repo "${MOCK_6}"
(
  cd "${MOCK_6}"
  bash scripts/install-hooks.sh >/dev/null 2>&1
  chmod 644 .githooks/pre-commit
  if bash scripts/validate-git-hooks.sh --check >/dev/null 2>&1; then
    echo "❌ Test 6 failed: validate-git-hooks.sh should fail when pre-commit is not executable"
    exit 1
  fi
)
echo "  ✓ Test 6: validate-git-hooks.sh fails when required hook is not executable"

# 7. A configured but ineffective path must not pass just because .githooks exists.
(
  cd "${MOCK_1}"
  for bad_path in /dev/null missing-hooks "$MOCK_2/.githooks"; do
    git config --local core.hooksPath "$bad_path"
    if bash scripts/validate-git-hooks.sh --check >"$TEST_TMP_DIR/stdout" 2>"$TEST_TMP_DIR/stderr"; then
      echo "Test 7 failed: accepted ineffective path $bad_path" >&2
      exit 1
    fi
    grep -q 'Error:' "$TEST_TMP_DIR/stderr"
    if grep -q 'Error:' "$TEST_TMP_DIR/stdout"; then
      echo "Test 7 failed: error diagnostic written to stdout" >&2
      exit 1
    fi
  done
  bash scripts/install-hooks.sh >/dev/null
)
echo "  ✓ Test 7: disabled, missing and sibling hook paths fail with stderr diagnostics"

# 8. Local config must override a global hook path; command overrides must fail.
git config --global core.hooksPath /dev/null
(
  cd "$MOCK_1"
  bash scripts/validate-git-hooks.sh --check >/dev/null
  export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/dev/null
  if bash scripts/validate-git-hooks.sh --check >/dev/null 2>&1; then
    echo "Test 8 failed: command override disabled hooks but validation passed" >&2
    exit 1
  fi
)
git config --global --unset core.hooksPath
echo "  ✓ Test 8: effective configuration honors local precedence and detects overrides"

# 9. Missing hooks fail before a partial fallback installation or config write.
MOCK_9="${TEST_TMP_DIR}/missing-hook"
init_mock_repo "$MOCK_9"
(
  cd "$MOCK_9"
  rm .githooks/pre-push
  if bash scripts/install-hooks.sh >"$TEST_TMP_DIR/stdout" 2>"$TEST_TMP_DIR/stderr"; then
    echo "Test 9 failed: installed an incomplete hook set" >&2
    exit 1
  fi
  [[ ! -f .git/hooks/pre-commit ]]
  [[ -z "$(git config --local core.hooksPath || true)" ]]
  grep -q 'Required hook pre-push missing' "$TEST_TMP_DIR/stderr"
)
echo "  ✓ Test 9: missing source hook fails before installation"

# 10. --install validates its result and remains idempotent; --link is compatible.
(
  cd "$MOCK_2"
  bash scripts/validate-git-hooks.sh --install >/dev/null
  bash scripts/install-hooks.sh --link >/dev/null
  bash scripts/install-hooks.sh >/dev/null
  bash scripts/validate-git-hooks.sh --check >/dev/null
  cmp .githooks/pre-commit .git/hooks/pre-commit
  cmp .githooks/pre-push .git/hooks/pre-push
)
echo "  ✓ Test 10: install entry points are idempotent and keep canonical hook copies"

# 11. Warning mode is explicit; unknown options fail with stderr diagnostics.
(
  cd "$MOCK_4"
  bash scripts/validate-git-hooks.sh --warn-only >/dev/null 2>&1
  for script in install-hooks.sh validate-git-hooks.sh; do
    if bash "scripts/$script" --unknown >"$TEST_TMP_DIR/stdout" 2>"$TEST_TMP_DIR/stderr"; then
      echo "Test 11 failed: $script accepted an unknown option" >&2
      exit 1
    fi
    [[ -s "$TEST_TMP_DIR/stderr" ]]
  done
)
echo "  ✓ Test 11: explicit warn-only mode and invalid option handling"

# Stub only the Rust toolchain to observe real Git invocation and propagation.
# This does not claim Rust compilation coverage; CI runs the real fmt/clippy.
mkdir -p "$TEST_TMP_DIR/bin"
cat >"$TEST_TMP_DIR/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf '%s|%s\n' "$PWD" "$*" >>"$HOOK_CARGO_LOG"
if [[ -n "${HOOK_SLEEP_SECONDS:-}" ]]; then sleep "$HOOK_SLEEP_SECONDS"; fi
if [[ "${HOOK_FAIL_COMMAND:-}" == "${1:-}" ]]; then exit 42; fi
CARGO
chmod 755 "$TEST_TMP_DIR/bin/cargo"
export PATH="$TEST_TMP_DIR/bin:$PATH"
export HOOK_CARGO_LOG="$TEST_TMP_DIR/cargo.log"
touch "$HOOK_CARGO_LOG"

# 12. Exercise installed hooks through Git, not just by inspecting config text.
MOCK_12="${TEST_TMP_DIR}/parent repo"
init_mock_repo "$MOCK_12"
(
  cd "$MOCK_12"
  mkdir src crates
  touch src/.gitkeep crates/.gitkeep
  bash scripts/install-hooks.sh >/dev/null
  git add .
  git commit -m 'chore(ci): hook fixture' >/dev/null
  grep -Fq "$MOCK_12|fmt" "$HOOK_CARGO_LOG"
  grep -Fq "$MOCK_12|clippy" "$HOOK_CARGO_LOG"
)
echo "  ✓ Test 12: Git invokes pre-commit from a repository path with spaces"

# 13. Installing in a sibling must not redirect the parent; both hooks run there.
WORKTREE="${TEST_TMP_DIR}/linked worktree"
git -C "$MOCK_12" worktree add -b hook-fixture "$WORKTREE" >/dev/null 2>&1
(
  cd "$WORKTREE"
  bash scripts/install-hooks.sh --link >/dev/null
  bash scripts/validate-git-hooks.sh --check >/dev/null
  touch worktree-file
  git add worktree-file
  git commit -m 'chore(ci): worktree fixture' >/dev/null
  .githooks/pre-push
  grep -Fq "$WORKTREE|fmt" "$HOOK_CARGO_LOG"
  grep -Fq "$WORKTREE|clippy" "$HOOK_CARGO_LOG"
)
(
  cd "$MOCK_12"
  bash scripts/validate-git-hooks.sh --check >/dev/null
  : >"$HOOK_CARGO_LOG"
  git commit --allow-empty -m 'chore(ci): parent still works' >/dev/null
  grep -Fq "$MOCK_12|fmt" "$HOOK_CARGO_LOG"
  if grep -Fq "$WORKTREE|" "$HOOK_CARGO_LOG"; then
    echo "Test 13 failed: parent invoked sibling hooks" >&2
    exit 1
  fi
)
echo "  ✓ Test 13: linked worktree installation preserves both checkout hook paths"

# 14. The common-dir fallback must resolve the active root, not its .git folder.
(
  cd "$WORKTREE"
  git config --local --unset core.hooksPath
  : >"$HOOK_CARGO_LOG"
  git commit --allow-empty -m 'chore(ci): fallback fixture' >/dev/null
  grep -Fq "$WORKTREE|fmt" "$HOOK_CARGO_LOG"
  "$(git rev-parse --git-common-dir)/hooks/pre-push"
  grep -Fq "$WORKTREE|clippy" "$HOOK_CARGO_LOG"
  bash scripts/install-hooks.sh >/dev/null
)
echo "  ✓ Test 14: common-dir fallback executes in the active worktree"

# 15. Gate failures must actually block a commit/push, not report success.
(
  cd "$WORKTREE"
  export HOOK_FAIL_COMMAND=fmt
  if git commit --allow-empty -m 'chore(ci): must fail' >/dev/null 2>&1; then
    echo "Test 15 failed: failed format check allowed a commit" >&2
    exit 1
  fi
  export HOOK_FAIL_COMMAND=clippy
  if .githooks/pre-push >/dev/null 2>&1; then
    echo "Test 15 failed: failed clippy check allowed a push" >&2
    exit 1
  fi
)
echo "  ✓ Test 15: format and clippy failures propagate through hooks"

# 16. A fresh clone has no local hook config; reproduce CI's bootstrap sequence.
CLONE="${TEST_TMP_DIR}/fresh clone"
git clone --quiet "$MOCK_12" "$CLONE"
(
  cd "$CLONE"
  if bash scripts/validate-git-hooks.sh --check >/dev/null 2>&1; then
    echo "Test 16 failed: unbootstrapped clone passed strict validation" >&2
    exit 1
  fi
  bash scripts/install-hooks.sh >/dev/null
  bash scripts/validate-git-hooks.sh --check >/dev/null
)
echo "  ✓ Test 16: fresh clone requires bootstrap before strict validation"

# 17. Filename spaces must not bypass the LOC gate's file iteration.
(
  cd "$WORKTREE"
  for ((line = 0; line < 501; line++)); do echo '// fixture'; done >'src/too many lines.rs'
  git add 'src/too many lines.rs'
  if git commit -m 'chore(ci): oversized fixture' >"$TEST_TMP_DIR/stdout" 2>"$TEST_TMP_DIR/stderr"; then
    echo "Test 17 failed: oversized source with spaces bypassed LOC gate" >&2
    exit 1
  fi
  # Git forwards hook output to stderr, including the LOC diagnostic.
  grep -Fq 'src/too many lines.rs has 501 lines' "$TEST_TMP_DIR/stderr"
)
echo "  ✓ Test 17: LOC checks handle filenames with spaces"

# 18. A cold/stuck Cargo command cannot turn pre-push into an unbounded wait.
(
  cd "$WORKTREE"
  if HOOK_SLEEP_SECONDS=2 CSM_PRE_PUSH_TIMEOUT_SECONDS=1 .githooks/pre-push >"$TEST_TMP_DIR/stdout" 2>"$TEST_TMP_DIR/stderr"; then
    echo "Test 18 failed: timed-out check allowed a push" >&2
    exit 1
  else
    hook_rc=$?
    [[ "$hook_rc" -eq 124 ]]
  fi
  grep -q 'pre-push exceeded 1s' "$TEST_TMP_DIR/stderr"
)
echo "  ✓ Test 18: pre-push timeout fails closed with a bounded wait"

# 19. Invalid budget input cannot silently disable the bound.
(
  cd "$WORKTREE"
  if CSM_PRE_PUSH_TIMEOUT_SECONDS=0 .githooks/pre-push >"$TEST_TMP_DIR/stdout" 2>"$TEST_TMP_DIR/stderr"; then
    echo "Test 19 failed: invalid timeout allowed a push" >&2
    exit 1
  fi
  grep -q 'must be a positive integer' "$TEST_TMP_DIR/stderr"
)
echo "  ✓ Test 19: invalid timeout settings fail explicitly"

# 20. The existing explicit bypass remains available.
(
  cd "$WORKTREE"
  SKIP_PRE_PUSH=1 HOOK_FAIL_COMMAND=fmt .githooks/pre-push >/dev/null
)
echo "  ✓ Test 20: explicit pre-push bypass is preserved"

echo "✅ All hook bootstrap tests passed!"
