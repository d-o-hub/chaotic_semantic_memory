#!/usr/bin/env bash
# Audit the one place this repository leaks nix's library path into host binaries.
#
# WHY THIS EXISTS (issue #949, measured 2026-11-01): scripts/run-with-libs.sh is the
# only exporter of LD_LIBRARY_PATH in the tree — it overwrites the variable with
# CSM_LD_LIBRARY_PATH (pkgs.stdenv.cc.cc.lib + openssl, flake.nix:90) and execs the
# command. Everything that command spawns inherits it, including the *host* git, which
# is what breaks: git-remote-https resolves libdl/libpthread against nix-store glibc
# 2.44 shims and dies on GLIBC_ABI_DT_X86_64_PLT. The devShell itself exports no
# LD_LIBRARY_PATH and ships no `git` (packages list, flake.nix:56-85 — `git-cliff`
# only), so an *unwrapped* `nix develop --command cargo deny` uses host git in a clean
# environment and works on any runner image.
#
# That measurement falsified the original diagnosis. The first fix pinned the `lint`
# and `Cargo Deny` jobs to ubuntu-24.04 on an Ubuntu-26 runner-image theory (586d1712);
# the pinned runner failed the identical line, and the unpinned Cargo Deny job — same
# label, same devShell, no wrapper — passed every time as the control. Both pins were
# reverted with that reasoning recorded in the ci.yml job comments, so runner labels are
# NOT the subject of this gate and must not be re-pinned on that theory.
#
# The leak class is one line wide: a host helper spawned under the wrapper. Measured
# today there is exactly one such site and it is scrubbed —
# `env -u LD_LIBRARY_PATH cargo deny --locked check` in scripts/validate.sh, because
# cargo-deny clones the RustSec advisory-db through the host git CLI. Local git
# plumbing (`git diff`, `git rev-parse` in scripts/mutation_test.sh, also wrapped) never
# execs a remote helper and passes untouched, which is why HOST_RE names network
# subcommands rather than git.
#
# What fails:
#   1. a host helper (network git, curl, wget, ssh, rsync) on a line of any
#      `nix develop` step body — the literal form of the issue's acceptance criterion;
#   2. such a helper, or `cargo deny`, in a *wrapped* step body or in a script a wrapped
#      step execs, unless that same line drops LD_LIBRARY_PATH. This is the shape that
#      would have caught the original bug before it shipped rather than two runner-pin
#      round-trips after it. A script a wrapped step execs that this tree cannot read is
#      a finding too: the execed scripts are where the leak actually lives, so passing on
#      an unreadable one is the gate reporting clean because it looked at nothing;
#   3. a nix-using job missing from the audited inventory below, or an inventory entry
#      that no longer invokes nix: adding one means re-auditing it in the same commit,
#      which is the durable form of the per-job inventory this issue asked for;
#   4. a devShell that sets LD_LIBRARY_PATH itself, which would move every unwrapped nix
#      step into the class and make the Cargo Deny control stop being one.
#
# Usage:   check-nix-git-env.sh [--print-inventory] [repo-root]
# Exit:    0 audited clean, 1 a leak shape is present, 2 the audit could not run.
set -uo pipefail

PRINT_INVENTORY="false"
if [[ "${1:-}" == "--print-inventory" ]]; then
  PRINT_INVENTORY="true"
  shift
fi
REPO="${1:-.}"
WORKFLOWS="${REPO}/.github/workflows"

# Every job whose steps invoke `nix develop`, as audited 2026-11-04. Keys are
# "<workflow file>::<job id>". Update by re-running the audit, not by wish.
INVENTORY=(
  "benchmark-ci.yml::benchmark-small"
  "ci.yml::cargo-deny"
  "ci.yml::lint"
  "ci.yml::mutation-shard"
  "ci.yml::test"
  "ci.yml::wasm"
)

# HOST_RE — helpers that resolve glibc against whatever LD_LIBRARY_PATH says. The flake
# ships none of them, so under `nix develop` they are the runner image's copies.
HOST_RE='(^|[;&|([:space:]])(git[[:space:]]+(clone|fetch|pull|push|ls-remote|submodule|archive)|curl|wget|ssh|rsync)([[:space:]]|$)'
# WRAP_RE — HOST_RE plus the tool that spawns a host helper on the caller's behalf.
WRAP_RE='(^|[;&|([:space:]])(git[[:space:]]+(clone|fetch|pull|push|ls-remote|submodule|archive)|curl|wget|ssh|rsync|cargo[[:space:]]+deny)([[:space:]]|$)'

