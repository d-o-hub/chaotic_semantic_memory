#!/usr/bin/env bash
# Fixture for scripts/workspace-version-edges.py (issue #964).
#
# Every case builds a throwaway workspace in ${WORK}, runs the tool against it, and
# asserts a failure *direction*. The tree is synthetic because the real repository is in
# exactly one state and can only ever prove that state; the shapes below are the ones
# scripts/sync-version.sh's root-only sed and verify-version-sync.sh's `crates/` glob
# were blind to, which is how 18 pins drifted while both tools printed a checkmark.
#
# Directions asserted, in order:
#   correct pins                          -> 0
#   member -> member, inline              -> 1     (the 16 edges #964 enumerated)
#   member -> root                        -> 1     (same direction the old sed missed)
#   [dev-dependencies]                    -> 1     (a section the old sed could not reach)
#   [dependencies.x] multi-line table     -> 1     (the form #964's own count missed: fuzz)
#   [target.'cfg(..)'.dependencies]       -> 1     (and cross-checked against tomllib)
#   workspace = true                      -> 0     (no literal to sync is not drift)
#   path edge with no version key         -> 0     (and still counted)
#   a member no `crates/*` pattern names  -> found, and found identically with or
#                                            without a git index (discovery is a walk)
#   a stale manifest under target/        -> 0     (the prune is real)
#   …declared as a member anyway          -> 2     (so the prune cannot hide a member)
#   a symlink back into the tree          -> counted once
#   a symlink out of the tree             -> 2     (followed far enough to refuse it)
#   a directory linking to itself         -> a verdict, not ENAMETOOLONG
#   a manifest reachable by two paths     -> one edge (the shape only the manifest key catches)
#   third-party version, no path          -> untouched by --sync
#   --sync / --dry-run / idempotence      -> rewrite, no rewrite, nothing to do
#   a pin tomllib reads and the parser does not, unbalanced table, no manifest,
#   a bad version argument                -> 2, never a verdict
#   a brand-new member crate              -> audited with no edit to the tool
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/workspace-version-edges.py"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

RUNS=0
FAILED=0
ok() { local desc="$1"; RUNS=$((RUNS + 1)); echo "  ok: ${desc}"; }
no() { local desc="$1"; RUNS=$((RUNS + 1)); FAILED=$((FAILED + 1)); echo "  FAIL: ${desc}"; }

if [[ ! -r "${SUBJECT}" ]]; then
    printf 'ERROR: cannot read %s\n' "${SUBJECT}" >&2
    exit 2
fi
if ! python3 -c 'import tomllib' 2>/dev/null; then
    if [[ "${CSM_VALIDATION_REQUIRED:-false}" == "true" ]]; then
        printf 'ERROR: python3 needs tomllib (3.11+) for this fixture.\n' >&2
        exit 2
    fi
    printf 'SKIPPED — NOT A PASS: no tomllib; the workspace edge fixture did not run\n'
    exit 0
fi

# tool <case> <args…>: run the subject against the case tree, capture rc + output.
tool() {
    local case_name="$1"
    shift
    local status=0
    python3 "${SUBJECT}" "$@" "${WORK}/${case_name}" >"${WORK}/${case_name}.out" 2>&1 || status=$?
    echo "${status}"
}

expect_rc() { # expect_rc <want> <description> <case> <args…>
    local want="$1" desc="$2" case_name="$3"
    shift 3
    local got
    got="$(tool "${case_name}" "$@")"
    if [[ "${got}" == "${want}" ]]; then
        ok "${desc} (exit ${got})"
    else
        no "${desc} — expected exit ${want}, got ${got}"
        sed 's/^/    /' "${WORK}/${case_name}.out"
    fi
}

# has / lacks read the output `tool` captured for a case. Argument order matches
# expect_has in scripts/test-check-nix-git-env.sh: description, needle, case.
has() { # has <description> <needle> <case>
    local desc="$1" needle="$2" case_name="$3"
    if grep -qF -- "${needle}" "${WORK}/${case_name}.out"; then
        ok "${desc}"
    else
        no "${desc} — missing [${needle}]"
        sed 's/^/    /' "${WORK}/${case_name}.out"
    fi
}

lacks() { # lacks <description> <needle> <case>
    local desc="$1" needle="$2" case_name="$3"
    if grep -qF -- "${needle}" "${WORK}/${case_name}.out"; then
        no "${desc} — unexpected [${needle}]"
    else
        ok "${desc}"
    fi
}

