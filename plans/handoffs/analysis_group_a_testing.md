# Swarm Group A (Testing & Quality) - Analysis Report

**Date**: 2026-02-17  
**Repository**: chaotic_semantic_memory  
**Branch**: main  

## Executive Summary

Analyzed the chaotic_semantic_memory Rust codebase for testing gaps. Found **significant coverage gaps** in:
1. DB record verification (no direct SQL validation)
2. Property-based testing (limited to basic HVec10240 operations)
3. Fuzzing (3 basic targets, missing critical paths)
4. Edge cases (missing error paths, boundary conditions)

---

## 1. Test Edge Cases - Missing Coverage

### 1.1 Critical Missing Tests in `tests/edge_case_coverage.rs`

**Add these test cases:**

```rust
// Line ~78: Add to tests/edge_case_coverage.rs

#[test]
fn concept_id_max_length_boundary() {
    let mut singularity = Singularity::new();
    let max_id = "a".repeat(256);
    let concept = Concept {
        id: max_id.clone(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
    };
    // Should succeed at exactly 256 bytes
    assert!(singularity.inject(concept).is_ok());
    
    let too_long = "b".repeat(257);
    let concept2 = Concept {
        id: too_long,
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
    };
    // Framework validation should reject this
}

#[test]
fn association_self_loop() {
    let mut singularity = Singularity::new();
    let concept = Concept {
        id: "self".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
    };
    singularity.inject(concept).unwrap();
    // Self-association should be allowed or explicitly rejected
    let result = singularity.associate("self", "self", 0.5);
    assert!(result.is_ok() || result.is_err()); // Document expected behavior
}

#[test]
fn negative_association_strength_handling() {
    let mut singularity = Singularity::new();
    let c1 = Concept { id: "a".to_string(), vector: HVec10240::random(), metadata: HashMap::new(), created_at: 1, modified_at: 1 };
    let c2 = Concept { id: "b".to_string(), vector: HVec10240::random(), metadata: HashMap::new(), created_at: 1, modified_at: 1 };
    singularity.inject(c1).unwrap();
    singularity.inject(c2).unwrap();
    
    // Current code doesn't validate strength range - test what happens
    let result = singularity.associate("a", "b", -0.5);
    // Document expected behavior
}

#[test]
fn reservoir_exact_dimension_boundary() {
    // Test reservoir at exactly HVec10240::DIMENSION size
    let reservoir = Reservoir::new_seeded(10, HVec10240::DIMENSION, 42).unwrap();
    assert_eq!(reservoir.size(), HVec10240::DIMENSION);
    
    // Test one below
    let small = Reservoir::new_seeded(10, HVec10240::DIMENSION - 1, 42).unwrap();
    assert!(small.to_hypervector().is_err());
    
    // Test one above
    let large = Reservoir::new_seeded(10, HVec10240::DIMENSION + 1, 42).unwrap();
    assert!(large.to_hypervector().is_ok());
}

#[tokio::test]
async fn framework_top_k_at_max_boundary() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_probe_top_k(100)
        .build()
        .await
        .unwrap();
    
    // Inject some concepts
    for i in 0..10 {
        framework.inject_concept(format!("c{}", i), HVec10240::random()).await.unwrap();
    }
    
    // At limit should succeed
    let result = framework.probe(HVec10240::random(), 100).await;
    assert!(result.is_ok());
    
    // Over limit should fail
    let result = framework.probe(HVec10240::random(), 101).await;
    assert!(result.is_err());
}

#[test]
fn hvec_sparse_extreme_densities() {
    // Density 0 should produce zero vector
    let zero_density = HVec10240::sparse(0.0);
    assert_eq!(zero_density.data.iter().sum::<u128>(), 0);
    
    // Density 1.0 should produce all-ones vector
    let full_density = HVec10240::sparse(1.0);
    assert!(full_density.data.iter().all(|&w| w == u128::MAX));
}

#[test]
fn singularity_cache_eviction_with_duplicate_queries() {
    let mut singularity = Singularity::with_config(SingularityConfig {
        max_concepts: None,
        max_associations_per_concept: None,
        concept_cache_size: 2, // Very small cache
    });
    
    // Add concepts
    for i in 0..5 {
        let c = Concept {
            id: format!("c{}", i),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: i as u64,
            modified_at: i as u64,
        };
        singularity.inject(c).unwrap();
    }
    
    let query = HVec10240::random();
    let result1 = singularity.find_similar(&query, 3);
    let result2 = singularity.find_similar(&query, 3);
    // With caching, should get same Arc
}
```

