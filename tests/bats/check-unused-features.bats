#!/usr/bin/env bats

setup() {
    TEST_DIR="$(mktemp -d)"
    export TEST_DIR
    SCRIPT_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")/../.." && pwd)"
    export SCRIPT_UNDER_TEST="${SCRIPT_DIR}/scripts/check-unused-features.sh"
}

teardown() {
    rm -rf "$TEST_DIR"
}

@test "unused feature: exits 1 when feature declared but no cfg usage" {
    mkdir -p "${TEST_DIR}/src" "${TEST_DIR}/tests"
    cat > "${TEST_DIR}/Cargo.toml" <<'EOF'
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[features]
unused-feature = []
EOF
    cat > "${TEST_DIR}/src/lib.rs" <<'EOF'
pub fn hello() -> &'static str { "hello" }
EOF

    run bash "$SCRIPT_UNDER_TEST"
    cd "$TEST_DIR" && run bash "$SCRIPT_UNDER_TEST"

    [ "$status" -eq 1 ]
    [[ "$output" == *"unused-feature"* ]]
}

@test "used in tests/: exits 0 when feature used only in tests/ directory" {
    mkdir -p "${TEST_DIR}/src" "${TEST_DIR}/tests"
    cat > "${TEST_DIR}/Cargo.toml" <<'EOF'
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[features]
test-only-feature = []
EOF
    cat > "${TEST_DIR}/src/lib.rs" <<'EOF'
pub fn hello() -> &'static str { "hello" }
EOF
    cat > "${TEST_DIR}/tests/integration.rs" <<'EOF'
#[cfg(feature = "test-only-feature")]
fn test_something() {}
EOF

    cd "$TEST_DIR" && run bash "$SCRIPT_UNDER_TEST"

    [ "$status" -eq 0 ]
    [[ "$output" == *"ok: all"* ]]
}

@test "default feature skipped: exits 0 when default is the only declaration" {
    mkdir -p "${TEST_DIR}/src"
    cat > "${TEST_DIR}/Cargo.toml" <<'EOF'
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[features]
default = []
EOF
    cat > "${TEST_DIR}/src/lib.rs" <<'EOF'
pub fn hello() -> &'static str { "hello" }
EOF

    cd "$TEST_DIR" && run bash "$SCRIPT_UNDER_TEST"

    [ "$status" -eq 0 ]
    [[ "$output" == *"ok: all 0"* ]]
}

@test "metadata features with const markers: exits 0 when const sentinel exists" {
    mkdir -p "${TEST_DIR}/src"
    cat > "${TEST_DIR}/Cargo.toml" <<'EOF'
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[features]
wasm = []
serde = []
signing = []
EOF
    cat > "${TEST_DIR}/src/lib.rs" <<'EOF'
#[cfg(feature = "wasm")]
const _FEATURE_WASM: () = ();

#[cfg(feature = "serde")]
const _FEATURE_SERDE: () = ();

#[cfg(feature = "signing")]
const _FEATURE_SIGNING: () = ();

pub fn hello() -> &'static str { "hello" }
EOF

    cd "$TEST_DIR" && run bash "$SCRIPT_UNDER_TEST"

    [ "$status" -eq 0 ]
    [[ "$output" == *"ok: all 3"* ]]
}
