#!/usr/bin/env bash
# Generate single-source skill catalog from .agents/skills/*/SKILL.md (ADR-0096).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SKILLS_DIR="${ROOT}/.agents/skills"
OUT="${ROOT}/.agents/skills/CATALOG.md"
CHECK=false

if [[ "${1:-}" == "--check" ]]; then
  CHECK=true
fi

tmp="$(mktemp)"
{
  echo "# Skill catalog (generated)"
  echo ""
  echo "Do not edit by hand. Regenerate: \`./scripts/generate-skill-catalog.sh\`"
  echo ""
  echo "| Skill | Path | LOC |"
  echo "|-------|------|----:|"
  count=0
  # Locale-stable sort so CI and local machines produce identical catalogs.
  while IFS= read -r -d '' skill_md; do
    dir="$(dirname "$skill_md")"
    name="$(basename "$dir")"
    loc=$(wc -l <"$skill_md" | tr -d ' ')
    rel="${skill_md#"$ROOT"/}"
    echo "| \`${name}\` | \`${rel}\` | ${loc} |"
    count=$((count + 1))
  done < <(find "$SKILLS_DIR" -mindepth 2 -maxdepth 2 -name SKILL.md -print0 | LC_ALL=C sort -z)
  echo ""
  echo "_Total: ${count} skills._"
} >"$tmp"

if [[ "$CHECK" == true ]]; then
  if [[ ! -f "$OUT" ]]; then
    echo "missing catalog: $OUT" >&2
    rm -f "$tmp"
    exit 1
  fi
  if ! diff -q "$tmp" "$OUT" >/dev/null; then
    echo "stale catalog: run ./scripts/generate-skill-catalog.sh" >&2
    rm -f "$tmp"
    exit 1
  fi
  echo "OK catalog current (${OUT})"
  rm -f "$tmp"
  exit 0
fi

mv "$tmp" "$OUT"
echo "wrote $OUT"
