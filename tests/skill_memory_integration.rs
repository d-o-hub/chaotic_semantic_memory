//! Skill-Memory Integration Tests
//!
//! Tests for skill-memory production usage patterns:
//! - DB location verification (.agents/csm-memory/skill-memory.db)
//! - Save/recall concept flow using skill-memory CLI patterns
//! - DB file creation verification
//! - Roundtrip persistence

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

/// Helper to create a CSM CLI command
fn csm() -> Command {
    let mut cmd = cargo_bin_cmd!("csm");
    cmd.current_dir(std::env::current_dir().expect("should have current dir"));
    cmd
}

/// Helper to create a temp directory with skill-memory DB path structure
fn skill_memory_tempdir() -> TempDir {
    TempDir::new().expect("failed to create temp dir")
}

/// Helper to generate skill-memory canonical ID
/// Format: skill::<skill_name>::<category>::<slug-or-timestamp>
fn skill_id(skill: &str, category: &str, slug: &str) -> String {
    format!("skill::{skill}::{category}::{slug}")
}

// =============================================================================
// DB Location Tests
// =============================================================================

#[test]
fn skill_memory_db_location_follows_convention() {
    let temp_dir = skill_memory_tempdir();
    let agents_dir = temp_dir.path().join(".agents").join("csm-memory");
    std::fs::create_dir_all(&agents_dir).expect("failed to create agents dir");

    let db_path = agents_dir.join("skill-memory.db");

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg("skill::test::validation::location-test")
        .assert()
        .success();

    // Verify DB file was created at expected location
    assert!(
        db_path.exists(),
        "DB file should exist at .agents/csm-memory/skill-memory.db"
    );
}

#[test]
fn skill_memory_db_requires_parent_directories() {
    let temp_dir = skill_memory_tempdir();
    // Path with non-existent parent directories
    let db_path = temp_dir
        .path()
        .join(".agents")
        .join("csm-memory")
        .join("skill-memory.db");

    // Parent directories don't exist yet - CSM requires them to exist
    assert!(!db_path.parent().unwrap().exists());

    // Create parent directories before injecting
    std::fs::create_dir_all(db_path.parent().unwrap()).expect("failed to create parent dirs");

    // CSM should now succeed
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg("skill::test::init::dir-creation-test")
        .assert()
        .success();

    // Verify DB file was created
    assert!(
        db_path.exists(),
        "DB file should be created when parent dirs exist"
    );
}

// =============================================================================
// Save/Recall Flow Tests (using query for semantic search)
// =============================================================================

#[test]
fn skill_memory_save_single_concept() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Save a skill-memory concept with canonical ID format
    let concept_id = skill_id(
        "rust-development",
        "code_refactoring",
        "thiserror-migration",
    );

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&concept_id)
        .arg("-m")
        .arg(
            r#"{"operation":"refactoring","context":"Converted error types to thiserror","result":"Reduced boilerplate 40 lines"}"#,
        )
        .assert()
        .success();
}

#[test]
fn skill_memory_recall_by_id() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // First, save a concept
    let concept_id = skill_id("debugging", "error_resolution", "spectral-radius-fix");

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&concept_id)
        .arg("-m")
        .arg(
            r#"{"operation":"error_resolution","context":"Spectral radius validation","result":"Clamped to valid range"}"#,
        )
        .assert()
        .success();

    // Probe by exact concept ID to find similar concepts
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("probe")
        .arg(&concept_id)
        .arg("-k")
        .arg("5")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("spectral-radius-fix"));
}

#[test]
fn skill_memory_query_by_semantic_text() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Save a concept with text preview for semantic search
    let concept_id = skill_id(
        "debugging",
        "error_resolution",
        "spectral-radius-validation",
    );
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&concept_id)
        .arg("-m")
        .arg(
            r#"{"operation":"error_resolution","text_preview":"Spectral radius out of range causing unstable reservoir dynamics","result":"Added validation to clamp values"}"#,
        )
        .assert()
        .success();

    // Query using semantic text search (not probe by ID)
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("query")
        .arg("spectral radius reservoir dynamics")
        .arg("-k")
        .arg("5")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success();
}

