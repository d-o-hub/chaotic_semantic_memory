#!/usr/bin/env bash
# scripts/check-goap-queue-issues.sh — issue #851: reconcile the GOAP action
# queue (plans/ACTIONS.md) against the GitHub tracker and plans/GOAP_STATE.md.
#
# WHY (measured, not hypothetical): PR #836 merged as 2f26214 with "Fixes #824",
# GitHub auto-closed #824, and on that same commit
# cover_sigterm_in_server_shutdown was still queued in plans/ACTIONS.md with
# status: in_progress while `git show 2f26214:src/shutdown.rs | grep -c sigterm`
# returned 3 — the defect it describes had landed. #837 (87fa734) and #838
# (f235874) drifted identically the same morning; nothing in scripts/ or
# .github/ read these two files, so the queue kept claiming finished work as live.
#
# CHECKS   STALE      queued action whose github_issue is CLOSED on GitHub
#          UNQUEUED   OPEN issue titled "GOAP: ..." no queued action carries
#          UNDECLARED queued action with no github_issue: key at all
#          STATE      GOAP_STATE.md self-consistency: action_last_completed
#                     exactly once and not still queued, queued_actions_count ==
#                     queue length, and no queued action whose effect is already
#                     reported true in world_state
#
# TRACKER  Exactly ONE "gh issue list --state all --limit 500 --json
# number,state,title" call feeds every tracker-dependent check — a per-issue
# loop is one API call per action on every validate.sh run, which is how lint
# jobs get rate-limited into silence. With gh missing or the query failing (no
# token, or a token without issues: read) the tracker checks print "SKIP — NOT A
# PASS" and are skipped while UNDECLARED/STATE still run;
# CSM_GOAP_QUEUE_REQUIRED=true (the CI lint job) makes an unreachable tracker
# fatal instead of a silent pass.
#
# Exit 0 clean, 1 any error (a missing input file or a zero-action parse is an
# error, never "all clear"), 2 on a bad invocation. Usage:
# check-goap-queue-issues.sh [--repo-root DIR]; CSM_PLANS_DIR=<dir> overrides
# where the two plans files live. Fixture: scripts/test-goap-queue-issues.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLANS_DIR=""
if [[ "${1:-}" == "--repo-root" ]]; then
    if [[ "$#" -lt 2 || ! -d "$2" ]]; then
        echo "Error: --repo-root requires an existing directory" >&2
        exit 2
    fi
    ROOT="$(cd "$2" && pwd)"
    PLANS_DIR="${ROOT}/plans"
    shift 2
fi
if [[ "$#" -gt 0 ]]; then
    echo "Error: unknown argument: $1" >&2
    exit 2
fi

# An explicit --repo-root outranks an ambient CSM_PLANS_DIR: measured on
# 2026-10-05, exporting CSM_PLANS_DIR while scripts/validate.sh ran the fixture
# made every sandboxed test read the real plans tree instead (40 assertions, 12
# red). A caller's environment must not be able to re-point an explicit argument.
PLANS_DIR="${PLANS_DIR:-${CSM_PLANS_DIR:-${ROOT}/plans}}"
ACTIONS="${PLANS_DIR}/ACTIONS.md"
STATE="${PLANS_DIR}/GOAP_STATE.md"

# Fail closed on our own inputs — a missing file is not "nothing to reconcile".
for required in "${ACTIONS}" "${STATE}"; do
    if [[ ! -f "${required}" ]]; then
        echo "Error: GOAP queue gate input missing: ${required}" >&2
        echo "       failing closed: an unreadable queue is not an empty, valid one" >&2
        exit 1
    fi
done

ERRORS=()
add_error() { ERRORS+=("$1"); }

