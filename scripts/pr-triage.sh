#!/usr/bin/env bash
# scripts/pr-triage.sh — Quick overview of all open PRs
# Shows: merge conflicts, CI failures, pending checks
set -euo pipefail

echo "=== Open PR Triage ==="
echo ""

# Get all open PRs
PRS=$(gh pr list --state open --json number,title,mergeable,headRefName \
  --jq '.[] | "\(.number)|\(.title)|\(.mergeable)|\(.headRefName)"')

if [[ -z "$PRS" ]]; then
  echo "No open PRs."
  exit 0
fi

echo "--- Merge Conflict Status ---"
while IFS='|' read -r num title mergeable branch; do
  if [[ "$mergeable" == "CONFLICTING" ]]; then
    status="❌ CONFLICTING"
  elif [[ "$mergeable" == "MERGEABLE" ]]; then
    status="✅ MERGEABLE"
  else
    status="⚠️  $mergeable"
  fi
  printf "#%-4s %s  %s\n" "$num" "$status" "$title"
done <<< "$PRS"

echo ""
echo "--- CI Status ---"
while IFS='|' read -r num title mergeable branch; do
  pass=$(gh pr checks "$num" 2>&1 | grep -c "pass" || true)
  fail=$(gh pr checks "$num" 2>&1 | grep -c "fail" || true)
  pend=$(gh pr checks "$num" 2>&1 | grep -c "pending" || true)
  
  if [[ "$fail" -gt 0 ]]; then
    ci="❌ ${fail} fail"
  elif [[ "$pend" -gt 0 ]]; then
    ci="⏳ ${pend} pending"
  else
    ci="✅ all pass"
  fi
  printf "#%-4s %s (%d pass)  %s\n" "$num" "$ci" "$pass" "$title"
done <<< "$PRS"

echo ""
echo "--- Recommended Merge Order ---"
echo "1. Fix CONFLICTING PRs first (resolve conflicts)"
echo "2. Merge independent PRs with all-green CI"
echo "3. Foundation PRs (config, lints) before dependent PRs"
echo "4. Rebase remaining PRs after each merge"