#[test]
fn skill_memory_associate_concepts() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Save two concepts
    let error_id = skill_id("debugging", "error", "timeout-issue");
    let solution_id = skill_id("rust", "fix", "timeout-solution");

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&error_id)
        .assert()
        .success();

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&solution_id)
        .assert()
        .success();

    // Associate them
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("associate")
        .arg(&error_id)
        .arg(&solution_id)
        .arg("-s")
        .arg("0.95")
        .assert()
        .success();
}

// =============================================================================
// Roundtrip Persistence Tests
// =============================================================================

#[test]
fn skill_memory_roundtrip_export_import() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");
    let export_path = temp_dir.path().join("export.json");

    // Save concepts with skill-memory canonical format
    let concept_id = skill_id("adr-creation", "architectural_decision", "cache-layer");

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&concept_id)
        .arg("-m")
        .arg(r#"{"operation":"adr","context":"LRU cache for concepts","result":"128 entry limit"}"#)
        .assert()
        .success();

    // Export to file
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("export")
        .arg("-o")
        .arg(&export_path)
        .assert()
        .success();

    // Verify export file exists
    assert!(export_path.exists(), "Export file should be created");

    // Import to new database
    let db_path2 = temp_dir.path().join("test2.db");
    csm()
        .arg("--database")
        .arg(&db_path2)
        .arg("import")
        .arg(&export_path)
        .assert()
        .success();

    // Verify concept exists in new database by probing it
    csm()
        .arg("--database")
        .arg(&db_path2)
        .arg("probe")
        .arg(&concept_id)
        .arg("-k")
        .arg("1")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("cache-layer"));
}

#[test]
fn skill_memory_persists_across_sessions() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Session 1: Write concept
    let concept_id = skill_id("session", "persistence", "cross-session-test");

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&concept_id)
        .arg("-m")
        .arg(r#"{"session":1,"persisted":true}"#)
        .assert()
        .success();

    // Session 2: Read concept (simulating new session via probe by ID)
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("probe")
        .arg(&concept_id)
        .arg("-k")
        .arg("1")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("cross-session-test"));
}

// =============================================================================
// Metadata and JSON Handling Tests
// =============================================================================

#[test]
fn skill_memory_metadata_json_parsing() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Test complex metadata
    let concept_id = skill_id("complex", "metadata", "json-test");

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&concept_id)
        .arg("-m")
        .arg(
            r#"{"operation":"test","nested":{"key":"value"},"array":[1,2,3],"string":"test string"}"#,
        )
        .assert()
        .success();

    // Verify concept was stored via probe
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("probe")
        .arg(&concept_id)
        .arg("-k")
        .arg("1")
        .assert()
        .success();
}

#[test]
fn skill_memory_concept_id_naming_convention() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");
    let export_path = temp_dir.path().join("export.json");

    // Test various valid skill-memory ID formats
    let valid_ids = vec![
        "skill::impl::refactor::2026-04-12T08-22-10Z",
        "skill::fix::error-resolution::merge-conflict-overwrite",
        "skill::adr-creation::architectural_decision::cache-layer-implementation",
        "skill::testing::regression::load-merge-no-overwrite",
    ];

    for id in &valid_ids {
        csm()
            .arg("--database")
            .arg(&db_path)
            .arg("inject")
            .arg(id)
            .assert()
            .success();
    }

    // Export and verify all IDs are present in file
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("export")
        .arg("-o")
        .arg(&export_path)
        .assert()
        .success();

    // Read and verify export file contents
    let export_content = std::fs::read_to_string(&export_path).expect("failed to read export");
    assert!(
        export_content.contains("skill::impl::refactor"),
        "Should contain impl::refactor ID"
    );
    assert!(
        export_content.contains("skill::fix::error-resolution"),
        "Should contain fix::error-resolution ID"
    );
    assert!(
        export_content.contains("skill::adr-creation::architectural_decision"),
        "Should contain adr-creation ID"
    );
    assert!(
        export_content.contains("skill::testing::regression"),
        "Should contain testing::regression ID"
    );
}

