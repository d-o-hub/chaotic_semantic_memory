//! Architecture fitness tests — enforce structural invariants at test-time.
//! Known exceptions are documented and accepted per ADR-0090.

use std::process::Command;

const MAX_LOC: usize = 500;

/// Known LOC exceptions in workspace crates (pre-existing, tracked for reduction).
const LOC_EXCEPTIONS_CRATES: &[&str] = &[
    "crates/csm-core/src/hyperdim.rs",
    "crates/csm-memory/src/singularity.rs",
    "crates/csm-memory/src/graph_traversal.rs",
    "src/mcp/handler.rs",
    "crates/csm-retrieval/src/hybrid.rs",
];

/// Known files with unsafe in root src/ (accepted uses with SAFETY docs).
const UNSAFE_EXCEPTIONS_ROOT: &[&str] = &[
    "src/retrieval/bm25.rs", // thread_local! + UnsafeCell for scoring buffers
    "src/embedding/mod.rs",  // FFI bridge for fastembed
];

/// Known files with unsafe in csm-core outside *simd*.rs.
const UNSAFE_EXCEPTIONS_CORE: &[&str] = &[
    "crates/csm-core/src/reservoir.rs",
    "crates/csm-core/src/reservoir_sparse.rs",
    "crates/csm-core/src/reservoir_chaotic.rs",
    "crates/csm-core/src/bundle.rs",
    "crates/csm-core/src/hyperdim.rs",
    "crates/csm-core/src/hyperdim_batch.rs",
    "crates/csm-core/src/hashing/chaotic_lsh.rs", // SIMD intrinsics in project_avx2/project_neon
];

fn find_rs_files(dir: &str) -> Vec<String> {
    let output = Command::new("find")
        .args([dir, "-name", "*.rs", "-type", "f"])
        .output()
        .expect("failed to run find");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

fn line_count(path: &str) -> usize {
    let output = Command::new("wc")
        .args(["-l", path])
        .output()
        .expect("failed to run wc");
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

#[test]
fn loc_gate_src_files_under_500_lines() {
    let mut violations = Vec::new();
    for file in find_rs_files("src") {
        let loc = line_count(&file);
        if loc > MAX_LOC {
            violations.push(format!("{file}: {loc} lines"));
        }
    }
    assert!(
        violations.is_empty(),
        "Files exceeding {MAX_LOC} LOC in src/:\n{}",
        violations.join("\n")
    );
}

#[test]
fn loc_gate_workspace_crate_src_files_under_500_lines() {
    let output = Command::new("find")
        .args(["crates", "-path", "*/src/*.rs", "-type", "f"])
        .output()
        .expect("failed to run find");
    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let mut violations = Vec::new();
    for file in files {
        let loc = line_count(&file);
        if loc > MAX_LOC && !LOC_EXCEPTIONS_CRATES.iter().any(|e| file.ends_with(e)) {
            violations.push(format!("{file}: {loc} lines"));
        }
    }
    assert!(
        violations.is_empty(),
        "Files exceeding {MAX_LOC} LOC in crates/*/src/ (excluding known exceptions):\n{}",
        violations.join("\n")
    );
}

#[test]
fn unsafe_audit_root_src_no_new_unsafe() {
    let output = Command::new("grep")
        .args(["-rl", "unsafe", "src/"])
        .output()
        .expect("failed to run grep");
    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|f| !UNSAFE_EXCEPTIONS_ROOT.iter().any(|e| f.ends_with(e)))
        .map(String::from)
        .collect();
    assert!(
        files.is_empty(),
        "Root src/ has new `unsafe` blocks not in exceptions list:\n{}",
        files.join("\n")
    );
}

#[test]
fn unsafe_audit_csm_core_only_allowed_files() {
    let output = Command::new("grep")
        .args(["-rl", "unsafe", "crates/csm-core/src/"])
        .output()
        .expect("failed to run grep");
    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    let mut violations = Vec::new();
    for file in &files {
        let basename = file.rsplit('/').next().unwrap_or(file);
        let is_simd = basename.contains("simd");
        let is_exception = UNSAFE_EXCEPTIONS_CORE.iter().any(|e| file.ends_with(e));
        if !is_simd && !is_exception {
            violations.push(file.clone());
        }
    }
    assert!(
        violations.is_empty(),
        "New `unsafe` in crates/csm-core/src/ outside allowed files:\n{}",
        violations.join("\n")
    );
}

#[test]
fn public_api_stability_prelude_exports() {
    let lib_src = include_str!("../src/lib.rs");
    let required = [
        "HVec10240",
        "ChaoticSemanticFramework",
        "FrameworkBuilder",
        "Concept",
        "ConceptBuilder",
        "MemoryError",
    ];
    let mut missing = Vec::new();
    for symbol in &required {
        if !lib_src.contains(symbol) {
            missing.push(*symbol);
        }
    }
    assert!(
        missing.is_empty(),
        "src/lib.rs must publicly export these symbols: {missing:?}"
    );
}

#[test]
fn module_layering_cli_does_not_import_persistence_directly() {
    let output = Command::new("grep")
        .args(["-rl", "use crate::persistence", "src/cli/"])
        .output()
        .expect("failed to run grep");
    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    assert!(
        files.is_empty(),
        "src/cli/ must not import from persistence directly (use framework instead):\n{}",
        files.join("\n")
    );
}