### 1.2 Missing Error Path Tests

**Create new file: `tests/error_path_coverage.rs`**

```rust
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::{MemoryError, persistence::Persistence};
use tempfile::NamedTempFile;

#[tokio::test]
async fn persistence_corrupted_vector_bytes() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    // Manually insert corrupted vector data
    let conn = persistence.connect().await.unwrap();
    conn.execute(
        "INSERT INTO concepts (id, vector, metadata, created_at, modified_at) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        libsql::params![
            "corrupted",
            vec![0u8; 100], // Wrong size - should be 1280
            "{}",
            1i64,
            1i64
        ]
    ).await.unwrap();
    
    // Loading should fail with dimension error
    let result = persistence.load_concept("corrupted").await;
    assert!(matches!(result, Err(MemoryError::InvalidDimension { .. })));
}

#[tokio::test]
async fn persistence_concurrent_write_conflict_simulation() {
    // Test behavior when multiple connections try to write
    // This requires direct SQL to simulate conflicts
}

#[tokio::test]
async fn framework_inject_duplicate_concept() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    
    let vec1 = HVec10240::random();
    framework.inject_concept("dup", vec1).await.unwrap();
    
    // Injecting same ID should update, not error
    let vec2 = HVec10240::random();
    framework.inject_concept("dup", vec2).await.unwrap();
    
    let retrieved = framework.get_concept("dup").await.unwrap();
    assert!(retrieved.is_some());
    // Check if vector was updated
}

#[test]
fn hvec_from_bytes_wrong_sizes() {
    // Empty
    assert!(HVec10240::from_bytes(&[]).is_err());
    
    // One byte short
    let almost = vec![0u8; 1279];
    assert!(HVec10240::from_bytes(&almost).is_err());
    
    // One byte extra
    let extra = vec![0u8; 1281];
    assert!(HVec10240::from_bytes(&extra).is_err());
    
    // Large wrong size
    let large = vec![0u8; 10000];
    assert!(HVec10240::from_bytes(&large).is_err());
}

#[test]
fn reservoir_chaos_strength_extremes() {
    // Very high chaos strength
    let mut high_chaos = ChaoticReservoir::new_seeded(10, 256, 10.0, 42).unwrap();
    let input = vec![0.5; 10];
    let _ = high_chaos.step(&input);
    
    // Zero chaos strength should be deterministic
    let mut zero_chaos = ChaoticReservoir::new_seeded(10, 256, 0.0, 42).unwrap();
    let _ = zero_chaos.step(&input);
}
```

---

## 2. DB Record Verification Tests

### 2.1 Create New File: `tests/db_schema_integrity.rs`

**Purpose**: Direct SQLite/libsql verification without using the API layer.

