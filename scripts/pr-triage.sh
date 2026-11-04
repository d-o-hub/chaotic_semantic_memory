#!/usr/bin/env bash
# scripts/pr-triage.sh — read-only overview of all open PRs.
#
# Issue #993: the previous version called `gh pr checks` twice per PR and counted
# the *text* "pass"/"fail"/"pending". A failed, rate-limited, empty or
# unauthenticated query could therefore report "✅ all pass"; a skipped check was
# counted as nothing; and the two calls could observe different heads.
#
# This version makes one check-inventory request per PR (`gh pr checks --json`),
# captures the query's exit status and stderr separately, pins the head SHA from
# `gh pr list` and re-reads it after the checks, and classifies every state
# explicitly. It never turns missing data into a pass.
#
# Usage: pr-triage.sh [--json] [--strict]
#   --json    emit a machine-readable array instead of the human table
#   --strict  exit 1 when any PR is not PASS (BLOCKED/UNKNOWN/STALE/FAIL/...)
#
# This is a triage helper, not a merge engine: a PASS is a check inventory only.
# Required contexts and the branch ruleset remain the merge gate.
set -uo pipefail

STRICT=0
JSON=0
for arg in "$@"; do
  case "${arg}" in
    --json) JSON=1 ;;
    --strict) STRICT=1 ;;
    -h|--help)
      sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "error: unknown argument: ${arg}" >&2
      exit 2
      ;;
  esac
done

fail_closed() {
  local message="$1"
  if [[ "${JSON}" -eq 1 ]]; then
    jq -cn --arg detail "${message}" \
      '[{number: null, title: "triage", mergeable: null, head: null, verdict: "BLOCKED", counts: {pass: 0, fail: 0, pending: 0, skipped: 0, cancelled: 0, unknown: 0}, detail: $detail}]'
  else
    echo "=== Open PR Triage ==="
    echo ""
    echo "❌ BLOCKED: ${message}"
    echo "No pass/fail/pending verdict is available; this is not green."
  fi
  [[ "${STRICT}" -eq 1 ]] && exit 1
  exit 0
}

command -v gh >/dev/null 2>&1 || fail_closed "gh is not installed"
command -v jq >/dev/null 2>&1 || fail_closed "jq is not installed"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT
RESULTS="${TMP_DIR}/results.jsonl"
: > "${RESULTS}"

if ! PRS="$(gh pr list --state open --json number,title,mergeable,headRefName,headRefOid 2>"${TMP_DIR}/list.err")"; then
  fail_closed "gh pr list failed: $(tr '\n' ' ' < "${TMP_DIR}/list.err")"
fi
if ! jq -e 'type == "array"' >/dev/null 2>&1 <<< "${PRS}"; then
  fail_closed "gh pr list returned data that is not a JSON array"
fi
if [[ "$(jq 'length' <<< "${PRS}")" -eq 0 ]]; then
  if [[ "${JSON}" -eq 1 ]]; then
    echo '[]'
  else
    echo "No open PRs."
  fi
  exit 0
fi

if [[ "${JSON}" -eq 0 ]]; then
  echo "=== Open PR Triage ==="
  echo ""
  printf '%-7s %-10s %-8s %s\n' "PR" "CI" "MERGE" "TITLE"
fi

ANY_NOT_PASS=0

