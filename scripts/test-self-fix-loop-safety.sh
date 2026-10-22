#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEMP_ROOT"' EXIT
MOCK_BIN="$TEMP_ROOT/bin"
mkdir -p "$MOCK_BIN"
git init --bare -q "$TEMP_ROOT/remote.git"
cat > "$MOCK_BIN/gh" <<'SH'
#!/usr/bin/env bash
[[ "${1:-}" == auth && "${2:-}" == status ]]
SH
chmod +x "$MOCK_BIN/gh"

new_repo() {
  local repo="$1" branch="$2"
  mkdir -p "$repo"
  git -C "$repo" init -q -b "$branch"
  git -C "$repo" config user.email test@example.invalid
  git -C "$repo" config user.name "Safety Fixture"
  printf 'base\n' > "$repo/tracked.txt"
  git -C "$repo" add tracked.txt
  git -C "$repo" commit -qm "initial"
  git -C "$repo" remote add origin "$TEMP_ROOT/remote.git"
}

expect_refusal() {
  local repo="$1" label="$2"
  if (cd "$repo" && PATH="$MOCK_BIN:$PATH" "$SCRIPT_DIR/self-fix-loop.sh" --dry-run) >"$TEMP_ROOT/output" 2>&1; then
    printf 'expected refusal for %s\n' "$label" >&2
    cat "$TEMP_ROOT/output" >&2
    exit 1
  fi
  printf 'verified refusal: %s\n' "$label"
}

main_repo="$TEMP_ROOT/main"
new_repo "$main_repo" main
expect_refusal "$main_repo" main

branch_repo="$TEMP_ROOT/detached"
new_repo "$branch_repo" feature/safety
(cd "$branch_repo" && git checkout -q --detach)
expect_refusal "$branch_repo" detached-HEAD


mismatch_repo="$TEMP_ROOT/mismatch"
new_repo "$mismatch_repo" feature/safety
if (cd "$mismatch_repo" && PATH="$MOCK_BIN:$PATH" "$SCRIPT_DIR/self-fix-loop.sh" --dry-run --branch other) >"$TEMP_ROOT/output" 2>&1; then
  printf 'expected refusal for mismatched --branch\n' >&2
  exit 1
fi

unstaged_repo="$TEMP_ROOT/unstaged"
new_repo "$unstaged_repo" feature/safety
printf 'unrelated\n' >> "$unstaged_repo/tracked.txt"
expect_refusal "$unstaged_repo" unstaged-worktree-change

untracked_repo="$TEMP_ROOT/untracked"
new_repo "$untracked_repo" feature/safety
printf 'unrelated\n' > "$untracked_repo/unrelated.txt"
expect_refusal "$untracked_repo" untracked-file

remote_repo="$TEMP_ROOT/no-confirm"
new_repo "$remote_repo" feature/safety
printf 'intended\n' > "$remote_repo/intended.txt"
git -C "$remote_repo" add intended.txt
if (cd "$remote_repo" && PATH="$MOCK_BIN:$PATH" "$SCRIPT_DIR/self-fix-loop.sh" --max-iterations 1) >"$TEMP_ROOT/output" 2>&1; then
  printf 'expected refusal before remote mutation without confirmation\n' >&2
  exit 1
fi
if git --git-dir="$TEMP_ROOT/remote.git" rev-parse --verify refs/heads/feature/safety >/dev/null 2>&1; then
  printf 'remote branch changed without confirmation\n' >&2
  exit 1
fi
git -C "$remote_repo" show --format= --name-only HEAD | grep -Fxq intended.txt || {
  printf 'expected only the staged file to be committed locally\n' >&2
  exit 1
}
printf 'verified refusal: push requires exact target confirmation\n'