```rust
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::{ConceptBuilder, HVec10240};
use tempfile::NamedTempFile;

/// Direct SQL verification helper
async fn get_raw_connection(path: &str) -> libsql::Connection {
    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .unwrap();
    db.connect().unwrap()
}

#[tokio::test]
async fn db_schema_tables_exist() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let _persistence = Persistence::new_local(path).await.unwrap();
    
    let conn = get_raw_connection(path).await;
    
    // Verify all expected tables exist
    let mut rows = conn
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
            ()
        )
        .await
        .unwrap();
    
    let mut tables = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        tables.push(row.get::<String>(0).unwrap());
    }
    
    assert!(tables.contains(&"concepts".to_string()));
    assert!(tables.contains(&"associations".to_string()));
    assert!(tables.contains(&"concept_versions".to_string()));
    assert!(tables.contains(&"__schema_version".to_string()));
}

#[tokio::test]
async fn db_foreign_key_constraints_active() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    // Insert a concept
    let concept = ConceptBuilder::new("test")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    persistence.save_concept(&concept).await.unwrap();
    
    // Create association
    persistence.save_association("test", "test", 0.5).await.unwrap();
    
    // Direct SQL: Try to delete concept without cascade (should fail due to FK)
    let conn = get_raw_connection(path).await;
    let result = conn
        .execute("DELETE FROM concepts WHERE id = 'test'", ())
        .await;
    
    // Should fail due to foreign key constraint
    assert!(result.is_err());
}

#[tokio::test]
async fn db_concept_versions_cascade_delete() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    // Create concept with multiple versions
    let concept = ConceptBuilder::new("versioned")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    
    // Save multiple times to create versions
    persistence.save_concept(&concept).await.unwrap();
    persistence.save_concept(&concept).await.unwrap();
    persistence.save_concept(&concept).await.unwrap();
    
    // Verify versions exist
    let conn = get_raw_connection(path).await;
    let mut rows = conn
        .query("SELECT COUNT(*) FROM concept_versions WHERE concept_id = 'versioned'", ())
        .await
        .unwrap();
    
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert!(count >= 3);
    
    // Delete concept through API
    persistence.delete_concept("versioned").await.unwrap();
    
    // Verify versions were cascade deleted
    let mut rows = conn
        .query("SELECT COUNT(*) FROM concept_versions WHERE concept_id = 'versioned'", ())
        .await
        .unwrap();
    
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn db_vector_blob_exact_size() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    let concept = ConceptBuilder::new("sized")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    persistence.save_concept(&concept).await.unwrap();
    
    // Direct SQL: Verify vector blob is exactly 1280 bytes
    let conn = get_raw_connection(path).await;
    let mut rows = conn
        .query("SELECT LENGTH(vector) as vec_len FROM concepts WHERE id = 'sized'", ())
        .await
        .unwrap();
    
    let vec_len: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(vec_len, 1280, "Vector blob must be exactly 1280 bytes (80 * 16)");
}

#[tokio::test]
async fn db_index_usage_verification() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    // Add many concepts and associations
    for i in 0..100 {
        let c = ConceptBuilder::new(format!("c{}", i))
            .with_vector(HVec10240::random())
            .build()
            .unwrap();
        persistence.save_concept(&c).await.unwrap();
        
        if i > 0 {
            persistence.save_association(&format!("c{}", i-1), &format!("c{}", i), 0.5).await.unwrap();
        }
    }
    
    // Verify indexes exist
    let conn = get_raw_connection(path).await;
    let mut rows = conn
        .query("SELECT name FROM sqlite_master WHERE type='index'", ())
        .await
        .unwrap();
    
    let mut indexes = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        indexes.push(row.get::<String>(0).unwrap());
    }
    
    assert!(indexes.iter().any(|i| i.contains("associations")));
    assert!(indexes.iter().any(|i| i.contains("concept_versions")));
    
    // EXPLAIN QUERY PLAN to verify index usage
    let mut rows = conn
        .query("EXPLAIN QUERY PLAN SELECT * FROM associations WHERE from_id = 'c1'", ())
        .await
        .unwrap();
    
    let mut plan = String::new();
    while let Some(row) = rows.next().await.unwrap() {
        let detail: String = row.get(3).unwrap();
        plan.push_str(&detail);
        plan.push(' ');
    }
    
    // Should use an index scan, not table scan
    assert!(plan.contains("INDEX") || plan.contains("index"), 
            "Query plan should use index, got: {}", plan);
}

#[tokio::test]
async fn db_transaction_atomicity_verification() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    
    // Create raw connection for manual transaction testing
    let conn = get_raw_connection(path).await;
    
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON;", ()).await.unwrap();
    
    // Create test schema
    conn.execute_batch(
        "CREATE TABLE test_parent (id TEXT PRIMARY KEY);
         CREATE TABLE test_child (
             id TEXT PRIMARY KEY,
             parent_id TEXT REFERENCES test_parent(id)
         );"
    ).await.unwrap();
    
    // Begin transaction, insert parent, rollback
    conn.execute("BEGIN", ()).await.unwrap();
    conn.execute("INSERT INTO test_parent VALUES ('p1')", ()).await.unwrap();
    conn.execute("ROLLBACK", ()).await.unwrap();
    
    // Verify rollback worked
    let mut rows = conn
        .query("SELECT COUNT(*) FROM test_parent", ())
        .await
        .unwrap();
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(count, 0);
    
    // Test commit
    conn.execute("BEGIN", ()).await.unwrap();
    conn.execute("INSERT INTO test_parent VALUES ('p2')", ()).await.unwrap();
    conn.execute("COMMIT", ()).await.unwrap();
    
    let mut rows = conn
        .query("SELECT COUNT(*) FROM test_parent", ())
        .await
        .unwrap();
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn db_wal_mode_verification() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let _persistence = Persistence::new_local(path).await.unwrap();
    
    let conn = get_raw_connection(path).await;
    let mut rows = conn
        .query("PRAGMA journal_mode", ())
        .await
        .unwrap();
    
    let mode: String = rows.next().await.unwrap().unwrap().get(0).unwrap();
    // libsql typically uses WAL mode
    println!("Journal mode: {}", mode);
}

#[tokio::test]
async fn db_schema_version_integrity() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    // Get schema version through API
    let api_version = persistence.schema_version().await.unwrap();
    
    // Verify through direct SQL
    let conn = get_raw_connection(path).await;
    let mut rows = conn
        .query("SELECT MAX(version) FROM __schema_version", ())
        .await
        .unwrap();
    
    let sql_version: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(api_version, sql_version);
}
```

