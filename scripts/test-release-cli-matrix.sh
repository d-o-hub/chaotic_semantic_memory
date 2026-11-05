#!/usr/bin/env bash
# scripts/test-release-cli-matrix.sh — issue #1011.
#
# release.yml's "Compute the CLI release matrix" step writes a multi-line JSON array into
# $GITHUB_OUTPUT. The runner parses that file as one `key=value` per line, so a bare
# `echo "json=$(…)"` produced lines 2-6 with no `=` and failed `validate` with
# "##[error]Invalid format". Neither bash -n, shellcheck, nor running the script can see
# that: only the runner validates the output-file format. This fixture executes the real
# step body against a temp $GITHUB_OUTPUT and applies the runner's own parsing rule, so
# the failure mode is reproducible off-GitHub.
#
# Usage: test-release-cli-matrix.sh [workflow-yaml]     (default: .github/workflows/release.yml)
# A workflow path may be passed so a mutated copy can be shown to fail.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="${1:-${REPO_ROOT}/.github/workflows/release.yml}"
SCRIPT="${REPO_ROOT}/scripts/release-cli-matrix.sh"
STEP_NAME="Compute the CLI release matrix"
EXPECTED_ENTRIES=5

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
[[ -f "$WORKFLOW" ]] || fail "no workflow at ${WORKFLOW}"
[[ -x "$SCRIPT" || -f "$SCRIPT" ]] || fail "missing ${SCRIPT}"

command -v python3 >/dev/null 2>&1 || fail "python3 is required for the output-file parser"

# ── 1. locate the step's run body ───────────────────────────────────────────
python3 - "$WORKFLOW" "$STEP_NAME" > "${TMPDIR:-/tmp}/csm-cli-matrix-body.sh" <<'PY'
import pathlib, sys

path, step_name = pathlib.Path(sys.argv[1]), sys.argv[2]
lines = path.read_text().splitlines()
start = next((i for i, line in enumerate(lines) if line.strip() == f"- name: {step_name}"), None)
if start is None:
    sys.exit(f"step {step_name!r} not found")

run_at = None
for i in range(start + 1, len(lines)):
    stripped = lines[i].strip()
    if stripped.startswith("- name:"):
        break
    if stripped.startswith("run:"):
        run_at = i
        break
if run_at is None:
    sys.exit(f"step {step_name!r} has no run:")

head = lines[run_at].split("run:", 1)[1].strip()
if head == "|":
    indent = len(lines[run_at]) - len(lines[run_at].lstrip())
    body = []
    for line in lines[run_at + 1:]:
        if line.strip() == "":
            body.append("")
            continue
        if (len(line) - len(line.lstrip())) <= indent:
            break
        body.append(line[indent + 2:])
    print("\n".join(body))
else:
    print(head)
PY
BODY_FILE="${TMPDIR:-/tmp}/csm-cli-matrix-body.sh"
[[ -s "$BODY_FILE" ]] || fail "could not extract the run body from ${WORKFLOW}"

# ── 2. run the step body exactly as the runner would ────────────────────────
OUT="$(mktemp)"
trap 'rm -f "$OUT" "$OUT.bad"' EXIT
GITHUB_OUTPUT="$OUT" bash -e "$BODY_FILE" || fail "step body exited non-zero"
[[ -s "$OUT" ]] || fail "step body wrote nothing to \$GITHUB_OUTPUT"

# ── 3. apply the runner's own OutputFile rules, then verify the value ───────
python3 - "$OUT" "$EXPECTED_ENTRIES" <<'PY'
import json, pathlib, re, sys

path, expected = pathlib.Path(sys.argv[1]), int(sys.argv[2])
lines = path.read_text().splitlines()
outputs, i, failures = {}, 0, []

while i < len(lines):
    line = lines[i]
    heredoc = re.fullmatch(r"([^<=]+)<<([A-Za-z0-9_]+)", line)
    if heredoc:
        key, delim = heredoc.group(1).strip(), heredoc.group(2)
        block = []
        i += 1
        while i < len(lines) and lines[i].strip() != delim:
            block.append(lines[i])
            i += 1
        if i >= len(lines):
            failures.append(f"{key}: unterminated heredoc delimiter {delim!r}")
            break
        outputs[key] = "\n".join(block)
        i += 1
        continue
    if "=" not in line:
        failures.append(f"line {i + 1}: no '=' and not a heredoc start -> {line[:72]!r}")
        i += 1
        continue
    key, _, value = line.partition("=")
    outputs[key.strip()] = value
    i += 1

if failures:
    print("Invalid format " + "; ".join(failures))
    sys.exit(1)

if "json" not in outputs:
    print("no 'json' output key")
    sys.exit(1)

matrix = json.loads(outputs["json"])
if not isinstance(matrix, list):
    print("json output is not an array")
    sys.exit(1)
if len(matrix) != expected:
    print(f"matrix has {len(matrix)} entries, expected {expected}")
    sys.exit(1)
for entry in matrix:
    missing = [k for k in ("runner", "target", "platform", "arch") if k not in entry]
    if missing:
        print(f"entry {entry} missing {missing}")
        sys.exit(1)

covered = {(e["platform"], e["arch"]) for e in matrix}
for want in (("linux", "x64"), ("darwin", "arm64"), ("win32", "x64")):
    if want not in covered:
        print(f"matrix lost target {want}")
        sys.exit(1)
print(f"parsed {len(matrix)} matrix entries from a {len(lines)}-line output file")
PY

# ── 4. the negative control: the pre-#1011 form must be rejected ───────────
BAD="$(mktemp)"
echo "json=$(bash "$SCRIPT" --json)" > "$BAD"
if python3 - "$BAD" <<'PY'
import pathlib, re, sys
lines = pathlib.Path(sys.argv[1]).read_text().splitlines()
bad = [n for n, l in enumerate(lines, 1) if "=" not in l and not re.fullmatch(r"[^<=]+<<[A-Za-z0-9_]+", l)]
sys.exit(1 if bad else 0)
PY
then
    fail "negative control: the bare echo form was not rejected (fixture is vacuous)"
fi
printf '  ok negative control — bare multi-line echo rejected as the runner rejects it\n'

printf 'release CLI matrix output fixture: passed\n'
