#!/usr/bin/env bash
# Fixture for scripts/check-squash-subject-length.sh (issue #965).
#
# What this pins: GitHub builds a squash-merged subject as `<PR title> (#<N>)`,
# and nothing before the merge measured that string. PR #947's title was 97 chars
# (legal) and its subject 104, which failed `header-max-length: 100` on `main`
# after the merge — and `main` is protected, so the red only appears once the
# damage is done. The directions that matter:
#   the boundary (a title exactly at the limit, and one digit narrower),
#   the *unit* (characters, the way commitlint counts, not bytes — and not whatever
#   the ambient locale makes `${#}` mean),
#   a failed measurement exits 2 instead of answering,
#   and what ci.yml *does* with each exit status, replayed from the live step bodies.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SUBJECT="${SCRIPT_DIR}/check-squash-subject-length.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

RUNS=0
FAILED=0
ok() { RUNS=$((RUNS + 1)); echo "  ok: $1"; }
no() { RUNS=$((RUNS + 1)); FAILED=$((FAILED + 1)); echo "  FAIL: $1"; }

check() { # check <expected-status> <case-name> <pr> <title...>
    local expected="$1" name="$2" pr="$3"
    shift 3
    local status=0
    bash "${SUBJECT}" "${pr}" "$*" >"${WORK}/${name}.log" 2>&1 || status=$?
    if [[ ${status} -ne ${expected} ]]; then
        no "${name}: exited ${status}, expected ${expected}"
        sed 's/^/    /' "${WORK}/${name}.log"
        return
    fi
    ok "${name}: exit ${status}"
}

has() { # has <description> <needle> <case-name>
    if grep -qF -- "$2" "${WORK}/$3.log"; then
        ok "$1"
    else
        no "$1"
        sed 's/^/    /' "${WORK}/$3.log"
    fi
}

# The recorded offender, verbatim from git log on main.
TITLE_947='feat(core): audit wave — durable-first mutations, versioned export, Streamable HTTP, bounded OTLP'
# What main actually received: the title plus the suffix GitHub appends. 97 chars
# is the title, 104 is the subject, and only the second one breaks the rule.
SUBJECT_947="${TITLE_947} (#947)"
# The fixture quotes 104 in several assertions, so the number it asserts against is
# measured here rather than carried over from the issue text. Counted independently of
# the script under test (python3 directly), so a bug shared between the two cannot
# make both agree.
MEASURED_104="$(printf '%s' "${SUBJECT_947}" | python3 -c 'import sys; print(len(sys.stdin.buffer.read().decode("utf-8")))')"
if [[ "${MEASURED_104}" == "104" ]]; then
    ok "the recorded offender really is 104 characters ($(printf '%s' "${SUBJECT_947}" | wc -c) bytes)"
else
    no "the recorded offender measures ${MEASURED_104} characters, not the 104 every assertion expects"
fi

echo "== the measured case: a legal title that produces an illegal subject =="
check 1 over947 947 "${TITLE_947}"
has "the failure prints the effective subject length" "squash subject: 104 chars" over947
has "the failure prints the title and suffix separately" "title 97 + suffix 7" over947
has "the failure names the rule it enforces" "header-max-length" over947
has "the failure prints the retitle budget" "at most 93 chars" over947