# --- parse the queue -------------------------------------------------------
# One awk pass, state machine: "^  - name:" opens an action, "^    effects:"
# opens the effect block and any other 4-space key closes it — so the 6-space
# prose inside a "notes: >" folded scalar (e.g. "      constrained: it must be
# a root-crate unit test") is never mistaken for an effect key.
# shellcheck disable=SC2016  # awk reads its record variable literally; no shell expansion wanted
RECORDS="$(awk '
    {
        t = $0
        if (t ~ /^  - name:/) {
            sub(/^  - name:[ \t]*/, "", t)
            sub(/[ \t].*$/, "", t)
            printf("ACTION\t%s\n", t)
            ineffect = 0
        } else if (t ~ /^    github_issue:/) {
            v = t
            sub(/^    github_issue:[ \t]*/, "", v)
            gsub(/[^0-9]+/, "", v)
            printf("ISSUE\t%s\n", v)
            ineffect = 0
        } else if (t ~ /^    effects:[ \t]*$/) {
            ineffect = 1
        } else if (t ~ /^    [A-Za-z0-9_]+:/) {
            ineffect = 0
        } else if (ineffect && t ~ /^      [A-Za-z0-9_]+:([ \t]|$)/) {
            k = t
            sub(/^      /, "", k)
            sub(/:.*$/, "", k)
            printf("EFFECT\t%s\n", k)
        }
    }
' "${ACTIONS}")"

declare -a ACTION_NAMES=()
declare -A ACTION_ISSUE_OF=()
declare -A EFFECT_OWNER=()
current=""
while IFS=$'\t' read -r kind field; do
    case "${kind}" in
        ACTION)
            current="${field}"
            ACTION_NAMES+=("${current}")
            ACTION_ISSUE_OF["${current}"]=""
            ;;
        ISSUE)
            if [[ -n "${current}" ]]; then ACTION_ISSUE_OF["${current}"]="${field}"; fi
            ;;
        EFFECT)
            if [[ -n "${current}" && -n "${field}" ]]; then EFFECT_OWNER["${field}"]="${current}"; fi
            ;;
    esac
done <<< "${RECORDS}"

if [[ "${#ACTION_NAMES[@]}" -eq 0 ]]; then
    echo "Error: GOAP queue gate parsed 0 actions from ${ACTIONS}" >&2
    echo "       the parser keys on '^  - name:'; zero actions means the queue is" >&2
    echo "       genuinely empty or the anchor broke. Both fail closed." >&2
    exit 1
fi

# --- UNDECLARED (tracker-independent) --------------------------------------
declare -A QUEUED_BY_ISSUE=()
for action in "${ACTION_NAMES[@]}"; do
    issue="${ACTION_ISSUE_OF[${action}]}"
    if [[ -z "${issue}" ]]; then
        add_error "UNDECLARED: action '${action}' has no github_issue: key — every queued action must be tracked as an issue"
        continue
    fi
    if [[ ! "${issue}" =~ ^[0-9]+$ ]]; then
        add_error "UNDECLARED: action '${action}' has github_issue: '${issue}' which is not an issue number"
        continue
    fi
    QUEUED_BY_ISSUE["${issue}"]="${action}"
done

# --- STATE (tracker-independent) -------------------------------------------
# Anchored to two spaces of indentation: line 10 of GOAP_STATE.md is prose in
# the header blockquote that names the same key, and an unanchored count reads 2.
LAST_COMPLETED_COUNT="$(grep -c '^  action_last_completed:' "${STATE}" || true)"
if [[ "${LAST_COMPLETED_COUNT}" -ne 1 ]]; then
    add_error "STATE: action_last_completed appears ${LAST_COMPLETED_COUNT} time(s) as a key in ${STATE}, must be exactly once (YAML last-key-wins makes every earlier duplicate silently dead)"
fi

DECLARED_COUNT="$(grep -m1 '^  queued_actions_count:' "${STATE}" \
    | sed -e 's/^  queued_actions_count:[ \t]*//' -e 's/[ \t#].*$//' || true)"
QUEUE_COUNT="$(grep -c '^  - name:' "${ACTIONS}" || true)"
if [[ "${QUEUE_COUNT}" -ne "${#ACTION_NAMES[@]}" ]]; then
    add_error "STATE: ${ACTIONS} has ${QUEUE_COUNT} '^  - name:' lines but the parser found ${#ACTION_NAMES[@]} action(s) — the gate and the file disagree, failing closed"
fi
if [[ -z "${DECLARED_COUNT}" ]]; then
    add_error "STATE: queued_actions_count has no '^  queued_actions_count:' key in ${STATE}"
