#![no_main]

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use chaotic_semantic_memory::HVec10240;
use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic counter used only to derive a unique temp-DB path per call.
/// No cross-call state is shared: each call opens its own throwaway database
/// and removes it before returning.
static DB_SEQ: AtomicU64 = AtomicU64::new(0);

fuzz_target!(|data: &[u8]| {
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(_) => return,
    };

    runtime.block_on(async {
        let id = if data.is_empty() {
            "fuzz-empty".to_string()
        } else {
            format!("fuzz-{:02x}", data[0])
        };

        let raw_metadata = serde_json::from_slice::<serde_json::Value>(data)
            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(data).to_string()));
        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        metadata.insert("fuzz".to_string(), raw_metadata);

        // Per-call database: unique temp path (never a hardcoded shared file),
        // removed after the call so no state leaks between fuzz iterations.
        let seq = DB_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "csm_fuzz_persistence_{}_{}.db",
            std::process::id(),
            seq
        ));
        let Some(path_str) = path.to_str() else {
            return;
        };

        let persistence = match Persistence::new_local(path_str).await {
            Ok(p) => p,
            Err(_) => return,
        };

        let concept = Concept {
            id,
            vector: HVec10240::random(),
            metadata,
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        };

        let _ = persistence.save_concept("default", &concept).await;

        // Close the database and clean up the temp files (incl. SQLite WAL/SHM).
        drop(persistence);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{path_str}-wal"));
        let _ = std::fs::remove_file(format!("{path_str}-shm"));
    });
});
