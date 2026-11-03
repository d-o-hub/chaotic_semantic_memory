#!/usr/bin/env bash
# Measure the subject GitHub will write when it squash-merges a PR.
#
# WHY THIS EXISTS (issue #965, measured 2026-10-30): a squash-merged commit's
# subject is not any string that PR CI lints. GitHub builds it as
# `<PR title> (#<number>)`, while the PR-side job lints the branch commits
# (`commitlint --from base --to head`) and the title *without* the suffix
# (ci.yml:160). PR #947's title was 97 chars — legal — and its squash subject
# `feat(core): audit wave — … bounded OTLP (#947)` was 104, which failed the
# `header-max-length: 100` rule on `main` (run 41059071360, check-run
# 124073904392) and made the next `workflow_run` release gate refuse with zero
# jobs. `main` is protected, so the only visible symptom appears after the merge.
#
# Usage:   check-squash-subject-length.sh <pr-number> <pr-title>
# Exit:    0 subject fits, 1 subject is over budget, 2 bad invocation or an
#          unusable length probe (a failed measurement is never a verdict).
set -euo pipefail

MAX_HEADER="${SQUASH_SUBJECT_MAX:-100}"

charlen() { # charlen <string> -> length in characters, the unit commitlint counts
  # `@commitlint/config-conventional` compares a JS string `.length`, so a UTF-8
  # title is counted in code points. Bash `${#}` counts characters only under a
  # UTF-8 locale and BYTES under LC_ALL=C — measured here: `LC_ALL=C` reads the
  # 34-character sample as 36, and #947's subject is 104 characters but 106 bytes
  # (the em dash is 3). A gate on the byte count refuses legal titles, and which
  # one you get would depend on the ambient locale of whoever ran it. Python
  # decodes stdin as UTF-8 whatever LC_* says, so the verdict is the same locally
  # and on a runner.
  printf '%s' "$1" | python3 -c 'import sys; print(len(sys.stdin.buffer.read().decode("utf-8")))'
}

if [[ $# -ne 2 ]]; then
  echo "usage: ${0##*/} <pr-number> <pr-title>" >&2
  exit 2
fi

PR_NUMBER="$1"
PR_TITLE="$2"

if [[ ! "${PR_NUMBER}" =~ ^[0-9]+$ ]]; then
  echo "error: PR number must be digits, got '${PR_NUMBER}'" >&2
  exit 2
fi
if [[ -z "${PR_TITLE}" ]]; then
  echo "error: empty PR title; nothing to measure" >&2
  exit 2
fi

# The suffix GitHub appends, measured in the same unit as the subject: 7 chars for
# a 3-digit number and 8 for a 4-digit one, so a title sized to 100 by itself is
# already over the limit before the merge happens.
SUFFIX=" (#${PR_NUMBER})"
SUBJECT="${PR_TITLE}${SUFFIX}"
if ! TITLE_LEN="$(charlen "${PR_TITLE}")" || ! SUFFIX_LEN="$(charlen "${SUFFIX}")" ||
  ! SUBJECT_LEN="$(charlen "${SUBJECT}")"; then
  echo "error: could not measure the subject; python3 is missing or the title is not valid UTF-8" >&2
  exit 2
fi
BUDGET=$((MAX_HEADER - SUFFIX_LEN))

echo "squash subject: ${SUBJECT_LEN} chars (title ${TITLE_LEN} + suffix ${SUFFIX_LEN}), budget ${MAX_HEADER}, title limit ${BUDGET}"

if [[ ${TITLE_LEN} -gt "${BUDGET}" ]]; then
  echo "error: the effective squash subject would be ${SUBJECT_LEN} chars, over the ${MAX_HEADER}-char header-max-length rule" >&2
  echo "  retitle to at most ${BUDGET} chars, or trim $(( SUBJECT_LEN - MAX_HEADER )) char(s) from the title" >&2
  exit 1
fi