# Deliberately absent from both: `npm install`, `cargo install`, `rustup install`,
# `cargo audit`. Under the wrapper those are nix-provided binaries doing their own HTTPS
# in-process — no host helper, no mixed glibc. Measured: the first draft flagged
# validate.sh's `npm install`, mutation_test.sh's `cargo install cargo-mutants`, and a
# banner `echo "==> cargo deny --locked check"` — three findings against code that is not
# in the class. A gate that cries about prose gets muted, and the mute outlives the
# finding.
#
# Quoted spans are stripped rather than excusing a whole echo line: skipping anything
# that *starts* with echo/printf would also hide `echo hi && git clone …`, and a banner
# is only prose for the part inside the quotes.
command_text() { printf '%s' "$1" | sed 's/"[^"]*"/ /g; s/'"'"'[^'"'"']*'"'"'/ /g'; }

# A pattern that cannot match anything still reports success: glibc grep treats an
# unmatched `)` as a literal instead of a syntax error, so the first version of this
# gate — one stray paren — printed "no unscrubbed host helper" for every input and passed
# the real tree. Each pattern is probed against its own reference sample before it is
# trusted: a zero from a probe that also returns zero on a known positive is the probe
# failing, not the code being clean.
probe() { # probe <name> <pattern> <must-match> <must-not-match>
    if ! printf '%s\n' "$3" | grep -qE "$2"; then
        printf 'error: %s does not match its reference sample; refusing to report clean\n' "$1" >&2
        exit 2
    fi
    if printf '%s\n' "$4" | grep -qE "$2"; then
        printf 'error: %s matches a line with no host helper; refusing to report clean\n' "$1" >&2
        exit 2
    fi
}
probe HOST_RE "${HOST_RE}" 'git clone https://example.invalid/advisory-db' 'cargo mutants --list --caught tests'
probe WRAP_RE "${WRAP_RE}" 'cargo deny --locked check' 'cargo mutants --list --caught tests'

if [[ ! -d "${WORKFLOWS}" ]]; then
  printf 'error: cannot read %s\n' "${WORKFLOWS}" >&2
  exit 2
fi
if ! python3 -c 'import yaml' 2>/dev/null; then
  # Auditing without PyYAML would mean grepping YAML, and grep is how a step hides in a
  # block scalar. Say the audit did not run instead of running a wrong one.
  printf 'error: python3 has no PyYAML (apt: python3-yaml); the workflow audit did not run\n' >&2
  exit 2
fi
FLAKE="${REPO}/flake.nix"
if [[ ! -r "${FLAKE}" ]]; then
  printf 'error: cannot read %s; the devShell env is a precondition of this audit\n' "${FLAKE}" >&2
  exit 2
fi

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT
STEPS="${WORK}/steps.tsv"

# One row per *line* of every nix-develop run step, so a rule judges a command by the
# line it is written on:
#   <file>::<job>  <runs-on>  <wrapped>  <step name>  <line no>  <line, tabs folded>
python3 - "${WORKFLOWS}" >"${STEPS}" <<'PY'
import glob
import os
import re
import sys

import yaml

nix_step = re.compile(r"nix\s+develop")
for path in sorted(glob.glob(os.path.join(sys.argv[1], "*.yml"))):
    with open(path, encoding="utf-8") as fh:
        wf = yaml.safe_load(fh)
    if not isinstance(wf, dict):
        continue
    for job_id, job in (wf.get("jobs") or {}).items():
        if not isinstance(job, dict):
            continue
        for step in job.get("steps") or []:
            if not isinstance(step, dict):
                continue
            run = step.get("run")
            if not isinstance(run, str) or not nix_step.search(run):
                continue
            wrapped = "yes" if "run-with-libs.sh" in run else "no"
            name = str(step.get("name", "(unnamed)")).replace("\t", " ")
            runs_on = str(step.get("runs-on", job.get("runs-on", "?"))).replace("\t", " ")
            key = f"{os.path.basename(path)}::{job_id}"
            for number, line in enumerate(run.splitlines(), start=1):
                print("\t".join([key, runs_on, wrapped, name, str(number), line.replace("\t", " ")]))
PY
parse_rc=$?
if [[ ${parse_rc} -ne 0 || ! -s "${STEPS}" ]]; then
  printf 'error: the workflow audit found no nix develop step (parse rc %s)\n' "${parse_rc}" >&2
  exit 2
fi

failures=0
report() {
  printf 'LEAK: %s\n' "$1" >&2
  failures=$((failures + 1))
}

if [[ "${PRINT_INVENTORY}" == "true" ]]; then
  printf '%-34s %-16s %-9s %s\n' "JOB" "RUNS-ON" "CONTEXT" "NIX-LINES"
  declare -A inv_runs_on=() inv_ctx=() inv_lines=()
  while IFS=$'\t' read -r job runs_on wrapped name lineno line; do
    inv_runs_on["${job}"]="${runs_on}"
    inv_lines["${job}"]=$(( ${inv_lines["${job}"]:-0} + 1 ))
    if [[ "${wrapped}" == "yes" ]]; then inv_ctx["${job}"]="wrapped"; else
      inv_ctx["${job}"]="${inv_ctx["${job}"]:-plain}"
    fi
  done <"${STEPS}"
  for job in "${!inv_runs_on[@]}"; do
    printf '%-34s %-16s %-9s %s\n' "${job}" "${inv_runs_on[${job}]}" "${inv_ctx[${job}]}" "${inv_lines[${job}]}"
  done | sort
