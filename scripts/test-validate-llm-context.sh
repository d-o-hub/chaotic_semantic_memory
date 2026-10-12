#!/usr/bin/env bash
# =============================================================================
# test-validate-llm-context.sh — fixture for tests/validate_llm_context.py
# =============================================================================
# Issue #828: docs/architecture/context.yaml drifted for months (skills 19/33,
# ADRs 16/95, active_wave 11/33, tests 134/1044, two dead key-file paths) and
# nothing read it. The validator now compares every counter and path against
# live sources; this fixture proves each failure direction on a synthetic tree
# so the gate cannot rot into a pass-for-the-wrong-reason check.
#
# Cases: a consistent tree passes; each corrupted counter (skills total,
# category vs CATALOG.md, ADR count, active_wave, tests_count), a dead
# key-file path, a dead module path, a wrong LOC-coverage count, an on-disk
# skill surplus, and a missing context.yaml each fail (rc 1, or rc 2 for the
# missing input). Without PyYAML the fixture SKIPS loudly — NOT A PASS.
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VALIDATOR="${REPO_ROOT}/tests/validate_llm_context.py"

if [[ ! -f "${VALIDATOR}" ]]; then
  echo "Error: ${VALIDATOR} not found" >&2
  exit 1
fi

if ! python3 -c 'import yaml' 2>/dev/null; then
  echo "SKIPPED — NOT A PASS: python3 has no PyYAML (apt: python3-yaml); the fixture cannot run"
  exit 0
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT
TREE="${TMP_DIR}/tree"

PASSED=0
FAILED=0
TESTS=0

write_tree() {
  rm -rf "${TREE}"
  mkdir -p "${TREE}/docs/architecture" \
           "${TREE}/.agents/skills/alpha" \
           "${TREE}/.agents/skills/beta" \
           "${TREE}/.agents/skills/gamma" \
           "${TREE}/plans/adr" \
           "${TREE}/src"

  : > "${TREE}/src/x.rs"
  : > "${TREE}/src/y.rs"
  : > "${TREE}/plans/adr/0001-a.md"
  : > "${TREE}/plans/adr/0002-b.md"
  for skill in alpha beta gamma; do
    : > "${TREE}/.agents/skills/${skill}/SKILL.md"
  done

  cat > "${TREE}/plans/GOAP_STATE.md" <<'EOF'
world_state:
  active_wave: 7
  tests_count: 42              # trailing comments are part of the format
EOF

  cat > "${TREE}/.agents/skills/CATALOG.md" <<'EOF'
# Skill Catalog

## Core (2)

| Skill | Trigger |
| --- | --- |
| `alpha` | a |
| `beta` | b |

## Swarm (1)

| Skill | Trigger |
| --- | --- |
| `gamma` | g |

## Consolidation Candidates

Sections without a count are not categories.
EOF

  cat > "${TREE}/docs/architecture/context.yaml" <<'EOF'
metadata:
  project_name: fixture
  last_updated: 2026-10-12

mission:
  statement: fixture tree

constraints:
  hard:
    - name: max_loc_per_file
      value: 500
    - name: database_libsql
      value: "libsql"
      forbidden: "turso-client"
    - name: wasm_threading
      gate: "#[cfg(not(target_arch = \"wasm32\"))]"

architecture:
  modules:
    - name: core
      file: src/x.rs
      purpose: fixture module

data_flow:
  flows:
    - name: concept_injection
      steps:
        - input: "id"
        - framework: "inject()"
        - output: "ok"

key_files:
  planning:
    - path: plans/GOAP_STATE.md
      purpose: state
  adrs:
    - path: plans/adr/
      purpose: Architecture Decision Records
      count: 2

skills:
  catalog: .agents/skills/CATALOG.md
  total_count: 3
  categories:
    - name: core
      count: 2
    - name: swarm
      count: 1

validation:
  gates:
    - name: check
      command: cargo check
    - name: test
      command: cargo test --all-features
    - name: format
      command: cargo fmt --check
    - name: clippy
      command: cargo clippy --all-targets --all-features -- -D warnings
    - name: loc
      command: scripts/validate.sh (includes LOC check)
  script: scripts/validate.sh
  performance_targets:
    - name: reservoir_step_50k
      target: "< 100μs"
      latest_measured: "76.627μs"
      status: passing

workflow:
  learning_loop:
    - step: 1
      action: one
    - step: 2
      action: two
    - step: 3
      action: three
    - step: 4
      action: four
    - step: 5
      action: five

current_status:
  active_wave: 7
  swarm_groups: 1
  total_tests: 42
  tests_passing: 42
  modules_under_loc_limit: 2/2

guardrails:
  - Never create unused code
EOF
}