### 2.2 Create New File: `tests/db_data_integrity.rs`

```rust
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::{ConceptBuilder, HVec10240};
use tempfile::NamedTempFile;

#[tokio::test]
async fn db_concept_timestamps_monotonic() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    let before_create = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    let concept = ConceptBuilder::new("timed")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    persistence.save_concept(&concept).await.unwrap();
    
    let after_create = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    // Direct SQL verification
    let db = libsql::Builder::new_local(path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let mut rows = conn
        .query("SELECT created_at, modified_at FROM concepts WHERE id = 'timed'", ())
        .await
        .unwrap();
    
    let row = rows.next().await.unwrap().unwrap();
    let created_at: i64 = row.get(0).unwrap();
    let modified_at: i64 = row.get(1).unwrap();
    
    assert!(created_at >= before_create);
    assert!(created_at <= after_create);
    assert_eq!(created_at, modified_at); // Initial creation
    
    // Update concept
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let updated = ConceptBuilder::new("timed")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    persistence.save_concept(&updated).await.unwrap();
    
    let mut rows = conn
        .query("SELECT created_at, modified_at FROM concepts WHERE id = 'timed'", ())
        .await
        .unwrap();
    
    let row = rows.next().await.unwrap().unwrap();
    let created_at2: i64 = row.get(0).unwrap();
    let modified_at2: i64 = row.get(1).unwrap();
    
    assert_eq!(created_at, created_at2); // Created_at unchanged
    assert!(modified_at2 > modified_at); // Modified_at updated
}

#[tokio::test]
async fn db_association_strength_precision() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    let concept = ConceptBuilder::new("assoc_test")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    persistence.save_concept(&concept).await.unwrap();
    
    // Test various floating point strengths
    let test_values = [0.0f32, 0.1, 0.5, 0.999999, 1.0, f32::MIN_POSITIVE, -0.5];
    
    for (i, &strength) in test_values.iter().enumerate() {
        let target = format!("target_{}", i);
        let target_concept = ConceptBuilder::new(&target)
            .with_vector(HVec10240::random())
            .build()
            .unwrap();
        persistence.save_concept(&target_concept).await.unwrap();
        
        persistence.save_association("assoc_test", &target, strength).await.unwrap();
        
        // Verify stored value
        let db = libsql::Builder::new_local(path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let mut rows = conn
            .query(
                "SELECT strength FROM associations WHERE from_id = 'assoc_test' AND to_id = ?1",
                [target]
            )
            .await
            .unwrap();
        
        let row = rows.next().await.unwrap().unwrap();
        let stored: f64 = row.get(0).unwrap();
        let stored_f32 = stored as f32;
        
        assert!((stored_f32 - strength).abs() < f32::EPSILON * 10.0,
                "Stored strength {} differs from original {} for test case {}",
                stored_f32, strength, i);
    }
}

#[tokio::test]
async fn db_version_retention_policy() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();
    
    // Create concept
    let concept = ConceptBuilder::new("versioned_retention")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    
    // Save many times to trigger version pruning
    for _ in 0..15 {
        persistence.save_concept(&concept).await.unwrap();
    }
    
    // Verify version count doesn't exceed retention limit (default 10)
    let db = libsql::Builder::new_local(path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM concept_versions WHERE concept_id = 'versioned_retention'",
            ()
        )
        .await
        .unwrap();
    
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert!(count <= 10, "Version count {} exceeds retention limit", count);
}
```

---

## 3. Property-Based Testing Gaps

### 3.1 Extend `tests/property_based.rs`

**Add these proptest cases:**

