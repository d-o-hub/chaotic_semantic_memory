#!/usr/bin/env bash
# =============================================================================
# release-derive.sh - derive the next release version from commit history
# =============================================================================
# Port of do-harness's release-derivation contract (its release-manager),
# adapted to this
# repository (root [package] version, scripts/sync-version.sh as the bump
# writer, CHANGELOG.md as the notes surface).
#
#   next [--ref <rev>] [--published <csv>] [--format plain|json]
#       Report the release state of <rev> (default HEAD). `bump` is one of:
#         major|minor|patch  the commits since the last release tag warrant a
#                            release (subject to the batch threshold)
#         pending            the workspace version has no tag: the missing step
#                            is a tag, not another bump
#         pending-release    the version's tag exists but its GitHub Release has
#                            not published yet (finish that release first)
#         held-rename        a rename of a shipped surface since the last tag
#                            waits for a `Rename-Confirmed:` trailer or a `!`
#                            breaking marker
#         none               nothing releasable landed
#
#   eligible <release-worthy-count> <bump> [--force]
#       Print `true` when a derived release is eligible for preparation. The
#       batch threshold is three release-worthy commits; `--force` overrides it
#       and never bypasses the green-main requirement the workflow enforces.
#
#   prepare <version>
#       Apply <version> to every managed surface via scripts/sync-version.sh,
#       fold the CHANGELOG [Unreleased] section into `## [<version>] - <date>`
#       (or generate the section from the commits since the last release tag),
#       and re-run scripts/verify-version-sync.sh so the gate proves the tree.
#
#   --self-test
#       Hermetic controls in a throwaway repository: every commit class maps to
#       its bump, a BREAKING CHANGE footer counts as breaking, merge commits are
#       ignored, an untagged workspace version reports `pending`, an unconfirmed
#       shipped-surface rename holds the release, and the published-list gate
#       turns a tagged-but-unreleased basis into `pending-release`.
#
# Bump policy (identical to do-harness):
#   breaking                    -> major at >=1.0, minor at 0.x, patch at 0.0.x
#   feat                        -> minor, patch at 0.0.x
#   fix, perf                   -> patch
#   other conventional types    -> nothing on their own
#   not a conventional subject  -> patch
#
# Usage in automation: .github/workflows/auto-release.yml calls `next`, gates on
# `eligible`, and calls `prepare` before opening the release pull request.
# =============================================================================

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Overridable so --self-test can point the derivation at a throwaway
# repository; every git call and manifest read goes through $ROOT.
ROOT="${CSM_RELEASE_ROOT:-$(cd "${SCRIPT_DIR}/.." && pwd)}"
readonly ROOT
readonly MIN_RELEASE_COMMITS=3
readonly RE_VERSION='^[0-9]+\.[0-9]+\.[0-9]+$'

die() {
    printf 'release-derive: %s\n' "$1" >&2
    exit 1
}

usage() {
    sed -n '2,50p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 2
}

require_version() {
    local version="$1" origin="$2"
    [[ "$version" =~ $RE_VERSION ]] || die "'$version' is not a MAJOR.MINOR.PATCH version (from $origin)"
}

# The batch rule lives here so it is hermetic and testable instead of hiding in
# workflow shell interpolation.
release_eligible() {
    local count="$1" bump="$2" force="${3:-false}"
    [[ "$count" =~ ^[0-9]+$ ]] || return 2
    case "$bump" in
        major | minor | patch) : ;;
        *) return 1 ;;
    esac
    [ "$force" = true ] || [ "$count" -ge "$MIN_RELEASE_COMMITS" ]
}

workspace_version() {
    # The root [package] version is the whole tree's version; every member
    # manifest, pin, package.json and the VERSION file must match it (the
    # sync gate proves that, this reader only has to be exact).
    local version
    version="$(sed -n 's/^version = "\(.*\)"/\1/p' "${ROOT}/Cargo.toml" | head -1)"
    [ -n "$version" ] || die "cannot read [package].version from ${ROOT}/Cargo.toml"
    printf '%s' "$version"
}

# The highest release tag reachable from <ref>, by version order.
latest_tag() {
    git -C "$ROOT" tag --merged "$1" --list 'v[0-9]*' --sort=-v:refname 2>/dev/null \
        | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || true
}