check_case() {
  local description="$1"
  local want_rc="$2"
  local output rc
  set +e
  output="$(python3 "${VALIDATOR}" --repo-root "${TREE}" 2>&1)"
  rc=$?
  set -e
  TESTS=$((TESTS + 1))
  if [[ "${rc}" -eq "${want_rc}" ]]; then
    PASSED=$((PASSED + 1))
    echo "✅ Success (${description})"
  else
    FAILED=$((FAILED + 1))
    echo "❌ Failure (${description}): expected rc ${want_rc}, got ${rc}"
    echo "${output}" | grep -E 'FAIL|ERROR' | head -4 || true
  fi
}

echo "🧪 Running LLM context drift gate fixture..."

# 1. A consistent tree passes.
write_tree
check_case "consistent tree passes" 0

# 2. Declared skill total disagrees with the disk.
write_tree
sed -i 's/^  total_count: 3$/  total_count: 4/' "${TREE}/docs/architecture/context.yaml"
check_case "skills total mismatch fails" 1

# 3. On-disk skill surplus (the 19-vs-33 shape).
write_tree
mkdir -p "${TREE}/.agents/skills/delta"
: > "${TREE}/.agents/skills/delta/SKILL.md"
check_case "extra skill on disk fails" 1

# 4. Category count disagrees with its CATALOG.md header.
write_tree
sed -i '/^    - name: core$/{n;s/^      count: 2$/      count: 3/}' "${TREE}/docs/architecture/context.yaml"
check_case "category vs CATALOG.md mismatch fails" 1

# 5. ADR count disagrees with plans/adr/.
write_tree
sed -i '/^  adrs:$/{n;n;n;s/^      count: 2$/      count: 9/}' "${TREE}/docs/architecture/context.yaml"
check_case "ADR count mismatch fails" 1

# 6. active_wave disagrees with GOAP_STATE.md.
write_tree
sed -i 's/^  active_wave: 7$/  active_wave: 8/' "${TREE}/docs/architecture/context.yaml"
check_case "active_wave mismatch fails" 1

# 7. Any test counter disagrees with GOAP_STATE.md tests_count.
write_tree
sed -i 's/^  tests_passing: 42$/  tests_passing: 41/' "${TREE}/docs/architecture/context.yaml"
check_case "tests_passing mismatch fails" 1

write_tree
sed -i 's/^  tests_count: 42.*$/  tests_count: 43/' "${TREE}/plans/GOAP_STATE.md"
check_case "GOAP tests_count moved fails" 1

# 8. A dead key-file path (the plans/SWARM_COORDINATION.md shape).
write_tree
sed -i 's|^    - path: plans/GOAP_STATE.md$|    - path: plans/MISSING.md|' "${TREE}/docs/architecture/context.yaml"
check_case "dead key-file path fails" 1

# 9. A dead module path (the src/reservoir.rs shape).
write_tree
sed -i 's|^      file: src/x.rs$|      file: src/nope.rs|' "${TREE}/docs/architecture/context.yaml"
check_case "dead module path fails" 1

# 10. modules_under_loc_limit disagrees with the src/**/*.rs count.
write_tree
sed -i 's|^  modules_under_loc_limit: 2/2$|  modules_under_loc_limit: 3/3|' "${TREE}/docs/architecture/context.yaml"
check_case "LOC coverage mismatch fails" 1

# 11. Missing context.yaml is an explicit error (rc 2), not a silent pass.
write_tree
rm -f "${TREE}/docs/architecture/context.yaml"
check_case "missing context.yaml fails closed" 2

echo "────────────────────────────────────────────────────────"
echo "Tests run: ${TESTS}, failed: ${FAILED}"
if [[ "${FAILED}" -ne 0 ]]; then
  exit 1
fi
echo "All tests passed!"