echo "== characters, not bytes: the em dash is 3 bytes and 1 character =="
# Titles are generated to an exact length rather than hand-typed and then counted:
# the bug this script exists to catch is a claim about a length nobody measured.
pad() { printf '%*s' "$1" '' | tr ' ' "$2"; }
mk_title() { # mk_title <total-chars> -> ASCII title of exactly that many characters
    local n="$1" prefix='feat(core): '
    printf '%s%s' "${prefix}" "$(pad $(( n - ${#prefix} )) a)"
}
# 93 characters that are 97 bytes: two em dashes. A byte-counting gate reads 100
# for the subject here and would then reject a 93-char ASCII title it should pass
# -- so both the pass and the reported unit are asserted.
TITLE_93_MB="feat(ci): —$(pad 81 a)—"
if [[ ${#TITLE_93_MB} -ne 93 ]]; then
    no "fixture multibyte title is ${#TITLE_93_MB} chars, expected 93"
else
    ok "fixture multibyte title is 93 chars ($(printf '%s' "${TITLE_93_MB}" | wc -c) bytes)"
fi
check 0 multibyte_ok 947 "${TITLE_93_MB}"
has "a multibyte title is measured in characters" "squash subject: 100 chars" multibyte_ok

echo "== the boundary is the suffix, which depends on the digit count =="
TITLE_93="$(mk_title 93)"
if [[ ${#TITLE_93} -ne 93 ]]; then
    no "fixture boundary title is ${#TITLE_93} chars, expected 93"
fi
# 93 chars + " (#999)" = 100 -> exactly at the limit, must pass.
check 0 boundary_three_digit 999 "${TITLE_93}"
has "the three-digit boundary is 100 chars" "squash subject: 100 chars" boundary_three_digit

# 93 chars + " (#9999)" = 101 -> the same title is over once the number gains a digit.
check 1 boundary_four_digit 9999 "${TITLE_93}"
has "the four-digit suffix costs one more char" "suffix 8" boundary_four_digit

# 92 chars + " (#9999)" = 100 -> passes: the budget is computed, not hardcoded.
check 0 fits_four_digit 9999 "$(mk_title 92)"

echo "== a bad invocation is not a verdict =="
status=0
bash "${SUBJECT}" 947 >"${WORK}/one_arg.log" 2>&1 || status=$?
if [[ ${status} -eq 2 ]]; then
    ok "missing title: exit 2 (usage)"
else
    no "missing title: exited ${status}, expected 2"
fi
status=0
bash "${SUBJECT}" "abc" "feat(ci): some title" >"${WORK}/badnum.log" 2>&1 || status=$?
if [[ ${status} -eq 2 ]] && grep -qF "PR number must be digits" "${WORK}/badnum.log"; then
    ok "non-numeric PR number: exit 2 naming the argument"
else
    no "non-numeric PR number: exit 2 naming the argument"
fi
status=0
bash "${SUBJECT}" "947" "" >"${WORK}/empty.log" 2>&1 || status=$?
if [[ ${status} -eq 2 ]]; then
    ok "empty title: exit 2, not a silent pass"
else
    no "empty title: exited ${status}, expected 2"
fi

echo "== the budget is the configurable part, the rule is the same =="
status=0
SQUASH_SUBJECT_MAX=100 bash "${SUBJECT}" 947 "feat(ci): short" >"${WORK}/cfg.log" 2>&1 || status=$?
if [[ ${status} -eq 0 ]] && grep -qF "budget 100" "${WORK}/cfg.log"; then
    ok "explicit SQUASH_SUBJECT_MAX=100 is honoured"
else
    no "explicit SQUASH_SUBJECT_MAX=100 is honoured"
fi
status=0
SQUASH_SUBJECT_MAX=20 bash "${SUBJECT}" 947 "feat(ci): short" >"${WORK}/cfg2.log" 2>&1 || status=$?
if [[ ${status} -eq 1 ]]; then
    ok "SQUASH_SUBJECT_MAX=20 refuses the same title"
else
    no "SQUASH_SUBJECT_MAX=20 refuses the same title (got ${status})"
fi

echo "== the unit is locale-independent, because the runner's is not knowable =="
# `${#}` counts bytes under LC_ALL=C: measured on a 34-char sample that reads 36.
# A byte-counting gate over-rejects every title carrying an em dash -- and this
# repository's titles are full of them -- so the pass/fail boundary must not move
# with the ambient locale. The two runs below must agree.
LC_ALL=C bash "${SUBJECT}" 947 "${TITLE_947}" >"${WORK}/cl_c.log" 2>&1 || true
LC_ALL=C.UTF-8 bash "${SUBJECT}" 947 "${TITLE_947}" >"${WORK}/cl_utf8.log" 2>&1 || true
if cmp -s "${WORK}/cl_c.log" "${WORK}/cl_utf8.log" && grep -qF "104 chars" "${WORK}/cl_c.log"; then
    ok "LC_ALL=C and C.UTF-8 report the same 104-char verdict"
else
    no "the reported length moved with the locale"
    diff "${WORK}/cl_c.log" "${WORK}/cl_utf8.log" || true
fi
# The 93-chars / 97-bytes multibyte title must still pass in C: a byte gate reads 101
# and refuses a title commitlint accepts.
status=0
LC_ALL=C bash "${SUBJECT}" 947 "${TITLE_93_MB}" >"${WORK}/cl_mb.log" 2>&1 || status=$?
if [[ ${status} -eq 0 ]] && grep -qF "squash subject: 100 chars" "${WORK}/cl_mb.log"; then
    ok "a multibyte title passes under LC_ALL=C too"
else
    no "multibyte title under LC_ALL=C exited ${status}"
    cat "${WORK}/cl_mb.log"
fi

echo "== an unusable length probe is not a pass =="
# The script measures through python3. With no python3 on PATH it must refuse to
# answer rather than report a fit, so that a stripped runner reads as red instead of
# green. Every external command the script uses is a bash builtin except python3, so
# a PATH holding only `bash` isolates the missing probe. `env PATH=…` rather than
# `PATH=… bash`: bash resolves the command name of a prefixed assignment through the
# *modified* PATH, so the plain form fails with 127 on the harness, not the subject.
NOPROBE="${WORK}/noprobin/bin"
mkdir -p "${NOPROBE}"
ln -sf "$(command -v bash)" "${NOPROBE}/bash"
status=0
env PATH="${NOPROBE}" bash "${SUBJECT}" 947 "feat(ci): short" >"${WORK}/nopy.log" 2>&1 || status=$?
if [[ ${status} -eq 2 ]] && ! grep -qF "squash subject:" "${WORK}/nopy.log"; then
    ok "no python3: exit 2, and no measurement is printed"
else
    no "no python3: exited ${status}, expected 2 with no verdict line"
    cat "${WORK}/nopy.log"
fi

# ------------------------------------------------------------------- CI wiring
# The script is only half the gate. ci.yml decides what a refusal *means*: fail the
# PR, warn for a bot, or fall through. That decision was written here first and got
# it wrong: `if bash check…; then exit 0; fi` followed by `status=$?` reads 0,
# because a bash `if` whose condition failed and whose branches were all skipped has
# exit status 0 (measured). That turns an over-length owner title into a green check
# -- the same fail-open this issue is about, and invisible to every test above. So
# replay the live step bodies, extracted with PyYAML and never transcribed: a
# fixture that tested a copy of the step would prove the copy.
echo "== ci.yml replays the pre-merge subject check =="
CI_YAML="${REPO_ROOT}/.github/workflows/ci.yml"
if ! python3 -c 'import yaml' 2>/dev/null; then
    if [[ "${CSM_VALIDATION_REQUIRED:-false}" == "true" ]]; then
        echo "ERROR: PyYAML is required to replay the ci.yml steps" >&2
        exit 2
    fi
    echo "SKIPPED — NOT A PASS: no PyYAML, so the ci.yml step bodies went un-replayed"
    RUNS=$((RUNS + 1))
elif [[ ! -r "${CI_YAML}" ]]; then
    no "cannot read ${CI_YAML}"
else
    extract_step() { # extract_step <step name> <dest>
        python3 - "${CI_YAML}" "$1" "$2" <<'PY'
import sys
import yaml

path, name, dest = sys.argv[1], sys.argv[2], sys.argv[3]
with open(path, encoding="utf-8") as fh:
    wf = yaml.safe_load(fh)
for job in wf["jobs"].values():
    for step in job.get("steps") or []:
        if step.get("name") == name and "run" in step:
            with open(dest, "w", encoding="utf-8") as out:
                out.write(step["run"].rstrip() + "\n")
            sys.exit(0)
sys.exit(3)
PY
    }

    SUBJECT_STEP="Check the effective squash subject length"
    if ! extract_step "${SUBJECT_STEP}" "${WORK}/subject_step.sh"; then
        no "ci.yml carries a step named '${SUBJECT_STEP}'"
    else
        ok "ci.yml carries a step named '${SUBJECT_STEP}'"
    fi
    # GitHub's default run shell is `bash -e {0}`, so replay with -e: the fail-open
    # above only bites under that flag combination.
    replay_subject_step() { # <body> <pr> <title> <author> <log> -> echo status
        local body="$1" status=0
        ( cd "${REPO_ROOT}" && env PR_NUMBER="$2" PR_TITLE="$3" PR_AUTHOR="$4" \
            bash -e "${body}" ) >"$5" 2>&1 || status=$?
        echo "${status}"
    }

    got="$(replay_subject_step "${WORK}/subject_step.sh" 947 "${TITLE_947}" "owner" "${WORK}/ci_owner.log")"
    if [[ "${got}" == "1" ]]; then
        ok "an over-length owner title fails the check"
    else
        no "an over-length owner title exited ${got}, expected 1 (a silent pass is the #947 bug)"
        cat "${WORK}/ci_owner.log"
    fi
    if grep -qF "104 chars" "${WORK}/ci_owner.log"; then
        ok "the failing step prints the measured subject length"
    else
        no "the failing step prints the measured subject length"
        cat "${WORK}/ci_owner.log"
    fi

    got="$(replay_subject_step "${WORK}/subject_step.sh" 947 "feat(ci): short title" "owner" "${WORK}/ci_ok.log")"
    if [[ "${got}" == "0" ]]; then
        ok "a short owner title passes"
    else
        no "a short owner title exited ${got}, expected 0"
        cat "${WORK}/ci_ok.log"
    fi

    got="$(replay_subject_step "${WORK}/subject_step.sh" 947 "${TITLE_947}" "dependabot[bot]" "${WORK}/ci_bot.log")"
    if [[ "${got}" == "0" ]] && grep -qF "::warning title=Squash subject::" "${WORK}/ci_bot.log"; then
        ok "an over-length bot title warns instead of blocking the bump"
    else
        no "an over-length bot title exited ${got} with a warning, expected 0 + ::warning::"
        cat "${WORK}/ci_bot.log"
    fi

    # A broken invocation must stay red even on a bot PR: only a measured over-length
    # (exit 1) is a downgradeable verdict.
    got="$(replay_subject_step "${WORK}/subject_step.sh" "abc" "${TITLE_947}" "dependabot[bot]" "${WORK}/ci_badarg.log")"
    if [[ "${got}" != "0" ]] && ! grep -qF "::warning title=Squash subject::" "${WORK}/ci_badarg.log"; then
        ok "exit 2 propagates on a bot PR instead of warning around a failed measurement"
    else
        no "a failed measurement was downgraded to a warning (got ${got})"
        cat "${WORK}/ci_badarg.log"
    fi

    # Sharpness control: un-fix the branch that reads the status and require the bug
    # to come back. Without this, the replay could pass on a step that had silently
    # regressed to always-green.
    sed 's/if \[\[ ${status} -eq 0 \]\]; then/if [[ ${status} -ge 0 ]]; then/' \
        "${WORK}/subject_step.sh" >"${WORK}/subject_step_broken.sh"
    if cmp -s "${WORK}/subject_step.sh" "${WORK}/subject_step_broken.sh"; then
        no "the sharpness edit applied to nothing (the step body changed shape)"
    else
        got="$(replay_subject_step "${WORK}/subject_step_broken.sh" 947 "${TITLE_947}" "owner" "${WORK}/ci_broken.log")"
        if [[ "${got}" == "0" ]]; then
            ok "the un-fixed step reproduces the fail-open (exit 0 on a 104-char subject)"
        else
            no "the un-fixed step did not reproduce the fail-open (got ${got})"
        fi
    fi

    echo "== ci.yml replays the red-main diagnostic =="
    DIAG_STEP="Name the failing subject on a red main lint"
    if ! extract_step "${DIAG_STEP}" "${WORK}/diag_step.sh"; then
        no "ci.yml carries a step named '${DIAG_STEP}'"
    else
        ok "ci.yml carries a step named '${DIAG_STEP}'"
        # A throwaway repo, so the range is real git input without touching this one.
        # hooksPath/gpgsign are pinned off: a global core.hooksPath or commit.gpgsign
        # would make the fixture's own commits the thing that fails.
        GITREPO="${WORK}/gitrepo"
        mkdir -p "${GITREPO}" "${WORK}/no-hooks"
        git -C "${GITREPO}" init -q
        git -C "${GITREPO}" config user.email ci@example.invalid
        git -C "${GITREPO}" config user.name "Fixture"
        git -C "${GITREPO}" config commit.gpgsign false
        git -C "${GITREPO}" config core.hooksPath "${WORK}/no-hooks"
        commit_subject() { # commit_subject <subject>
            echo x >>"${GITREPO}/f"
            git -C "${GITREPO}" add -A
            git -C "${GITREPO}" commit -q -m "$1"
            git -C "${GITREPO}" rev-parse HEAD
        }
        SHA_SEED="$(commit_subject 'chore: seed')"
        SHA_SHORT="$(commit_subject 'feat(ci): a short subject')"
        SHA_LONG="$(commit_subject "${SUBJECT_947}")"

        replay_diag() { # replay_diag <from> <to> <log> -> echo status
            local status=0
            ( cd "${GITREPO}" && env LINT_FROM="$1" LINT_TO="$2" \
                bash -e "${WORK}/diag_step.sh" ) >"$3" 2>&1 || status=$?
            echo "${status}"
        }

        got="$(replay_diag "${SHA_SHORT}" "${SHA_LONG}" "${WORK}/ci_diag.log")"
        if [[ "${got}" == "0" ]] &&
            grep -qF "::error title=header-max-length on main::104 chars" "${WORK}/ci_diag.log"; then
            ok "the diagnostic names the offending subject with its character length"
        else
            no "the diagnostic names the offending subject with its character length (exit ${got})"
            cat "${WORK}/ci_diag.log"
        fi
        if grep -qF "1 of 1 subject(s) over the limit" "${WORK}/ci_diag.log"; then
            ok "the diagnostic counts how much of the range is bad"
        else
            no "the diagnostic counts how much of the range is bad"
            cat "${WORK}/ci_diag.log"
        fi

        # A range whose subjects all fit must say so: commitlint can go red on a
        # type/scope/body rule, and silence here would read as "the length was fine"
        # without having checked anything.
        got="$(replay_diag "${SHA_SEED}" "${SHA_SHORT}" "${WORK}/ci_diag_clean.log")"
        if [[ "${got}" == "0" ]] && ! grep -qF "::error" "${WORK}/ci_diag_clean.log" &&
            grep -qF "none of the 1 subject(s)" "${WORK}/ci_diag_clean.log"; then
            ok "a clean range reports the count it checked and emits no ::error::"
        else
            no "a clean range reports its count and stays quiet (exit ${got})"
            cat "${WORK}/ci_diag_clean.log"
        fi
        got="$(replay_diag "${SHA_LONG}" "${SHA_LONG}" "${WORK}/ci_diag_empty.log")"
        if [[ "${got}" == "0" ]] && grep -qF "none of the 0 subject(s)" "${WORK}/ci_diag_empty.log"; then
            ok "an empty range says it checked nothing instead of staying silent"
        else
            no "an empty range speaks up (exit ${got})"
            cat "${WORK}/ci_diag_empty.log"
        fi
    fi
fi

printf '\nsquash subject fixture: %s case(s), %s failure(s)\n' "${RUNS}" "${FAILED}"
[[ ${FAILED} -eq 0 ]] || exit 1
