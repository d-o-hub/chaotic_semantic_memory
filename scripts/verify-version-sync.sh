#!/bin/bash
# verify-version-sync.sh - Ensure all version numbers are synchronized
# Run this before releases to catch version drift

set -e

# Extract version from Cargo.toml
CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

# Extract version from wasm/package.json
NPM_LOCAL_VERSION=$(grep -m 1 '"version":' wasm/package.json | sed 's/.*"version": "\(.*\)".*/\1/')

# Extract version from cli-npm/package.json
CLI_NPM_VERSION=$(grep -m 1 '"version":' cli-npm/package.json | sed 's/.*"version": "\(.*\)".*/\1/')

# Extract version from VERSION file
VERSION_FILE_CONTENT=$(tr -d '[:space:]' < VERSION)

echo "Cargo.toml version: $CARGO_VERSION"
echo "wasm/package.json version: $NPM_LOCAL_VERSION"
echo "cli-npm/package.json version: $CLI_NPM_VERSION"
echo "VERSION file: $VERSION_FILE_CONTENT"

FAILED=0

if [ "$CARGO_VERSION" != "$NPM_LOCAL_VERSION" ]; then
    echo "ERROR: Version mismatch in wasm/package.json!"
    echo "  Cargo.toml: $CARGO_VERSION"
    echo "  wasm/package.json: $NPM_LOCAL_VERSION"
    FAILED=1
fi

if [ "$CARGO_VERSION" != "$CLI_NPM_VERSION" ]; then
    echo "ERROR: Version mismatch in cli-npm/package.json!"
    echo "  Cargo.toml: $CARGO_VERSION"
    echo "  cli-npm/package.json: $CLI_NPM_VERSION"
    FAILED=1
fi

if [ "$CARGO_VERSION" != "$VERSION_FILE_CONTENT" ]; then
    echo "ERROR: Version mismatch in VERSION file!"
    echo "  Cargo.toml: $CARGO_VERSION"
    echo "  VERSION: $VERSION_FILE_CONTENT"
    FAILED=1
fi

# Verify every intra-workspace dependency pin, in both directions and in every section.
# The block this replaces grepped `path = "crates/csm-…", version = "…"` out of the root
# manifest only — the single direction sync-version.sh already wrote correctly — while 18
# member -> anything pins sat at 0.3.8 (and 0.3.7 in fuzz/Cargo.toml) and this script
# printed "✓ All local versions synchronized" and exited 0 (issue #964). Same parser as
# the writer, so the tool and the gate cannot disagree about what a pin is.
echo "Checking intra-workspace dependency pins (all manifests, all sections)..."
if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required to audit workspace version pins" >&2
    exit 2
fi
if [[ ! -f scripts/workspace-version-edges.py ]]; then
    # Fail closed and say why: a verifier that silently skipped the pin audit would print
    # the same "✓ All local versions synchronized" line that let 18 stale pins ship.
    echo "ERROR: scripts/workspace-version-edges.py is missing — the pin audit cannot run" >&2
    exit 2
fi
EDGES_RC=0
python3 scripts/workspace-version-edges.py --check . || EDGES_RC=$?
if [[ "$EDGES_RC" -eq 2 ]]; then
    # A sensor that did not run is not a pass: exit 2 keeps "could not audit" distinct
    # from "drift", which is how #903/#948/#949 classify an unrunnable probe.
    echo "ERROR: the workspace version-pin audit could not run (exit 2) — not a verdict" >&2
    exit 2
elif [[ "$EDGES_RC" -ne 0 ]]; then
    FAILED=1
fi

# Verify workspace member versions match CARGO_VERSION. `crates/` alone missed
# benchmarks, a declared root [workspace] member.
echo "Checking workspace crate versions in crates/*/Cargo.toml and benchmarks/Cargo.toml..."
CRATE_VERSION_MISMATCHES=$(find crates benchmarks -name "Cargo.toml" -exec grep -H '^version = ' {} + | grep -v "version = \"$CARGO_VERSION\"" || true)
if [ -n "$CRATE_VERSION_MISMATCHES" ]; then
    echo "ERROR: Workspace crate version mismatch found:"
    echo "$CRATE_VERSION_MISMATCHES"
    FAILED=1
fi

# Check every version literal in tests/examples/docs against the live version.
# The block this replaces grepped only `"0\.2\.[0-4]"`, so the seven `0.3.x`
# literals left behind by `sync-version.sh`'s previous-literal substitution were
# invisible to it (issue #982). The enumerator derives its comparison from
# Cargo.toml, so it cannot be outgrown by the version it is checking.
echo ""
echo "Checking version literals in tests/examples/docs..."
if [[ ! -f scripts/check-version-literals.py ]]; then
    echo "ERROR: scripts/check-version-literals.py is missing — the literal gate cannot run" >&2
    exit 2
fi
if ! python3 scripts/check-version-literals.py .; then
    FAILED=1
fi

if [ $FAILED -ne 0 ]; then
    exit 1
fi

# Check npm registry version (warning only)
echo ""
echo "Checking npm registry..."
NPM_REGISTRY_VERSION=$(npm view @d-o-hub/chaotic_semantic_memory version 2>/dev/null || echo "not published")

if [ "$NPM_REGISTRY_VERSION" = "not published" ]; then
    echo "⚠ WASM package not yet published to npm"
elif [ "$NPM_REGISTRY_VERSION" != "$CARGO_VERSION" ]; then
    echo "⚠ npm registry version ($NPM_REGISTRY_VERSION) differs from local ($CARGO_VERSION)"
    echo "  Run: cd wasm && npm publish"
else
    echo "✓ npm registry version: $NPM_REGISTRY_VERSION"
fi

echo ""
echo "✓ All local versions synchronized: $CARGO_VERSION"