range_for() {
    if [ -n "$BASIS" ]; then
        printf '%s..%s' "$BASIS" "$1"
    else
        printf '%s' "$1"
    fi
}

commit_count() {
    local range="$1" count
    if ! count="$(git -C "$ROOT" rev-list --count "$range")"; then
        die "cannot count commits in range '$range' in $ROOT"
    fi
    printf '%s' "$count"
}

# Sets BREAKING/FEATURE/FIX/NONCONVENTIONAL/RELEASE_WORTHY from <range>.
# Both subject and body are read per commit; a range git rejects must die, never
# classify as "nothing landed".
readonly RE_BREAKING='^[a-zA-Z]+(\([^)]*\))?!:'
readonly RE_FEAT='^feat(\([^)]*\))?:'
readonly RE_FIX='^(fix|perf)(\([^)]*\))?:'
readonly RE_CONVENTIONAL='^[a-zA-Z]+(\([^)]*\))?!?:'
classify() {
    local range="$1" record subject body lowered line records
    BREAKING=false
    FEATURE=false
    FIX=false
    RELEASE_WORTHY=0
    NONCONVENTIONAL=false
    RENAME_TRAILER=false
    if ! records="$(git -C "$ROOT" log --no-merges --format='%s%x1f%b%x1e' "$range")"; then
        die "cannot read commit subjects and bodies in range '$range' in $ROOT"
    fi
    while IFS= read -r -d $'\x1e' record; do
        record="${record#$'\n'}"
        [ -n "$record" ] || continue
        subject="${record%%$'\x1f'*}"
        body="${record#*$'\x1f'}"
        local release_worthy=false
        lowered="$(printf '%s' "$subject" | tr '[:upper:]' '[:lower:]')"
        if [[ "$lowered" =~ $RE_BREAKING ]]; then
            BREAKING=true
            release_worthy=true
        elif [[ "$lowered" =~ $RE_FEAT ]]; then
            FEATURE=true
            release_worthy=true
        elif [[ "$lowered" =~ $RE_FIX ]]; then
            FIX=true
            release_worthy=true
        fi
        if [[ ! "$lowered" =~ $RE_CONVENTIONAL ]]; then
            NONCONVENTIONAL=true
            release_worthy=true
        fi
        while IFS= read -r line; do
            case "$line" in
                'BREAKING CHANGE:'* | 'BREAKING-CHANGE:'*)
                    BREAKING=true
                    release_worthy=true
                    ;;
                'Rename-Confirmed:'*)
                    RENAME_TRAILER=true
                    ;;
            esac
        done <<<"$body"
        if [ "$release_worthy" = true ]; then
            RELEASE_WORTHY=$((RELEASE_WORTHY + 1))
        fi
    done <<<"$records"
}

# A path is internal when it can never carry shipped surface: renames of these
# never trigger the rename hold.
is_internal_path() {
    local path="$1" root
    case "$path" in
        *.md) return 0 ;;
    esac
    for root in tests docs book plans progress examples .agents .claude .github scripts agents-docs benchmarks fuzz; do
        [ "${path#"$root"/}" != "$path" ] && return 0
    done
    return 1
}