while IFS= read -r row; do
  num="$(jq -r '.number' <<< "${row}")"
  title="$(jq -r '.title' <<< "${row}")"
  mergeable="$(jq -r '.mergeable' <<< "${row}")"
  head="$(jq -r '.headRefOid' <<< "${row}")"

  verdict=""
  detail=""
  pass=0 fail=0 pending=0 skipped=0 cancelled=0 unknown=0
  counts_json='{"pass":0,"fail":0,"pending":0,"skipped":0,"cancelled":0,"unknown":0}'

  checks_rc=0
  CHECKS="$(gh pr checks "${num}" --json name,state,link 2>"${TMP_DIR}/checks.err")" || checks_rc=$?
  if [[ "${checks_rc}" -ne 0 ]]; then
    if jq -e 'type == "array" and length == 0' >/dev/null 2>&1 <<< "${CHECKS}"; then
      verdict="UNKNOWN"
      detail="gh pr checks returned no checks (exit ${checks_rc}) — an empty inventory is not green"
    else
      verdict="BLOCKED"
      detail="gh pr checks failed (exit ${checks_rc}): $(tr '\n' ' ' < "${TMP_DIR}/checks.err")"
    fi
  fi

  if [[ -z "${verdict}" ]]; then
    if ! jq -e 'type == "array"' >/dev/null 2>&1 <<< "${CHECKS}"; then
      verdict="BLOCKED"
      detail="gh pr checks returned data that is not a JSON array"
    elif [[ "$(jq 'length' <<< "${CHECKS}")" -eq 0 ]]; then
      verdict="UNKNOWN"
      detail="no checks reported — an empty inventory is not green"
    else
      counts_json="$(jq -c '
        def norm: (.state // "" | ascii_upcase);
        reduce .[] as $c
          ({pass: 0, fail: 0, pending: 0, skipped: 0, cancelled: 0, unknown: 0};
            ($c | norm) as $s |
            if $s == "SUCCESS" then .pass += 1
            elif ($s == "PENDING" or $s == "QUEUED" or $s == "IN_PROGRESS" or $s == "WAITING" or $s == "REQUESTED") then .pending += 1
            elif ($s == "FAILURE" or $s == "ERROR" or $s == "TIMED_OUT" or $s == "ACTION_REQUIRED" or $s == "STARTUP_FAILURE") then .fail += 1
            elif ($s == "CANCELLED" or $s == "STALE") then .cancelled += 1
            elif ($s == "SKIPPED" or $s == "NEUTRAL") then .skipped += 1
            else .unknown += 1 end)' <<< "${CHECKS}")"
      pass="$(jq -r '.pass' <<< "${counts_json}")"
      fail="$(jq -r '.fail' <<< "${counts_json}")"
      pending="$(jq -r '.pending' <<< "${counts_json}")"
      skipped="$(jq -r '.skipped' <<< "${counts_json}")"
      cancelled="$(jq -r '.cancelled' <<< "${counts_json}")"
      unknown="$(jq -r '.unknown' <<< "${counts_json}")"

      unknown_names="$(jq -r '
        def known: ["SUCCESS","PENDING","QUEUED","IN_PROGRESS","WAITING","REQUESTED",
          "FAILURE","ERROR","TIMED_OUT","ACTION_REQUIRED","STARTUP_FAILURE",
          "CANCELLED","STALE","SKIPPED","NEUTRAL"];
        [.[] | select((.state // "" | ascii_upcase) as $s | (known | index($s)) == null) | .name]
        | join(", ")' <<< "${CHECKS}")"

      if ! HEAD_AFTER_JSON="$(gh pr view "${num}" --json headRefOid 2>"${TMP_DIR}/view.err")"; then
        verdict="BLOCKED"
        detail="could not re-read the PR head: $(tr '\n' ' ' < "${TMP_DIR}/view.err")"
      else
        head_after="$(jq -r '.headRefOid' <<< "${HEAD_AFTER_JSON}")"
        if [[ "${head_after}" != "${head}" ]]; then
          verdict="STALE"
          detail="head moved during collection: ${head} -> ${head_after}; verdict rejected"
        elif [[ "${fail}" -gt 0 ]]; then
          verdict="FAIL"
          detail="${fail} failing/error/timed-out/action-required check(s)"
        elif [[ "${cancelled}" -gt 0 ]]; then
          verdict="CANCELLED"
          detail="${cancelled} cancelled check(s)"
        elif [[ "${unknown}" -gt 0 ]]; then
          verdict="UNKNOWN"
          detail="${unknown} check(s) with an unrecognised state: ${unknown_names}"
        elif [[ "${pending}" -gt 0 ]]; then
          verdict="PENDING"
          detail="${pending} pending/queued check(s)"
        elif [[ "${pass}" -eq 0 ]]; then
          verdict="NO_VERDICT"
          detail="only skipped/neutral checks — nothing proves the gates ran"
        else
          verdict="PASS"
          detail="check inventory only; required contexts/ruleset are the merge gate"
          [[ "${skipped}" -gt 0 ]] && detail="${detail}; ${skipped} skipped/neutral"
        fi
      fi
    fi
  fi

  if [[ "${verdict}" != "PASS" ]]; then
    ANY_NOT_PASS=1
  fi

  jq -cn \
    --argjson number "${num}" \
    --arg title "${title}" \
    --arg mergeable "${mergeable}" \
    --arg head "${head}" \
    --arg verdict "${verdict}" \
    --arg detail "${detail}" \
    --argjson counts "${counts_json}" \
    '{number: $number, title: $title, mergeable: $mergeable, head: $head,
      verdict: $verdict, counts: $counts, detail: $detail}' >> "${RESULTS}"

  if [[ "${JSON}" -eq 0 ]]; then
    case "${verdict}" in
      PASS) icon="✅" ;;
      PENDING) icon="⏳" ;;
      FAIL|CANCELLED) icon="❌" ;;
      *) icon="⚠️ " ;;
    esac
    printf '#%-6s %s %-8s %-8s %s\n' "${num}" "${icon}" "${verdict}" "${mergeable}" "${title}"
    printf '        %s\n' "${detail}"
  fi
done < <(jq -c '.[]' <<< "${PRS}")

if [[ "${JSON}" -eq 1 ]]; then
  jq -s '.' "${RESULTS}"
else
  echo ""
  echo "--- Merge Status ---"
  echo "CI verdicts above are check inventories only (one request per PR, head-pinned)."
  echo "Required contexts and the branch ruleset remain the merge gate; never merge"
  echo "from a stale head and never use gh pr merge --auto (AGENTS.md Rule 18)."
fi

if [[ "${STRICT}" -eq 1 && "${ANY_NOT_PASS}" -eq 1 ]]; then
  exit 1
fi
exit 0