// =============================================================================
// Workflow Integration Tests
// =============================================================================

#[test]
fn skill_memory_full_workflow_adr_pattern() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Simulate full ADR workflow from SKILL.md

    // 1. Record ADR decision
    let adr_id = skill_id("adr-creation", "architectural_decision", "lru-cache-impl");

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&adr_id)
        .arg("-m")
        .arg(
            r#"{"operation":"decision","context":"Implement LRU cache for concept storage","result":"Use lru crate with 128 entry limit"}"#,
        )
        .assert()
        .success();

    // 2. Record refactoring
    let refactor_id = skill_id(
        "rust-development",
        "code_refactoring",
        "thiserror-migration",
    );

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&refactor_id)
        .arg("-m")
        .arg(
            r#"{"operation":"refactoring","context":"Error handling refactoring","result":"Converted to thiserror"}"#,
        )
        .assert()
        .success();

    // 3. Associate refactoring with ADR
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("associate")
        .arg(&refactor_id)
        .arg(&adr_id)
        .arg("-s")
        .arg("0.7")
        .assert()
        .success();

    // 4. Record error resolution
    let error_id = skill_id(
        "debugging",
        "error_resolution",
        "spectral-radius-validation",
    );

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&error_id)
        .arg("-m")
        .arg(
            r#"{"operation":"error_resolution","context":"Spectral radius out of range","result":"Added validation to clamp to [0.9, 1.1]"}"#,
        )
        .assert()
        .success();

    // 5. Verify all can be recalled by ID probe
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("probe")
        .arg(&adr_id)
        .arg("-k")
        .arg("1")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("lru-cache-impl"));

    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("probe")
        .arg(&error_id)
        .arg("-k")
        .arg("1")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("spectral-radius-validation"));
}

#[test]
fn skill_memory_quick_validation_pattern() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Test the quick validation pattern from SKILL.md

    // Write smoke test concept
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg("skill::internal::smoke::quick-validation")
        .arg("-m")
        .arg(r#"{"smoke":true}"#)
        .assert()
        .success();

    // Read it back by probing the ID
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("probe")
        .arg("skill::internal::smoke::quick-validation")
        .arg("-k")
        .arg("1")
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("quick-validation"));
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn skill_memory_duplicate_id_succeeds() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    let concept_id = skill_id("test", "dup", "same-id");

    // First inject should succeed
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&concept_id)
        .arg("-m")
        .arg(r#"{"version":1}"#)
        .assert()
        .success();

    // Second inject with same ID should also succeed (upsert behavior)
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg(&concept_id)
        .arg("-m")
        .arg(r#"{"version":2}"#)
        .assert()
        .success();
}

#[test]
fn skill_memory_empty_metadata() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Inject without metadata should work
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("inject")
        .arg("skill::test::minimal::no-metadata")
        .assert()
        .success();

    // Verify it was stored via probe
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("probe")
        .arg("skill::test::minimal::no-metadata")
        .arg("-k")
        .arg("1")
        .assert()
        .success();
}

#[test]
fn skill_memory_probe_missing_returns_failure() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Probe on empty DB should fail
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("probe")
        .arg("nonexistent-concept-id")
        .assert()
        .failure();
}

#[test]
fn skill_memory_associate_missing_concept_fails() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("test.db");

    // Associate non-existent concepts should fail
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("associate")
        .arg("skill::missing::concept::a")
        .arg("skill::missing::concept::b")
        .assert()
        .failure();
}

#[test]
fn skill_memory_export_empty_database() {
    let temp_dir = skill_memory_tempdir();
    let db_path = temp_dir.path().join("empty.db");
    let export_path = temp_dir.path().join("empty-export.json");

    // Export from empty database should succeed (with warning)
    csm()
        .arg("--database")
        .arg(&db_path)
        .arg("export")
        .arg("-o")
        .arg(&export_path)
        .assert()
        .success();

    // Verify export file was created
    assert!(
        export_path.exists(),
        "Export file should be created even for empty DB"
    );
}