elif [[ ! "${DECLARED_COUNT}" =~ ^[0-9]+$ ]]; then
    add_error "STATE: queued_actions_count value '${DECLARED_COUNT}' in ${STATE} is not a number"
elif [[ "${DECLARED_COUNT}" -ne "${QUEUE_COUNT}" ]]; then
    add_error "STATE: queued_actions_count is ${DECLARED_COUNT} in ${STATE} but ${ACTIONS} holds ${QUEUE_COUNT} action(s)"
fi

# The effect of a *queued* action must not already be reported true in
# world_state. The opposite reading — require every queued effect to be declared
# as a key — was tried and measured out: on main 83e9a6c it flagged 8 of the 10
# effect keys of the 8 queued actions, because a pending effect is by definition
# not held yet, and pre-declaring each as `false` only relocates the drift to
# "flag says false, reality says true".
declare -A STATE_KEY_VALUE=()
while IFS='|' read -r key value; do
    [[ -n "${key}" ]] || continue
    STATE_KEY_VALUE["${key}"]="${value}"
done < <(grep -E '^  [A-Za-z0-9_]+:' "${STATE}" \
    | sed -E 's/^  ([A-Za-z0-9_]+):[ \t]*/\1|/; s/\|[ \t]*/|/; s/[ \t]#.*$//')

if [[ "${#EFFECT_OWNER[@]}" -gt 0 ]]; then
    mapfile -t SORTED_EFFECTS < <(printf '%s\n' "${!EFFECT_OWNER[@]}" | LC_ALL=C sort)
    for effect in "${SORTED_EFFECTS[@]}"; do
        if [[ "${STATE_KEY_VALUE[${effect}]:-}" == "true" ]]; then
            add_error "STATE: action '${EFFECT_OWNER[${effect}]}' is still queued but ${STATE} already reports its effect '${effect}: true' — the effect is held, so the action is done: delete it from ${ACTIONS}"
        fi
    done
fi

# The one completed action this file *can* name is the one it names: an
# `action_last_completed` still present in the queue means the removal step was
# skipped, which is exactly how #824/#827/#831/#832 drifted.
LAST_COMPLETED_VALUE="$(grep -m1 '^  action_last_completed:' "${STATE}" \
    | sed -e 's/^  action_last_completed:[ \t]*//' -e 's/[ \t#].*$//' || true)"
if [[ -n "${LAST_COMPLETED_VALUE}" ]]; then
    for action in "${ACTION_NAMES[@]}"; do
        if [[ "${action}" == "${LAST_COMPLETED_VALUE}" ]]; then
            add_error "STATE: ${STATE} records action_last_completed: ${LAST_COMPLETED_VALUE} but that action is still queued in ${ACTIONS} — a completed action is removed, not left claiming the defect is live"
            break
        fi
    done
fi

# --- STALE / UNQUEUED (tracker-dependent) ----------------------------------
TRACKER_OK=1
TRACKER_REASON=""
ISSUE_JSON=""
if ! command -v gh >/dev/null 2>&1; then
    TRACKER_OK=0
    TRACKER_REASON="gh CLI not on PATH"
else
    # stderr is folded into the captured output so a token that cannot read
    # issues says why instead of vanishing behind /dev/null. There is deliberately
    # no `gh auth status` probe in front of this call: it is a second, independent
    # way to conclude "tracker unreachable", and it can fail on a runner whose
    # token would have answered the real query perfectly well — which is how a
    # gate ends up printing SKIP forever.
    if ! ISSUE_JSON="$(gh issue list --state all --limit 500 --json number,state,title 2>&1)"; then
        TRACKER_OK=0
        TRACKER_REASON="gh issue list failed: ${ISSUE_JSON%%$'\n'*}"
        ISSUE_JSON=""
    elif ! printf '%s\n' "${ISSUE_JSON}" | jq -e 'type == "array"' >/dev/null 2>&1; then
        # A rc=0 that is not a JSON array (proxy error page, truncated body) must
        # not become "zero issues" — that reads as an empty backlog and passes.
        TRACKER_OK=0
        TRACKER_REASON="gh issue list returned rc=0 but not a JSON array"
        ISSUE_JSON=""
    fi
fi

