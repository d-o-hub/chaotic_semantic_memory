#!/usr/bin/env bash
# =============================================================================
# test-check-shared-conventions.sh - Fixture for check-shared-conventions.sh
# =============================================================================
# Verifies pass direction on valid conventions file and fail directions on:
#   - Broken file path link
#   - Duplicated skill catalog table
#   - Line count ceiling violation (> 150 lines)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CHECKER="${SCRIPT_DIR}/check-shared-conventions.sh"

if [[ ! -x "${CHECKER}" ]]; then
    echo "❌ Error: Checker missing or not executable: ${CHECKER}" >&2
    exit 1
fi

TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TEMP_DIR}"' EXIT

echo "==> Testing check-shared-conventions.sh on valid file..."
if ! "${CHECKER}" --root "${PROJECT_ROOT}"; then
    echo "❌ Expected pass on valid file, but failed" >&2
    exit 1
fi

# Failure direction 1: Broken link
echo "==> Testing failure direction: Broken path link..."
BROKEN_FILE="${TEMP_DIR}/broken_link.md"
cp "${PROJECT_ROOT}/.agents/context/shared-conventions.md" "${BROKEN_FILE}"
echo "See also @nonexistent_missing_file_12345.md" >> "${BROKEN_FILE}"

if "${CHECKER}" --target "${BROKEN_FILE}" --root "${PROJECT_ROOT}" >/dev/null 2>&1; then
    echo "❌ Expected failure for broken path link, but passed" >&2
    exit 1
fi

# Failure direction 2: Duplicated skill catalog table
echo "==> Testing failure direction: Duplicated skill catalog table..."
CATALOG_FILE="${TEMP_DIR}/skill_catalog.md"
cp "${PROJECT_ROOT}/.agents/context/shared-conventions.md" "${CATALOG_FILE}"
cat <<'EOF' >> "${CATALOG_FILE}"

## Skill Catalog
| Skill | Trigger |
|---|---|
| `test-skill` | Do testing |
EOF

if "${CHECKER}" --target "${CATALOG_FILE}" --root "${PROJECT_ROOT}" >/dev/null 2>&1; then
    echo "❌ Expected failure for duplicated skill catalog table, but passed" >&2
    exit 1
fi

# Failure direction 3: Line count ceiling violation
echo "==> Testing failure direction: Excessive line count..."
LONG_FILE="${TEMP_DIR}/long_file.md"
cp "${PROJECT_ROOT}/.agents/context/shared-conventions.md" "${LONG_FILE}"
for i in $(seq 1 160); do
    echo "Line $i extra content" >> "${LONG_FILE}"
done

if "${CHECKER}" --target "${LONG_FILE}" --root "${PROJECT_ROOT}" >/dev/null 2>&1; then
    echo "❌ Expected failure for exceeding line count limit, but passed" >&2
    exit 1
fi

echo "✅ All test-check-shared-conventions fixture assertions passed successfully."
exit 0