# The shipped-surface list for this repository: the manifests consumers pin,
# the npm packages, and the CLI/trait sources that define the public surface.
shipped_surface_path() {
    local path="$1" root
    case "$path" in
        'Cargo.toml' | crates/*/Cargo.toml | 'VERSION' | 'wasm/package.json' | 'cli-npm/package.json')
            return 0
            ;;
    esac
    for root in src/cli crates/csm-cli/src crates/csm-traits/src cli-npm wasm/src; do
        [ "${path#"$root"/}" != "$path" ] && return 0
    done
    return 1
}

# Scans renames git detects (-M) in <range>; an unconfirmed rename of a shipped
# surface holds the release. A range git refuses fails the run.
detect_renames() {
    local range="$1" out line old new
    RENAME_UNCONFIRMED=false
    RENAME_PATH=""
    if ! out="$(git -C "$ROOT" log --no-merges --format='%H' --name-status -M "$range")"; then
        die "cannot read rename status in range '$range' in $ROOT"
    fi
    while IFS= read -r line; do
        case "$line" in
            R*$'\t'*)
                old="${line#*$'\t'}"
                old="${old%%$'\t'*}"
                new="${line##*$'\t'}"
                if is_internal_path "$old" && is_internal_path "$new"; then
                    continue
                fi
                if shipped_surface_path "$old" || shipped_surface_path "$new"; then
                    RENAME_UNCONFIRMED=true
                    [ -n "$RENAME_PATH" ] || RENAME_PATH="$old -> $new"
                fi
                ;;
        esac
    done <<<"$out"
}

decide_bump() {
    local current="$1" major minor
    major="${current%%.*}"
    minor="${current#*.}"
    minor="${minor%%.*}"
    # `0.0.x` is the pre-API phase: keep it on patches so the step to 0.1.0
    # stays a decision rather than an automatic side effect.
    if [ "$major" -eq 0 ] && [ "$minor" -eq 0 ]; then
        printf 'patch'
        return 0
    fi
    if [ "$BREAKING" = true ]; then
        if [ "$major" -ge 1 ]; then
            printf 'major'
        else
            printf 'minor'
        fi
        return 0
    fi
    if [ "$FEATURE" = true ]; then
        printf 'minor'
        return 0
    fi
    if [ "$FIX" = true ] || [ "$NONCONVENTIONAL" = true ]; then
        printf 'patch'
        return 0
    fi
    printf 'none'
}

next_version() {
    local current="$1" bump="$2" major minor patch
    IFS=. read -r major minor patch <<<"$current"
    case "$bump" in
        major) printf '%s.0.0' "$((major + 1))" ;;
        minor) printf '%s.%s.0' "$major" "$((minor + 1))" ;;
        patch) printf '%s.%s.%s' "$major" "$minor" "$((patch + 1))" ;;
        *) printf '%s' "$current" ;;
    esac
}

# Numeric comparison, component by component; `sort -V` is a GNU extension
# BSD sort (macOS) rejects.
version_lt() {
    local a="$1" b="$2" i ai bi
    local -a ap bp
    IFS=. read -r -a ap <<<"$a"
    IFS=. read -r -a bp <<<"$b"
    for i in 0 1 2; do
        ai="${ap[i]:-0}"
        bi="${bp[i]:-0}"
        if [ "$ai" -lt "$bi" ]; then return 0; fi
        if [ "$ai" -gt "$bi" ]; then return 1; fi
    done
    return 1
}

# True when the comma-separated <list> contains <tag>.
list_has() {
    local list="$1" want="$2" item
    list="${list// /}"
    local IFS=,
    for item in $list; do
        [ "$item" = "$want" ] && return 0
    done
    return 1
}

derive() {
    local ref="$1" range
    git -C "$ROOT" rev-parse --verify --quiet "$ref^{commit}" >/dev/null ||
        die "cannot resolve ref '$ref' as a commit in $ROOT"
    CURRENT="$(workspace_version)"
    require_version "$CURRENT" "[package].version in ${ROOT}/Cargo.toml"
    BASIS="$(latest_tag "$ref")"
    range="$(range_for "$ref")"
    COMMITS="$(commit_count "$range")"
    classify "$range"
    detect_renames "$range"
    # A manifest behind the newest release tag derives from the wrong baseline:
    # refuse instead of reporting a plausible number.
    local basis_version=""
    if [ -n "$BASIS" ]; then
        basis_version="${BASIS#v}"
    fi
    if [ -n "$basis_version" ] && [ "$basis_version" != "$CURRENT" ] &&
        version_lt "$CURRENT" "$basis_version"; then
        die "workspace version $CURRENT is behind the newest release tag $BASIS; apply the bump first"
    fi
    # An untagged workspace version is a release waiting for its tag; deriving a
    # bump here would double-bump what a merged release PR already applied.
    if git -C "$ROOT" rev-parse -q --verify "refs/tags/v$CURRENT" >/dev/null; then
        BUMP="$(decide_bump "$CURRENT")"
    else
        BUMP="pending"
    fi
    NEXT="$(next_version "$CURRENT" "$BUMP")"
    # A `Rename-Confirmed:` trailer promotes an ambiguous rename to a deliberate
    # breaking change; re-derive the bump it now implies.
    if [ "$RENAME_UNCONFIRMED" = true ] && [ "$RENAME_TRAILER" = true ]; then
        BREAKING=true
        RELEASE_WORTHY=$((RELEASE_WORTHY + 1))
        if [ "$BUMP" != "pending" ]; then
            BUMP="$(decide_bump "$CURRENT")"
            NEXT="$(next_version "$CURRENT" "$BUMP")"
        fi
    fi
    # An unconfirmed shipped-surface rename is a contract break nobody signed
    # off on: hold instead of letting it slip out as a patch. A range that
    # already derives as breaking ships the rename under that bump.
    if [ "$RENAME_UNCONFIRMED" = true ] && [ "$BREAKING" = false ]; then
        BUMP="held-rename"
        NEXT="$CURRENT"
    fi
    # A tag is not a finished release: the caller passes the tags that have a
    # published GitHub Release, and a basis missing from that list means the
    # next step is to finish that release, not to derive a version on top of it.
    if [ -n "$BASIS" ] && [ "$PUBLISHED_KNOWN" = true ] && ! list_has "$PUBLISHED" "$BASIS"; then
        case "$BUMP" in
            major | minor | patch)
                BUMP="pending-release"
                NEXT="$CURRENT"
                ;;
        esac
    fi
    require_version "$NEXT" "the derived next version (current=$CURRENT bump=$BUMP)"
}

print_plain() {
    printf 'current=%s\nbasis=%s\ncommits=%s\nrelease_worthy=%s\nbreaking=%s\nfeature=%s\nfix=%s\nnonconventional=%s\nrename_unconfirmed=%s\nrename_path=%s\nbump=%s\nnext=%s\n' \
        "$CURRENT" "${BASIS:-none}" "$COMMITS" "$RELEASE_WORTHY" "$BREAKING" "$FEATURE" "$FIX" \
        "$NONCONVENTIONAL" "$RENAME_UNCONFIRMED" "${RENAME_PATH:-}" "$BUMP" "$NEXT"
}

print_json() {
    printf '{"current":"%s","basis":"%s","commits":%s,"release_worthy":%s,"breaking":%s,"feature":%s,"fix":%s,"nonconventional":%s,"rename_unconfirmed":%s,"rename_path":"%s","bump":"%s","next":"%s"}\n' \
        "$CURRENT" "${BASIS:-none}" "$COMMITS" "$RELEASE_WORTHY" "$BREAKING" "$FEATURE" "$FIX" \
        "$NONCONVENTIONAL" "$RENAME_UNCONFIRMED" "${RENAME_PATH:-}" "$BUMP" "$NEXT"
}

# The CHANGELOG section for the release commit. Keep-a-Changelog order:
# an [Unreleased] section with content becomes the version section; without
# one, the section is generated from the commit subjects in the derivation
# range so the release gate's header/date requirement is met either way.
seed_changelog() {
    local version="$1" basis="$2" today range
    today="$(date -u +%Y-%m-%d)"
    if ! grep -q '^## \[Unreleased\]' CHANGELOG.md; then
        printf '\n## [%s] - %s\n' "$version" "$today" >>CHANGELOG.md
        printf 'See commit history for the changes in this release.\n' >>CHANGELOG.md
        return 0
    fi
    local unreleased
    unreleased="$(sed -n '/^## \[Unreleased\]/,/^## \[/p' CHANGELOG.md | sed '1d;$d')"
    if [ -n "$(printf '%s' "$unreleased" | tr -d '[:space:]')" ]; then
        sed -i "s/^## \[Unreleased\]/## [Unreleased]\n\n## [$version] - $today/" CHANGELOG.md
        return 0
    fi
    # An empty [Unreleased]: replace it with a generated section.
    range="$basis..HEAD"
    [ -n "$basis" ] || range="HEAD"
    python3 - "$version" "$today" "$range" <<'PYEOF'
import subprocess, sys, re
version, today, rng = sys.argv[1], sys.argv[2], sys.argv[3]
text = open("CHANGELOG.md").read()
subjects = subprocess.run(
    ["git", "log", "--no-merges", "--format=%s", rng],
    capture_output=True, text=True, check=True,
).stdout.splitlines()
bullets = "\n".join(f"- {s}" for s in subjects[:30]) or "- See commit history for the changes in this release."
section = f"## [{version}] - {today}\n\n{bullets}\n"
new = re.sub(r"## \[Unreleased\]\n", section + "\n## [Unreleased]\n", text, count=1)
open("CHANGELOG.md", "w").write(new)
PYEOF
}

# Apply <version>: the full bump via the canonical writer, then the changelog
# section, then the sync gate as the proof.
cmd_prepare() {
    local version="$1"
    version="${version#v}"
    require_version "$version" "prepare argument"
    cd "$ROOT"
    bash scripts/sync-version.sh "$version"
    local basis
    basis="$(git tag --list 'v[0-9]*' --sort=-v:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || true)"
    seed_changelog "$version" "${basis:-}"
    bash scripts/verify-version-sync.sh
    printf 'release-derive: prepared v%s\n' "$version"
}

self_test() {
    # Not local: the EXIT trap removes it after the function's scope is gone.
    failures=0
    scratch="$(mktemp -d)"
    trap 'rm -rf "$scratch"' EXIT
    git -C "$scratch" init -q
    git -C "$scratch" config user.email t@t
    git -C "$scratch" config user.name t
    git -C "$scratch" config commit.gpgsign false
    git -C "$scratch" config tag.gpgsign false
    commit() {
        (cd "$scratch" && printf '%s\n' "$2" >"$1" && git add "$1" && git commit -qm "$2")
    }
    manifest() {
        (cd "$scratch" && printf '[package]\nname = "x"\nversion = "%s"\n' "$1" >Cargo.toml && git add Cargo.toml)
    }
    manifest 0.3.9
    commit VERSION 0.3.9
    git -C "$scratch" commit -qm "chore: baseline" --allow-empty
    git -C "$scratch" tag v0.3.9
    expect_next() {
        local args="$1" want="$2" label="$3" got
        got="$(cd "$scratch" && CSM_RELEASE_ROOT="$scratch" bash "$SCRIPT_DIR/release-derive.sh" next $args --format json | jq -r .bump)"
        if [ "$got" = "$want" ]; then
            printf 'ok   %s\n' "$label"
        else
            printf 'FAIL %s (bump=%s, expected=%s)\n' "$label" "$got" "$want" >&2
            failures=$((failures + 1))
        fi
    }
    git -C "$scratch" commit -qm "fix(cli): one" --allow-empty
    git -C "$scratch" commit -qm "fix(cli): two" --allow-empty
    expect_next "" "patch" "a two-commit fix batch still derives its bump (the threshold is eligible's)"
    git -C "$scratch" commit -qm "fix(cli): three" --allow-empty
    expect_next "" "patch" "a three-commit fix batch derives a patch"
    git -C "$scratch" commit -qm "feat(cli): new thing" --allow-empty
    expect_next "" "minor" "a feat in the batch derives a minor"
    git -C "$scratch" commit -qm "fix(cli)!: breaking change" --allow-empty
    expect_next "" "minor" "breaking at 0.x derives a minor, not a major"
    git -C "$scratch" commit -qm "chore: docs only" --allow-empty
    git -C "$scratch" commit -qm "not a conventional subject at all" --allow-empty
    expect_next "" "minor" "breaking still wins over later nonconventional noise"
    expect_eligible() {
        local want="$1" got
        got="$(bash "$SCRIPT_DIR/release-derive.sh" eligible "$2" "$3" ${4:+--force})"
        if [ "$got" = "$want" ]; then
            printf 'ok   eligible(%s, %s%s) = %s\n' "$2" "$3" "${4:+, force}" "$want"
        else
            printf 'FAIL eligible(%s, %s%s): got %s, expected %s\n' "$2" "$3" "${4:+, force}" "$got" "$want" >&2
            failures=$((failures + 1))
        fi
    }
    expect_eligible "false" 2 patch
    expect_eligible "true" 3 patch
    expect_eligible "true" 2 patch --force
    expect_eligible "false" 3 none
    # pending: the manifest moved to a version with no tag.
    (cd "$scratch" && sed -i 's/version = "0.3.9"/version = "0.4.0"/' Cargo.toml && git add Cargo.toml && git commit -qm "chore(release): v0.4.0")
    expect_next "" "pending" "an untagged workspace version is pending, not a new bump"
    # pending-release: the tag exists but the published list lacks it.
    git -C "$scratch" tag v0.4.0
    # A tag without a published release holds only when new release-worthy
    # commits would otherwise be derived on top of it; an empty range proposes
    # nothing either way.
    expect_next "--published v0.3.9" "none" "an empty post-release range proposes nothing"
    git -C "$scratch" commit -qm "fix(cli): after the tag" --allow-empty
    git -C "$scratch" commit -qm "fix(cli): after the tag two" --allow-empty
    git -C "$scratch" commit -qm "fix(cli): after the tag three" --allow-empty
    expect_next "--published v0.3.9" "pending-release" "a tagged-but-unpublished basis holds the next derivation"
    expect_next "--published v0.4.0" "patch" "with the release published, the batch derives normally"
    # held-rename: a shipped surface moved with no confirmation marker.
    (cd "$scratch" && git mv VERSION VERSION_OLD 2>/dev/null || {
        git rm -q --cached VERSION
        mv "$scratch/VERSION" "$scratch/VERSION_OLD"
        printf '0.4.0\n' >"$scratch/VERSION_OLD"
        (cd "$scratch" && git add -A)
    })
    git -C "$scratch" commit -qm "chore: move version file"
    expect_next "--published v0.4.0" "held-rename" "an unconfirmed shipped-surface rename holds the release"
    if [ "$failures" -eq 0 ]; then
        printf 'release-derive self-test: pass\n'
    else
        printf 'release-derive self-test: %s failure(s)\n' "$failures" >&2
        exit 1
    fi
}

PUBLISHED=""
PUBLISHED_KNOWN=false
REF="HEAD"
FORMAT="plain"
CMD=""
ELIGIBLE_FORCE=false
require_value() {
    [ -n "${1:-}" ] || die "$2 needs a value"
}
while [ $# -gt 0 ]; do
    case "$1" in
        next) CMD="next" ;;
        eligible)
            CMD="eligible"
            shift
            ELIGIBLE_COUNT="${1:-}"
            require_value "$ELIGIBLE_COUNT" "eligible"
            shift
            ELIGIBLE_BUMP="${1:-}"
            require_value "$ELIGIBLE_BUMP" "eligible"
            ;;
        prepare)
            CMD="prepare"
            shift
            PREPARE_VERSION="${1:-}"
            require_value "$PREPARE_VERSION" "prepare"
            ;;
        --self-test) CMD="self-test" ;;
        --ref) shift; REF="${1:-}"; require_value "$REF" "--ref" ;;
        --published) shift; PUBLISHED="${1:-}"; PUBLISHED_KNOWN=true ;;
        --format) shift; FORMAT="${1:-}" ;;
        --force) ELIGIBLE_FORCE=true ;;
        --help | -h) usage ;;
        *) die "unknown argument: $1" ;;
    esac
    shift
done

case "$CMD" in
    next)
        derive "$REF"
        case "$FORMAT" in
            json) print_json ;;
            plain) print_plain ;;
            *) die "unknown format '$FORMAT' (plain|json)" ;;
        esac
        ;;
    eligible)
        [ -n "${ELIGIBLE_COUNT:-}" ] && [ -n "${ELIGIBLE_BUMP:-}" ] || die "usage: release-derive.sh eligible <count> <bump> [--force]"
        if release_eligible "$ELIGIBLE_COUNT" "$ELIGIBLE_BUMP" "${ELIGIBLE_FORCE:-false}"; then
            printf 'true\n'
        else
            printf 'false\n'
        fi
        ;;
    prepare)
        [ -n "${PREPARE_VERSION:-}" ] || die "usage: release-derive.sh prepare <version>"
        cmd_prepare "$PREPARE_VERSION"
        ;;
    self-test) self_test ;;
    *) usage ;;
esac