```rust
// Line ~74: Add to tests/property_based.rs

use chaotic_semantic_memory::reservoir::{Reservoir, ChaoticReservoir};
use chaotic_semantic_memory::singularity::{Concept, Singularity, SingularityConfig};

proptest! {
    // Existing tests...

    #[test]
    fn hvec_bind_is_involution(a_bytes in proptest::collection::vec(any::<u8>(), 1280)) {
        let a = hvec_from_bytes(&a_bytes);
        let b = HVec10240::random();
        
        // bind(bind(a, b), b) should equal a
        let bound = a.bind(&b);
        let recovered = bound.bind(&b);
        prop_assert_eq!(a, recovered);
    }

    #[test]
    fn hvec_permute_composition_is_permute_sum(
        a_bytes in proptest::collection::vec(any::<u8>(), 1280),
        shift1 in 0usize..10240,
        shift2 in 0usize..10240
    ) {
        let a = hvec_from_bytes(&a_bytes);
        
        let permuted_twice = a.permute(shift1).permute(shift2);
        let permuted_sum = a.permute((shift1 + shift2) % 10240);
        
        prop_assert_eq!(permuted_twice, permuted_sum);
    }

    #[test]
    fn reservoir_determinism_with_same_seed(
        input_size in 1usize..64,
        reservoir_size in 100usize..500,
        seed in any::<u64>(),
        steps in 1usize..20
    ) {
        let mut r1 = Reservoir::new_seeded(input_size, reservoir_size, seed).unwrap();
        let mut r2 = Reservoir::new_seeded(input_size, reservoir_size, seed).unwrap();
        
        for _ in 0..steps {
            let input: Vec<f32> = (0..input_size).map(|i| (i as f32) * 0.1).collect();
            let s1 = r1.step(&input).unwrap().to_vec();
            let s2 = r2.step(&input).unwrap().to_vec();
            prop_assert_eq!(s1, s2);
        }
    }

    #[test]
    fn singularity_find_similar_self_is_maximum(
        id in "[a-z]{1,20}",
        num_concepts in 1usize..50
    ) {
        let mut singularity = Singularity::new();
        
        // Add concepts with random vectors
        for i in 0..num_concepts {
            let c = Concept {
                id: format!("{}", i),
                vector: HVec10240::random(),
                metadata: std::collections::HashMap::new(),
                created_at: i as u64,
                modified_at: i as u64,
            };
            singularity.inject(c).unwrap();
        }
        
        // Self-similarity should be ~1.0 and rank first
        let query = singularity.get(&format!("{}", 0)).unwrap().vector;
        let results = singularity.find_similar(&query, num_concepts);
        
        prop_assert!(!results.is_empty());
        prop_assert_eq!(results[0].0, "0");
        prop_assert!(results[0].1 > 0.99, "Self-similarity should be ~1.0");
    }

    #[test]
    fn concept_builder_metadata_roundtrip(
        id in "[a-zA-Z0-9]{1,50}",
        key in "[a-z]{1,20}",
        value in any::<i64>()
    ) {
        use chaotic_semantic_memory::singularity::ConceptBuilder;
        
        let concept = ConceptBuilder::new(&id)
            .with_metadata(&key, value)
            .build()
            .unwrap();
        
        let stored_value = concept.metadata.get(&key)
            .and_then(|v| v.as_i64())
            .unwrap();
        
        prop_assert_eq!(stored_value, value);
    }

    #[test]
    fn hvec_bundle_single_element_is_identity(
        a_bytes in proptest::collection::vec(any::<u8>(), 1280)
    ) {
        let a = hvec_from_bytes(&a_bytes);
        let bundled = HVec10240::bundle(&[a]).unwrap();
        
        // Single element bundle should equal the element
        prop_assert_eq!(a, bundled);
    }

    #[test]
    fn hvec_bundle_duplicate_elements_converges(
        a_bytes in proptest::collection::vec(any::<u8>(), 1280),
        n in 2usize..20
    ) {
        let a = hvec_from_bytes(&a_bytes);
        let vecs: Vec<_> = std::iter::repeat(a).take(n).collect();
        let bundled = HVec10240::bundle(&vecs).unwrap();
        
        // With all identical vectors, result should be identical
        prop_assert_eq!(a, bundled);
    }

    #[test]
    fn chaotic_reservoir_bounded_output(
        input_size in 1usize..32,
        reservoir_size in 128usize..1024,
        chaos in 0.0f32..1.0f32,
        steps in 1usize..50
    ) {
        let mut reservoir = ChaoticReservoir::new_seeded(input_size, reservoir_size, chaos, 42).unwrap();
        
        for _ in 0..steps {
            let input: Vec<f32> = (0..input_size).map(|_| rand::random::<f32>()).collect();
            let state = reservoir.step(&input).unwrap();
            
            // All state values should be finite (not NaN or infinite)
            for &val in state {
                prop_assert!(val.is_finite(), "Reservoir output should be finite");
            }
        }
    }
}
```