fi

# ------------------------------------------- rules 1 and 2: the step bodies
# `env -u LD_LIBRARY_PATH` on the same line as the command is the sanctioned shape.
declare -A seen_jobs=()
declare -A wrapped_scripts=()
while IFS=$'\t' read -r job runs_on wrapped name lineno line; do
  seen_jobs["${job}"]=1
  if [[ "${wrapped}" == "yes" ]]; then
    for script in $(printf '%s' "${line}" | grep -oE '[A-Za-z0-9_./-]+\.sh' | sort -u); do
      wrapped_scripts["${REPO}/${script}"]=1
    done
  fi
  if [[ "${line}" =~ ^[[:space:]]*# ]] || [[ "${line}" == *LD_LIBRARY_PATH* ]]; then
    continue
  fi
  cmd="$(command_text "${line}")"
  if printf '%s\n' "${cmd}" | grep -qE "${HOST_RE}"; then
    report "${job} step '${name}' body line ${lineno}: host helper inside nix develop: ${line}"
  elif [[ "${wrapped}" == "yes" ]] && printf '%s\n' "${cmd}" | grep -qE "${WRAP_RE}"; then
    report "${job} step '${name}' body line ${lineno}: host helper under the wrapper with no LD_LIBRARY_PATH scrub: ${line}"
  fi
done <"${STEPS}"

# ---------------------------------- rule 2 (cont.): inside the wrapped scripts
# A wrapped step that names a script this tree cannot read is a finding, not a skip: the
# step is how the audit discovers the leak sites, so an unreadable one means the audit
# scanned nothing and would otherwise report clean.
for script in "${!wrapped_scripts[@]}"; do
  if [[ ! -r "${script}" ]]; then
    report "${script#"${REPO}/"} is execed under the wrapper but is not readable: the audit cannot verify its environment"
    continue
  fi
  lineno=0
  while IFS= read -r line; do
    lineno=$((lineno + 1))
    [[ "${line}" =~ ^[[:space:]]*# ]] && continue
    [[ "${line}" == *LD_LIBRARY_PATH* ]] && continue
    printf '%s\n' "$(command_text "${line}")" | grep -qE "${WRAP_RE}" || continue
    report "${script#"${REPO}/"}:${lineno} spawns a host helper with no LD_LIBRARY_PATH scrub"
  done <"${script}"
done

# ------------------------------------------------- rule 3: the inventory ratchet
for want in "${INVENTORY[@]}"; do
  if [[ -z "${seen_jobs[${want}]:-}" ]]; then
    report "${want} is in the audited inventory but no longer invokes nix develop — re-audit and drop it"
  fi
done
for got in "${!seen_jobs[@]}"; do
  found="false"
  for want in "${INVENTORY[@]}"; do
    [[ "${got}" == "${want}" ]] && found="true"
  done
  if [[ "${found}" == "false" ]]; then
    report "${got} invokes nix develop but is not in the audited inventory — audit its host helpers, then add it"
  fi
done

# ------------------------------ rule 4: the devShell stays out of the leak class
# The whole asymmetry rests on `nix develop` exporting nothing into LD_LIBRARY_PATH and
# scripts/run-with-libs.sh deciding when to. A devShell that set the variable itself
# would move every unwrapped nix step — the Cargo Deny control included — into the
# class, and the pins this issue reverted would look correct again.
if grep -qE '^[[:space:]]*LD_LIBRARY_PATH[[:space:]]*=' "${FLAKE}"; then
  report "flake.nix sets LD_LIBRARY_PATH in a devShell env: unwrapped nix steps would join the leak class and the Cargo Deny control stops being one"
fi

if [[ ${failures} -gt 0 ]]; then
  printf '\nnix environment audit: %s finding(s). Runner labels are not the fix: the\n' "${failures}" >&2
  printf 'ubuntu-24.04 pin that blamed the Ubuntu 26 migration failed the same line, and\n' >&2
  printf 'the unpinned Cargo Deny job passed it as the control (issue #949).\n' >&2
  exit 1
fi

printf 'nix environment audit: %s nix develop line(s) across %s job(s), %s wrapped entrypoint script(s) scanned, no unscrubbed host helper\n' \
  "$(wc -l <"${STEPS}")" "${#seen_jobs[@]}" "${#wrapped_scripts[@]}"
exit 0