if [[ "${TRACKER_OK}" -eq 0 ]]; then
    echo "SKIP — NOT A PASS: STALE and UNQUEUED did not run (${TRACKER_REASON})."
    echo "       UNDECLARED and STATE were still checked; the tracker half is unverified."
    echo "       In CI this needs a job with 'permissions: issues: read' and"
    echo "       GITHUB_TOKEN in the step env; CSM_GOAP_QUEUE_REQUIRED=true makes it fatal."
    if [[ "${CSM_GOAP_QUEUE_REQUIRED:-}" == "true" ]]; then
        add_error "STALE/UNQUEUED unverifiable while CSM_GOAP_QUEUE_REQUIRED=true — the tracker must be reachable in CI"
    fi
else
    declare -A ISSUE_STATE=()
    while IFS=$'\t' read -r number state; do
        ISSUE_STATE["${number}"]="${state}"
    done < <(printf '%s\n' "${ISSUE_JSON}" | jq -r 'sort_by(.number)[] | "\(.number)\t\(.state)"')

    if [[ "${#QUEUED_BY_ISSUE[@]}" -gt 0 ]]; then
        mapfile -t SORTED_QUEUED < <(printf '%s\n' "${!QUEUED_BY_ISSUE[@]}" | LC_ALL=C sort -n)
        for issue in "${SORTED_QUEUED[@]}"; do
            action="${QUEUED_BY_ISSUE[${issue}]}"
            state="${ISSUE_STATE[${issue}]:-}"
            if [[ -z "${state}" ]]; then
                add_error "STALE: action '${action}' references #${issue} but the single tracker call returned no such issue — the payload was truncated (--limit 500) or the number is wrong; failing closed instead of assuming OPEN"
            elif [[ "${state}" == "CLOSED" ]]; then
                add_error "STALE: action '${action}' is queued with github_issue: #${issue}, which GitHub reports as CLOSED — the work landed, delete this action from ${ACTIONS}"
            fi
        done
    fi

    while IFS= read -r issue; do
        if [[ -z "${issue}" ]]; then
            continue
        fi
        if [[ -z "${QUEUED_BY_ISSUE[${issue}]:-}" ]]; then
            add_error "UNQUEUED: issue #${issue} is OPEN and titled 'GOAP: ...' but no action in ${ACTIONS} carries github_issue: #${issue} — queue it or close the issue"
        fi
    done < <(printf '%s\n' "${ISSUE_JSON}" | jq -r \
        'sort_by(.number)[] | select(.state == "OPEN" and (.title | startswith("GOAP:"))) | .number')
fi

# --- report ----------------------------------------------------------------
if [[ "${#ERRORS[@]}" -gt 0 ]]; then
    printf 'Error: %d GOAP queue/tracker reconciliation problem(s):\n' "${#ERRORS[@]}" >&2
    for err in "${ERRORS[@]}"; do
        printf '  %s\n' "${err}" >&2
    done
    printf '\nFix: STALE -> delete the action from plans/ACTIONS.md, move its summary to the\n' >&2
    printf '     header as a dated "Last completed" block, and set action_last_completed in\n' >&2
    printf '     plans/GOAP_STATE.md exactly once. UNQUEUED -> add the action block with its\n' >&2
    printf '     github_issue: key, or close the issue if the work already landed.\n' >&2
    printf '     UNDECLARED -> the queue is the tracker backlog, not a private notebook.\n' >&2
    printf '     STATE -> re-measure queued_actions_count against the queue length, keep\n' >&2
    printf '     action_last_completed to exactly one key naming a removed action, and drop\n' >&2
    printf '     any action whose advertised effect world_state already reports true.\n' >&2
    exit 1
fi

if [[ "${TRACKER_OK}" -eq 1 ]]; then
    echo "ok: GOAP queue reconciled — ${#ACTION_NAMES[@]} action(s), ${#QUEUED_BY_ISSUE[@]} tracked issue(s), ${#EFFECT_OWNER[@]} effect key(s), one gh call"
else
    echo "ok: GOAP queue locally consistent — ${#ACTION_NAMES[@]} action(s), ${#EFFECT_OWNER[@]} effect key(s) (tracker checks SKIPPED, not passed)"
fi