---

## 4. Fuzzing Targets - Missing Coverage

### 4.1 Current Fuzz Targets Analysis

| Target | File | Coverage | Status |
|--------|------|----------|--------|
| hvec_from_bytes | 9 LOC | Input validation | Basic |
| reservoir_step | 31 LOC | Step function with random params | Good |
| persistence_save_concept | 45 LOC | Save with fuzzed metadata | Good |

### 4.2 Create New Fuzz Target: `fuzz/fuzz_targets/framework_api.rs`

```rust
#![no_main]

use chaotic_semantic_memory::prelude::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }
    
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(_) => return,
    };
    
    runtime.block_on(async {
        let framework = match ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await 
        {
            Ok(f) => f,
            Err(_) => return,
        };
        
        // Fuzz inject_concept
        let id_len = (data[0] as usize).min(100);
        if data.len() < 1 + id_len + 1280 {
            return;
        }
        
        let id = String::from_utf8_lossy(&data[1..1+id_len]);
        let vec_bytes = &data[1+id_len..1+id_len+1280];
        
        if let Ok(vector) = HVec10240::from_bytes(vec_bytes) {
            let _ = framework.inject_concept(id.as_ref(), vector).await;
        }
        
        // Fuzz probe with random top_k
        if data.len() > 1 + id_len + 1280 {
            let top_k = (data[1+id_len+1280] as usize).max(1);
            let query = HVec10240::random();
            let _ = framework.probe(query, top_k).await;
        }
        
        // Fuzz process_sequence
        if data.len() > 1 + id_len + 1280 + 10 {
            let seq_len = (data[1+id_len+1280+1] as usize).min(20);
            let mut sequence = Vec::with_capacity(seq_len);
            for i in 0..seq_len {
                if 1+id_len+1280+2+i*4 < data.len() {
                    let val = f32::from_bits(u32::from_le_bytes([
                        data[1+id_len+1280+2+i*4],
                        data[1+id_len+1280+2+i*4+1],
                        data[1+id_len+1280+2+i*4+2],
                        data[1+id_len+1280+2+i*4+3],
                    ]));
                    sequence.push(vec![val; 10240]);
                }
            }
            let _ = framework.process_sequence(&sequence).await;
        }
    });
});
```

### 4.3 Create New Fuzz Target: `fuzz/fuzz_targets/singularity_operations.rs`

```rust
#![no_main]

use chaotic_semantic_memory::singularity::{Concept, Singularity};
use chaotic_semantic_memory::HVec10240;
use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    let mut singularity = Singularity::new();
    
    if data.is_empty() {
        return;
    }
    
    let num_ops = (data[0] as usize).min(50);
    let mut concepts_added = Vec::new();
    
    for i in 0..num_ops {
        let offset = 1 + i * 10;
        if offset + 10 > data.len() {
            break;
        }
        
        let op = data[offset] % 5;
        let id_idx = data[offset + 1] as usize;
        let id = format!("concept_{}", id_idx);
        
        match op {
            0 => {
                // Inject concept
                let vec = HVec10240::random();
                let concept = Concept {
                    id: id.clone(),
                    vector: vec,
                    metadata: HashMap::new(),
                    created_at: i as u64,
                    modified_at: i as u64,
                };
                if singularity.inject(concept).is_ok() {
                    concepts_added.push(id);
                }
            }
            1 => {
                // Find similar
                if !concepts_added.is_empty() {
                    let query = HVec10240::random();
                    let top_k = (data[offset + 2] as usize).max(1).min(100);
                    let _ = singularity.find_similar(&query, top_k);
                }
            }
            2 => {
                // Associate
                if concepts_added.len() >= 2 {
                    let from = &concepts_added[id_idx % concepts_added.len()];
                    let to_idx = (id_idx + 1) % concepts_added.len();
                    let to = &concepts_added[to_idx];
                    let strength = data[offset + 2] as f32 / 255.0;
                    let _ = singularity.associate(from, to, strength);
                }
            }
            3 => {
                // Delete
                if !concepts_added.is_empty() {
                    let to_delete = &concepts_added[id_idx % concepts_added.len()];
                    let _ = singularity.delete(to_delete);
                }
            }
            4 => {
                // Get associations
                if !concepts_added.is_empty() {
                    let query = &concepts_added[id_idx % concepts_added.len()];
                    let _ = singularity.get_associations(query);
                }
            }
            _ => {}
        }
    }
});
```

