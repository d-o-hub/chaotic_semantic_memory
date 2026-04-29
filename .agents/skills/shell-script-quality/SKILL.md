---
name: shell-script-quality
description: Ensure shell scripts pass ShellCheck linting and have BATS tests for complex logic. Use when writing or modifying shell scripts in scripts/ directory.
---

# Shell Script Quality

## When to Use

- Creating new shell scripts in `scripts/`
- Modifying existing shell scripts
- Scripts with complex logic (conditionals, loops, error handling)
- Scripts called by CI/CD pipelines

## Process

```
1. Write Script
   - Follow ShellCheck-compliant patterns from start
   - Use set -euo pipefail for strict mode

2. Run ShellCheck
   shellcheck scripts/my-script.sh
   Fix all warnings before committing

3. Add BATS Tests (if complex)
   - Create test file in tests/bats/
   - Test happy path, error cases, edge cases

4. Verify
   - Script runs correctly locally
   - ShellCheck passes with no warnings
   - BATS tests pass (if applicable)
```

## Strict Mode Template

```bash
#!/usr/bin/env bash
set -euo pipefail

# Script description
# Usage: scripts/my-script.sh [options] <arg>

# Constants (use readonly)
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Functions before main logic
log_info() {
    echo "[INFO] $*"
}

log_error() {
    echo "[ERROR] $*" >&2
}

# Exit handlers
cleanup() {
    # Cleanup logic here
    :
}
trap cleanup EXIT

# Main logic
main() {
    local arg="${1:-default_value}"

    # Validate inputs
    if [[ ! -f "$arg" ]]; then
        log_error "File not found: $arg"
        exit 1
    fi

    log_info "Processing $arg"
    # ... main logic
}

main "$@"
```

## Common ShellCheck Warnings

| Code | Issue | Fix |
|------|-------|-----|
| SC2086 | Double quote to prevent globbing | `"$var"` not `$var` |
| SC2046 | Quote to avoid word splitting | `"$(cmd)"` not `$(cmd)` |
| SC2034 | Unused variable | Remove or prefix with `_` |
| SC2004 | Use $((..)) for arithmetic | `$((x + 1))` not `$(( $x + 1 ))` |
| SC2006 | Use $(..) not backticks | `$(cmd)` not `` `cmd` `` |
| SC2010 | Don't use ls | Use globs: `for f in *.txt` |
| SC2181 | Check exit code directly | `if cmd; then` not `cmd; if [ $? -eq 0 ]` |
| SC2230 | Use `command -v` not `which` | `command -v git` not `which git` |

## ShellCheck Inline Directives

```bash
# shellcheck disable=SC2086  # Explanation of why
some_command $unquoted_args  # Intentional word splitting

# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"
```

## BATS Test Template

```bash
#!/usr/bin/env bats

# tests/bats/my-script.bats

setup() {
    # Create temp directory for test files
    TEST_DIR="$(mktemp -d)"
    export TEST_DIR
}

teardown() {
    rm -rf "$TEST_DIR"
}

@test "happy path: process valid input" {
    echo "test content" > "${TEST_DIR}/input.txt"
    run scripts/my-script.sh "${TEST_DIR}/input.txt"

    [ "$status" -eq 0 ]
    [[ "$output" == *"Processing"* ]]
}

@test "error case: missing file" {
    run scripts/my-script.sh "${TEST_DIR}/nonexistent.txt"

    [ "$status" -eq 1 ]
    [[ "$output" == *"not found"* ]]
}

@test "edge case: empty input" {
    touch "${TEST_DIR}/empty.txt"
    run scripts/my-script.sh "${TEST_DIR}/empty.txt"

    [ "$status" -eq 0 ]
}
```

## BATS Installation

```bash
# Ubuntu/Debian
sudo apt-get install bats

# macOS
brew install bats-core

# Run tests
bats tests/bats/
```

## CI Integration

```yaml
# In .github/workflows/ci.yml
shellcheck:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install ShellCheck
      run: sudo apt-get install shellcheck
    - name: Run ShellCheck
      run: shellcheck scripts/*.sh

bats-tests:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install BATS
      run: sudo apt-get install bats
    - name: Run BATS tests
      run: bats tests/bats/
```

## Quality Checklist

- [ ] Script starts with `#!/usr/bin/env bash`
- [ ] Uses `set -euo pipefail` for strict mode
- [ ] All variables are quoted (`"$var"`)
- [ ] Uses `[[ ]]` for conditionals (not `[ ]`)
- [ ] Uses `$((..))` for arithmetic
- [ ] Uses `local` for function-scoped variables
- [ ] ShellCheck passes with no warnings
- [ ] BATS tests cover happy path and errors (if complex)
- [ ] CI pipeline runs ShellCheck automatically