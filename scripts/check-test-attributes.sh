#!/usr/bin/env bash
# check-test-attributes.sh — fail when an integration-test file holds a
# test-shaped function that can never run.
#
# WHY THIS GATE EXISTS (2026-10-03)
# `cli_each_subcommand_has_help` in `tests/cli_parity.rs` had no `#[test]`
# attribute, so the harness never registered it:
# `cargo test --test cli_parity --features cli -- --list` showed 2 tests while
# the file contained a third test body. Nothing in the toolchain noticed —
# recompiling that target emits **no** `dead_code` warning (rustc's
# reachability analysis in a `--test` build does not flag a private `fn` that
# only the harness would call), so neither `cargo clippy --all-targets` nor the
# `cargo test --no-run` warning scan in `scripts/validate.sh` can be the
# detector. The only reliable sensor is textual: read the source.
#
# THE HEURISTIC
# For each integration-test file, a *top-level* (column 0) `fn` is a violation
# when BOTH hold:
#   1. the contiguous `#[...]` attribute block above it contains nothing that
#      registers or parks it with the harness, and
#   2. its name is not mentioned by any other non-comment line of the same
#      file — i.e. it is neither a test nor a helper something calls.
# Registered/parking attributes: any `#[<path>test...]` (covers `#[test]`,
# `#[test_case(..)]`, `#[test_matrix(..)]`, `#[tokio::test]`,
# `#[test_log::test]`, `#[async_std::test]`) plus `#[ignore]`, `#[rstest]`,
# `#[bench]`, `#[should_panic]`. `pub` functions inside a `tests/<dir>/`
# sub-module (the `tests/common/mod.rs` sharing convention, `mod`-importable by
# other test targets) are exempt because their call sites live in other files.
#
# BLIND SPOTS (stated so nobody trusts this gate more than it deserves)
#   - It is textual, not semantic: a dead body whose name appears in a string,
#     a macro or a comment-adjacent call site counts as "helper in use" and
#     passes, so a test that is merely *mentioned* but never registered slips
#     through.
#   - Only column-0 `fn`s are inspected. Functions inside a nested
#     `mod tests { ... }` block — including the `#[cfg(test)]` modules that hold
#     most `src/**` and `crates/**` unit tests — are NOT checked; catching those
#     would need a real parser rather than a 20 ms scan.
#   - It cannot tell "this should be a test" from "this helper is dead code";
#     both are reported identically. That is intentional: add the attribute or
#     delete the body.
#   - A `#[cfg(...)]`-gated, unreferenced private `fn` at column 0 is reported.
#     None exists in the tree today; if one is added, it needs a call site or a
#     sub-module `pub`.
#
# Cost: one awk process over the test sources. No Cargo, no network, no
# compilation. Measured 0.02 s for the 75 test files on 2026-10-03.
#
# Usage:
#   scripts/check-test-attributes.sh                  # scan tests/ + crates/*/tests/
#   scripts/check-test-attributes.sh FILE [FILE...]   # scan explicit files
# Negative fixture: scripts/test-check-test-attributes.sh

set -euo pipefail

if [[ "$#" -gt 0 ]]; then
    FILES=("$@")
else
    FILES=()
    while IFS= read -r found; do
        FILES+=("${found}")
    done < <(find tests crates/*/tests -name '*.rs' -type f 2>/dev/null | sort)
fi

if [[ "${#FILES[@]}" -eq 0 ]]; then
    echo "ok: no integration-test files found"
    exit 0
fi

for file in "${FILES[@]}"; do
    if [[ ! -f "${file}" ]]; then
        echo "Error: test-attribute gate input missing: ${file}" >&2
        exit 1
    fi
done

# Two phases per file: collect the lines, then walk them with an attribute
# state machine (flushed at each FNR == 1 boundary and in END).
STATUS=0
awk '
function bracket_delta(s,   t, opens, closes) {
    t = s
    opens = gsub(/\[/, "[", t)
    t = s
    closes = gsub(/\]/, "]", t)
    return opens - closes
}

function is_registered(attrs) {
    return (attrs ~ /#\[[ \t]*[A-Za-z0-9_:+.]*test/ \
        || attrs ~ /#\[[ \t]*(ignore|rstest|bench|should_panic)/)
}

function report(lineno, name,   msg) {
    printf("missing test attribute: %s:%d %s\n", curfile, lineno, name)
    printf("  why: private top-level fn with no #[test]-style attribute and no\n")
    printf("       call site in this file, so the harness never registers it\n")
    printf("       and clippy/dead_code stays silent\n")
    bad++
}

function flush_file(   i, ln, attrs, depth, j, t, name, refs, is_pub, rest, submodule) {
    if (n == 0) {
        return
    }
    rest = curfile
    sub(/^.*tests\//, "", rest)
    submodule = (index(rest, "/") > 0)

    i = 1
    attrs = ""
    while (i <= n) {
        ln = lines[i]
        if (ln ~ /^[ \t]*#!/) {              # inner attribute (#![allow(..)])
            attrs = ""
            i++
            continue
        }
        if (ln ~ /^[ \t]*#\[/) {             # outer attribute, possibly multi-line
            depth = bracket_delta(ln)
            attrs = attrs ln " "
            while (depth > 0 && i < n) {
                i++
                depth += bracket_delta(lines[i])
                attrs = attrs lines[i] " "
            }
            i++
            continue
        }
        if (ln ~ /^[ \t]*\/\// || ln ~ /^[ \t]*$/) {   # comment or blank: keep attrs
            i++
            continue
        }
        if (ln ~ /^(pub(\([a-zA-Z0-9_]+\))?[ \t]+)?(async[ \t]+)?(unsafe[ \t]+)?(const[ \t]+)?fn[ \t]+/) {
            if (!is_registered(attrs)) {
                name = ln
                sub(/^([A-Za-z0-9_]+[ \t]+)*fn[ \t]+/, "", name)
                sub(/[^A-Za-z0-9_].*$/, "", name)
                is_pub = (ln ~ /^pub/)
                if (!(is_pub && submodule)) {
                    refs = 0
                    for (j = 1; j <= n; j++) {
                        if (j == i) continue
                        if (lines[j] ~ /^[ \t]*\/\//) continue
                        t = lines[j]
                        gsub(/[^A-Za-z0-9_]/, " ", t)
                        if (index(" " t " ", " " name " ") > 0) refs++
                    }
                    if (refs == 0) {
                        report(i, name)
                    }
                }
            }
            attrs = ""
            i++
            continue
        }
        attrs = ""
        i++
    }
}

FNR == 1 {
    flush_file()
    for (drop = 1; drop <= n; drop++) {
        delete lines[drop]
    }
    n = 0
    curfile = FILENAME
}

{ lines[++n] = $0 }

END {
    flush_file()
    if (bad > 0) {
        printf("test-attribute gate failed: %d unregistered test-shaped function(s)\n", bad)
        exit 1
    }
}
' "${FILES[@]}" || STATUS=$?

if [[ "${STATUS}" -ne 0 ]]; then
    exit "${STATUS}"
fi

echo "ok: ${#FILES[@]} test files scanned, no unregistered test-shaped functions"