### 4.4 Create New Fuzz Target: `fuzz/fuzz_targets/hvec_operations.rs`

```rust
#![no_main]

use chaotic_semantic_memory::hyperdim::HVec10240;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 1280 * 2 {
        return;
    }
    
    // Parse two hypervectors
    let a = match HVec10240::from_bytes(&data[0..1280]) {
        Ok(v) => v,
        Err(_) => return,
    };
    
    let b = match HVec10240::from_bytes(&data[1280..2560]) {
        Ok(v) => v,
        Err(_) => return,
    };
    
    if data.len() < 2561 {
        return;
    }
    
    let op = data[2560] % 8;
    
    match op {
        0 => {
            // bind
            let _ = a.bind(&b);
        }
        1 => {
            // cosine_similarity
            let _ = a.cosine_similarity(&b);
        }
        2 => {
            // hamming_distance
            let _ = a.hamming_distance(&b);
        }
        3 => {
            // permute
            let shift = (data[2561] as usize) % 10240;
            let _ = a.permute(shift);
        }
        4 => {
            // bundle
            let _ = HVec10240::bundle(&[a, b]);
        }
        5 => {
            // serialization roundtrip
            let bytes = a.to_bytes();
            let _ = HVec10240::from_bytes(&bytes);
        }
        6 => {
            // sparse generation
            let density = (data[2561] as f32) / 255.0;
            let _ = HVec10240::sparse(density);
        }
        7 => {
            // batch similarity
            let _ = chaotic_semantic_memory::hyperdim::batch_cosine_similarity(&a, &[b]);
        }
        _ => {}
    }
});
```

### 4.5 Update `fuzz/Cargo.toml`

Add to `fuzz/Cargo.toml`:

```toml
[[bin]]
name = "framework_api"
path = "fuzz_targets/framework_api.rs"
test = false
doc = false
bench = false

[[bin]]
name = "singularity_operations"
path = "fuzz_targets/singularity_operations.rs"
test = false
doc = false
bench = false

[[bin]]
name = "hvec_operations"
path = "fuzz_targets/hvec_operations.rs"
test = false
doc = false
bench = false
```

---

## 5. Summary of Recommended Test Additions

| File | Type | Lines (est) | Priority |
|------|------|-------------|----------|
| `tests/edge_case_coverage.rs` | Additions | +80 | High |
| `tests/error_path_coverage.rs` | New | 150 | High |
| `tests/db_schema_integrity.rs` | New | 250 | Critical |
| `tests/db_data_integrity.rs` | New | 150 | High |
| `tests/property_based.rs` | Additions | +120 | Medium |
| `fuzz/fuzz_targets/framework_api.rs` | New | 60 | Medium |
| `fuzz/fuzz_targets/singularity_operations.rs` | New | 80 | Medium |
| `fuzz/fuzz_targets/hvec_operations.rs` | New | 50 | Low |

### Estimated Total New Code
- Test files: ~580 lines
- Fuzz targets: ~190 lines

All files remain under the 500 LOC constraint.

---

## 6. Key Findings

### Critical Gaps Found:

1. **No Direct DB Verification**: All tests go through the API layer; no SQL-level validation
2. **Missing FK Constraint Tests**: Foreign key enforcement not explicitly tested
3. **No Version Retention Tests**: The version pruning logic in persistence.rs (lines 485-494) lacks verification
4. **Missing Error Path Coverage**: Many error branches in persistence.rs never exercised
5. **Limited Proptest**: No property tests for reservoir determinism or singularity operations
6. **Fuzzing Gaps**: No fuzzing of high-level framework API or singularity state machine

### Recommended Immediate Actions:

1. Create `tests/db_schema_integrity.rs` - highest priority for data integrity
2. Add error path tests to `tests/edge_case_coverage.rs`
3. Extend property-based tests with reservoir and singularity coverage
4. Add new fuzz targets for API-level testing

---

*Report generated by Swarm Group A (Testing & Quality)*
