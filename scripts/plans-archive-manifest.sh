#!/usr/bin/env bash
# Non-destructive plan archive: write manifest + optional move of immutable history.
# ADR-0096 compact_active_plans_non_destructively.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLANS="${ROOT}/plans"
ARCHIVE="${PLANS}/.archive"
MANIFEST="${ARCHIVE}/MANIFEST.md"
mkdir -p "$ARCHIVE"

{
  echo "# Plan archive manifest"
  echo ""
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo ""
  echo "## Active (do not move without reference audit)"
  echo ""
  echo "- \`plans/GOAP_STATE.md\` — canonical world state"
  echo "- \`plans/ACTIONS.md\` — action queue"
  echo "- \`plans/GOALS.md\` — goal targets"
  echo "- \`plans/ADR_REGISTRY.md\` — ADR index"
  echo "- \`plans/adr/*.md\` — architecture decisions"
  echo "- \`plans/WAVE_32_P2_PROGRESS.md\` — current wave progress"
  echo "- \`plans/GOAP_AUDIT_2026_07_14.md\` — wave 32 roadmap"
  echo "- \`plans/RECOMMENDATIONS_2026_07_14.md\` — user-owned (never auto-archive)"
  echo ""
  echo "## History candidates (immutable snapshots; safe to relocate with redirects)"
  echo ""
  for f in \
    VERIFICATION_2026_04_29.md \
    VERIFICATION_2026_04_30.md \
    GAP_ANALYSIS_2026_04_30.md \
    GAP_ANALYSIS_2026_06_26.md \
    GOAP_ANALYSIS_2026_04_25.md \
    WAVE_21_P0_COMPLETION.md
  do
    if [[ -f "${PLANS}/${f}" ]]; then
      echo "- \`plans/${f}\` → proposed \`plans/.archive/history/${f}\`"
    fi
  done
  echo ""
  echo "## Redirects"
  echo ""
  echo "When a file is moved, leave a stub at the old path:"
  echo ""
  echo '```markdown'
  echo '# Moved'
  echo 'This document was archived. See: plans/.archive/history/<name>'
  echo '```'
  echo ""
  echo "## Policy"
  echo ""
  echo "1. Never bulk-delete; always manifest + redirect stubs."
  echo "2. Audit inbound links (\`rg path plans AGENTS.md\`)."
  echo "3. User-owned recommendations require explicit approval."
} >"$MANIFEST"

# Optional apply: PLAN_ARCHIVE_APPLY=1 moves history candidates with stubs
if [[ "${PLAN_ARCHIVE_APPLY:-0}" == "1" ]]; then
  mkdir -p "${ARCHIVE}/history"
  for f in \
    VERIFICATION_2026_04_29.md \
    VERIFICATION_2026_04_30.md \
    GAP_ANALYSIS_2026_04_30.md \
    GAP_ANALYSIS_2026_06_26.md \
    GOAP_ANALYSIS_2026_04_25.md \
    WAVE_21_P0_COMPLETION.md
  do
    src="${PLANS}/${f}"
    [[ -f "$src" ]] || continue
    # skip if already a redirect stub
    if head -1 "$src" | grep -q '^# Moved'; then
      continue
    fi
    dest="${ARCHIVE}/history/${f}"
    if [[ ! -f "$dest" ]]; then
      mv "$src" "$dest"
    fi
    cat >"$src" <<EOF
# Moved

This document was archived non-destructively on $(date -u +%Y-%m-%d).

See: [\`plans/.archive/history/${f}\`](.archive/history/${f})
EOF
  done
  echo "applied archive moves with redirect stubs"
fi

echo "wrote $MANIFEST"