# ------------------------------------------------------------------ tree factory
# mk_tree <name> <declared-pin-for-stale-edges>: one root package + two members +
# a fourth manifest that is not a workspace member. (`vendor` would be a more realistic
# name for it, but that directory is pruned by discovery; this one must be *read*.)
# Versions: members are 1.2.3; the pin passed in is what a dependency *declares*.
mk_tree() {
    local root="${WORK}/$1" declared="$2"
    local extra=""
    if [[ ${#} -gt 2 ]]; then
        extra="$3"
    fi
    mkdir -p "${root}/crates/alpha/src" "${root}/crates/beta/src" "${root}/notamember"

    cat >"${root}/Cargo.toml" <<TOML
# Root manifest — comment must survive any rewrite.
[package]
name = "root-pkg"
version = "1.2.3"
edition = "2021"

[workspace]
members = [".", "crates/alpha", "crates/beta"]

[dependencies]
alpha = { path = "crates/alpha", version = "1.2.3" }
beta = { path = "crates/beta", version = "1.2.3" }
serde = { version = "1.0.200", features = ["derive"] }
TOML

    cat >"${root}/crates/alpha/Cargo.toml" <<TOML
[package]
name = "alpha"
version = "1.2.3"
edition = "2021"

[dependencies]
root-pkg = { path = "../..", version = "${declared}" }
beta = { path = "../beta", version = "${declared}" }
tokio = { version = "1.43.0", features = ["rt"] }

[dev-dependencies]
beta = { path = "../beta", version = "${declared}" }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
beta = { path = "../beta", version = "${declared}" }
TOML

    # The multi-line explicit-table form, which is what fuzz/Cargo.toml uses and what
    # issue #964's own single-line enumeration missed.
    cat >"${root}/crates/beta/Cargo.toml" <<TOML
[package]
name = "beta"
version = "1.2.3"
edition = "2021"

[dependencies]
alpha = { path = "../alpha", version = "${declared}" }

[dependencies.root-pkg]
path = "../.."
version = "${declared}"
TOML

    # Not a workspace member: no [package] in a discovered manifest, and a path edge
    # to something the tool has never heard of.
    cat >"${root}/notamember/Cargo.toml" <<TOML
[package]
name = "vendored"
version = "9.9.9"

[dependencies]
whatever = { path = "../nowhere", version = "0.0.1" }
TOML

    if [[ "${extra}" == "git" ]]; then
        # Only to prove discovery does not need it: the walk reads the filesystem, so a
        # tree under git and the same tree without one must yield the identical verdict.
        # See the "outside" case below, which asserts exactly that pair.
        git -C "${root}" init -q
        git -C "${root}" add -A >/dev/null 2>&1
    fi
    printf '%s' "${root}"
}

# --------------------------------------------------------------- case 1: clean tree
echo "== a tree whose pins are current =="
mk_tree clean 1.2.3 >/dev/null
expect_rc 0 "every member pin matches its package" clean --check
has "the clean verdict names the count it checked" "8 path edge(s) to workspace members, every pin matches" clean
lacks "a clean tree prints no finding" "DRIFT:" clean

# ------------------------------------------------- case 2: the eight stale directions
echo "== the shapes the root-only sed could not reach =="
mk_tree stale 1.0.0 git >/dev/null
expect_rc 1 "stale pins are drift" stale --check
has "member -> sibling inline edge is named" "crates/alpha/Cargo.toml [dependencies] beta declares \"1.0.0\"" stale
has "member -> root edge is named" "crates/alpha/Cargo.toml [dependencies] root-pkg declares \"1.0.0\"" stale
has "dev-dependencies section is reached" "[dev-dependencies] beta" stale
# The quoted cfg key is normalized to what tomllib decodes (`'cfg(..)'` -> `cfg(..)`),
# because cross_check compares section names against tomllib's own view and a label that
# kept the quotes would make the two disagree — which is an exit 2, not a cosmetic diff.
# Two such edges exist in this repository: `--list` shows them under Cargo.toml.
has "target-cfg section is reached" "[target.cfg(not(target_arch = \"wasm32\")).dependencies] beta" stale
has "the multi-line explicit table is reached" "[dependencies] root-pkg" stale
has "the finding prints the version it expected" "but the package is \"1.2.3\"" stale
has "the remedy is printed" "--sync" stale

echo "== the same tree, listed =="
expect_rc 0 "--list is a view, not a verdict" stale --list
has "the listing marks the stale pin" "1.0.0" stale
has "the listing reports the count of stale edges" "stale" stale

# --------------------------------------------------- case 3: the ratchet (new member)
echo "== a new member is audited without editing the tool =="
NEW="$(mk_tree newmember 1.2.3 git)"
mkdir -p "${NEW}/crates/gamma/src"
cat >"${NEW}/crates/gamma/Cargo.toml" <<'TOML'
[package]
name = "gamma"
version = "1.2.3"

[dependencies]
root-pkg = { path = "../..", version = "1.0.0" }
TOML
# Deliberately not `git add`ed: the tree has an index, and the index does not know about
# gamma. Discovery by index would report clean over a stale pin that exists on disk; the
# walk reads the disk, which is where the pin that ships actually lives.
expect_rc 1 "a member added on disk but never tracked is drift on its first run" newmember --check
has "the finding names the new member's manifest" "crates/gamma/Cargo.toml" newmember

# --------------------------------------------- case 3b: discovery reaches past the globs
echo "== a member no crates/* pattern names, with and without a git index =="
# `benchmarks/Cargo.toml` is a real workspace member here and matches no `crates/*`
# pattern; it shipped at 0.3.6 for three releases because every enumerator was glob- or
# list-scoped. The pair below is the same bytes twice, differing only by `.git`: a tool
# that enumerated the index would go silent on the second run.
OUTSIDE="$(mk_tree outside 1.2.3 git)"
mkdir -p "${OUTSIDE}/benchmarks/src"
cat >"${OUTSIDE}/benchmarks/Cargo.toml" <<'TOML'
[package]
name = "bench-pkg"
version = "1.2.3"

[dependencies]
root-pkg = { path = "..", version = "1.0.0" }
TOML
python3 - "${OUTSIDE}" <<'PY'
import sys
p = f"{sys.argv[1]}/Cargo.toml"
s = open(p, encoding="utf-8").read()
anchor = 'members = [".", "crates/alpha", "crates/beta"]'
if s.count(anchor) != 1:
    raise SystemExit(f"the members anchor must appear once, found {s.count(anchor)}")
open(p, "w", encoding="utf-8").write(s.replace(anchor, 'members = [".", "crates/alpha", "crates/beta", "benchmarks"]'))
PY
git -C "${OUTSIDE}" add -A >/dev/null 2>&1
expect_rc 1 "a member the globs cannot name is still audited" outside --check
has "the finding names it" "benchmarks/Cargo.toml [dependencies] root-pkg" outside
rm -rf "${OUTSIDE}/.git"
expect_rc 1 "…and the same bytes with no git at all give the same verdict, because discovery walks the filesystem" outside --check
has "…naming the same edge" "benchmarks/Cargo.toml [dependencies] root-pkg declares \"1.0.0\"" outside

# ------------------------------------------------ case 3c: the prune, and its counter-check
echo "== pruned directories are not a way to hide a pin =="
# `target/` is where `cargo package` unpacks a copy of every published crate, each with
# its own stale pins, and `.qoder/worktrees/` holds full checkout copies of this
# repository — so both must be pruned or the gate fails on bytes nobody can edit. That
# prune is only safe because coverage is cross-checked against `[workspace] members`:
# same tree, same stale pin, and declaring the hidden manifest a member turns the pass
# into exit 2.
PRUNED="$(mk_tree pruned 1.2.3)"
mkdir -p "${PRUNED}/target/package/alpha-0.3.0" "${PRUNED}/.qoder/worktrees/wt/crates/gamma"
cat >"${PRUNED}/target/package/alpha-0.3.0/Cargo.toml" <<'TOML'
[package]
name = "hidden"
version = "1.2.3"

[dependencies]
root-pkg = { path = "../../..", version = "1.0.0" }
TOML
cp "${PRUNED}/target/package/alpha-0.3.0/Cargo.toml" "${PRUNED}/.qoder/worktrees/wt/crates/gamma/Cargo.toml"
expect_rc 0 "a stale pin under target/ or .qoder/ is build output, not the workspace's business" pruned --check
has "…and the count proves those manifests were never opened" "8 path edge(s) to workspace members, every pin matches" pruned
python3 - "${PRUNED}" <<'PY'
import sys
p = f"{sys.argv[1]}/Cargo.toml"
s = open(p, encoding="utf-8").read()
anchor = 'members = [".", "crates/alpha", "crates/beta"]'
if s.count(anchor) != 1:
    raise SystemExit(f"the members anchor must appear once, found {s.count(anchor)}")
open(p, "w", encoding="utf-8").write(
    s.replace(anchor, 'members = [".", "crates/alpha", "crates/beta", "target/package/alpha-0.3.0"]'))
PY
expect_rc 2 "…but a declared member the prune hides is exit 2, not a clean report" pruned --check
has "the refusal names the member it could not find" "target/package/alpha-0.3.0/Cargo.toml" pruned
has "and says discovery, not the pins, is what failed" "discovery was blind" pruned
lacks "and no verdict about matching pins is printed" "every pin matches" pruned

# ------------------------------------------------- case 3d: a symlink back into the tree
echo "== a symlinked directory is walked once, not twice =="
# Discovery follows symlinked directories (refusing only when the target escapes the
# root), which is what `.blackbox/skills -> ../.agents/skills` is in this repository. The
# risk that comes with following them is a file counted along both routes — inflating the
# edge set, and `--sync`'s write count — and two guards stand between a link and a wrong
# number: a directory's realpath is entered once, and each discovered manifest is keyed by
# realpath. Either one alone holds the line below; case 3g isolates the manifest key.
LINKED="$(mk_tree linked 1.2.3)"
ln -s crates "${LINKED}/crateslink"
expect_rc 0 "the same manifest reached twice is still one edge" linked --check
has "…and the count proves nothing was double-counted" "8 path edge(s) to workspace members, every pin matches" linked

# --------------------------------------------- case 3e: a symlinked directory escaping
echo "== a symlinked directory is followed to the root edge, then refused =="
# Following symlinked directories is what makes the case above necessary, and it is what
# makes this one the safety half: a tree can name a manifest that lives somewhere the
# caller never pointed at. Not following it would be the quieter bug — the audit skips the
# escaping subtree and prints "every pin matches" about bytes it never opened.
ESCAPED_DIR="$(mk_tree escaped-dir 1.2.3)"
mkdir -p "${WORK}/elsewhere"
cat >"${WORK}/elsewhere/Cargo.toml" <<'TOML'
[package]
name = "smuggled"
version = "1.2.3"

[dependencies]
root-pkg = { path = "../escaped-dir", version = "1.0.0" }
TOML
ln -s "${WORK}/elsewhere" "${ESCAPED_DIR}/sneaky"
expect_rc 2 "a symlinked directory reaching outside refuses the whole audit" escaped-dir --check
has "…naming the escaping manifest rather than skipping it" "resolve outside the repository root" escaped-dir
expect_rc 2 "…and --sync refuses it too, instead of rewriting through the link" escaped-dir --sync 9.9.9

# ----------------------------------------------- case 3f: a symlink cycle must terminate
echo "== a directory that links to itself does not make discovery recurse forever =="
# Each directory's realpath is entered once, so `self -> .` cannot walk down
# `./self/self/self/…` to ENAMETOOLONG. This asserts the verdict, not the cost: the
# realpath key on the manifest set already collapses any duplicate path the descent
# produces, so the directory guard is insurance against wasted work rather than the only
# thing standing between this tool and a wrong number. Proved by case 3g below, which is
# the shape only the manifest key can catch.
CYCLE="$(mk_tree cycle 1.2.3)"
ln -s . "${CYCLE}/self"
expect_rc 0 "a self-linking directory still reaches a verdict" cycle --check
has "…and reads each manifest exactly once" "8 path edge(s) to workspace members, every pin matches" cycle

# ------------------------------------------- case 3g: a symlinked manifest, kept inside
echo "== a manifest reachable by two in-tree paths is one edge, not two =="
# A real directory holding a symlinked Cargo.toml that points back into the tree: the
# directory walk enters it (no directory realpath repeats), so the same file arrives along
# two paths. Keyed by path, `alpha` is then audited twice, `--list` prints 10 edges for 8
# dependencies, and `--sync` counts a rewrite that writes the same span twice — a second
# pass would find nothing, so no idempotence check would notice.
MIRROR="$(mk_tree mirror 1.0.0)"
mkdir -p "${MIRROR}/mirror"
ln -s ../crates/alpha/Cargo.toml "${MIRROR}/mirror/Cargo.toml"
expect_rc 1 "a mirrored manifest is still drift" mirror --check
has "…against the same eight edges, not twelve" "stale pin(s) of 8" mirror
expect_rc 0 "--list counts the mirrored tree once" mirror --list
has "…and says so in the summary" "8 path edge(s) to workspace members; 6 stale" mirror
lacks "…with no doubled edge anywhere in the listing" "12 path edge(s)" mirror

# --------------------------------------------------- case 4: no literal to sync
echo "== workspace inheritance, an unpinned path edge, and non-members =="
# An edge inherited from [workspace.dependencies] carries no literal at all, so it is
# not drift and not a rewrite target; it leaves the counted edge set entirely.
INHERIT="$(mk_tree inherit 1.2.3)"
python3 - "${INHERIT}" <<'PY'
import sys
p = f"{sys.argv[1]}/crates/alpha/Cargo.toml"
s = open(p, encoding="utf-8").read()
anchor = '[dev-dependencies]\nbeta = { path = "../beta", version = "1.2.3" }'
if s.count(anchor) != 1:
    raise SystemExit(f"the inherit anchor must appear once, found {s.count(anchor)}")
s = s.replace(anchor, '[dev-dependencies]\nbeta = { workspace = true }')
open(p, "w", encoding="utf-8").write(s)
PY
expect_rc 0 "an inherited pin has no literal to sync, so it is not drift" inherit --check
has "…and it leaves the counted edge set" "7 path edge(s)" inherit

# A path edge with no version key is legal Cargo and a real shape here (root's own
# member deps could be written that way); "no pin" is not "stale pin".
NOPIN="$(mk_tree nopin 1.2.3)"
python3 - "${NOPIN}" <<'PY'
import sys
p = f"{sys.argv[1]}/Cargo.toml"
s = open(p, encoding="utf-8").read()
anchor = 'alpha = { path = "crates/alpha", version = "1.2.3" }'
if s.count(anchor) != 1:
    raise SystemExit(f"the nopin anchor must appear once, found {s.count(anchor)}")
s = s.replace(anchor, 'alpha = { path = "crates/alpha" }')
open(p, "w", encoding="utf-8").write(s)
PY
expect_rc 0 "a path edge with no version is not drift" nopin --check
has "…but it is still counted and cross-checked" "8 path edge(s)" nopin

# The case is literally named `vendor` on purpose: `PRUNED_DIRS` prunes a *child* named
# vendor, never the root it was handed, and a tree whose own name is on the prune list
# must still be audited. Rename this case and that regression becomes invisible.
mk_tree vendor 1.2.3 git >/dev/null
expect_rc 0 "a path edge to a non-member is not the workspace's business" vendor --check
expect_rc 0 "…even though discovery does read the non-member manifest" vendor --list
has "the non-member edge is excluded from the count" "8 path edge(s)" vendor
lacks "and the non-member edge never becomes a finding" "whatever" vendor

# ------------------------------------------------------------- case 5: --sync works
echo "== --sync rewrites every direction =="
SYNCED="$(mk_tree synced 1.0.0 git)"
expect_rc 0 "--sync rewrites the stale pins" synced --sync 1.2.3
has "--sync reports the per-file count" "rewrote 4 edge(s) in crates/alpha/Cargo.toml" synced
has "--sync reports the total" "rewrote 6 intra-workspace pin(s) to 1.2.3" synced
expect_rc 0 "the tree is clean after the sync" synced --check
if grep -q "^# Root manifest — comment must survive any rewrite." "${SYNCED}/Cargo.toml"; then
    ok "the sync preserved the manifest comment"
else
    no "the sync destroyed a comment line"
fi
if grep -q 'tokio = { version = "1.43.0", features = \["rt"\] }' "${SYNCED}/crates/alpha/Cargo.toml"; then
    ok "a third-party pin with no path was not touched"
else
    no "the sync rewrote a third-party dependency version"
fi
if grep -q 'serde = { version = "1.0.200"' "${SYNCED}/Cargo.toml"; then
    ok "the root's own third-party pins were not touched"
else
    no "the sync rewrote the root's serde pin"
fi
if grep -q 'version = "9.9.9"' "${SYNCED}/notamember/Cargo.toml"; then
    ok "a manifest outside the member graph was not rewritten"
else
    no "the sync rewrote the non-member manifest"
fi
expect_rc 0 "--sync is idempotent: a second run has nothing to do" synced --sync 1.2.3
has "the second run says so out loud" "nothing to sync" synced

# ------------------------------------------------------------ case 6: --dry-run lies
echo "== --dry-run must not touch a byte =="
DRY="$(mk_tree dry 1.0.0 git)"
cp -a "${DRY}/crates/alpha/Cargo.toml" "${WORK}/dry-before.toml"
expect_rc 0 "--dry-run reports what it would do" dry --sync 1.2.3 --dry-run
has "--dry-run uses the conditional voice" "would rewrite" dry
if cmp -s "${DRY}/crates/alpha/Cargo.toml" "${WORK}/dry-before.toml"; then
    ok "--dry-run left the manifest byte-identical"
else
    no "--dry-run modified the manifest"
fi
expect_rc 1 "--check is still red after a dry run" dry --check

# --------------------------------------------- case 7: a pin the parser cannot see
echo "== a form the text parser cannot read is exit 2, not a pass =="
SINGLE="$(mk_tree single 1.2.3)"
python3 - "${SINGLE}" <<'PY'
import sys
p = f"{sys.argv[1]}/crates/alpha/Cargo.toml"
s = open(p, encoding="utf-8").read()
# Valid TOML, and stale — but single-quoted, so a /version = "x"/ regex never sees it.
s = s.replace('beta = { path = "../beta", version = "1.2.3" }',
              "beta = { path = \"../beta\", version = '1.0.0' }")
open(p, "w", encoding="utf-8").write(s)
PY
expect_rc 2 "a pin tomllib reads and the parser does not refuses to report clean" single --check
has "the refusal names the edge it could not read" "could not read a pin tomllib can" single

UNBALANCED="$(mk_tree unbalanced 1.2.3)"
printf '\n[build-dependencies]\nbroken = { path = "../beta", version = "1.2.3"\n' \
    >>"${UNBALANCED}/crates/alpha/Cargo.toml"
expect_rc 2 "an unbalanced inline table is a parse failure, not a verdict" unbalanced --check
has "the refusal says which file" "unbalanced inline dependency table" unbalanced

DEEP="$(mk_tree deep 1.2.3)"
printf '\n[dependencies.alpha.features]\n' >>"${DEEP}/crates/beta/Cargo.toml"
expect_rc 2 "a dependency table nested deeper than the entry is refused" deep --check
has "the refusal names the header it rejected" "nested too deep" deep

# ------------------------------------------------------------- case 8: exit 2 shapes
echo "== preconditions =="
mkdir -p "${WORK}/empty"
expect_rc 2 "no manifest at all is exit 2" empty --check
expect_rc 2 "a version argument that is not semver is exit 2" clean --sync 1.2
expect_rc 2 "…and a nonsense version is exit 2 too" clean --sync banana
expect_rc 2 "no mode selected is a usage error, not a verdict" clean --list-not-a-mode

# `resolve_root` is the single containment boundary: the caller's path is realpath'd
# there, so a root that does not exist is "could not run" — not an empty edge set that
# `--check` would print as clean.
expect_rc 2 "a root that is not a directory is exit 2, not an empty audit" no-such-dir --check
has "it names the path it refused" "not a repository directory: ${WORK}/no-such-dir" no-such-dir
lacks "and reports no verdict about a tree it never opened" "every pin matches" no-such-dir

# A manifest that resolves outside the root is refused, not filtered. Quietly dropping it
# would print "every pin matches" about a file the audit never read, and `--sync` would
# write through the symlink to somewhere the caller never pointed at.
mkdir -p "${WORK}/outside"
ESCAPE="$(mk_tree escape 1.2.3)"
cat >"${WORK}/outside/Cargo.toml" <<TOML
[package]
name = "beta"
version = "1.2.3"
TOML
ln -sf "${WORK}/outside/Cargo.toml" "${ESCAPE}/crates/beta/Cargo.toml"
expect_rc 2 "a manifest symlinked out of the tree refuses instead of auditing around it" escape --check
has "the refusal names the escaping file" "resolve outside the repository root" escape
expect_rc 2 "…and --sync refuses too, because a partial rewrite is still a rewrite" escape --sync 9.9.9

# ------------------------------------------------------- case 9: the real repository
echo "== the shipped repository is the positive control =="
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
status=0
python3 "${SUBJECT}" --check "${REPO_ROOT}" >"${WORK}/repo.out" 2>&1 || status=$?
if [[ "${status}" == "0" ]]; then
    ok "--check passes on this repository (walked from the filesystem, git or not)"
    sed 's/^/    /' "${WORK}/repo.out"
else
    no "--check on the real tree exited ${status}, expected 0"
    sed 's/^/    /' "${WORK}/repo.out"
fi

printf '\nworkspace version-edge fixture: %s case(s), %s failure(s)\n' "${RUNS}" "${FAILED}"
[[ ${FAILED} -eq 0 ]] || exit 1
